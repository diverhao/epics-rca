use crate::channel::channel::{Channel, ChannelCallback};
use crate::context::context::get_context;
use core::num;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{
    Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
    atomic::{AtomicU32, Ordering},
};
use tokio::sync::Notify;

use crate::ca::message::CaMsg;
use crate::channel::dbr::{
    self, ChannelAccessRights, ChannelSeverity, ChannelState, ChannelStatus,
};
use crate::channel::dbr::{DbrType, DbrValue};
use log::{debug, error, warn};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MonitorState {
    NotRunning,
    Starting,
    Running,
}

#[derive(Debug, Copy, Clone)]
pub enum MonitorDataType {
    Exact(DbrType),
    Native,
    NativeSts,
    NativeTime,
    NativeGr,
    NativeCtrl,
}
impl MonitorDataType {
    pub fn resolve(self, channel: &Channel) -> DbrType {
        match self {
            MonitorDataType::Exact(data_type) => data_type,
            MonitorDataType::Native => channel.data_type_native(),
            MonitorDataType::NativeSts => channel.data_type_native_as_sts(),
            MonitorDataType::NativeTime => channel.data_type_native_as_time(),
            MonitorDataType::NativeGr => channel.data_type_native_as_gr(),
            MonitorDataType::NativeCtrl => channel.data_type_native_as_ctrl(),
        }
    }
}

pub struct MonitorConfig {
    pub data_type: Option<MonitorDataType>,
    pub data_count: Option<u32>,
}

pub struct Monitor {
    pub state: MonitorState,
    pub data_type: DbrType,
    pub data_count: u32,
    pub callback: Option<ChannelCallback>,
    pub user_config: MonitorConfig,
}

impl Monitor {
    pub fn new() -> Monitor {
        Monitor {
            state: MonitorState::NotRunning,
            data_type: DbrType::Double,
            data_count: 0,
            callback: None,
            user_config: MonitorConfig {
                data_type: None,
                data_count: None,
            },
        }
    }

    // ---------------- getters -------------------

    pub fn state(self: &Self) -> MonitorState {
        self.state
    }

    pub fn callback(self: &Self) -> Option<ChannelCallback> {
        self.callback.clone()
    }

    pub fn data_count(self: &Self) -> u32 {
        self.data_count
    }

    pub fn data_type(self: &Self) -> DbrType {
        self.data_type
    }

    // ---------------- setters ------------------

    pub fn set_callback(self: &mut Self, callback: Option<ChannelCallback>) {
        self.callback = callback;
    }

    pub fn set_data_count(self: &mut Self, count: u32) {
        self.data_count = count;
    }

    pub fn set_data_type(self: &mut Self, data_type: DbrType) {
        self.data_type = data_type;
    }

    pub fn set_state(self: &mut Self, state: MonitorState) {
        self.state = state;
    }

    // ------------ user config ----------------

    pub fn user_config(self: &Self) -> &MonitorConfig {
        &self.user_config
    }
    pub fn user_config_mut(self: &mut Self) -> &mut MonitorConfig {
        &mut self.user_config
    }

    pub fn user_config_data_count(self: &Self) -> Option<u32> {
        self.user_config.data_count
    }

    pub fn user_config_data_type(self: &Self) -> Option<MonitorDataType> {
        self.user_config.data_type
    }

    pub fn set_user_config_data_count(self: &mut Self, data_count: Option<u32>) {
        self.user_config_mut().data_count = data_count;
    }

    pub fn set_user_config_data_type(self: &mut Self, data_type: Option<MonitorDataType>) {
        self.user_config_mut().data_type = data_type;
    }
}

impl std::fmt::Display for Monitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let callback = if self.callback.is_some() {
            "Some"
        } else {
            "None"
        };

        writeln!(f, "Monitor {{")?;
        writeln!(f, "    state: {:?},", self.state)?;
        writeln!(f, "    data_type: {:?},", self.data_type)?;
        writeln!(f, "    data_count: {},", self.data_count)?;
        writeln!(f, "    callback: {}", callback)?;
        write!(f, "}}")
    }
}

impl Channel {
    // ------------ start/cancel monitor --------

