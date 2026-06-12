use log::{debug, warn};

use crate::ca::message::{MAX_UDP_SEND, build_name_search_buf, build_version_buf};
use crate::channel::channel::ChannelState;
use crate::{channel::channel::Channel, context::context::get_context};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;
use std::sync::atomic::{AtomicU32, Ordering};

pub struct Channels {
    by_name: RwLock<HashMap<String, Arc<Channel>>>,
    by_cid: RwLock<HashMap<u32, Arc<Channel>>>,
    next_cid: AtomicU32,
}

impl fmt::Display for Channels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "|name|cid|state|status|severity|time|")?;
        for channel in self.by_cid().values() {
            writeln!(
                f,
                "|{}|{}|{:?}|{:?}|{:?}|{}.{:09}|",
                channel.name(),
                channel.cid(),
                channel.state(),
                channel.status(),
                channel.severity(),
                channel.seconds_since_epoch(),
                channel.nano_seconds()
            )?;
        }
        Ok(())
    }
}

impl Channels {
    pub fn new() -> Self {
        Self {
            by_name: RwLock::new(HashMap::new()),
            by_cid: RwLock::new(HashMap::new()),
            next_cid: AtomicU32::new(1),
        }
    }

    pub fn create_channel(self: &Self, name: &str) -> Arc<Channel> {
        let mut by_name = self.by_name_mut();
        if let Some(channel) = by_name.get(name) {
            warn!("Channel {name} is already created");
            return Arc::clone(channel);
        }

        let id = self.next_cid();
        let channel = Arc::new(Channel::new(name, id));
        let mut by_cid = self.by_cid_mut();
        by_name.insert(String::from(name), Arc::clone(&channel));
        by_cid.insert(id, Arc::clone(&channel));
        debug!("Channel {name} created");
        channel
    }

    pub fn create_channels(self: &Self, names: Vec<String>) {
        for name in names {
            self.create_channel(name.as_str());
        }
    }

    // remove channel
    pub fn remove_channel(self: &Self, name: &str) {}

    // --------------- network -----------------

    pub async fn search_ca(self: &Self) {
        let channels: Vec<Arc<Channel>> = self.by_cid().values().cloned().collect();
        let context = get_context();
        let udp = context.udp();
        let mut buf_full: Vec<u8> = build_version_buf();

        for channel in channels {
            let name = channel.name();
            let cid = channel.cid();
            let channel_state = channel.state();
            if channel_state != ChannelState::NeverConnected
                && channel_state != ChannelState::NameSearching
            {
                debug!("Channel {name} is already found, skip name search");
                continue;
            }
            debug!("Searching {name}");
            let buf = build_name_search_buf(name, cid);
            match buf {
                Some(buf) => {
                    if buf.len() + buf_full.len() > MAX_UDP_SEND {
                        udp.send(&buf_full).await;
                        buf_full = build_version_buf();
                    }
                    buf_full.extend_from_slice(&buf);
                }
                None => {}
            }
        }

        // send residual
        udp.send(&buf_full).await;
    }

    // ------------- getters --------------------

    pub fn by_name(self: &Self) -> RwLockReadGuard<'_, HashMap<String, Arc<Channel>>> {
        self.by_name.read().unwrap()
    }

    pub fn by_cid(self: &Self) -> RwLockReadGuard<'_, HashMap<u32, Arc<Channel>>> {
        self.by_cid.read().unwrap()
    }

    pub fn by_name_mut(self: &Self) -> RwLockWriteGuard<'_, HashMap<String, Arc<Channel>>> {
        self.by_name.write().unwrap()
    }

    pub fn by_cid_mut(self: &Self) -> RwLockWriteGuard<'_, HashMap<u32, Arc<Channel>>> {
        self.by_cid.write().unwrap()
    }

    pub fn channel_by_cid(self: &Self, cid: u32) -> Option<Arc<Channel>> {
        let by_cid = self.by_cid();
        by_cid.get(&cid).cloned()
    }

    pub fn channel_by_name(self: &Self, name: &str) -> Option<Arc<Channel>> {
        let by_name = self.by_name();
        by_name.get(name).cloned()
    }

    pub fn next_cid(self: &Self) -> u32 {
        let id = self.next_cid.fetch_add(1, Ordering::Relaxed);
        id
    }
}
