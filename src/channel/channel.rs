use crate::context::context::get_context;
use core::num;
use std::net::SocketAddr;
use std::sync::{
    Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
    atomic::{AtomicU32, Ordering},
};
use tokio::sync::Notify;

use crate::ca::message::CaMsg;
use log::{debug, error, warn};
use crate::channel::dbr::{ChannelSeverity, ChannelStatus, ChannelState, ChannelAccessRights};
use crate::channel::dbr::{DbrType, DbrValue};

struct ChannelMeta {
    // state
    state: ChannelState,
    access_right: ChannelAccessRights,
    // status, severity, and native dbr_type
    status: ChannelStatus,
    severity: ChannelSeverity,
    dbr_type_native: DbrType,
    // time
    seconds_since_epoch: i32, // the Unix time, not epics time
    nano_seconds: u32,
    // data
    units: String, // 8 C chars
    precision: i16,
    padding: i16,
    // enum
    number_of_string_used: i16,
    strings: [String; 16], // 16 elements, each with up to 26 C chars
    // limits
    upper_display_limit: i16,
    lower_display_limit: i16,
    upper_alarm_limit: i16,
    lower_alarm_limit: i16,
    upper_warning_limit: i16,
    lower_warning_limit: i16,
}

pub struct Channel {
    name: String,
    cid: u32,       // client ID
    sid: AtomicU32, // server ID, assigned after channel created on server
    meta: RwLock<ChannelMeta>,
    value: RwLock<Option<DbrValue>>,
    search_counter: AtomicU32,
    addr: RwLock<Option<SocketAddr>>,
    state_change_notifier: Notify,
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let meta = self.meta();

        f.debug_struct("Channel")
            .field("name", &self.name)
            .field("cid", &self.cid)
            .field("state", &meta.state)
            .field("access_right", &meta.access_right)
            .field("status", &meta.status)
            .field("severity", &meta.severity)
            .field("seconds_since_epoch", &meta.seconds_since_epoch)
            .field("nano_seconds", &meta.nano_seconds)
            .field("units", &meta.units)
            .field("precision", &meta.precision)
            .field("padding", &meta.padding)
            .field("number_of_string_used", &meta.number_of_string_used)
            .field("strings", &meta.strings)
            .field("upper_display_limit", &meta.upper_display_limit)
            .field("lower_display_limit", &meta.lower_display_limit)
            .field("upper_alarm_limit", &meta.upper_alarm_limit)
            .field("lower_alarm_limit", &meta.lower_alarm_limit)
            .field("upper_warning_limit", &meta.upper_warning_limit)
            .field("lower_warning_limit", &meta.lower_warning_limit)
            .finish()
    }
}

