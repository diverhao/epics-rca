use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy)]
pub struct TrafficStats {
    pub build_version: usize,
    pub build_name_search: usize,
    pub build_create_chan: usize,
    pub build_event_add: usize,
    pub build_other: usize,
    pub built_payload_bytes: usize,
    pub msg_to_buf_calls: usize,
    pub msg_to_buf_bytes: usize,
    pub from_buf_msgs: usize,
    pub from_buf_payload_bytes: usize,
    pub search_batch_max_len: usize,
    pub search_batch_max_capacity: usize,
    pub tcp_queue_max_len: usize,
    pub tcp_queue_max_capacity: usize,
    pub tcp_drain_max_len: usize,
    pub tcp_drain_max_capacity: usize,
    pub tcp_write_buf_max_capacity: usize,
    pub udp_write_buf_max_capacity: usize,
}

static BUILD_VERSION: AtomicUsize = AtomicUsize::new(0);
static BUILD_NAME_SEARCH: AtomicUsize = AtomicUsize::new(0);
static BUILD_CREATE_CHAN: AtomicUsize = AtomicUsize::new(0);
static BUILD_EVENT_ADD: AtomicUsize = AtomicUsize::new(0);
static BUILD_OTHER: AtomicUsize = AtomicUsize::new(0);
static BUILT_PAYLOAD_BYTES: AtomicUsize = AtomicUsize::new(0);
static MSG_TO_BUF_CALLS: AtomicUsize = AtomicUsize::new(0);
static MSG_TO_BUF_BYTES: AtomicUsize = AtomicUsize::new(0);
static FROM_BUF_MSGS: AtomicUsize = AtomicUsize::new(0);
static FROM_BUF_PAYLOAD_BYTES: AtomicUsize = AtomicUsize::new(0);
static SEARCH_BATCH_MAX_LEN: AtomicUsize = AtomicUsize::new(0);
static SEARCH_BATCH_MAX_CAPACITY: AtomicUsize = AtomicUsize::new(0);
static TCP_QUEUE_MAX_LEN: AtomicUsize = AtomicUsize::new(0);
static TCP_QUEUE_MAX_CAPACITY: AtomicUsize = AtomicUsize::new(0);
static TCP_DRAIN_MAX_LEN: AtomicUsize = AtomicUsize::new(0);
static TCP_DRAIN_MAX_CAPACITY: AtomicUsize = AtomicUsize::new(0);
static TCP_WRITE_BUF_MAX_CAPACITY: AtomicUsize = AtomicUsize::new(0);
static UDP_WRITE_BUF_MAX_CAPACITY: AtomicUsize = AtomicUsize::new(0);

pub fn record_build_version() {
    BUILD_VERSION.fetch_add(1, Ordering::Relaxed);
}

pub fn record_build_name_search(payload_bytes: usize) {
    BUILD_NAME_SEARCH.fetch_add(1, Ordering::Relaxed);
    BUILT_PAYLOAD_BYTES.fetch_add(payload_bytes, Ordering::Relaxed);
}

pub fn record_build_create_chan(payload_bytes: usize) {
    BUILD_CREATE_CHAN.fetch_add(1, Ordering::Relaxed);
    BUILT_PAYLOAD_BYTES.fetch_add(payload_bytes, Ordering::Relaxed);
}

pub fn record_build_event_add(payload_bytes: usize) {
    BUILD_EVENT_ADD.fetch_add(1, Ordering::Relaxed);
    BUILT_PAYLOAD_BYTES.fetch_add(payload_bytes, Ordering::Relaxed);
}

pub fn record_build_other(payload_bytes: usize) {
    BUILD_OTHER.fetch_add(1, Ordering::Relaxed);
    BUILT_PAYLOAD_BYTES.fetch_add(payload_bytes, Ordering::Relaxed);
}

pub fn record_msg_to_buf(bytes: usize) {
    MSG_TO_BUF_CALLS.fetch_add(1, Ordering::Relaxed);
    MSG_TO_BUF_BYTES.fetch_add(bytes, Ordering::Relaxed);
}

pub fn record_from_buf_msg(payload_bytes: usize) {
    FROM_BUF_MSGS.fetch_add(1, Ordering::Relaxed);
    FROM_BUF_PAYLOAD_BYTES.fetch_add(payload_bytes, Ordering::Relaxed);
}

pub fn record_search_batch(len: usize, capacity: usize) {
    update_max(&SEARCH_BATCH_MAX_LEN, len);
    update_max(&SEARCH_BATCH_MAX_CAPACITY, capacity);
}

pub fn record_tcp_queue(len: usize, capacity: usize) {
    update_max(&TCP_QUEUE_MAX_LEN, len);
    update_max(&TCP_QUEUE_MAX_CAPACITY, capacity);
}

pub fn record_tcp_drain(len: usize, capacity: usize) {
    update_max(&TCP_DRAIN_MAX_LEN, len);
    update_max(&TCP_DRAIN_MAX_CAPACITY, capacity);
}

pub fn record_tcp_write_buf(capacity: usize) {
    update_max(&TCP_WRITE_BUF_MAX_CAPACITY, capacity);
}

pub fn record_udp_write_buf(capacity: usize) {
    update_max(&UDP_WRITE_BUF_MAX_CAPACITY, capacity);
}

pub fn snapshot() -> TrafficStats {
    TrafficStats {
        build_version: BUILD_VERSION.load(Ordering::Relaxed),
        build_name_search: BUILD_NAME_SEARCH.load(Ordering::Relaxed),
        build_create_chan: BUILD_CREATE_CHAN.load(Ordering::Relaxed),
        build_event_add: BUILD_EVENT_ADD.load(Ordering::Relaxed),
        build_other: BUILD_OTHER.load(Ordering::Relaxed),
        built_payload_bytes: BUILT_PAYLOAD_BYTES.load(Ordering::Relaxed),
        msg_to_buf_calls: MSG_TO_BUF_CALLS.load(Ordering::Relaxed),
        msg_to_buf_bytes: MSG_TO_BUF_BYTES.load(Ordering::Relaxed),
        from_buf_msgs: FROM_BUF_MSGS.load(Ordering::Relaxed),
        from_buf_payload_bytes: FROM_BUF_PAYLOAD_BYTES.load(Ordering::Relaxed),
        search_batch_max_len: SEARCH_BATCH_MAX_LEN.load(Ordering::Relaxed),
        search_batch_max_capacity: SEARCH_BATCH_MAX_CAPACITY.load(Ordering::Relaxed),
        tcp_queue_max_len: TCP_QUEUE_MAX_LEN.load(Ordering::Relaxed),
        tcp_queue_max_capacity: TCP_QUEUE_MAX_CAPACITY.load(Ordering::Relaxed),
        tcp_drain_max_len: TCP_DRAIN_MAX_LEN.load(Ordering::Relaxed),
        tcp_drain_max_capacity: TCP_DRAIN_MAX_CAPACITY.load(Ordering::Relaxed),
        tcp_write_buf_max_capacity: TCP_WRITE_BUF_MAX_CAPACITY.load(Ordering::Relaxed),
        udp_write_buf_max_capacity: UDP_WRITE_BUF_MAX_CAPACITY.load(Ordering::Relaxed),
    }
}

fn update_max(atom: &AtomicUsize, value: usize) {
    let mut current = atom.load(Ordering::Relaxed);
    while value > current {
        match atom.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }
}
