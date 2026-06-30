use crate::ca;
use crate::ca::message::CaMsg;
use crate::channel::dbr::{ChannelAccessRights, ChannelSeverity, ChannelState, ChannelStatus};
use crate::channel::dbr::{DbrType, DbrValue};
use crate::channel::meta::Meta;
use crate::channel::monitor::{self, Monitor, MonitorCallback, MonitorState};
use crate::context::context::get_context;
use crate::tcp::tcp::TCP;
use core::num;
use log::{debug, error, warn};
use std::net::SocketAddr;
use std::sync::{
    Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
    atomic::{AtomicU32, Ordering},
};
use tokio::sync::Notify;

pub struct Channel {
    name: String,
    cid: u32,       // client ID
    sid: AtomicU32, // server ID, assigned after channel created on server
    meta: Arc<Meta>,
    value: RwLock<Option<DbrValue>>,
    search_counter: AtomicU32,
    addr: RwLock<Option<SocketAddr>>,
    state_change_notifier: Notify,
    monitor: Arc<Monitor>,
}

impl Channel {
    pub fn new(name: &str, cid: u32) -> Self {
        Channel {
            name: name.to_string(),
            cid: cid,
            sid: AtomicU32::new(0),
            search_counter: AtomicU32::new(1),
            meta: Meta::new(),
            value: RwLock::new(None),
            addr: RwLock::new(None),
            state_change_notifier: Notify::new(),
            monitor: Monitor::new(),
        }
    }

    /**
     * Connect to the server tcp if this channel is not connected.
     *  - connect tcp if not connected yet
     *  - correlate Channel and TCP
     *  - send out CA_PROTO_VERSION, CA_PROTO_CLIENT_NAME, CA_PROTO_HOST_NAME to tcp
     */
    pub async fn connect(self: &Self, addr: SocketAddr) {
        let state = self.state();
        if state != ChannelState::NameFound {
            error!("Channel must be in NameFound state to connect tcp");
            return;
        }

        // find TCP
        let context = get_context();
        let tcps = context.tcps();
        let tcp = tcps.tcp(&addr);
        let cid = self.cid();
        match tcp {
            Some(tcp) => {
                self.set_state(ChannelState::TcpConnected, true);
                // add this channel to TCP
                tcp.add_cid(cid);
                // assign TCP to this channel
                self.set_addr(Some(addr));
                // send handshake messages
                self.send_handshake().await;
            }
            None => {
                // connect this tcp address
                let tcps = get_context().tcps();
                let tcp = tcps.create_tcp(addr).await;
                self.set_state(ChannelState::TcpConnected, true);
                // assign this channel to TCP
                match tcp {
                    Ok(tcp) => {
                        Arc::clone(&tcp).start_to_listen().await;
                        // add this channel to TCP
                        tcp.add_cid(cid);
                        // assign to this channel
                        self.set_addr(Some(addr));
                        // send handshake messages
                        self.send_handshake().await;
                    }
                    Err(_) => {
                        // tcp connection failed
                        self.set_state(ChannelState::NameSearching, true);
                    }
                }
            }
        }
    }

    pub async fn send_handshake(self: &Self) {
        let dest = self.addr();

        match dest {
            Some(dest) => {
                let context = get_context();
                let tcp = context.tcps().tcp(&dest);
                match tcp {
                    Some(tcp) => {
                        let dests = vec![dest];
                        let version_msg = CaMsg::build_version(&dests);
                        let client_name_msg = match CaMsg::build_client_name(&dests) {
                            Ok(msg) => msg,
                            Err(_) => return,
                        };
                        let host_name_msg = match CaMsg::build_host_name(&dests) {
                            Ok(msg) => msg,
                            Err(_) => return,
                        };
                        let create_chan_msg =
                            match CaMsg::build_create_chan(self.name(), self.cid(), &dests) {
                                Ok(msg) => msg,
                                Err(_) => return,
                            };

                        match tcp
                            .send_msgs(vec![
                                version_msg,
                                client_name_msg,
                                host_name_msg,
                                create_chan_msg,
                            ])
                            .await
                        {
                            Ok(_) => {}
                            Err(error) => {}
                        };
                    }
                    None => {}
                }
            }
            None => {}
        }
    }

