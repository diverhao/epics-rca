use crate::ca::message::CaMsg;
use crate::channel::dbr::{ChannelAccessRights, ChannelSeverity, ChannelState, ChannelStatus};
use crate::channel::dbr::{DbrType, DbrValue};
use crate::channel::dbr_data::{self, DbrData};
use crate::channel::meta::Meta;
use crate::channel::monitor::{self, Monitor, MonitorDataType, MonitorState};
use crate::context::context::get_context;
use crate::tcp::tcp::TCP;
use crate::{ca, channel};
use core::num;
use log::{debug, error, warn};
use std::net::SocketAddr;
use std::sync::{
    Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
    atomic::{AtomicU32, Ordering},
};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::timeout;

// channel monitor callback function
pub type ChannelCallback = Arc<dyn Fn(u32, DbrType, u32, &DbrData) + Send + Sync + 'static>;

pub struct Channel {
    // fixed, never change
    name: String,
    cid: u32,       // client ID
    // dynamic data
    // from server, sid, access right, data count, data type, 
    // search: state, server address
    meta: RwLock<Meta>,
    search_counter: AtomicU32,
    state_change_notifier: Notify,
    monitor: RwLock<Monitor>,
}

impl Channel {
    pub fn new(name: &str, cid: u32) -> Self {
        Channel {
            name: name.to_string(),
            cid: cid,
            search_counter: AtomicU32::new(1),
            meta: RwLock::new(Meta::new()),
            state_change_notifier: Notify::new(),
            monitor: RwLock::new(Monitor::new()),
        }
    }

    /**
     * Connect the channel
     *  - connect tcp if not connected yet
     *  - set up relationship between Channel and TCP
     *  - send out handshake packets: CA_PROTO_VERSION, CA_PROTO_CLIENT_NAME, CA_PROTO_HOST_NAME
     */

    pub async fn connect(self: &Self, addr: SocketAddr) {
        let state = self.state();
        // must be in NameFound state
        if state != ChannelState::NameFound {
            error!("Channel must be in NameFound state to connect tcp");
            return;
        }

        let cid = self.cid();
        let context = get_context();
        let tcps = context.tcps();

        // create TCP (if not exist) or get TCP (if already exist)
        let tcp = tcps.create_tcp(addr).await;

        // failed to create TCP: this TCP is automatically disgarded, then reconnect the channel
        let tcp = match tcp {
            Ok(tcp) => tcp,
            Err(_) => {
                // The TCP is not created, simply reconnect
                self.reconnect().await;
                return;
            }
        };

        // During the creat_tcp().await, the channel may have been Destroyed,
        // or reconnected (in NameSearching state), ensure we are on the right track
        if self.state() != ChannelState::NameFound {
            if tcp.cids().len() == 0 {
                tcp.disconnect(true, true).await;
            }
            return;
        }

        self.set_state(ChannelState::TcpConnected, true);
        // add this channel to TCP
        tcp.add_cid(cid);
        // assign TCP to this channel
        self.set_addr(Some(addr));
        // send handshake messages
        self.send_connect_chan();
    }

    pub fn send_connect_chan(self: &Self) {
        let dest = match self.addr() {
            Some(dest) => dest,
            None => return,
        };

        let context = get_context();
        let tcp = match context.tcps().tcp(&dest) {
            Some(tcp) => tcp,
            None => return,
        };

        let dests = vec![dest];
        let version_msg = CaMsg::build_version(&dests);
        let client_name_msg = CaMsg::build_client_name(&dests);
        let host_name_msg = CaMsg::build_host_name(&dests);
        let create_chan_msg = CaMsg::build_create_chan(self.name(), self.cid(), &dests);

        tcp.send_msgs(vec![
            // version_msg,
            // client_name_msg,
            // host_name_msg,
            create_chan_msg,
        ]);
        //     .await
        // {
        //     Ok(_) => {}
        //     Err(error) => {
        //         // reconnect channel, TCP's lifecycle is handled by its check alive task
        //         self.reconnect().await;
        //     }
        // };
    }

