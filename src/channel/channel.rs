use crate::context::context::get_context;
use std::net::SocketAddr;
use std::sync::{
    Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
    atomic::{AtomicU32, Ordering},
};

use crate::ca::message::CaMsg;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ChannelState {
    NeverConnected, // initial state
    NameSearching,  // start to send CA_PROTO_SEARCH
    NameFound, // after get CA_PROTO_SEARCH reply, next try to establish tcp connection with server
    TcpConnected, // after tcp connected, next send handshake packets: CA_PROTO_VERSION, CA_PROTO_CLIENT_NAME, CA_PROTO_HOST_NAME, CA_PROTO_CREATE_CHAN
    Created, // after CA_PROTO_CREATE_CHAN reply received, after which we can read/write the channel
    Destroyed, // no more name search, release all resources
}

#[derive(Copy, Clone, Debug)]
pub enum ChannelAccessRights {
    None,      // Neither read nor write
    Read,      // Read access only
    ReadWrite, // Both read and write
}

#[derive(Copy, Clone, Debug)]
pub enum ChannelSeverity {
    NoAlarm,
    Minor,
    Major,
    Invalid,
}

#[derive(Copy, Clone, Debug)]
pub enum ChannelStatus {
    NoAlarm,
    Read,
    Write,
    Hihi,
    High,
    Lolo,
    Low,
    State,
    Cos,
    Comm,
    Timeout,
    HwLimit,
    Calc,
    Scan,
    Link,
    Soft,
    BadSub,
    Udf,
    Disable,
    Simm,
    ReadAccess,
    WriteAccess,
}

#[derive(Copy, Clone, Debug)]
pub enum DbrType {
    String,
    // Int,
    Short,
    Float,
    Enum,
    Char,
    Long,
    Double,

    StsString,
    // StsInt,
    StsShort,
    StsFloat,
    StsEnum,
    StsChar,
    StsLong,
    StsDouble,

    TimeString,
    // TimeInt,
    TimeShort,
    TimeFloat,
    TimeEnum,
    TimeChar,
    TimeLong,
    TimeDouble,

    GrString,
    // GrInt,
    GrShort,
    GrFloat,
    GrEnum,
    GrChar,
    GrLong,
    GrDouble,

    CtrlString,
    // CtrlInt,
    CtrlShort,
    CtrlFloat,
    CtrlEnum,
    CtrlChar,
    CtrlLong,
    CtrlDouble,

    PutAckt,
    PutAcks,
    StsAckString,
    ClassName,
}

struct ChannelData {
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
    cid: u32, // client ID
    data: RwLock<ChannelData>,
    search_counter: AtomicU32,
    addr: RwLock<Option<SocketAddr>>,
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.data();

        f.debug_struct("Channel")
            .field("name", &self.name)
            .field("cid", &self.cid)
            .field("state", &data.state)
            .field("access_right", &data.access_right)
            .field("status", &data.status)
            .field("severity", &data.severity)
            .field("seconds_since_epoch", &data.seconds_since_epoch)
            .field("nano_seconds", &data.nano_seconds)
            .field("units", &data.units)
            .field("precision", &data.precision)
            .field("padding", &data.padding)
            .field("number_of_string_used", &data.number_of_string_used)
            .field("strings", &data.strings)
            .field("upper_display_limit", &data.upper_display_limit)
            .field("lower_display_limit", &data.lower_display_limit)
            .field("upper_alarm_limit", &data.upper_alarm_limit)
            .field("lower_alarm_limit", &data.lower_alarm_limit)
            .field("upper_warning_limit", &data.upper_warning_limit)
            .field("lower_warning_limit", &data.lower_warning_limit)
            .finish()
    }
}

