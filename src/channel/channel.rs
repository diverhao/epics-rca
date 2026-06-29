use crate::ca::message::CaMsg;
use crate::channel::dbr::{ChannelAccessRights, ChannelSeverity, ChannelState, ChannelStatus};
use crate::channel::dbr::{DbrType, DbrValue};
use crate::channel::meta::Meta;
use crate::channel::monitor::{Monitor, MonitorCallback, MonitorState};
use crate::context::context::get_context;
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
            meta: Arc::new(Meta {
                state: RwLock::new(ChannelState::NeverConnected),
                access_right: RwLock::new(ChannelAccessRights::None),
                status: RwLock::new(ChannelStatus::NoAlarm),
                severity: RwLock::new(ChannelSeverity::NoAlarm),
                dbr_type_native: RwLock::new(DbrType::Double),
                data_count_native: RwLock::new(0),
                seconds_since_epoch: RwLock::new(0),
                nano_seconds: RwLock::new(0),
                units: RwLock::new(String::new()),
                precision: RwLock::new(0),
                padding: RwLock::new(0),
                number_of_string_used: RwLock::new(0),
                strings: RwLock::new(std::array::from_fn(|_| String::new())),
                upper_display_limit: RwLock::new(0),
                lower_display_limit: RwLock::new(0),
                upper_alarm_limit: RwLock::new(0),
                lower_alarm_limit: RwLock::new(0),
                upper_warning_limit: RwLock::new(0),
                lower_warning_limit: RwLock::new(0),
            }),
            value: RwLock::new(None),
            addr: RwLock::new(None),
            state_change_notifier: Notify::new(),
            monitor: Arc::new(Monitor {
                state: RwLock::new(MonitorState::NotRunning),
                dbr_type: RwLock::new(DbrType::Double),
                data_count: AtomicU32::new(0),
                callback: RwLock::new(None),
            }),
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
                self.set_state(ChannelState::TcpConnected);
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
                self.set_state(ChannelState::TcpConnected);
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
                        self.set_state(ChannelState::NameSearching);
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

                        tcp.send_msgs(vec![
                            version_msg,
                            client_name_msg,
                            host_name_msg,
                            create_chan_msg,
                        ])
                        .await;
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

        // Mark state as Destroyed and notify waiters.
        self.set_state(ChannelState::Destroyed);
        self.state_change_notifier().notify_waiters();

        // Cancel monitor
        self.cancel_monitor().await;

        // Remove pending ios entries for this cid.
        get_context().channels().remove_io_by_cid(self.cid());

        let addr = self.addr();
        match addr {
            Some(addr) => {
                let tcp = get_context().tcps().tcp(&addr);
                match tcp {
                    Some(tcp) => {
                        // Send CA_PROTO_CLEAR_CHANNEL if sid != 0.
                        let msg = CaMsg::build_clear_channel(self.sid(), self.cid(), &vec![addr]);
                        tcp.send_msgs(vec![msg]).await;
                        // Remove cid from the associated TCP.
                        tcp.remove_cid(self.cid());
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
                        tcp.send_msgs(vec![msg]).await;
                        let msg = rx.await;
                        match msg {
                            Ok(msg) => {
                                // todo: decode the payload!!
                                debug!("{msg}");
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
        // block until state becomes Created
        self.wait_state_change(ChannelState::Created).await;

        let dbr_type = match dbr_type {
            Some(dbr_type) => dbr_type,
            None => self.dbr_type_native(),
        };

        let data_count = match data_count {
            Some(data_count) => data_count,
            None => self.data_count_native(),
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
                        self.set_monitor_state(MonitorState::Starting);
                        // send out CA_PROTO_EVENT_ADD
                        tcp.send_msgs(vec![msg]).await;
                    }
                    None => {}
                }
            }
            None => {}
        }
    }

    pub async fn cancel_monitor(self: &Self) {
        if self.monitor_state() == MonitorState::NotRunning {
            // already stopped
            return;
        }
        // clear the monitor related stuff
        self.set_monitor_callback(None);
        self.set_monitor_state(MonitorState::NotRunning);

        // send CA_PROTO_EVENT_CANCEL
        let dbr_type = self.monitor_data_type();
        let data_count = self.monitor_data_count();

        let sid = self.sid();
        let subid = self.cid();
        let dest = self.addr();
        let context = get_context();

        match dest {
            Some(dest) => {
                let msg: CaMsg =
                    CaMsg::build_event_cancel(dbr_type, data_count, sid, subid, &vec![dest]);
                let tcp: Option<Arc<crate::tcp::tcp::TCP>> = context.tcps().tcp(&dest);
                match tcp {
                    Some(tcp) => {
                        // send out CA_PROTO_EVENT_CANCEL
                        tcp.send_msgs(vec![msg]).await;
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