    /**
     * Destroy or reconnect the channel
     *
     * Do not assume the TCP is destroyed or broken
     *
     * It does not destroy/disconnect the TCP
     */
    async fn destroy_chan(self: &Self, reconnect: bool) {
        debug!("Destroying the channel");

        let context = get_context();
        let channels = context.channels();

        // Get current states for later use
        let addr: Option<SocketAddr> = self.addr();
        let sid = self.sid();
        let cid = self.cid();
        let had_monitor = !(self.monitor().state() == MonitorState::NotRunning);

        // Reset all data, clear IO,
        self.reset();

        if reconnect {
            self.set_state(ChannelState::NameSearching, true);
        } else {
            self.set_state(ChannelState::Destroyed, true);
        }

        // Cancel monitor if there is one
        if had_monitor {
            self.cancel_monitor(sid, cid, addr);
            if reconnect {
                // If reconnect, start the monitor
                self.set_monitor_state(MonitorState::Starting);
            }
        }

        // send out CA_PROTO_CLEAR_CHANNEL, if TCP still exists in TCPs
        match addr {
            Some(addr) => {
                let tcp = get_context().tcps().tcp(&addr);
                match tcp {
                    Some(tcp) => {
                        // Tell server to clear channel: Send CA_PROTO_CLEAR_CHANNEL
                        let msg = CaMsg::build_clear_channel(sid, cid, &vec![addr]);
                        tcp.send_msgs(vec![msg]);
                        // .await {
                        //     Ok(_) => {}
                        //     Err(error) => {}
                        // };
                        // Remove cid from the associated TCP.
                        tcp.remove_cid(cid);
                        // if TCP has no channel, disconnect it
                        if tcp.cids().len() == 0 {
                            tcp.disconnect(true, true).await;
                        }
                    }
                    None => {}
                }
            }
            None => {}
        }

        // Clear addr.
        // todo: why? the reset already
        // self.set_addr(None);

        // Remove from Channels.by_cid.
        if !reconnect {
            channels.remove_by_cid(self.cid());
        }
    }

    /**
     * Reconnect the channel
     *
     * Do not assume the TCP is destroyed or broken
     *
     * It does not destroy/disconnect the TCP
     */
    pub async fn reconnect(self: &Self) {
        self.destroy_chan(true).await;
    }

    /**
     * Destroy the channel
     *
     * Do not assume the TCP is destroyed or broken
     *
     * It does not destroy/disconnect the TCP
     */
    pub async fn destroy(self: &Self) {
        self.destroy_chan(false).await;
    }

    /**
     * Reset channel's meta data, and clear the IOs.
     *
     * Note: it sets channel's state to NameSearching
     *       it does not reset monitor's data
     */
    pub fn reset(self: &Self) {
        self.set_sid(0);
        self.set_search_counter(1);
        self.meta_mut().reset();
        self.set_addr(None);
        self.set_addr(None);
        get_context().channels().remove_io_by_cid(self.cid());
        self.set_state(ChannelState::NameSearching, false);
    }

    // ------------------ get/put --------------

    // todo: return a value
    pub fn get(
        self: &Self,
        timeout_sec: Option<f64>,
        data_type: Option<MonitorDataType>,
        data_count: Option<u32>,
        callback: Option<ChannelCallback>,
    ) {
        let context = get_context();
        let ioid: u32 = context.channels().next_ioid();
        let cid = self.cid();

        let timeout_sec = {
            match timeout_sec {
                Some(timeout_sec) => timeout_sec,
                None => 1_000_000_000.0, // 31 years is long enough for a CA client
            }
        };

        let state = self.state();
        if state != ChannelState::Created {
            // append IO
            get_context()
                .channels()
                .add_io(ioid, cid, data_type, data_count, callback);
            return;
        }
        self.get_step_2();
    }

    //
    /**
     * Invoked after CA_PROTO_READ_NOTIFY message
     *
     * Update channel value, call callback, then return the value
     */
    pub fn get_step_3(self: &Self, msg: CaMsg) -> Result<DbrData, String> {
        let ioid = msg.header().param2;
        let cid = self.cid();
        let data_count = msg.header().data_count;
        let data_type = match DbrType::from_u16(msg.header().data_type) {
            Some(data_type) => data_type,
            None => {
                // remove this IO
                get_context().channels().remove_io_by_ioid(ioid);
                return Err("Message error: no data type".to_string());
            }
        };

        let dbr_data = DbrData::from_buf(msg.payload(), data_type, data_count);

        // remove and get IO
        let io = match get_context().channels().remove_io_by_ioid(ioid) {
            Some(io) => io,
            None => return Err("No IO ID".to_string()), // no side effect, just return
        };

        match dbr_data {
            Ok(dbr_data) => {
                match io.callback {
                    Some(callback) => {
                        callback(cid, data_type, data_count, &dbr_data);
                    },
                    None => {},
                }
                // todo: return dbr_data first, then call callback
                return Ok(dbr_data);
            }
            Err(_) => {
                return Err("No dbr data".to_string());
            }
        }
    }

