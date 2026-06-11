use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ChannelState {
    NeverConnected, // initial state
    NameSearching,  // send CA_PROTO_SEARCH
    NameFound,      // after get CA_PROTO_SEARCH reply
    TcpConnected, // TCP connected, start to send CA_PROTO_VERSION, CA_PROTO_CLIENT_NAME, CA_PROTO_HOST_NAME, CA_PROTO_CREATE_CHAN
    Created,      // after CA_PROTO_CREATE_CHAN succeed
    Destroyed,    // no more name search, release all resources
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
    // status and severity
    status: ChannelStatus,
    severity: ChannelSeverity,
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
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.data();

        f.debug_struct("Channel")
            .field("name", &self.name)
            .field("cid", &self.cid)
            .field("state", &data.state)
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
            data: RwLock::new(ChannelData {
                state: ChannelState::NeverConnected,
                status: ChannelStatus::NoAlarm,
                severity: ChannelSeverity::NoAlarm,
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
        }
    }

    // ------------------ data -------------------
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn data(&self) -> RwLockReadGuard<'_, ChannelData> {
        self.data.read().unwrap()
    }

    pub fn data_mut(&self) -> RwLockWriteGuard<'_, ChannelData> {
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
}
