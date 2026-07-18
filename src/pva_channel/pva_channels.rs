use crate::ca_channel::dbr::ChannelState;
use crate::ca_message::message::MAX_UDP_SEND;
use crate::env::env::EnvType;
use crate::pva_message::header::MsgEndian;
use crate::pva_message::message::build_search;
use crate::{context::context::get_context, pva_channel::pva_channel::PvaChannel};
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

// #[derive(Clone)]
// pub struct ChannelIo {
//     pub cid: u32,
//     pub callback: Option<ChannelCallback>,
//     // user requested
//     pub data_type: Option<MonitorDataType>,
//     pub data_count: Option<u32>,
// }

pub struct PvaChannels {
    searching_by_cid: RwLock<HashMap<u32, Arc<PvaChannel>>>,
    not_searching_by_cid: RwLock<HashMap<u32, Arc<PvaChannel>>>,
    next_cid: AtomicU32,
    next_ioid: AtomicU32, // read and write
    searching_pva: AtomicBool,
    // ios: RwLock<HashMap<u32, ChannelIo>>,
    // pub resolved_count: AtomicU32,
}

impl PvaChannels {
    pub fn new() -> Self {
        Self {
            searching_by_cid: RwLock::new(HashMap::new()),
            not_searching_by_cid: RwLock::new(HashMap::new()),
            next_cid: AtomicU32::new(1),
            next_ioid: AtomicU32::new(0),
            searching_pva: AtomicBool::new(false),
            // ios: RwLock::new(HashMap::new()),
            // resolved_count: AtomicU32::new(0),
        }
    }

    pub fn create_channel(self: &Self, name: &str) -> Arc<PvaChannel> {
        let id = self.next_cid();
        let channel = Arc::new(PvaChannel::new(name, id));

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
        let channels: Vec<Arc<PvaChannel>> = self.searching_by_cid().values().cloned().collect();
        for channel in channels {
            channel.destroy().await;
        }
        let channels: Vec<Arc<PvaChannel>> =
            self.not_searching_by_cid().values().cloned().collect();
        for channel in channels {
            channel.destroy().await;
        }
    }

    // ------------- IO ------------------------

    // pub fn clear_ios(self: &Self) {
    //     self.ios_mut().clear();
    // }

    // pub fn remove_io_by_ioid(self: &Self, ioid: u32) -> Option<ChannelIo> {
    //     self.ios_mut().remove(&ioid)
    // }

    // pub fn remove_io_by_cid(self: &Self, cid: u32) {
    //     self.ios_mut().retain(|_, io| io.cid != cid);
    // }

    // fn ios(self: &Self) -> RwLockReadGuard<'_, HashMap<u32, ChannelIo>> {
    //     self.ios.read().unwrap()
    // }

    // pub fn ios_of_cid(self: &Self, cid: u32) -> Vec<(u32, ChannelIo)> {
    //     self.ios()
    //         .iter()
    //         .filter(|(_, io)| io.cid == cid)
    //         .map(|(ioid, io)| (*ioid, io.clone()))
    //         .collect()
    // }

    // pub fn io(self: &Self, ioid: u32) -> Option<ChannelIo> {
    //     self.ios().get(&ioid).cloned()
    // }

    // fn ios_mut(self: &Self) -> RwLockWriteGuard<'_, HashMap<u32, ChannelIo>> {
    //     self.ios.write().unwrap()
    // }

    // pub fn add_io(
    //     self: &Self,
    //     ioid: u32,
    //     cid: u32,
    //     data_type: Option<MonitorDataType>,
    //     data_count: Option<u32>,
    //     callback: Option<ChannelCallback>,
    // ) {
    //     self.ios_mut().insert(
    //         ioid,
    //         ChannelIo {
    //             cid,
    //             data_type,
    //             data_count,
    //             callback,
    //         },
    //     );
    // }

    pub fn next_ioid(self: &Self) -> u32 {
        let id = self.next_ioid.fetch_add(1, Ordering::Relaxed);
        id
    }

    // -------------- channels -----------------