    /**
     * Start a monitor subscription for this channel.
     *
     * If the channel has already reached [`ChannelState::Created`], this sends
     * `CA_PROTO_EVENT_ADD` immediately. Otherwise the monitor is marked as
     * [`MonitorState::Starting`], and the create-channel response handler will
     * send the subscription request once the channel is ready.
     *
     * `data_type` and `data_count` are optional user overrides. When they are
     * omitted, the channel's native DBR type and native element count are used
     * at the time the subscription request is sent. If a monitor is already
     * starting or running, this call leaves the existing monitor unchanged.
     */
    pub fn start_to_monitor(
        self: &Self,
        data_type: Option<MonitorDataType>,
        data_count: Option<u32>,
        callback: Option<ChannelCallback>,
    ) {
        // Monitor must be NotRunning or
        if self.monitor().state() != MonitorState::NotRunning {
            return;
        }

        // store user-defined
        self.monitor_mut().set_user_config_data_count(data_count);
        self.monitor_mut().set_user_config_data_type(data_type);

        let callback = match callback {
            Some(callback) => Some(callback),
            None => None,
        };

        self.set_monitor_state(MonitorState::Starting);
        self.set_monitor_callback(callback);
        if self.state() == ChannelState::Created {
            self.send_monitor_add();
        } else {
            // do nothing
        }
    }

    /**
     * Send the monitor subscription request for a channel that is ready.
     *
     * This is called either directly by [`Self::start_to_monitor`] when the
     * channel is already created, or later by the create-channel response
     * handler after a pending monitor reaches a usable channel state.
     *
     * The request is only sent while the channel is [`ChannelState::Created`]
     * and the monitor is [`MonitorState::Starting`]. Before building
     * `CA_PROTO_EVENT_ADD`, user-provided DBR type/count overrides are
     * resolved; omitted values fall back to the channel's native DBR type and
     * native element count.
     */
    pub fn send_monitor_add(self: &Self) {
        if self.state() != ChannelState::Created {
            return;
        }

        if self.monitor().state() != MonitorState::Starting {
            return;
        }

        // Snapshot user config before taking a write lock. Holding the read guard
        // from self.monitor() across self.monitor_mut() can deadlock.
        let (user_config_data_count, user_config_data_type) = {
            let monitor = self.monitor();
            (
                monitor.user_config_data_count(),
                monitor.user_config_data_type(),
            )
        };

        let data_count = user_config_data_count.unwrap_or_else(|| self.data_count_native());
        let data_type = user_config_data_type
            .map(|data_type| data_type.resolve(self))
            .unwrap_or_else(|| self.data_type_native());

        {
            let mut monitor = self.monitor_mut();
            monitor.set_data_count(data_count);
            monitor.set_data_type(data_type);
        }

        let data_type = self.monitor().data_type();
        let data_count = self.monitor().data_count();
        let sid = self.sid();
        let subid = self.cid();
        let dest = match self.addr() {
            Some(dest) => dest,
            None => return,
        };
        let context = get_context();

        let msg: CaMsg = CaMsg::build_event_add(data_type, data_count, sid, subid, &vec![dest]);
        let tcp = match context.tcps().tcp(&dest) {
            Some(tcp) => tcp,
            None => return,
        };

        // tell server to start the monitor: send out CA_PROTO_EVENT_ADD
        tcp.send_msgs(vec![msg]);

        // .await {
        //     Ok(_) => {}
        //     Err(error) => {
        //         // do nothing, this is handled by periodic TCP alive check
        //     }
        // };
    }

    /**
     * Cancel this channel's active or pending monitor subscription.
     *
     * The local monitor state is changed to [`MonitorState::NotRunning`] then send
     * send `CA_PROTO_EVENT_CANCEL`
     */
    pub fn cancel_monitor(self: &Self, sid: u32, subid: u32, dest: Option<SocketAddr>) {
        if self.monitor_state() == MonitorState::NotRunning {
            // already stopped
            return;
        }

        self.monitor_mut().set_state(MonitorState::NotRunning);

        let data_type = self.monitor_data_type();
        let data_count = self.monitor_data_count();
        let context = get_context();
        match dest {
            Some(dest) => {
                let msg: CaMsg =
                    CaMsg::build_event_cancel(data_type, data_count, sid, subid, &vec![dest]);
                let tcp: Option<Arc<crate::tcp::tcp::TCP>> = context.tcps().tcp(&dest);
                match tcp {
                    Some(tcp) => {
                        // tell server to release resource: send out CA_PROTO_EVENT_CANCEL
                        tcp.send_msgs(vec![msg]);
                        // .await {
                        //     Ok(_) => {}
                        //     Err(error) => {}
                        // };
                    }
                    None => {}
                }
            }
            None => {}
        }
    }

    // --------------- getters ------------------

    pub fn monitor_callback(self: &Self) -> Option<ChannelCallback> {
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

    pub fn set_monitor_callback(self: &Self, callback: Option<ChannelCallback>) {
        self.monitor_mut().set_callback(callback);
    }

    pub fn set_monitor_data_count(self: &Self, count: u32) {
        self.monitor_mut().set_data_count(count)
    }

    pub fn set_monitor_data_type(self: &Self, data_type: DbrType) {
        self.monitor_mut().set_data_type(data_type);
    }

    pub fn set_monitor_state(self: &Self, state: MonitorState) {
        self.monitor_mut().set_state(state);
    }
}
