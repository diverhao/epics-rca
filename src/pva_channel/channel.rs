use std::sync::{RwLock, atomic::{AtomicU32, Ordering}};

use tokio::sync::Notify;

use crate::pva_channel::meta::Meta;


pub struct Channel {
    // fixed, never change
    name: String,
    cid: u32, // client ID
    // dynamic data
    // from server, sid, access right, data count, data type,
    // search: state, server address
    // meta: RwLock<Meta>,
    search_counter: AtomicU32,
    // state_change_notifier: Notify,
    // monitor: RwLock<Monitor>,
}

impl Channel {
    // ---------------- getter -----------------
    pub fn name(self: &Self) -> &String {
        &self.name
    }

    pub fn cid(self: &Self) -> u32 {
        self.cid
    }

    pub fn search_counter(&self) -> u32 {
        self.search_counter.load(Ordering::Relaxed)
    }

    // ------------- data setter ----------------

    pub fn increment_search_counter(&self) -> u32 {
        self.search_counter.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn reset_search_counter(&self) -> u32 {
        self.search_counter.swap(0, Ordering::Relaxed)
    }

    pub fn set_search_counter(&self, counter: u32) {
        self.search_counter.store(counter, Ordering::Relaxed);
    }
}