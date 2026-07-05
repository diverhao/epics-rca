use crate::channel::channel::Channel;
use crate::context::context::get_context;
use core::num;
use std::net::SocketAddr;
use std::sync::{
    Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
    atomic::{AtomicU32, Ordering},
};
use tokio::sync::Notify;

use crate::ca::message::CaMsg;
use crate::channel::dbr::{ChannelAccessRights, ChannelSeverity, ChannelState, ChannelStatus};
use crate::channel::dbr::{DbrType, DbrValue};
use log::{debug, error, warn};

pub struct Meta {
    // state
    pub state: RwLock<ChannelState>,
    pub access_right: RwLock<ChannelAccessRights>,
    // status, severity, and native dbr_type
    pub status: RwLock<ChannelStatus>,
    pub severity: RwLock<ChannelSeverity>,
    pub dbr_type_native: RwLock<DbrType>,
    pub data_count_native: RwLock<u32>,
    // time
    pub seconds_since_epoch: RwLock<i32>, // the Unix time, not epics time
    pub nano_seconds: RwLock<u32>,
    // data
    pub units: RwLock<String>, // 8 C chars
    pub precision: RwLock<i16>,
    pub padding: RwLock<i16>,
    // enum
    pub number_of_string_used: RwLock<i16>,
    pub strings: RwLock<Vec<String>>, // 16 elements, each with up to 26 C chars
    // limits
    pub upper_display_limit: RwLock<i16>,
    pub lower_display_limit: RwLock<i16>,
    pub upper_alarm_limit: RwLock<i16>,
    pub lower_alarm_limit: RwLock<i16>,
    pub upper_warning_limit: RwLock<i16>,
    pub lower_warning_limit: RwLock<i16>,
}

impl Meta {
    pub fn new() -> Arc<Meta> {
        Arc::new(Meta {
            state: RwLock::new(ChannelState::NameSearching),
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
            strings: RwLock::new(vec![]),
            upper_display_limit: RwLock::new(0),
            lower_display_limit: RwLock::new(0),
            upper_alarm_limit: RwLock::new(0),
            lower_alarm_limit: RwLock::new(0),
            upper_warning_limit: RwLock::new(0),
            lower_warning_limit: RwLock::new(0),
        })
    }

    // getters

    pub fn state(&self) -> ChannelState {
        self.state.read().unwrap().clone()
    }

    pub fn access_right(&self) -> ChannelAccessRights {
        self.access_right.read().unwrap().clone()
    }

    pub fn status(&self) -> ChannelStatus {
        self.status.read().unwrap().clone()
    }

    pub fn severity(&self) -> ChannelSeverity {
        self.severity.read().unwrap().clone()
    }

    pub fn dbr_type_native(self: &Self) -> DbrType {
        self.dbr_type_native.read().unwrap().clone()
    }

    pub fn seconds_since_epoch(&self) -> i32 {
        self.seconds_since_epoch.read().unwrap().clone()
    }

    pub fn data_count_native(&self) -> u32 {
        self.data_count_native.read().unwrap().clone()
    }

    pub fn nano_seconds(&self) -> u32 {
        self.nano_seconds.read().unwrap().clone()
    }

    pub fn units(&self) -> String {
        self.units.read().unwrap().clone()
    }

    pub fn precision(&self) -> i16 {
        self.precision.read().unwrap().clone()
    }

    pub fn padding(&self) -> i16 {
        self.padding.read().unwrap().clone()
    }

    pub fn number_of_string_used(&self) -> i16 {
        self.number_of_string_used.read().unwrap().clone()
    }

    pub fn strings(&self) -> Vec<String> {
        self.strings.read().unwrap().clone()
    }

    pub fn upper_display_limit(&self) -> i16 {
        self.upper_display_limit.read().unwrap().clone()
    }

    pub fn lower_display_limit(&self) -> i16 {
        self.lower_display_limit.read().unwrap().clone()
    }

    pub fn upper_alarm_limit(&self) -> i16 {
        self.upper_alarm_limit.read().unwrap().clone()
    }

    pub fn lower_alarm_limit(&self) -> i16 {
        self.lower_alarm_limit.read().unwrap().clone()
    }

    pub fn upper_warning_limit(&self) -> i16 {
        self.upper_warning_limit.read().unwrap().clone()
    }

    pub fn lower_warning_limit(&self) -> i16 {
        self.lower_warning_limit.read().unwrap().clone()
    }

    // setters
    pub fn reset(self: &Self) {
        self.set_state(ChannelState::NameSearching);
        self.set_access_right(ChannelAccessRights::None);
        self.set_status(ChannelStatus::NoAlarm);
        self.set_severity(ChannelSeverity::NoAlarm);
        self.set_dbr_type_native(DbrType::Double);
        self.set_data_count_native(0);
        self.set_seconds_since_epoch(0);
        self.set_nano_seconds(0);
        self.set_units(String::new());
        self.set_precision(0);
        self.set_padding(0);
        self.set_number_of_string_used(0);
        self.set_strings(vec![]);
        self.set_upper_display_limit(0);
        self.set_lower_display_limit(0);
        self.set_upper_alarm_limit(0);
        self.set_lower_alarm_limit(0);
        self.set_upper_warning_limit(0);
        self.set_lower_warning_limit(0);
    }

