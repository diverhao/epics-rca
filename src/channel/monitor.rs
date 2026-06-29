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

// channel monitor callback function
pub type MonitorCallback = Arc<dyn Fn(&Channel) + Send + Sync + 'static>;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MonitorState {
    NotRunning,
    Starting,
    Running,
}

pub struct Monitor {
    pub state: RwLock<MonitorState>,
    pub dbr_type: RwLock<DbrType>,
    pub data_count: AtomicU32,
    pub callback: RwLock<Option<MonitorCallback>>,
}

impl Monitor {

    // ---------------- getters -------------------

    pub fn state(self: &Self) -> MonitorState {
        self.state.read().unwrap().clone()
    }

    pub fn callback(self: &Self) -> Option<MonitorCallback> {
        self.callback.read().unwrap().clone()
    }

    pub fn data_count(self: &Self) -> u32 {
        self.data_count.load(Ordering::Relaxed)
    }

    pub fn data_type(self: &Self) -> DbrType {
        self.dbr_type.read().unwrap().clone()
    }

    // ---------------- setters ------------------

    pub fn set_callback(self: &Self, callback: Option<MonitorCallback>) {
        *self.callback.write().unwrap() = callback;
    }

    pub fn set_data_count(self: &Self, count: u32) {
        self.data_count.store(count, Ordering::Relaxed);
    }

    pub fn set_data_type(self: &Self, data_type: DbrType) {
        *self.dbr_type.write().unwrap() = data_type;
    }

    pub fn set_state(self: &Self, state: MonitorState) {
        *self.state.write().unwrap() = state;
    }
}

impl std::fmt::Display for Monitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = *self.state.read().unwrap();
        let dbr_type = *self.dbr_type.read().unwrap();
        let data_count = self.data_count.load(Ordering::Relaxed);
        let callback = if self.callback.read().unwrap().is_some() {
            "Some"
        } else {
            "None"
        };

        writeln!(f, "Monitor {{")?;
        writeln!(f, "    state: {:?},", state)?;
        writeln!(f, "    dbr_type: {:?},", dbr_type)?;
        writeln!(f, "    data_count: {},", data_count)?;
        writeln!(f, "    callback: {}", callback)?;
        write!(f, "}}")
    }
}

impl Channel {
    // --------------- getters ------------------

    pub fn monitor_callback(self: &Self) -> Option<MonitorCallback> {
        self.monitor().callback()
    }

    pub fn monitor_data_count(self: &Self) -> u32 {
        self.monitor().data_count()
    }

    pub fn monitor_data_type(self: &Self) -> DbrType {
        self.monitor().data_type()
    }

    pub fn monitor_state(self: &Self) -> MonitorState {
        self.monitor().state()
    }

    // ------------- setters ------------------

    pub fn set_monitor_callback(self: &Self, callback: Option<MonitorCallback>) {
        self.monitor().set_callback(callback);
    }

    pub fn set_monitor_data_count(self: &Self, count: u32) {
        self.monitor().set_data_count(count)
    }

    pub fn set_monitor_data_type(self: &Self, data_type: DbrType) {
        self.monitor().set_data_type(data_type);
    }

    pub fn set_monitor_state(self: &Self, state: MonitorState) {
        self.monitor().set_state(state);
    }
}