impl Channel {
    pub fn new(name: &str, cid: u32) -> Self {
        Channel {
            name: name.to_string(),
            cid: cid,
            sid: AtomicU32::new(0),
            search_counter: AtomicU32::new(1),
            meta: RwLock::new(ChannelMeta {
                state: ChannelState::NeverConnected,
                access_right: ChannelAccessRights::None,
                status: ChannelStatus::NoAlarm,
                severity: ChannelSeverity::NoAlarm,
                dbr_type_native: DbrType::Double,
                seconds_since_epoch: 0,
                nano_seconds: 0,
                units: String::new(),
                precision: 0,
                padding: 0,
                number_of_string_used: 0,
                strings: std::array::from_fn(|_| String::new()),
                upper_display_limit: 0,
                lower_display_limit: 0,
                upper_alarm_limit: 0,
                lower_alarm_limit: 0,
                upper_warning_limit: 0,
                lower_warning_limit: 0,
            }),
            value: RwLock::new(None),
            addr: RwLock::new(None),
            state_change_notifier: Notify::new(),
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

    // ------------------ get/put/monitor --------------

    pub async fn get(self: &Self, dbr_type: DbrType, data_count: u32) {
        // block until state becomes Created
        self.wait_state_change(ChannelState::Created).await;

        let sid = self.sid();
        let cid = self.cid();
        let context = get_context();
        let ioid = context.channels().next_ioid();
        let dest = self.addr();

        match dest {
            Some(dest) => {
                let msg = CaMsg::build_read_notify(dbr_type, data_count, sid, ioid, &vec![dest]);
                let tcp = context.tcps().tcp(&dest);
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
                                        self.update_from_buf(msg.payload(), num_elem, dbr_type);
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

    // ------------------ data -------------------
    pub fn name(&self) -> &str {
        &self.name
    }

    fn meta(&self) -> RwLockReadGuard<'_, ChannelMeta> {
        self.meta.read().unwrap()
    }

    fn meta_mut(&self) -> RwLockWriteGuard<'_, ChannelMeta> {
        self.meta.write().unwrap()
    }

    // ------------- data setter ----------------

    pub fn set_sid(&self, new_sid: u32) {
        self.sid.store(new_sid, Ordering::Relaxed);
    }

    pub fn set_state(&self, new_state: ChannelState) {
        self.meta_mut().state = new_state;
        debug!("state change>>>>>>>>>>>>>>>>>>>>>>>>");
        self.state_change_notifier().notify_waiters();
        debug!("state change>>>>>>>>>>>>>>>>>>>>>>>> 1");
    }

    pub fn set_status(&self, new_status: ChannelStatus) {
        self.meta_mut().status = new_status;
    }

    pub fn set_severity(&self, new_severity: ChannelSeverity) {
        self.meta_mut().severity = new_severity;
    }

    pub fn set_dbr_type_native(&self, new_dbr_type_native: DbrType) {
        self.meta_mut().dbr_type_native = new_dbr_type_native;
    }

    pub fn set_seconds_since_epoch(&self, new_seconds_since_epoch: i32) {
        self.meta_mut().seconds_since_epoch = new_seconds_since_epoch;
    }

    pub fn set_nano_seconds(&self, new_nano_seconds: u32) {
        self.meta_mut().nano_seconds = new_nano_seconds;
    }

    pub fn set_units(&self, new_units: String) {
        self.meta_mut().units = new_units;
    }

    pub fn set_precision(&self, new_precision: i16) {
        self.meta_mut().precision = new_precision;
    }

    pub fn set_padding(&self, new_padding: i16) {
        self.meta_mut().padding = new_padding;
    }

    pub fn set_number_of_string_used(&self, new_number_of_string_used: i16) {
        self.meta_mut().number_of_string_used = new_number_of_string_used;
    }

    pub fn set_strings(&self, new_strings: [String; 16]) {
        self.meta_mut().strings = new_strings;
    }

    pub fn set_upper_display_limit(&self, new_upper_display_limit: i16) {
        self.meta_mut().upper_display_limit = new_upper_display_limit;
    }

    pub fn set_lower_display_limit(&self, new_lower_display_limit: i16) {
        self.meta_mut().lower_display_limit = new_lower_display_limit;
    }

    pub fn set_upper_alarm_limit(&self, new_upper_alarm_limit: i16) {
        self.meta_mut().upper_alarm_limit = new_upper_alarm_limit;
    }

    pub fn set_lower_alarm_limit(&self, new_lower_alarm_limit: i16) {
        self.meta_mut().lower_alarm_limit = new_lower_alarm_limit;
    }

    pub fn set_upper_warning_limit(&self, new_upper_warning_limit: i16) {
        self.meta_mut().upper_warning_limit = new_upper_warning_limit;
    }

    pub fn set_lower_warning_limit(&self, new_lower_warning_limit: i16) {
        self.meta_mut().lower_warning_limit = new_lower_warning_limit;
    }

    pub fn set_access_right(self: &Self, access_right: ChannelAccessRights) {
        self.meta_mut().access_right = access_right;
    }

    pub fn set_addr(self: &Self, new_addr: Option<SocketAddr>) {
        *self.addr.write().unwrap() = new_addr;
    }

    pub fn set_value(&self, new_value: Option<DbrValue>) {
        *self.value.write().unwrap() = new_value;
    }

    // ------------- data getter ----------------

    pub fn state(&self) -> ChannelState {
        self.meta().state
    }

    pub fn value(&self) -> RwLockReadGuard<'_, Option<DbrValue>> {
        self.value.read().unwrap()
    }

    pub fn status(&self) -> ChannelStatus {
        self.meta().status
    }

    pub fn severity(&self) -> ChannelSeverity {
        self.meta().severity
    }

    pub fn dbr_type_native(self: &Self) -> DbrType {
        self.meta().dbr_type_native
    }

    pub fn seconds_since_epoch(&self) -> i32 {
        self.meta().seconds_since_epoch
    }

    pub fn nano_seconds(&self) -> u32 {
        self.meta().nano_seconds
    }

    pub fn units(&self) -> String {
        self.meta().units.clone()
    }

    pub fn precision(&self) -> i16 {
        self.meta().precision
    }

    pub fn padding(&self) -> i16 {
        self.meta().padding
    }

    pub fn number_of_string_used(&self) -> i16 {
        self.meta().number_of_string_used
    }

    pub fn strings(&self) -> [String; 16] {
        self.meta().strings.clone()
    }

    pub fn upper_display_limit(&self) -> i16 {
        self.meta().upper_display_limit
    }

    pub fn lower_display_limit(&self) -> i16 {
        self.meta().lower_display_limit
    }

    pub fn upper_alarm_limit(&self) -> i16 {
        self.meta().upper_alarm_limit
    }

    pub fn lower_alarm_limit(&self) -> i16 {
        self.meta().lower_alarm_limit
    }

    pub fn upper_warning_limit(&self) -> i16 {
        self.meta().upper_warning_limit
    }

    pub fn lower_warning_limit(&self) -> i16 {
        self.meta().lower_warning_limit
    }

    pub fn cid(&self) -> u32 {
        self.cid
    }

    pub fn sid(&self) -> u32 {
        self.sid.load(Ordering::Relaxed)
    }

    pub fn increment_search_counter(&self) -> u32 {
        self.search_counter.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn reset_search_counter(&self) -> u32 {
        self.search_counter.swap(0, Ordering::Relaxed)
    }

    pub fn search_counter(&self) -> u32 {
        self.search_counter.load(Ordering::Relaxed)
    }

    pub fn addr(self: &Self) -> Option<SocketAddr> {
        *self.addr.read().unwrap()
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
}
