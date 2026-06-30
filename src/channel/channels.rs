use crate::ca::message::{CaMsg, MAX_UDP_SEND};
use crate::channel;
use crate::channel::dbr::{ChannelAccessRights, ChannelSeverity, ChannelState, ChannelStatus};
use crate::channel::dbr::{DbrType, DbrValue};
use crate::env::env::EnvType;
use crate::{channel::channel::Channel, context::context::get_context};
use log::{debug, warn};
use std::char::MAX;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::oneshot::Sender;
use tokio::time::{self, Duration};

pub struct Channels {
    by_name: RwLock<HashMap<String, Arc<Channel>>>,
    by_cid: RwLock<HashMap<u32, Arc<Channel>>>,
    next_cid: AtomicU32,
    next_ioid: AtomicU32,                            // read and write
    ios: RwLock<HashMap<u32, (Sender<CaMsg>, u32)>>, // HashMap<ioid, (Sender<msg>, cid)>
}

impl Channels {
    pub fn new() -> Self {
        Self {
            by_name: RwLock::new(HashMap::new()),
            by_cid: RwLock::new(HashMap::new()),
            next_cid: AtomicU32::new(1),
            next_ioid: AtomicU32::new(0),
            ios: RwLock::new(HashMap::new()),
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
        debug!("Channel {name} created with id {id}");
        channel
    }

    pub fn create_channels(self: &Self, names: Vec<String>) {
        for name in names {
            self.create_channel(name.as_str());
        }
    }

    pub async fn destroy_channel_by_cid(self: &Self, cid: u32) {
        let channel = self.channel_by_cid(cid);
        match channel {
            Some(channel) => channel.destroy().await,
            None => {}
        }
    }

    pub async fn destroy_channel_by_name(self: &Self, name: String) {
        let channel = self.channel_by_name(&name);
        match channel {
            Some(channel) => channel.destroy().await,
            None => {}
        }
    }

    pub async fn destroy_channels(self: &Self) {
        let channels: Vec<Arc<Channel>> = self.by_cid().values().cloned().collect();
        for channel in channels {
            channel.destroy().await;
        }
    }

    // ------------- IO ------------------------

    pub fn clear_ios(self: &Self) {
        self.ios_mut().clear();
    }

    pub fn remove_io_by_ioid(self: &Self, ioid: u32) -> Option<(Sender<CaMsg>, u32)> {
        self.ios_mut().remove(&ioid)
    }

    pub fn remove_io_by_cid(self: &Self, cid: u32) {
        self.ios_mut().retain(|_, (_, io_cid)| *io_cid != cid);
    }

    fn ios(self: &Self) -> RwLockReadGuard<'_, HashMap<u32, (Sender<CaMsg>, u32)>> {
        self.ios.read().unwrap()
    }

    fn ios_mut(self: &Self) -> RwLockWriteGuard<'_, HashMap<u32, (Sender<CaMsg>, u32)>> {
        self.ios.write().unwrap()
    }

    pub fn add_io(self: &Self, ioid: u32, tx: Sender<CaMsg>, cid: u32) {
        self.ios_mut().insert(ioid, (tx, cid));
    }

    pub fn next_ioid(self: &Self) -> u32 {
        let id = self.next_ioid.fetch_add(1, Ordering::Relaxed);
        id
    }

    // -------------- channels -----------------

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

    pub fn remove_by_cid_channel(self: &Self, cid: u32) {
        self.by_cid_mut().remove(&cid);
    }

    pub fn remove_by_name_channel(self: &Self, name: String) {
        self.by_name_mut().remove(&name);
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

    // --------------- search channel -----------------

    /**
     * Start periodic task to search
     */
    pub fn start_search_ca(self: Arc<Self>) {
        let min_search_period = get_context()
            .env()
            .get_env("EPICS_CA_MIN_SEARCH_PERIOD")
            .and_then(|value| match value {
                EnvType::Double(value) => Some(*value),
                _ => None,
            })
            .unwrap_or(0.1); // default to 0.1 second

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs_f64(min_search_period));

            loop {
                interval.tick().await;
                self.search_ca().await;
            }
        });
    }

    /**
     * send out name search packets for all unconnected channels
     */
    async fn search_ca(self: &Self) {
        let channels: Vec<Arc<Channel>> = self.by_cid().values().cloned().collect();
        let context = get_context();
        let udp = context.udp();

        let mut msgs: Vec<CaMsg> = vec![];
        msgs.push(CaMsg::build_version(udp.ca_addr_list()));
        let mut buf_len: u32 = 16;

        for channel in channels {
            let name = channel.name();
            let cid = channel.cid();
            let channel_state = channel.state();
            if channel_state != ChannelState::NeverConnected
                && channel_state != ChannelState::NameSearching
            {
                debug!("Channel {name} is already found ({:?}), skip name search", channel_state);
                continue;
            }
            let search_counter = channel.search_counter();
            channel.increment_search_counter();
            if !search_counter.is_power_of_two() {
                debug!("Skip this search for {name} as search counter = {search_counter}");
                continue;
            }
            debug!("Searching {name}");
            let msg = CaMsg::build_name_search(name, cid, udp.ca_addr_list());

            match msg {
                Ok(msg) => {
                    channel.set_state(ChannelState::NameSearching, true);
                    if buf_len + msg.size() as u32 > MAX_UDP_SEND as u32 {
                        udp.send_msgs(&msgs).await;
                        msgs.clear();
                        msgs.push(CaMsg::build_version(udp.ca_addr_list()));
                        buf_len = 16;
                    }
                    msgs.push(msg);
                }
                Err(_) => {
                    // skip
                }
            }
        }
        if msgs.len() > 1 {
            udp.send_msgs(&msgs).await;
        }
    }
}

impl fmt::Display for Channels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ios: Vec<(u32, u32)> = self
            .ios()
            .iter()
            .map(|(ioid, (_, cid))| (*ioid, *cid))
            .collect();
        ios.sort_by_key(|(ioid, _)| *ioid);

        let mut channels: Vec<Arc<Channel>> = self.by_cid().values().cloned().collect();
        channels.sort_by_key(|channel| channel.cid());

        writeln!(f, "Channels {{")?;
        writeln!(f, "    ios:")?;
        writeln!(f, "        |ioid|cid|")?;
        for (ioid, cid) in ios {
            writeln!(f, "        |{}|{}|", ioid, cid)?;
        }

        writeln!(f, "    channels:")?;
        for channel in channels {
            let channel_text = channel.to_string().replace('\n', "\n        ");
            writeln!(f, "        {}", channel_text.trim_start())?;
        }
        write!(f, "}}")
    }
}