impl Channel {
    pub fn new(name: &str, cid: u32) -> Self {
        Channel {
            name: name.to_string(),
            cid: cid,
            search_counter: AtomicU32::new(1),
            data: RwLock::new(ChannelData {
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
            addr: RwLock::new(None),
        }
    }

    /**
     * Connect to the server tcp if this channel is not connected.
     *  - connect tcp if not connected yet
     *  - correlate Channel and TCP
     *  - send out CA_PROTO_VERSION, CA_PROTO_CLIENT_NAME, CA_PROTO_HOST_NAME to tcp
     */
    pub async fn connect(self: &Self, addr: SocketAddr) {
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

    // ------------------ data -------------------
    pub fn name(&self) -> &str {
        &self.name
    }

    fn data(&self) -> RwLockReadGuard<'_, ChannelData> {
        self.data.read().unwrap()
    }

    fn data_mut(&self) -> RwLockWriteGuard<'_, ChannelData> {
        self.data.write().unwrap()
    }

    // ------------- data setter ----------------

    pub fn set_state(&self, new_state: ChannelState) {
        self.data_mut().state = new_state;
    }
    pub fn set_status(&self, new_status: ChannelStatus) {
        self.data_mut().status = new_status;
    }

    pub fn set_severity(&self, new_severity: ChannelSeverity) {
        self.data_mut().severity = new_severity;
    }

    pub fn set_dbr_type_native(&self, new_dbr_type_native: DbrType) {
        self.data_mut().dbr_type_native = new_dbr_type_native;
    }

    pub fn set_seconds_since_epoch(&self, new_seconds_since_epoch: i32) {
        self.data_mut().seconds_since_epoch = new_seconds_since_epoch;
    }

    pub fn set_nano_seconds(&self, new_nano_seconds: u32) {
        self.data_mut().nano_seconds = new_nano_seconds;
    }

    pub fn set_units(&self, new_units: String) {
        self.data_mut().units = new_units;
    }

    pub fn set_precision(&self, new_precision: i16) {
        self.data_mut().precision = new_precision;
    }

    pub fn set_padding(&self, new_padding: i16) {
        self.data_mut().padding = new_padding;
    }

    pub fn set_number_of_string_used(&self, new_number_of_string_used: i16) {
        self.data_mut().number_of_string_used = new_number_of_string_used;
    }

    pub fn set_strings(&self, new_strings: [String; 16]) {
        self.data_mut().strings = new_strings;
    }

    pub fn set_upper_display_limit(&self, new_upper_display_limit: i16) {
        self.data_mut().upper_display_limit = new_upper_display_limit;
    }

    pub fn set_lower_display_limit(&self, new_lower_display_limit: i16) {
        self.data_mut().lower_display_limit = new_lower_display_limit;
    }

    pub fn set_upper_alarm_limit(&self, new_upper_alarm_limit: i16) {
        self.data_mut().upper_alarm_limit = new_upper_alarm_limit;
    }

    pub fn set_lower_alarm_limit(&self, new_lower_alarm_limit: i16) {
        self.data_mut().lower_alarm_limit = new_lower_alarm_limit;
    }

    pub fn set_upper_warning_limit(&self, new_upper_warning_limit: i16) {
        self.data_mut().upper_warning_limit = new_upper_warning_limit;
    }

    pub fn set_lower_warning_limit(&self, new_lower_warning_limit: i16) {
        self.data_mut().lower_warning_limit = new_lower_warning_limit;
    }

    pub fn set_access_right(self: &Self, access_right: ChannelAccessRights) {
        self.data_mut().access_right = access_right;
    }

    pub fn set_addr(self: &Self, new_addr: Option<SocketAddr>) {
        *self.addr.write().unwrap() = new_addr;
    }

    // ------------- data getter ----------------

    pub fn state(&self) -> ChannelState {
        self.data().state
    }

    pub fn status(&self) -> ChannelStatus {
        self.data().status
    }

    pub fn severity(&self) -> ChannelSeverity {
        self.data().severity
    }

    pub fn dbr_type_native(self: &Self) -> DbrType {
        self.data().dbr_type_native
    }

    pub fn seconds_since_epoch(&self) -> i32 {
        self.data().seconds_since_epoch
    }

    pub fn nano_seconds(&self) -> u32 {
        self.data().nano_seconds
    }

    pub fn units(&self) -> String {
        self.data().units.clone()
    }

    pub fn precision(&self) -> i16 {
        self.data().precision
    }

    pub fn padding(&self) -> i16 {
        self.data().padding
    }

    pub fn number_of_string_used(&self) -> i16 {
        self.data().number_of_string_used
    }

    pub fn strings(&self) -> [String; 16] {
        self.data().strings.clone()
    }

    pub fn upper_display_limit(&self) -> i16 {
        self.data().upper_display_limit
    }

    pub fn lower_display_limit(&self) -> i16 {
        self.data().lower_display_limit
    }

    pub fn upper_alarm_limit(&self) -> i16 {
        self.data().upper_alarm_limit
    }

    pub fn lower_alarm_limit(&self) -> i16 {
        self.data().lower_alarm_limit
    }

    pub fn upper_warning_limit(&self) -> i16 {
        self.data().upper_warning_limit
    }

    pub fn lower_warning_limit(&self) -> i16 {
        self.data().lower_warning_limit
    }

    pub fn cid(&self) -> u32 {
        self.cid
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
}
