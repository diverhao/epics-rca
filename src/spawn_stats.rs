use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Copy)]
pub enum SpawnSite {
    UdpV4Listen,
    UdpV6Listen,
    SearchCaLoop,
    TcpWriteLoop,
    TcpReadLoop,
    TcpCheckAliveLoop,
    ChannelConnect,
    ChannelReconnect,
}

#[derive(Debug, Clone, Copy)]
pub struct SpawnStats {
    pub total: usize,
    pub udp_v4_listen: usize,
    pub udp_v6_listen: usize,
    pub search_ca_loop: usize,
    pub tcp_write_loop: usize,
    pub tcp_read_loop: usize,
    pub tcp_check_alive_loop: usize,
    pub channel_connect: usize,
    pub channel_reconnect: usize,
}

static TOTAL: AtomicUsize = AtomicUsize::new(0);
static UDP_V4_LISTEN: AtomicUsize = AtomicUsize::new(0);
static UDP_V6_LISTEN: AtomicUsize = AtomicUsize::new(0);
static SEARCH_CA_LOOP: AtomicUsize = AtomicUsize::new(0);
static TCP_WRITE_LOOP: AtomicUsize = AtomicUsize::new(0);
static TCP_READ_LOOP: AtomicUsize = AtomicUsize::new(0);
static TCP_CHECK_ALIVE_LOOP: AtomicUsize = AtomicUsize::new(0);
static CHANNEL_CONNECT: AtomicUsize = AtomicUsize::new(0);
static CHANNEL_RECONNECT: AtomicUsize = AtomicUsize::new(0);

pub fn spawn<F>(site: SpawnSite, future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    TOTAL.fetch_add(1, Ordering::Relaxed);
    counter(site).fetch_add(1, Ordering::Relaxed);
    tokio::spawn(future)
}

pub fn snapshot() -> SpawnStats {
    SpawnStats {
        total: TOTAL.load(Ordering::Relaxed),
        udp_v4_listen: UDP_V4_LISTEN.load(Ordering::Relaxed),
        udp_v6_listen: UDP_V6_LISTEN.load(Ordering::Relaxed),
        search_ca_loop: SEARCH_CA_LOOP.load(Ordering::Relaxed),
        tcp_write_loop: TCP_WRITE_LOOP.load(Ordering::Relaxed),
        tcp_read_loop: TCP_READ_LOOP.load(Ordering::Relaxed),
        tcp_check_alive_loop: TCP_CHECK_ALIVE_LOOP.load(Ordering::Relaxed),
        channel_connect: CHANNEL_CONNECT.load(Ordering::Relaxed),
        channel_reconnect: CHANNEL_RECONNECT.load(Ordering::Relaxed),
    }
}

fn counter(site: SpawnSite) -> &'static AtomicUsize {
    match site {
        SpawnSite::UdpV4Listen => &UDP_V4_LISTEN,
        SpawnSite::UdpV6Listen => &UDP_V6_LISTEN,
        SpawnSite::SearchCaLoop => &SEARCH_CA_LOOP,
        SpawnSite::TcpWriteLoop => &TCP_WRITE_LOOP,
        SpawnSite::TcpReadLoop => &TCP_READ_LOOP,
        SpawnSite::TcpCheckAliveLoop => &TCP_CHECK_ALIVE_LOOP,
        SpawnSite::ChannelConnect => &CHANNEL_CONNECT,
        SpawnSite::ChannelReconnect => &CHANNEL_RECONNECT,
    }
}
