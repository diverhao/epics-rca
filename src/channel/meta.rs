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

pub struct ChannelMeta {
    // state
    pub state: ChannelState,
    pub access_right: ChannelAccessRights,
    // status, severity, and native dbr_type
    pub status: ChannelStatus,
    pub severity: ChannelSeverity,
    pub dbr_type_native: DbrType,
    pub data_count_native: u32,
    // time
    pub seconds_since_epoch: i32, // the Unix time, not epics time
    pub nano_seconds: u32,
    // data
    pub units: String, // 8 C chars
    pub precision: i16,
    pub padding: i16,
    // enum
    pub number_of_string_used: i16,
    pub strings: [String; 16], // 16 elements, each with up to 26 C chars
    // limits
    pub upper_display_limit: i16,
    pub lower_display_limit: i16,
    pub upper_alarm_limit: i16,
    pub lower_alarm_limit: i16,
    pub upper_warning_limit: i16,
    pub lower_warning_limit: i16,
}

impl std::fmt::Display for ChannelMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ChannelMeta {{")?;
        writeln!(f, "    state: {:?},", self.state)?;
        writeln!(f, "    access_right: {:?},", self.access_right)?;
        writeln!(f, "    status: {:?},", self.status)?;
        writeln!(f, "    severity: {:?},", self.severity)?;
        writeln!(f, "    dbr_type_native: {:?},", self.dbr_type_native)?;
        writeln!(f, "    data_count_native: {},", self.data_count_native)?;
        writeln!(f, "    seconds_since_epoch: {},", self.seconds_since_epoch)?;
        writeln!(f, "    nano_seconds: {},", self.nano_seconds)?;
        writeln!(f, "    units: {:?},", self.units)?;
        writeln!(f, "    precision: {},", self.precision)?;
        writeln!(f, "    padding: {},", self.padding)?;
        writeln!(
            f,
            "    number_of_string_used: {},",
            self.number_of_string_used
        )?;
        writeln!(f, "    strings: {:?},", self.strings)?;
        writeln!(f, "    upper_display_limit: {},", self.upper_display_limit)?;
        writeln!(f, "    lower_display_limit: {},", self.lower_display_limit)?;
        writeln!(f, "    upper_alarm_limit: {},", self.upper_alarm_limit)?;
        writeln!(f, "    lower_alarm_limit: {},", self.lower_alarm_limit)?;
        writeln!(f, "    upper_warning_limit: {},", self.upper_warning_limit)?;
        writeln!(f, "    lower_warning_limit: {}", self.lower_warning_limit)?;
        write!(f, "}}")
    }
}
