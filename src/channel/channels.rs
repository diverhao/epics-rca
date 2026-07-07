use crate::ca::message::{CaMsg, MAX_UDP_SEND};
use crate::channel;
use crate::channel::channel::ChannelCallback;
use crate::channel::dbr::{ChannelAccessRights, ChannelSeverity, ChannelState, ChannelStatus};
use crate::channel::dbr::{DbrType, DbrValue};
use crate::channel::monitor::{MonitorDataType, MonitorState};
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
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use tokio::time::{self, Duration};

#[derive(Clone)]
pub struct ChannelIo {
    pub cid: u32,
    pub callback: Option<ChannelCallback>,
    // user requested
    pub dbr_type: Option<MonitorDataType>,
    pub data_count: Option<u32>,
}

pub struct Channels {
    searching_by_cid: RwLock<HashMap<u32, Arc<Channel>>>,
    not_searching_by_cid: RwLock<HashMap<u32, Arc<Channel>>>,
    next_cid: AtomicU32,
    next_ioid: AtomicU32, // read and write
    searching_ca: AtomicBool,
    ios: RwLock<HashMap<u32, ChannelIo>>,
    pub resolved_count: AtomicU32,
}

impl Channels {
    pub fn new() -> Self {
        Self {
            searching_by_cid: RwLock::new(HashMap::new()),
            not_searching_by_cid: RwLock::new(HashMap::new()),
            next_cid: AtomicU32::new(1),
            next_ioid: AtomicU32::new(0),
            searching_ca: AtomicBool::new(false),
            ios: RwLock::new(HashMap::new()),
            resolved_count: AtomicU32::new(0),
        }
    }

    pub fn create_channel(self: &Self, name: &str) -> Arc<Channel> {
        let id = self.next_cid();
        let channel = Arc::new(Channel::new(name, id));

        let mut by_cid = self.searching_by_cid_mut();
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

    pub async fn destroy_channels(self: &Self) {
        let channels: Vec<Arc<Channel>> = self.searching_by_cid().values().cloned().collect();
        for channel in channels {
            channel.destroy().await;
        }
        let channels: Vec<Arc<Channel>> = self.not_searching_by_cid().values().cloned().collect();
        for channel in channels {
            channel.destroy().await;
        }
    }

    // ------------- IO ------------------------

    pub fn clear_ios(self: &Self) {
        self.ios_mut().clear();
    }

    pub fn remove_io_by_ioid(self: &Self, ioid: u32) -> Option<ChannelIo> {
        self.ios_mut().remove(&ioid)
    }

    pub fn remove_io_by_cid(self: &Self, cid: u32) {
        self.ios_mut().retain(|_, io| io.cid != cid);
    }

    fn ios(self: &Self) -> RwLockReadGuard<'_, HashMap<u32, ChannelIo>> {
        self.ios.read().unwrap()
    }

    pub fn ios_of_cid(self: &Self, cid: u32) -> Vec<(u32, ChannelIo)> {
        self.ios()
            .iter()
            .filter(|(_, io)| io.cid == cid)
            .map(|(ioid, io)| (*ioid, io.clone()))
            .collect()
    }

    pub fn io(self: &Self, ioid: u32) -> Option<ChannelIo> {
        self.ios().get(&ioid).cloned()
    }

