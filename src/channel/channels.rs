use crate::channel::channel::ChannelState;
use crate::context::context::CA_MINOR_VERSION;
use crate::udp::udp::CaCmd;
use crate::{channel::channel::Channel, context::context::get_context, udp::udp::MAX_UDP_SEND};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;
use std::sync::atomic::{AtomicU32, Ordering};

enum SearchReplyFlag {
    DoReply = 0x0a,
    DontReply = 0x05,
}

pub struct Channels {
    by_name: RwLock<HashMap<String, Arc<Channel>>>,
    by_cid: RwLock<HashMap<u32, Arc<Channel>>>,
    next_cid: AtomicU32,
}

impl Channels {
    pub fn new() -> Self {
        Self {
            by_name: RwLock::new(HashMap::new()),
            by_cid: RwLock::new(HashMap::new()),
            next_cid: AtomicU32::new(0),
        }
    }

    pub fn create_channel(self: &Self, name: &str) -> Arc<Channel> {
        let mut by_name = self.by_name_mut();
        if let Some(channel) = by_name.get(name) {
            return Arc::clone(channel);
        }

        let id = self.next_cid();
        let channel = Arc::new(Channel::new(name, id));
        let mut by_cid = self.by_cid_mut();
        by_name.insert(String::from(name), Arc::clone(&channel));
        by_cid.insert(id, Arc::clone(&channel));
        channel
    }

    // --------------- network -----------------

    /**
     * Build Channel Access name search payload for CA_PROTO_SEARCH
     */
    fn build_search_buf_ca(self: &Self) -> Vec<Vec<u8>> {
        let mut payloads: Vec<Vec<u8>> = vec![];
        let mut payload: Vec<u8> = vec![];
        let by_name = self.by_name();

        for (name, channel) in by_name.iter() {
            if channel.state() != ChannelState::NeverConnected
                && channel.state() != ChannelState::NameSearching
            {
                continue;
            }
            let name_bytes = name.as_bytes();
            if payload.len() + name_bytes.len() + 16 > MAX_UDP_SEND {
                payloads.push(payload.clone());
                payload.clear();
            }
            payload.extend_from_slice(name_bytes);
            payload.push(0);
        }

        // push the final non-empty payload
        if !payload.is_empty() {
            payloads.push(payload);
        }

        payloads
    }

    pub async fn search_ca(self: &Self) {
        let payloads = self.build_search_payloads_ca();
        let context = get_context();
        let udp = context.udp();
        for payload in payloads {
            udp.send_ca(
                CaCmd::CaProtoSearch,
                SearchReplyFlag::DontReply as u32,
                CA_MINOR_VERSION as u32,
                1,
                1,
                payload,
            )
            .await;
        }
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