    pub fn set_state(&self, new_state: ChannelState) {
        *self.state.write().unwrap() = new_state;
    }

    pub fn set_status(&self, new_status: ChannelStatus) {
        *self.status.write().unwrap() = new_status;
    }

    pub fn set_severity(&self, new_severity: ChannelSeverity) {
        *self.severity.write().unwrap() = new_severity;
    }

    pub fn set_dbr_type_native(&self, new_dbr_type_native: DbrType) {
        *self.dbr_type_native.write().unwrap() = new_dbr_type_native;
    }

    pub fn set_seconds_since_epoch(&self, new_seconds_since_epoch: i32) {
        *self.seconds_since_epoch.write().unwrap() = new_seconds_since_epoch;
    }

    pub fn set_nano_seconds(&self, new_nano_seconds: u32) {
        *self.nano_seconds.write().unwrap() = new_nano_seconds;
    }

    pub fn set_data_count_native(&self, data_count: u32) {
        *self.data_count_native.write().unwrap() = data_count;
    }

    pub fn set_units(&self, new_units: String) {
        *self.units.write().unwrap() = new_units;
    }

    pub fn set_precision(&self, new_precision: i16) {
        *self.precision.write().unwrap() = new_precision;
    }

    pub fn set_padding(&self, new_padding: i16) {
        *self.padding.write().unwrap() = new_padding;
    }

    pub fn set_number_of_string_used(&self, new_number_of_string_used: i16) {
        *self.number_of_string_used.write().unwrap() = new_number_of_string_used;
    }

    pub fn set_strings(&self, new_strings: Vec<String>) {
        *self.strings.write().unwrap() = new_strings;
    }

    pub fn set_upper_display_limit(&self, new_upper_display_limit: i16) {
        *self.upper_display_limit.write().unwrap() = new_upper_display_limit;
    }

    pub fn set_lower_display_limit(&self, new_lower_display_limit: i16) {
        *self.lower_display_limit.write().unwrap() = new_lower_display_limit;
    }

    pub fn set_upper_alarm_limit(&self, new_upper_alarm_limit: i16) {
        *self.upper_alarm_limit.write().unwrap() = new_upper_alarm_limit;
    }

    pub fn set_lower_alarm_limit(&self, new_lower_alarm_limit: i16) {
        *self.lower_alarm_limit.write().unwrap() = new_lower_alarm_limit;
    }

    pub fn set_upper_warning_limit(&self, new_upper_warning_limit: i16) {
        *self.upper_warning_limit.write().unwrap() = new_upper_warning_limit;
    }

    pub fn set_lower_warning_limit(&self, new_lower_warning_limit: i16) {
        *self.lower_warning_limit.write().unwrap() = new_lower_warning_limit;
    }

    pub fn set_access_right(self: &Self, access_right: ChannelAccessRights) {
        *self.access_right.write().unwrap() = access_right;
    }
}

impl std::fmt::Display for Meta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Meta {{")?;
        writeln!(f, "    state: {:?},", self.state())?;
        writeln!(f, "    access_right: {:?},", self.access_right())?;
        writeln!(f, "    status: {:?},", self.status())?;
        writeln!(f, "    severity: {:?},", self.severity())?;
        writeln!(f, "    dbr_type_native: {:?},", self.dbr_type_native())?;
        writeln!(f, "    data_count_native: {},", self.data_count_native())?;
        writeln!(
            f,
            "    seconds_since_epoch: {},",
            self.seconds_since_epoch()
        )?;
        writeln!(f, "    nano_seconds: {},", self.nano_seconds())?;
        writeln!(f, "    units: {:?},", self.units())?;
        writeln!(f, "    precision: {},", self.precision())?;
        writeln!(f, "    padding: {},", self.padding())?;
        writeln!(
            f,
            "    number_of_string_used: {},",
            self.number_of_string_used()
        )?;
        writeln!(f, "    strings: {:?},", self.strings())?;
        writeln!(
            f,
            "    upper_display_limit: {},",
            self.upper_display_limit()
        )?;
        writeln!(
            f,
            "    lower_display_limit: {},",
            self.lower_display_limit()
        )?;
        writeln!(f, "    upper_alarm_limit: {},", self.upper_alarm_limit())?;
        writeln!(f, "    lower_alarm_limit: {},", self.lower_alarm_limit())?;
        writeln!(
            f,
            "    upper_warning_limit: {},",
            self.upper_warning_limit()
        )?;
        writeln!(f, "    lower_warning_limit: {}", self.lower_warning_limit())?;
        write!(f, "}}")
    }
}

impl Channel {
    // ------------------ getters ----------------

    pub fn state(&self) -> ChannelState {
        self.meta().state()
    }

    pub fn status(&self) -> ChannelStatus {
        self.meta().status()
    }