    fn ios_mut(self: &Self) -> RwLockWriteGuard<'_, HashMap<u32, ChannelIo>> {
        self.ios.write().unwrap()
    }

    pub fn add_io(
        self: &Self,
        ioid: u32,
        cid: u32,
        dbr_type: Option<MonitorDataType>,
        data_count: Option<u32>,
        callback: Option<ChannelCallback>,
    ) {
        self.ios_mut().insert(
            ioid,
            ChannelIo {
                cid,
                dbr_type,
                data_count,
                callback,
            },
        );
    }

    pub fn next_ioid(self: &Self) -> u32 {
        let id = self.next_ioid.fetch_add(1, Ordering::Relaxed);
        id
    }

    // -------------- channels -----------------

    pub fn searching_by_cid(self: &Self) -> RwLockReadGuard<'_, HashMap<u32, Arc<Channel>>> {
        self.searching_by_cid.read().unwrap()
    }

    pub fn searching_by_cid_mut(self: &Self) -> RwLockWriteGuard<'_, HashMap<u32, Arc<Channel>>> {
        self.searching_by_cid.write().unwrap()
    }

    pub fn not_searching_by_cid(self: &Self) -> RwLockReadGuard<'_, HashMap<u32, Arc<Channel>>> {
        self.not_searching_by_cid.read().unwrap()
    }

    pub fn not_searching_by_cid_mut(
        self: &Self,
    ) -> RwLockWriteGuard<'_, HashMap<u32, Arc<Channel>>> {
        self.not_searching_by_cid.write().unwrap()
    }

    pub fn channel_by_cid(self: &Self, cid: u32) -> Option<Arc<Channel>> {
        let searching = self.searching_by_cid();
        let not_searching = self.not_searching_by_cid();

        let channel = searching.get(&cid);
        match channel {
            Some(channel) => return Some(channel.clone()),
            None => {
                let channel = not_searching.get(&cid);
                match channel {
                    Some(channel) => {
                        return Some(channel.clone());
                    }
                    None => return None,
                }
            }
        }
    }

    pub fn remove_by_cid(self: &Self, cid: u32) {
        self.searching_by_cid_mut().remove(&cid);
        self.not_searching_by_cid_mut().remove(&cid);
    }

    pub fn searching_channel_by_cid(self: &Self, cid: u32) -> Option<Arc<Channel>> {
        let by_cid = self.searching_by_cid();
        by_cid.get(&cid).cloned()
    }

    pub fn not_searching_channel_by_cid(self: &Self, cid: u32) -> Option<Arc<Channel>> {
        let by_cid = self.not_searching_by_cid();
        by_cid.get(&cid).cloned()
    }

    pub fn move_to_searching_by_cid(self: &Self, cid: u32) {
        // lock both maps
        let mut searching = self.searching_by_cid_mut();
        let mut not_searching = self.not_searching_by_cid_mut();

        if let Some(channel) = not_searching.remove(&cid) {
            searching.insert(cid, channel);
        }
    }

    pub fn move_to_not_searching_by_cid(&self, cid: u32) {
        // lock both maps
        let mut searching = self.searching_by_cid_mut();
        let mut not_searching = self.not_searching_by_cid_mut();

        if let Some(channel) = searching.remove(&cid) {
            not_searching.insert(cid, channel);
        }
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
        // in case there is a second search_ca() invoked
        if self.searching_ca.swap(true, Ordering::AcqRel) {
            debug!("CA search is already running, skip this tick");
            return;
        }

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
        let context = get_context();
        let udp = context.udp();

        let mut buf: Vec<Vec<u8>> = vec![];
        let mut buf_packet = Vec::with_capacity(MAX_UDP_SEND);

        let version_buf: Vec<u8> = CaMsg::build_version(udp.ca_addr_list()).to_buf();
        // msgs_size = version_msg.size();
        // msgs.push(version_msg);
        buf_packet.extend_from_slice(&version_buf);

        // let mut name_searching_counter = 0;
        // let mut name_found_counter = 0;
        // let mut tcp_connected_counter = 0;
        // let mut created_counter = 0;
        // let mut destroyed_counter = 0;
        // let mut monitor_not_running_counter = 0;
        // let mut monitor_starting_counter = 0;
        // let mut monitor_running_counter = 0;

        // let mut version_count = 0;

        for channel in self.searching_by_cid().values() {
            let name = channel.name();
            let cid = channel.cid();
            let channel_state = channel.state();
            if channel_state != ChannelState::NameSearching {
                debug!(
                    "Channel {name} is already found ({:?}), skip name search",
                    channel_state
                );
                continue;
            }

            // match channel_state {
            //     ChannelState::NameSearching => name_searching_counter += 1,
            //     ChannelState::NameFound => name_found_counter += 1,
            //     ChannelState::TcpConnected => tcp_connected_counter += 1,
            //     ChannelState::Created => created_counter += 1,
            //     ChannelState::Destroyed => destroyed_counter += 1,
            // }

            // let monitor_state = channel.monitor_state();
            // match monitor_state {
            //     MonitorState::NotRunning => monitor_not_running_counter += 1,
            //     MonitorState::Starting => monitor_starting_counter += 1,
            //     MonitorState::Running => monitor_running_counter += 1,
            // }

            channel.set_state(ChannelState::NameSearching, true);

            let search_counter = channel.search_counter();
            if !search_counter.is_power_of_two() {
                channel.increment_search_counter();
                debug!("Skip this search for {name} as search counter = {search_counter}");
                continue;
            } else {
                channel.increment_search_counter();
            }

            let name_search_buf = CaMsg::build_name_search(name, cid, udp.ca_addr_list()).to_buf();
            if buf_packet.len() + name_search_buf.len() > MAX_UDP_SEND {
                buf.push(buf_packet);

                buf_packet = Vec::with_capacity(MAX_UDP_SEND);
                let version_buf: Vec<u8> = CaMsg::build_version(udp.ca_addr_list()).to_buf();
                buf_packet.extend_from_slice(&version_buf);
            }
            buf_packet.extend_from_slice(&name_search_buf);

            // if msgs_size + name_search_msg.size() > MAX_UDP_SEND as u32 {
            //     // patch with one or few CA_PROTO_VERSION messages
            //     loop {
            //         let version_msg = CaMsg::build_version(udp.ca_addr_list());
            //         if msgs_size + version_msg.size() > MAX_UDP_SEND as u32 {
            //             break;
            //         }
            //         msgs_size += version_msg.size();
            //         msgs.push(version_msg);
            //     }
            //     let version_msg = CaMsg::build_version(udp.ca_addr_list());
            //     msgs_size = version_msg.size();
            //     msgs.push(version_msg);
            //     version_count += 1;
            // }

            // msgs_size = msgs_size + name_search_msg.size();
            // msgs.push(CaMsg::build_version(udp.ca_addr_list()));
            // msgs.push(name_search_msg);
        }

        // println!(
        //     "channel states: name_searching={name_searching_counter}, name_found={name_found_counter}, tcp_connected={tcp_connected_counter}, created={created_counter}, destroyed={destroyed_counter}"
        // );
        // println!(
        //     "msgs len {}, version count {}, searing channels: {}, not-searching channels: {}",
        //     msgs.len(),
        //     version_count,
        //     self.searching_by_cid().len(),
        //     self.not_searching_by_cid().len()
        // );
        // if monitor_running_counter == created_counter {
        //     println!("{}", context.tcps().tcps().len());
        //     println!(
        //         "======================================, {}, {}, {}, {}",
        //         monitor_running_counter, name_found_counter, tcp_connected_counter, created_counter
        //     );
        // }
        buf.push(buf_packet);

        for buf_packet in buf {
            if buf_packet.len() > 16 {
                udp.send_buf(&buf_packet).await;
            }
        }

        // if buf.len() > 1 {
        //     udp.send_buf(&buf).await;
        // } else {
        // }
    }
}

impl fmt::Display for Channels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ios: Vec<(u32, u32)> = self
            .ios()
            .iter()
            .map(|(ioid, io)| (*ioid, io.cid))
            .collect();
        ios.sort_by_key(|(ioid, _)| *ioid);

        let mut channels: Vec<Arc<Channel>> = self.searching_by_cid().values().cloned().collect();
        channels.extend(self.not_searching_by_cid().values().cloned());
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