    pub async fn destroy(self: &Self) {
        let context = get_context();
        let channels = context.channels();

        let addr: Option<SocketAddr> = self.addr();
        let sid = self.sid();
        let cid = self.cid();
        let had_monitor = !(self.monitor().state() == MonitorState::NotRunning);

        // Reset all data, clear IO, 
        // since we are destroying the channel, no need to notify the state change
        self.reset(false);

        // Mark state as Destroyed and notify waiters.
        self.set_state(ChannelState::Destroyed, true);

        // Cancel monitor if there is one
        if had_monitor {
            self.cancel_monitor(MonitorState::NotRunning, sid, cid, addr).await;
        }

        match addr {
            Some(addr) => {
                let tcp = get_context().tcps().tcp(&addr);
                match tcp {
                    Some(tcp) => {
                        // Tell server to clear channel: Send CA_PROTO_CLEAR_CHANNEL
                        let msg = CaMsg::build_clear_channel(sid, cid, &vec![addr]);
                        match tcp.send_msgs(vec![msg]).await {
                            Ok(_) => {}
                            Err(error) => {}
                        };
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
        self.set_addr(None);

        // Remove from Channels.by_name and Channels.by_cid.
        channels.remove_by_cid_channel(self.cid());
        channels.remove_by_name_channel(self.name().to_string());
    }

    /**
     * If there is unreconverable error in TCP, the TCP should already have been destroyed
     *
     * It is similar to destroy, but the channel's registration in Context is kept
     */
    pub async fn reconnect(self: &Self) {
        debug!("Reconnecting channel {}", self.name());

        let had_monitor = !(self.monitor().state() == MonitorState::NotRunning);
        let sid = self.sid();
        let subid = self.cid();
        let addr = self.addr();

        // Set monitor state, no need to notify the server because in this case the TCP is already broken
        self.reset(true);
        println!("{:?}", self.state());

        // cancel monitor if there is a monitor
        if had_monitor {
            self.cancel_monitor(MonitorState::Reconnecting, sid, subid, addr).await;
        }
    }

    /**
     * Reset channel's meta data, and clear the IOs.
     * 
     * Note: it sets channel's state to NameSearching
     *       it does not reset monitor's data
     */
    pub fn reset(self: &Self, notify_state: bool) {
        self.set_sid(0);
        self.set_search_counter(1);
        self.meta().reset();
        self.set_value(None);
        self.set_addr(None);
        // if monitor_state == MonitorState::NotRunning {
        //     self.monitor().set_state(monitor_state);
        // } else if monitor_state == MonitorState::Reconnecting
        //     && self.monitor_state() != MonitorState::NotRunning
        // {
        //     self.monitor().set_state(monitor_state);
        // }
        get_context().channels().remove_io_by_cid(self.cid());
        self.set_state(ChannelState::NameSearching, notify_state);
    }

    // ------------------ get/put/monitor --------------

    pub async fn get(self: &Self, dbr_type: Option<DbrType>, data_count: Option<u32>) {
        // block until state becomes Created
        self.wait_state_change(ChannelState::Created).await;

        let sid = self.sid();
        let cid = self.cid();
        let context = get_context();
        let ioid = context.channels().next_ioid();
        let dest = self.addr();

        let dbr_type = {
            match dbr_type {
                Some(dbr_type) => dbr_type,
                None => self.dbr_type_native(),
            }
        };

        let data_count = {
            match data_count {
                Some(data_count) => data_count,
                None => self.data_count_native(),
            }
        };

        match dest {
            Some(dest) => {
                let msg = CaMsg::build_read_notify(dbr_type, data_count, sid, ioid, &vec![dest]);
                let tcp: Option<Arc<crate::tcp::tcp::TCP>> = context.tcps().tcp(&dest);
                match tcp {
                    Some(tcp) => {
                        let (tx, rx) = tokio::sync::oneshot::channel::<CaMsg>();
                        get_context().channels().add_io(ioid, tx, cid);
                        // send out CA_PROTO_READ_NOTIFY
                        match tcp.send_msgs(vec![msg]).await {
                            Ok(_) => {}
                            Err(error) => {}
                        };
                        let msg = rx.await;
                        match msg {
                            Ok(msg) => {
                                let num_elem = msg.header().data_count;
                                let dbr_type_num = msg.header().data_type;
                                let dbr_type = DbrType::from_u16(dbr_type_num);
                                match dbr_type {
                                    Some(dbr_type) => {
                                        self.update_from_payload_buf(
                                            msg.payload(),
                                            num_elem,
                                            dbr_type,
                                        );
                                    }
                                    None => {}
                                }
                            }
                            Err(_) => {}
                        }
                    }
                    None => {}
                }
                // "blocks" until get the CA_PROTO_READ_NOTIFY reply
            }
            None => {}
        }
    }

    pub async fn start_to_monitor(
        self: &Self,
        dbr_type: Option<DbrType>,
        data_count: Option<u32>,
        callback: Option<MonitorCallback>,
    ) {
        // Monitor must be either in NotRunning or Reconnecting state
        if self.monitor().state() != MonitorState::NotRunning
            && self.monitor().state() != MonitorState::Reconnecting
        {
            return;
        }

        // Wait for Channel state becomes Created
        self.wait_state_change(ChannelState::Created).await;

        let dbr_type = match dbr_type {
            Some(dbr_type) => dbr_type,
            None => {
                if self.monitor().state() == MonitorState::NotRunning {
                    self.dbr_type_native()
                } else {
                    self.monitor_data_type()
                }
            }
        };
        let data_count = match data_count {
            Some(data_count) => data_count,
            None => {
                if self.monitor().state() == MonitorState::NotRunning {
                    self.data_count_native()
                } else {
                    self.monitor_data_count()
                }
            }
        };

        let callback = match callback {
            Some(callback) => Some(callback),
            None => {
                if self.monitor().state() == MonitorState::NotRunning {
                    None
                } else {
                    self.monitor_callback()
                }
            }
        };

        let sid = self.sid();
        let subid = self.cid();
        let dest = self.addr();
        let context = get_context();

        match dest {
            Some(dest) => {
                let msg: CaMsg =
                    CaMsg::build_event_add(dbr_type, data_count, sid, subid, &vec![dest]);
                let tcp: Option<Arc<crate::tcp::tcp::TCP>> = context.tcps().tcp(&dest);
                match tcp {
                    Some(tcp) => {
                        // set monitor's callback
                        self.set_monitor_callback(callback);
                        // set monitor's state to Starting
                        self.set_monitor_state(MonitorState::Starting);
                        // tell server to start the monitor: send out CA_PROTO_EVENT_ADD
                        match tcp.send_msgs(vec![msg]).await {
                            Ok(_) => {}
                            Err(error) => {}
                        };
                    }
                    None => {}
                }
            }
            None => {}
        }
    }

    /**
     * Reset monitor data, and tell server to cancel the monitor
     */
    pub async fn cancel_monitor(self: &Self, new_state: MonitorState, sid: u32, subid: u32, dest: Option<SocketAddr>) {
        if self.monitor_state() == MonitorState::NotRunning
            || self.monitor_state() == MonitorState::Reconnecting
        {
            // already stopped or reconnecting
            return;
        }

        // set monitor's new state: NotRunning (completely stop), or Reconnecting (reconnect)
        if new_state == MonitorState::NotRunning || new_state == MonitorState::Reconnecting {
            self.monitor().set_state(new_state);
        } else {
            // should not be in this state
            return;
        }

        let dbr_type = self.monitor_data_type();
        let data_count = self.monitor_data_count();
        let context = get_context();
        match dest {
            Some(dest) => {
                let msg: CaMsg =
                    CaMsg::build_event_cancel(dbr_type, data_count, sid, subid, &vec![dest]);
                let tcp: Option<Arc<crate::tcp::tcp::TCP>> = context.tcps().tcp(&dest);
                match tcp {
                    Some(tcp) => {
                        // tell server to release resource: send out CA_PROTO_EVENT_CANCEL
                        match tcp.send_msgs(vec![msg]).await {
                            Ok(_) => {}
                            Err(error) => {}
                        };
                    }
                    None => {}
                }
            }
            None => {}
        }
    }

    // ------------- data setter ----------------

    pub fn set_sid(&self, new_sid: u32) {
        self.sid.store(new_sid, Ordering::Relaxed);
    }

    pub fn set_addr(self: &Self, new_addr: Option<SocketAddr>) {
        *self.addr.write().unwrap() = new_addr;
    }

    pub fn set_value(&self, new_value: Option<DbrValue>) {
        *self.value.write().unwrap() = new_value;
    }

    pub fn increment_search_counter(&self) -> u32 {
        self.search_counter.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn reset_search_counter(&self) -> u32 {
        self.search_counter.swap(0, Ordering::Relaxed)
    }

    pub fn set_search_counter(&self, counter: u32) {
        self.search_counter.store(counter, Ordering::Relaxed);
    }

    pub fn reset_sid(self: &Self) -> u32 {
        self.sid.swap(0, Ordering::Relaxed)
    }

    pub fn reset_meta(self: &Self) {
        self.meta().reset();
    }

    pub fn reset_value(self: &Self) {
        self.set_value(None);
    }

    // --------------- data getter ---------------------

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn meta(&self) -> Arc<Meta> {
        self.meta.clone()
    }

    pub fn monitor(self: &Self) -> Arc<Monitor> {
        self.monitor.clone()
    }

    pub fn value(&self) -> RwLockReadGuard<'_, Option<DbrValue>> {
        self.value.read().unwrap()
    }

    pub fn cid(&self) -> u32 {
        self.cid
    }

    pub fn sid(&self) -> u32 {
        self.sid.load(Ordering::Relaxed)
    }

    pub fn search_counter(&self) -> u32 {
        self.search_counter.load(Ordering::Relaxed)
    }

    pub fn addr(self: &Self) -> Option<SocketAddr> {
        self.addr.read().unwrap().clone()
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

    pub fn call_monitor_callback(self: &Self) {
        if let Some(callback) = self.monitor_callback().clone() {
            callback(self);
        } else {
            // no callback
        }
    }
}
impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let meta = self.meta.as_ref().to_string().replace('\n', "\n    ");
        let monitor = self.monitor.as_ref().to_string().replace('\n', "\n    ");

        writeln!(f, "\nChannel {{")?;
        writeln!(f, "    name: {:?},", self.name)?;
        writeln!(f, "    cid: {},", self.cid)?;
        writeln!(f, "    sid: {},", self.sid())?;
        writeln!(f, "    meta: {},", meta)?;
        writeln!(f, "    value: {:?},", self.value.read().unwrap().as_ref())?;
        writeln!(f, "    search_counter: {},", self.search_counter())?;
        writeln!(f, "    addr: {:?},", self.addr())?;
        writeln!(f, "    monitor: {}", monitor)?;
        write!(f, "}}")
    }
}