    pub fn searching_by_cid(self: &Self) -> RwLockReadGuard<'_, HashMap<u32, Arc<PvaChannel>>> {
        self.searching_by_cid.read().unwrap()
    }

    pub fn searching_by_cid_mut(
        self: &Self,
    ) -> RwLockWriteGuard<'_, HashMap<u32, Arc<PvaChannel>>> {
        self.searching_by_cid.write().unwrap()
    }

    pub fn not_searching_by_cid(self: &Self) -> RwLockReadGuard<'_, HashMap<u32, Arc<PvaChannel>>> {
        self.not_searching_by_cid.read().unwrap()
    }

    pub fn not_searching_by_cid_mut(
        self: &Self,
    ) -> RwLockWriteGuard<'_, HashMap<u32, Arc<PvaChannel>>> {
        self.not_searching_by_cid.write().unwrap()
    }

    pub fn channel_by_cid(self: &Self, cid: u32) -> Option<Arc<PvaChannel>> {
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

    pub fn searching_channel_by_cid(self: &Self, cid: u32) -> Option<Arc<PvaChannel>> {
        let by_cid = self.searching_by_cid();
        by_cid.get(&cid).cloned()
    }

    pub fn not_searching_channel_by_cid(self: &Self, cid: u32) -> Option<Arc<PvaChannel>> {
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
    pub fn start_search(self: Arc<Self>) {
        // in case there is a second search_ca() invoked
        if self.searching_pva.swap(true, Ordering::AcqRel) {
            debug!("CA search is already running, skip this tick");
            return;
        }

        let min_search_period = get_context()
            .env()
            .get_env("EPICS_PVA_MIN_SEARCH_PERIOD")
            .and_then(|value| match value {
                EnvType::Double(value) => Some(*value),
                _ => None,
            })
            .unwrap_or(0.1); // default to 0.1 second

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs_f64(min_search_period));

            loop {
                interval.tick().await;
                self.search().await;
            }
        });
    }

    /**
     * send out name search packets for all unconnected channels
     */
    async fn search(self: &Self) -> Result<(), String> {
        let context = get_context();
        let udp = context.udp();
        let response_addr = match udp.socket_v4().local_addr() {
            Ok(socket_addr) => socket_addr,
            Err(_) => return Err("".to_string()),
        };
        let endian = MsgEndian::Big;

        let mut packet_size: usize = 30;
        let mut channel_names_cids: Vec<(String, i32)> = vec![];

        for channel in self.searching_by_cid().values() {
            let name = channel.name();
            let cid = channel.cid();

            // cid = 4 bytes, 1 byte for name size, I'm betting the pv name length is less than 254
            packet_size += 4 + name.len() + 1;

            if packet_size > MAX_UDP_SEND {
                // send out
                let context = get_context();
                let udp = context.udp();
                let search_seq_id = udp.increment_search_seq_id() as i32;
                let buf = build_search(search_seq_id, endian, response_addr, &channel_names_cids)?;
                udp.send_buf(&buf);
                // clear names
                packet_size = 30;
                channel_names_cids.clear();
            } else {
                let channel_state = channel.state();
                if channel_state != ChannelState::NameSearching {
                    debug!(
                        "Channel {name} is already found ({:?}), skip name search",
                        channel_state
                    );
                    continue;
                }

                let search_counter = channel.search_counter();
                if !search_counter.is_power_of_two() {
                    channel.increment_search_counter();
                    debug!("Skip this search for {name} as search counter = {search_counter}");
                    continue;
                } else {
                    channel.increment_search_counter();
                }
                channel.set_state(ChannelState::NameSearching, true);
                channel_names_cids.push((name.to_string(), cid as i32));
            }
        }


        if channel_names_cids.len() > 0 {
            let context = get_context();
            let udp = context.udp();
            let search_seq_id = udp.increment_search_seq_id() as i32;
            let buf = build_search(search_seq_id, endian, response_addr, &channel_names_cids)?;
            udp.send_buf(&buf);
        }

        Ok(())
    }
}

// impl fmt::Display for PvaChannels {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let mut ios: Vec<(u32, u32)> = self
//             .ios()
//             .iter()
//             .map(|(ioid, io)| (*ioid, io.cid))
//             .collect();
//         ios.sort_by_key(|(ioid, _)| *ioid);

//         let mut channels: Vec<Arc<PvaChannel>> = self.searching_by_cid().values().cloned().collect();
//         channels.extend(self.not_searching_by_cid().values().cloned());
//         channels.sort_by_key(|channel| channel.cid());

//         writeln!(f, "PvaChannels {{")?;
//         writeln!(f, "    ios:")?;
//         writeln!(f, "        |ioid|cid|")?;
//         for (ioid, cid) in ios {
//             writeln!(f, "        |{}|{}|", ioid, cid)?;
//         }

//         writeln!(f, "    channels:")?;
//         for channel in channels {
//             let channel_text = channel.to_string().replace('\n', "\n        ");
//             writeln!(f, "        {}", channel_text.trim_start())?;
//         }
//         write!(f, "}}")
//     }
// }
