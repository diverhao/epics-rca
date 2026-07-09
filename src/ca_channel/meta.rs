use crate::ca_channel::channel::Channel;
use crate::context::context::get_context;
use core::num;
use std::net::SocketAddr;
use std::sync::{
    Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
    atomic::{AtomicU32, Ordering},
};
use tokio::sync::Notify;

use crate::ca_channel::dbr::{ChannelAccessRights, ChannelSeverity, ChannelState, ChannelStatus};
use crate::ca_channel::dbr::{DbrType, DbrValue};
use crate::ca_message::message::CaMsg;
use log::{debug, error, warn};

pub struct Meta {
    pub state: ChannelState,
    pub sid: u32, // server ID, assigned after channel created on server
    pub addr: Option<SocketAddr>,
    pub access_right: ChannelAccessRights,
    pub data_type_native: DbrType,
    pub data_count_native: u32,
}

impl Meta {
    pub fn new() -> Meta {
        Meta {
            state: ChannelState::NameSearching,
            sid: 0,
            addr: None,
            access_right: ChannelAccessRights::None,
            data_type_native: DbrType::Double,
            data_count_native: 0,
        }
    }

    // getters

    pub fn state(&self) -> ChannelState {
        self.state
    }

    pub fn sid(&self) -> u32 {
        self.sid
    }

    pub fn addr(&self) -> Option<SocketAddr> {
        self.addr
    }

    pub fn access_right(&self) -> ChannelAccessRights {
        self.access_right
    }

    pub fn data_type_native(self: &Self) -> DbrType {
        self.data_type_native
    }

    pub fn data_count_native(&self) -> u32 {
        self.data_count_native
    }

    // setters
    pub fn reset(self: &mut Self) {
        self.set_state(ChannelState::NameSearching);
        self.set_sid(0);
        self.set_addr(None);
        self.set_access_right(ChannelAccessRights::None);
        self.set_data_type_native(DbrType::Double);
        self.set_data_count_native(0);
    }

    pub fn set_state(&mut self, new_state: ChannelState) {
        self.state = new_state;
    }

    pub fn set_sid(&mut self, new_sid: u32) {
        self.sid = new_sid;
    }

    pub fn set_addr(&mut self, new_addr: Option<SocketAddr>) {
        self.addr = new_addr;
    }

    pub fn set_data_type_native(&mut self, new_data_type_native: DbrType) {
        self.data_type_native = new_data_type_native;
    }

    pub fn set_data_count_native(&mut self, data_count: u32) {
        self.data_count_native = data_count;
    }

    pub fn set_access_right(self: &mut Self, access_right: ChannelAccessRights) {
        self.access_right = access_right;
    }
}

impl std::fmt::Display for Meta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Meta {{")?;
        writeln!(f, "    state: {:?},", self.state())?;
        writeln!(f, "    access_right: {:?},", self.access_right())?;
        writeln!(f, "    data_type_native: {:?},", self.data_type_native())?;
        writeln!(f, "    data_count_native: {},", self.data_count_native())?;
        write!(f, "}}")
    }
}

impl Channel {
    // ------------------ getters ----------------

    pub fn state(&self) -> ChannelState {
        self.meta().state()
    }

    pub fn sid(&self) -> u32 {
        self.meta().sid()
    }

    pub fn addr(&self) -> Option<SocketAddr> {
        self.meta().addr()
    }

    pub fn data_type_native(self: &Self) -> DbrType {
        self.meta().data_type_native()
    }

    pub fn data_count_native(&self) -> u32 {
        self.meta().data_count_native()
    }

    // --------------- setters -------------------

    pub fn set_state(&self, new_state: ChannelState, notify_state: bool) {
        let old_state = self.state();

        let channels = get_context().channels();
        if old_state == ChannelState::NameSearching
            && (new_state == ChannelState::Destroyed
                || new_state == ChannelState::Created
                || new_state == ChannelState::NameFound
                || new_state == ChannelState::TcpConnected)
        {
            channels.move_to_not_searching_by_cid(self.cid());
        } else if new_state == ChannelState::NameSearching
            && (old_state == ChannelState::Destroyed
                || old_state == ChannelState::Created
                || old_state == ChannelState::NameFound
                || old_state == ChannelState::TcpConnected)
        {
            channels.move_to_searching_by_cid(self.cid());
        } else {
            // do nothing
        }

        self.meta_mut().set_state(new_state);

        // if notify_state {
        //     self.state_change_notifier().notify_waiters();
        // }
    }

    pub fn set_sid(&self, new_sid: u32) {
        self.meta_mut().set_sid(new_sid);
    }

    pub fn set_addr(&self, new_addr: Option<SocketAddr>) {
        self.meta_mut().set_addr(new_addr);
    }

    pub fn set_data_type_native(&self, new_data_type_native: DbrType) {
        self.meta_mut().set_data_type_native(new_data_type_native);
    }

    pub fn set_data_count_native(&self, data_count: u32) {
        self.meta_mut().set_data_count_native(data_count);
    }

    pub fn set_access_right(self: &Self, access_right: ChannelAccessRights) {
        self.meta_mut().set_access_right(access_right);
    }
}
