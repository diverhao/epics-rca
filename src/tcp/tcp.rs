use crate::ca::message::CaMsg;
use crate::ca::message_handler::handle_tcp_msgs;
use crate::channel;
use crate::context::context::get_context;
use log::debug;
use log::error;
use log::warn;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::collections::vec_deque;
use std::hash::Hash;
use std::sync::atomic::AtomicU32;
use std::time::Instant;
use std::{
    net::SocketAddr,
    sync::{
        Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Notify;
use tokio::sync::{Mutex, MutexGuard};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio::time::{self, Duration};

const TCP_READ_BUF_SIZE: usize = 64 * 1024;
const TCP_READ_COALESCE_US: u64 = 250;
const TCP_READ_MAX_DRAIN_ATTEMPTS: usize = 64;

#[derive(PartialEq, Copy, Clone)]
pub enum TcpState {
    NotConnected,
    Connecting,
    Connected,
}

pub struct TCP {
    reader: Arc<Mutex<Option<OwnedReadHalf>>>,
    writer: Arc<Mutex<Option<OwnedWriteHalf>>>,
    read_task: Mutex<Option<JoinHandle<()>>>,
    state: RwLock<TcpState>,
    connect_notify: Notify,
    addr: SocketAddr,
    cids: RwLock<HashSet<u32>>,
    check_alive_task: Mutex<Option<JoinHandle<()>>>,
    alive: AtomicBool,
    paused: AtomicBool,
    queue_msg_send: RwLock<VecDeque<CaMsg>>,
}

impl TCP {
    // pub async fn new(addr: SocketAddr) -> Result<Self, String> {
    // }

    pub fn start_to_write(self: Arc<Self>) {
        let tcp = Arc::new(self);
        tokio::spawn(async move {
            loop {
                if !tcp.is_connected() {
                    break;
                }
                // wait 5 ms
                tokio::time::sleep(Duration::from_millis(5)).await;
                tcp.send_queue_msg().await;
            }
        });
    }

    /**
     * Close tcp connection, update connection state, stop the periodic task, and unregister this
     * TCP from TCPs.
     */
    pub async fn disconnect(self: &Self, abort_read_task: bool, stop_check_alive_task: bool) {
        // Update connect state
        // self.set_state(TcpState::NotConnected);
        self.set_state(TcpState::NotConnected);

        // clear send queue
        self.queue_msg_send_mut().clear();

        // Stop self-checking alive, i.e. periodically sending out CA_PROTO_ECHO
        if stop_check_alive_task {
            self.stop_check_alive().await;
        }

        // Unregister from TCPs
        {
            get_context().tcps().remove_tcp(self.addr());
        }

        if let Some(handle) = self.read_task.lock().await.take() {
            // Cancel the tcp read loop even in reader.read(&mut buf_pending).await
            if abort_read_task {
                handle.abort();
            }
        }

        {
            if let Ok(mut writer_guard) = self.writer.try_lock() {
                if let Some(mut writer) = writer_guard.take() {
                    let _ =
                        tokio::time::timeout(Duration::from_millis(100), writer.shutdown()).await;
                }
            }
        }
    }

    /**
     * Reconnect all channels in this TCP
     *
     * This function is invoked after this TCP struct drops the connection
     */
    async fn reconnect_channels(self: &Self) {
        // make copy so that cids is not held across await
        let cids: Vec<u32> = self.cids().iter().copied().collect();
        for cid in cids {
            let channel = get_context().channels().channel_by_cid(cid);
            match channel {
                Some(channel) => {
                    // a lightweight await, will not block too long
                    // so it is ok to do it sequentially
                    channel.reconnect().await;
                }
                None => {}
            }
        }
    }
    pub async fn start_to_listen(self: Arc<Self>) {
        let mut read_task = self.read_task.lock().await;
        if read_task.is_some() {
            return;
        }

        let tcp = Arc::clone(&self);

        let handle = tokio::spawn(async move {
            let mut buf: Vec<u8> = vec![];
            let mut buf_pending: Vec<u8> = vec![0_u8; TCP_READ_BUF_SIZE];
            let mut reader = {
                let mut reader = tcp.reader.lock().await;
                match reader.take() {
                    Some(reader) => reader,
                    None => {
                        error!("No reader for TCP");
                        return;
                    }
                }
            };

            loop {
                // hygrid read: async reader.read() + wait 250 micro-seconds for more data + sync reader.try_read()
                //
                // async read() wakes up when there is one byte in socket
                // wait 250 micro-seconds for more data
                // sync try_read() obtain as much data as possible from socket
                let num_bytes = reader.read(&mut buf_pending).await;

                // check reader status
                match num_bytes {
                    Ok(0) => {
                        // tcp connection closed, e.g. ctrl-c in IOC
                        error!("TCP disconnected");
                        tcp.handle_tcp_failure(false, true).await;
                        break;
                    }
                    Ok(mut size) => {
                        if TCP_READ_COALESCE_US > 0 {
                            tokio::time::sleep(Duration::from_micros(TCP_READ_COALESCE_US)).await;
                        }

                        for _ in 0..TCP_READ_MAX_DRAIN_ATTEMPTS {
                            if size == buf_pending.len() {
                                break;
                            }
                            // synchronous reading
                            match reader.try_read(&mut buf_pending[size..]) {
                                Ok(0) => break,
                                Ok(extra_size) => size += extra_size,
                                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                                Err(err) => {
                                    tcp.handle_tcp_failure(false, true).await;
                                    error!("TCP error: {err}");
                                    return;
                                }
                            }
                        }

                        debug!("Received {size} TCP bytes from {}", tcp.addr());

                        buf.extend_from_slice(&buf_pending[..size]);

                        let msgs =
                            CaMsg::from_buf(&mut buf, Some(tcp.addr().clone()), vec![], true);
                        let src = *tcp.addr();
                        handle_tcp_msgs(&src, msgs);
                    }
                    Err(err) => {
                        tcp.handle_tcp_failure(false, true).await;
                        error!("TCP error: {err}");
                        break;
                    }
                }
            }
        });

        *read_task = Some(handle);
    }

    /**
     * Send out a buffer.
     *
     * Note: It does not invoke handle_tcp_failure() if there is anything wrong for the connection.
     *       The periodic CA_PROTO_ECHO will be responsible for detecting error from writing
     */
    async fn send_buf(&self, buf: &Vec<u8>) -> Result<(), String> {
        let mut writer = self.writer().await;

        match writer.as_mut() {
            Some(writer) => match writer.write_all(buf).await {
                Ok(_) => {
                    return Ok(());
                }
                Err(err) => {
                    error!("Send buffer failed: {err}");
                    return Err(err.to_string());
                }
            },
            None => {
                // no writer
                return Err("No TCP writer".to_string());
            }
        }
    }

    pub async fn send_queue_msg(self: &Self) {
        let mut buf: Vec<u8> = vec![];
        let msgs = {
            let mut queue = self.queue_msg_send_mut();
            let n = queue.len().min(100000);

            // println!("Sending {} tcp messages", n);
            queue.drain(..n).collect::<Vec<_>>()
        };

        if msgs.is_empty() {
            return;
        }

        for msg in msgs.iter() {
            debug!("\nSending TCP message {msg}");
            buf.extend_from_slice(&msg.to_buf());
        }

        match self.send_buf(&buf).await {
            Ok(_) => {
                // return Ok(());
            }
            Err(error) => {
                println!("fail");
            }
        }
    }

    /**
     * Push messages to TCP send queue
     */
    pub fn send_msgs(&self, msgs: Vec<CaMsg>) {
        if !self.is_connected() {
            return; // Err("TCP not connected".to_string());
        }
        // add to send-buffer queue
        self.add_to_queue_msg_send(msgs);
    }

    /**
     * Disconnect tcp connection, no matter if it is in good or bad state.
     * After this, this TCP struct will not be used anymore.
     *
     * Then reconnect all channels.
     */
    pub async fn handle_tcp_failure(
        self: &Self,
        abort_read_task: bool,
        stop_check_alive_task: bool,
    ) {
        debug!(
            "TCP {} failed, now we are handling this situation",
            self.addr()
        );
        self.disconnect(abort_read_task, stop_check_alive_task)
            .await;
        self.reconnect_channels().await;
    }

    // pub fn state(&self) -> RwLockReadGuard<'_, TcpState> {
    //     self.state.read().unwrap()
    // }

    async fn reader(self: &Self) -> MutexGuard<'_, Option<OwnedReadHalf>> {
        self.reader.lock().await
    }

    async fn writer(self: &Self) -> MutexGuard<'_, Option<OwnedWriteHalf>> {
        self.writer.lock().await
    }
    pub fn addr(self: &Self) -> &SocketAddr {
        &self.addr
    }

    pub fn add_cid(self: &Self, cid: u32) {
        self.cids_mut().insert(cid);
    }

    pub fn remove_cid(self: &Self, cid: u32) {
        self.cids_mut().remove(&cid);
    }

    // pub fn set_state(self: &Self, new_state: TcpState) {
    //     *self.state.write().unwrap() = new_state;
    // }

    pub fn cids(self: &Self) -> RwLockReadGuard<'_, HashSet<u32>> {
        self.cids.read().unwrap()
    }

    pub fn cids_mut(self: &Self) -> RwLockWriteGuard<'_, HashSet<u32>> {
        self.cids.write().unwrap()
    }

    pub async fn start_check_alive(self: Arc<Self>) {
        let mut task = self.check_alive_task.lock().await;
        if task.is_some() {
            return;
        }

        let tcp = Arc::clone(&self);
        let handle = tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                // check alive when tcp is connected
                if !tcp.is_connected() {
                    // this task starts after tcp is connected
                    // if the tcp status becomes not-connected, it means the tcp is broken
                    // we can stop this task
                    // tcp only has one state change: connected --> not-connected
                    break;
                }

                println!("current alive status is {}", tcp.alive());
                if tcp.alive() {
                    // will be reset to true when tcp receives the echo reply
                    tcp.set_alive(false);
                } else {
                    // TCP did not receive the echo reply in last 30 seconds
                    // Connection is broken, handle the failure
                    tcp.set_alive(true);
                    // tcp failed
                    error!("TCP alive check failed");
                    tcp.handle_tcp_failure(true, false).await;
                    break;
                }

                // send CA_PROTO_ECHO
                let addr = tcp.addr().clone();
                let msg = CaMsg::build_echo(&vec![addr]);
                tcp.send_msgs(vec![msg]);
            }
        });
        *task = Some(handle);
    }

    pub async fn stop_check_alive(self: &Self) {
        if let Some(handle) = self.check_alive_task.lock().await.take() {
            handle.abort();
        }
    }

    pub fn set_alive(&self, alive: bool) {
        self.alive
            .store(alive, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn alive(self: &Self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    pub fn is_connected(self: &Self) -> bool {
        // self.connected.read().unwrap().clone()
        self.state().clone() == TcpState::Connected
    }

    // pub fn set_connected(self: &Self, connected: bool) {
    //     *self.connected.write().unwrap() = connected;
    // }

    pub fn state(self: &Self) -> RwLockReadGuard<'_, TcpState> {
        self.state.read().unwrap()
    }

    pub fn set_state(self: &Self, new_state: TcpState) {
        *self.state.write().unwrap() = new_state;
    }

    pub async fn set_reader(self: &Self, reader: OwnedReadHalf) {
        *self.reader().await = Some(reader);
    }

    pub async fn set_writer(self: &Self, writer: OwnedWriteHalf) {
        *self.writer().await = Some(writer);
    }

    pub fn set_paused(&self, alive: bool) {
        self.paused
            .store(alive, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn paused(self: &Self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn pause(self: &Self) {
        let dest = self.addr().clone();
        let msg = CaMsg::build_event_off(&vec![dest]);
        self.set_paused(true);
        self.send_msgs(vec![msg]);
        // .await {
        //     Ok(_) => {}
        //     Err(_) => {
        //         self.set_paused(false);
        //     }
        // }
    }

    pub fn unpause(self: &Self) {
        let dest = self.addr().clone();
        let msg = CaMsg::build_event_on(&vec![dest]);
        self.set_paused(false);
        self.send_msgs(vec![msg]);
        // .await {
        //     Ok(_) => {}
        //     Err(_) => {
        //         self.set_paused(true);
        //     }
        // }
    }

    pub fn queue_msg_send_mut(self: &Self) -> RwLockWriteGuard<'_, VecDeque<CaMsg>> {
        self.queue_msg_send.write().unwrap()
    }

    pub fn queue_msg_send(self: &Self) -> RwLockReadGuard<'_, VecDeque<CaMsg>> {
        self.queue_msg_send.read().unwrap()
    }

    pub fn add_to_queue_msg_send(self: &Self, msgs: Vec<CaMsg>) {
        self.queue_msg_send_mut().extend(msgs);
    }
    pub async fn wait_connected(&self) -> Result<(), String> {
        loop {
            let notified = self.connect_notify.notified();
            let state = { *self.state() };

            match state {
                TcpState::Connected => return Ok(()),
                TcpState::NotConnected => {
                    return Err(format!("TCP {} failed to connect", self.addr()));
                }
                TcpState::Connecting => notified.await,
            }
        }
    }
}

pub struct TCPs {
    // use array instead of hashmap, usually there are not
    // many TCP clients (< 200), it is OK to iterate over
    // the vector
    tcps: RwLock<Vec<Arc<TCP>>>,
    connecting_tcps: RwLock<Vec<Arc<TCP>>>,
    pub wait_connected_count: AtomicU32,
    pub already_connected_count: AtomicU32,
    pub self_connect_count: AtomicU32,
    pub running_monitor_count: AtomicU32,
    pub start: std::time::Instant,
}

impl TCPs {
    pub fn new() -> Self {
        TCPs {
            tcps: RwLock::new(vec![]),
            connecting_tcps: RwLock::new(vec![]),
            wait_connected_count: AtomicU32::new(0),
            already_connected_count: AtomicU32::new(0),
            self_connect_count: AtomicU32::new(0),
            running_monitor_count: AtomicU32::new(0),
            start: Instant::now(),
        }
    }

    pub fn tcps(self: &Self) -> RwLockReadGuard<'_, Vec<Arc<TCP>>> {
        self.tcps.read().unwrap()
    }

    pub fn remove_tcp(self: &Self, addr: &SocketAddr) {
        let mut tcps = self.tcps_mut();
        for index in 0..tcps.len() {
            let tcp = &tcps[index];
            if *(*tcp).addr() == *addr {
                tcps.remove(index);
                return;
            }
        }
        warn!("Failed to remove {addr} from TCPs: it does not exist ");
    }

    pub fn tcps_mut(self: &Self) -> RwLockWriteGuard<'_, Vec<Arc<TCP>>> {
        self.tcps.write().unwrap()
    }

    pub fn tcp(self: &Self, addr: &SocketAddr) -> Option<Arc<TCP>> {
        for tcp in self.tcps().iter() {
            if tcp.addr() == addr {
                return Some(Arc::clone(tcp));
            }
        }
        None
    }

    pub fn connecting_tcps(self: &Self) -> RwLockReadGuard<'_, Vec<Arc<TCP>>> {
        self.connecting_tcps.read().unwrap()
    }

    pub fn connecting_tcps_mut(self: &Self) -> RwLockWriteGuard<'_, Vec<Arc<TCP>>> {
        self.connecting_tcps.write().unwrap()
    }

    pub fn connecting_tcp(self: &Self, addr: &SocketAddr) -> Option<Arc<TCP>> {
        for tcp in self.connecting_tcps().iter() {
            if tcp.addr() == addr {
                return Some(Arc::clone(tcp));
            }
        }
        None
    }

    /**
     * Returns Ok<Arc<TCP>> if creation is successful.
     *
     * Returns Err<String> if creation fails or timeout (10 seconds).
     */
    pub async fn create_tcp(self: &Self, addr: SocketAddr) -> Result<Arc<TCP>, String> {
        if let Some(tcp) = self.tcp(&addr)
            && tcp.is_connected()
        {
            // this tcp already exists
            self.already_connected_count.fetch_add(1, Ordering::Relaxed);
            debug!("TCP {addr} already exists");
            return Ok(tcp);
        } else {
            let (tcp, should_connect): (Arc<TCP>, bool) = {
                let mut connecting = self.connecting_tcps_mut();
                if let Some(tcp) = connecting.iter().find(|tcp| *tcp.addr() == addr).cloned() {
                    (tcp, false)
                } else {
                    self.self_connect_count.fetch_add(1, Ordering::Relaxed);
                    let tcp: Arc<TCP> = Arc::new(TCP {
                        reader: Arc::new(Mutex::new(None)),
                        writer: Arc::new(Mutex::new(None)),
                        read_task: Mutex::new(None),
                        state: RwLock::new(TcpState::NotConnected),
                        connect_notify: Notify::new(),
                        addr: addr,
                        cids: RwLock::new(HashSet::new()),
                        check_alive_task: Mutex::new(None),
                        alive: AtomicBool::new(true),
                        paused: AtomicBool::new(false),
                        queue_msg_send: RwLock::new(VecDeque::from([])),
                    });
                    tcp.set_state(TcpState::Connecting);
                    connecting.push(Arc::clone(&tcp));
                    (tcp, true)
                }
            };

            if !should_connect {
                self.wait_connected_count.fetch_add(1, Ordering::Relaxed);
                tcp.wait_connected().await?;
                return Ok(tcp);
            }

            let tcp_1 = Arc::clone(&tcp);

            match timeout(Duration::from_secs(10), async move {
                // let tcp: Result<TCP, String> = TCP::new(addr).await;
                let ip = addr.ip();
                let port = addr.port();
                let stream = TcpStream::connect(addr).await;
                if let Ok(stream) = stream {
                    let (reader, writer) = stream.into_split();
                    tcp.set_reader(reader).await;
                    tcp.set_writer(writer).await;
                    Ok(tcp)
                } else {
                    Err(String::from(
                        "Error: failed to create TCP stream with {ip}:{port}",
                    ))
                }
            })
            .await
            {
                Ok(Ok(tcp)) => {
                    tcp.set_state(TcpState::Connected);
                    {
                        let mut connecting = self.connecting_tcps_mut();
                        if let Some(index) = connecting.iter().position(|t| *t.addr() == addr) {
                            connecting.remove(index);
                        }
                    }

                    let tcp_listener = Arc::clone(&tcp);
                    tcp_listener.start_to_listen().await;

                    let tcp_writer = Arc::clone(&tcp);
                    tcp_writer.start_to_write();

                    let tcp_check_alive = Arc::clone(&tcp);
                    tcp_check_alive.start_check_alive().await;

                    // send handshake
                    let dests = vec![addr];
                    let version_msg = CaMsg::build_version(&dests);
                    let client_name_msg = CaMsg::build_client_name(&dests);
                    let host_name_msg = CaMsg::build_host_name(&dests);
                    tcp.send_msgs(vec![version_msg, client_name_msg, host_name_msg]);
                    self.tcps_mut().push(Arc::clone(&tcp));

                    // wait for 500 ms to send out the packet
                    // match timeout(Duration::from_millis(500), async move {}).await {
                    //     Ok(_) => {}
                    //     Err(_) => {}
                    // };

                    // async work is done
                    // todo: notify waiters to go, which is in this function
                    tcp.connect_notify.notify_waiters();
                    return Ok(tcp);
                }
                Ok(Err(_)) => {
                    // todo: notify waiters
                    tcp_1.set_state(TcpState::NotConnected);
                    {
                        let mut connecting = self.connecting_tcps_mut();
                        if let Some(index) = connecting.iter().position(|t| *t.addr() == addr) {
                            connecting.remove(index);
                        }
                    }
                    tcp_1.connect_notify.notify_waiters();

                    return Err("".to_string());
                }
                Err(_) => {
                    // todo: notify waiters
                    tcp_1.set_state(TcpState::NotConnected);
                    {
                        let mut connecting = self.connecting_tcps_mut();
                        if let Some(index) = connecting.iter().position(|t| *t.addr() == addr) {
                            connecting.remove(index);
                        }
                    }
                    tcp_1.connect_notify.notify_waiters();

                    return Err("".to_string());
                }
            }
        }
    }
}