    pub fn severity(&self) -> ChannelSeverity {
        self.meta().severity()
    }

    pub fn dbr_type_native(self: &Self) -> DbrType {
        self.meta().dbr_type_native()
    }

    pub fn seconds_since_epoch(&self) -> i32 {
        self.meta().seconds_since_epoch()
    }

    pub fn data_count_native(&self) -> u32 {
        self.meta().data_count_native()
    }

    pub fn nano_seconds(&self) -> u32 {
        self.meta().nano_seconds()
    }

    pub fn units(&self) -> String {
        self.meta().units()
    }

    pub fn precision(&self) -> i16 {
        self.meta().precision()
    }

    pub fn padding(&self) -> i16 {
        self.meta().padding()
    }

    pub fn number_of_string_used(&self) -> i16 {
        self.meta().number_of_string_used()
    }

    pub fn strings(&self) -> Vec<String> {
        self.meta().strings()
    }

    pub fn upper_display_limit(&self) -> i16 {
        self.meta().upper_display_limit()
    }

    pub fn lower_display_limit(&self) -> i16 {
        self.meta().lower_display_limit()
    }

    pub fn upper_alarm_limit(&self) -> i16 {
        self.meta().upper_alarm_limit()
    }

    pub fn lower_alarm_limit(&self) -> i16 {
        self.meta().lower_alarm_limit()
    }

    pub fn upper_warning_limit(&self) -> i16 {
        self.meta().upper_warning_limit()
    }

    pub fn lower_warning_limit(&self) -> i16 {
        self.meta().lower_warning_limit()
    }

    // --------------- setters -------------------

    pub fn set_state(&self, new_state: ChannelState, notify_state: bool) {
        let old_state = self.state();

        let channels = get_context().channels();
        if old_state == ChannelState::NameSearching
            && (new_state == ChannelState::Destroyed
                || new_state == ChannelState::Created
                || new_state == ChannelState::NameFound
                || new_state == ChannelState::TcpConnected
                )
        {
            channels.move_to_not_searching_by_cid(self.cid());
        } else if new_state == ChannelState::NameSearching
            && (old_state == ChannelState::Destroyed
                || old_state == ChannelState::Created
                || old_state == ChannelState::NameFound
                || old_state == ChannelState::TcpConnected
                )
        {
            channels.move_to_searching_by_cid(self.cid());
        } else {
            // do nothing
        }

        self.meta().set_state(new_state);

        // if notify_state {
        //     self.state_change_notifier().notify_waiters();
        // }
    }

    pub fn set_status(&self, new_status: ChannelStatus) {
        self.meta().set_status(new_status);
    }

    pub fn set_severity(&self, new_severity: ChannelSeverity) {
        self.meta().set_severity(new_severity);
    }

    pub fn set_dbr_type_native(&self, new_dbr_type_native: DbrType) {
        self.meta().set_dbr_type_native(new_dbr_type_native);
    }

    pub fn set_seconds_since_epoch(&self, new_seconds_since_epoch: i32) {
        self.meta().set_seconds_since_epoch(new_seconds_since_epoch);
    }

    pub fn set_nano_seconds(&self, new_nano_seconds: u32) {
        self.meta().set_nano_seconds(new_nano_seconds);
    }

    pub fn set_data_count_native(&self, data_count: u32) {
        self.meta().set_data_count_native(data_count);
    }

    pub fn set_units(&self, new_units: String) {
        self.meta().set_units(new_units);
    }

    pub fn set_precision(&self, new_precision: i16) {
        self.meta().set_precision(new_precision);
    }

    pub fn set_padding(&self, new_padding: i16) {
        self.meta().set_padding(new_padding);
    }

    pub fn set_number_of_string_used(&self, new_number_of_string_used: i16) {
        self.meta()
            .set_number_of_string_used(new_number_of_string_used);
    }

    pub fn set_strings(&self, new_strings: Vec<String>) {
        self.meta().set_strings(new_strings);
    }

    pub fn set_upper_display_limit(&self, new_upper_display_limit: i16) {
        self.meta().set_upper_display_limit(new_upper_display_limit);
    }

    pub fn set_lower_display_limit(&self, new_lower_display_limit: i16) {
        self.meta().set_lower_display_limit(new_lower_display_limit);
    }

    pub fn set_upper_alarm_limit(&self, new_upper_alarm_limit: i16) {
        self.meta().set_upper_alarm_limit(new_upper_alarm_limit);
    }

    pub fn set_lower_alarm_limit(&self, new_lower_alarm_limit: i16) {
        self.meta().set_lower_alarm_limit(new_lower_alarm_limit);
    }

    pub fn set_upper_warning_limit(&self, new_upper_warning_limit: i16) {
        self.meta().set_upper_warning_limit(new_upper_warning_limit);
    }

    pub fn set_lower_warning_limit(&self, new_lower_warning_limit: i16) {
        self.meta().set_lower_warning_limit(new_lower_warning_limit);
    }

    pub fn set_access_right(self: &Self, access_right: ChannelAccessRights) {
        self.meta().set_access_right(access_right);
    }
}
