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
pub enum ChannelMonitorState {
    NotRunning,
    Starting,
    Running,
}

pub struct ChannelMonitor {
    pub state: RwLock<ChannelMonitorState>,
    pub dbr_type: RwLock<DbrType>,
    pub data_count: AtomicU32,
    pub callback: RwLock<Option<MonitorCallback>>,
}

impl std::fmt::Display for ChannelMonitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = *self.state.read().unwrap();
        let dbr_type = *self.dbr_type.read().unwrap();
        let data_count = self.data_count.load(Ordering::Relaxed);
        let callback = if self.callback.read().unwrap().is_some() {
            "Some"
        } else {
            "None"
        };

        writeln!(f, "ChannelMonitor {{")?;
        writeln!(f, "    state: {:?},", state)?;
        writeln!(f, "    dbr_type: {:?},", dbr_type)?;
        writeln!(f, "    data_count: {},", data_count)?;
        writeln!(f, "    callback: {}", callback)?;
        write!(f, "}}")
    }
}