    // invoked after channel is Created
    // find all IOs for this channel, each sends CA_PROTO_READ_NOTIFY
    pub fn get_step_2(self: &Self) {
        // get all IOs of this channel
        let cid = self.cid();
        let ios = get_context().channels().ios_of_cid(cid);

        for (ioid, io) in ios {
            let sid = self.sid();

            let dest = match self.addr() {
                Some(addr) => addr,
                None => return, // let TCP check-alive handle it
            };

            let data_type = {
                match io.data_type {
                    Some(data_type) => data_type.resolve(self),
                    None => self.data_type_native(),
                }
            };

            let data_count = {
                match io.data_count {
                    Some(data_count) => data_count,
                    None => self.data_count_native(),
                }
            };

            let msg = CaMsg::build_read_notify(data_type, data_count, sid, ioid, &vec![dest]);
            let tcp: Arc<TCP> = match get_context().tcps().tcp(&dest) {
                Some(tcp) => tcp,
                None => return, // no such TCP, let TCP alive check handle it
            };

            // send out CA_PROTO_READ_NOTIFY
            tcp.send_msgs(vec![msg]);
            // .await {
            //     Ok(_) => {}
            //     Err(error) => {
            //         return; // let alive check handle the TCP issue
            //     }
            // };
        }
    }

    // ------------- data setter ----------------


    pub fn increment_search_counter(&self) -> u32 {
        self.search_counter.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn reset_search_counter(&self) -> u32 {
        self.search_counter.swap(0, Ordering::Relaxed)
    }

    pub fn set_search_counter(&self, counter: u32) {
        self.search_counter.store(counter, Ordering::Relaxed);
    }

    pub fn reset_meta(self: &Self) {
        self.meta_mut().reset();
    }

    // --------------- data getter ---------------------

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn meta(&self) -> RwLockReadGuard<'_, Meta> {
        self.meta.read().unwrap()
    }

    pub fn meta_mut(&self) -> RwLockWriteGuard<'_, Meta> {
        self.meta.write().unwrap()
    }

    pub fn monitor(self: &Self) -> RwLockReadGuard<'_, Monitor> {
        self.monitor.read().unwrap()
    }

    pub fn monitor_mut(self: &Self) -> RwLockWriteGuard<'_, Monitor> {
        self.monitor.write().unwrap()
    }

    pub fn cid(&self) -> u32 {
        self.cid
    }

    pub fn search_counter(&self) -> u32 {
        self.search_counter.load(Ordering::Relaxed)
    }

    pub fn state_change_notifier(self: &Self) -> &Notify {
        &self.state_change_notifier
    }

    pub fn tcp(self: &Self) -> Option<Arc<TCP>> {
        let addr = self.addr();
        match addr {
            Some(addr) => {
                let tcp = get_context().tcps().tcp(&addr);
                match tcp {
                    Some(tcp) => Some(tcp),
                    None => None,
                }
            }
            None => None,
        }
    }

    // ------------- event ----------------

    /**
     * Wait for the channel state change.
     *
     * Note: This method does not have exit mechanism by itself. It is only used in self.get().
     *       Use it with cautious.
     */
    async fn wait_state_change(self: &Self, state: ChannelState) {
        loop {
            let notified = self.state_change_notifier().notified();

            match self.state() {
                current_state if current_state == state => {
                    return;
                }
                _ => notified.await,
            }
        }
    }
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let meta = self.meta().to_string().replace('\n', "\n    ");
        let monitor = self.monitor().to_string().replace('\n', "\n    ");

        writeln!(f, "\nChannel {{")?;
        writeln!(f, "    name: {:?},", self.name)?;
        writeln!(f, "    cid: {},", self.cid)?;
        writeln!(f, "    sid: {},", self.sid())?;
        writeln!(f, "    meta: {},", meta)?;
        writeln!(f, "    search_counter: {},", self.search_counter())?;
        writeln!(f, "    addr: {:?},", self.addr())?;
        writeln!(f, "    monitor: {}", monitor)?;
        write!(f, "}}")
    }
}
