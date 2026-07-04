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
use tokio::sync::{Mutex, MutexGuard};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio::time::{self, Duration};

pub struct TCP {
    reader: Arc<Mutex<Option<OwnedReadHalf>>>,
    writer: Arc<Mutex<Option<OwnedWriteHalf>>>,
    read_task: Mutex<Option<JoinHandle<()>>>,
    connected: RwLock<bool>,
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
        self.set_connected(false);

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
            // read up to 4 kB each time
            let mut buf_pending: [u8; 4096] = [0_u8; 4096];
            let mut packet_count = 0;
            loop {
                if !tcp.is_connected() {
                    break;
                }

                let num_bytes = {
                    // take the lock
                    let mut reader = tcp.reader.lock().await;
                    // read data into buf_pending
                    match reader.as_mut() {
                        Some(reader) => reader.read(&mut buf_pending).await,
                        None => {
                            // no reader
                            error!("No reader for TCP");
                            break;
                        }
                    }
                };

                // check reader status
                match num_bytes {
                    Ok(0) => {
                        // tcp connection closed, e.g. ctrl-c in IOC
                        error!("TCP disconnected");
                        tcp.handle_tcp_failure(false, true).await;
                        break;
                    }
                    Ok(size) => {
                        packet_count = packet_count + 1;
                        debug!("Received {size} TCP bytes from {}", tcp.addr());
                        buf.extend_from_slice(&buf_pending[..size]);
                        let msgs = CaMsg::from_buf(&mut buf, Some(tcp.addr().clone()), vec![]);
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
            let n = queue.len().min(1024);
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
        self.connected.read().unwrap().clone()
    }

    pub fn set_connected(self: &Self, connected: bool) {
        *self.connected.write().unwrap() = connected;
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
}

pub struct TCPs {
    // use array instead of hashmap, usually there are not
    // many TCP clients (< 200), it is OK to iterate over
    // the vector
    tcps: RwLock<Vec<Arc<TCP>>>,
}

impl TCPs {
    pub fn new() -> Self {
        TCPs {
            tcps: RwLock::new(vec![]),
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

    // pub fn remove_tcp_by_id(self: &Self, id: u32) {
    //     let mut tcps = self.tcps_mut();
    //     for index in 0..tcps.len() {
    //         let tcp = &tcps[index];
    //         if (*tcp).id() == id {
    //             tcps.remove(index);
    //             return;
    //         }
    //     }
    //     warn!("Failed to remove {id} from TCPs: it does not exist ");
    // }

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

    // pub fn find_connected_tcp(self: &Self, addr: &SocketAddr) -> Option<Arc<TCP>> {
    //     for tcp in self.tcps().iter() {
    //         if *tcp.addr() == *addr && tcp.is_connected() {
    //             return Some(tcp.clone());
    //         }
    //     }
    //     return None;
    // }

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
            debug!("TCP {addr} already exists");
            return Ok(tcp);
        } else {
            // tcp may take long while to create
            let id = rand::random::<u32>();
            let id1 = id;
            let tcp: Arc<TCP> = Arc::new(TCP {
                reader: Arc::new(Mutex::new(None)),
                writer: Arc::new(Mutex::new(None)),
                read_task: Mutex::new(None),
                connected: RwLock::new(false),
                addr: addr,
                cids: RwLock::new(HashSet::new()),
                check_alive_task: Mutex::new(None),
                alive: AtomicBool::new(true),
                paused: AtomicBool::new(false),
                queue_msg_send: RwLock::new(VecDeque::from([])),
            });

            self.tcps_mut().push(Arc::clone(&tcp));

            match timeout(Duration::from_secs(10), async move {
                // let tcp: Result<TCP, String> = TCP::new(addr).await;
                let ip = addr.ip();
                let port = addr.port();
                // check if TCPs has one such TCP, if a tcp exists and already connected
                // just return it, otherwise start to connect this TCP
                if let Some(another_tcp) = self.tcp(&addr) {
                    return Ok((another_tcp, false));
                }

                let stream = TcpStream::connect(addr).await;
                if let Some(another_tcp) = self.tcp(&addr) {
                    return Ok((another_tcp, false));
                }

                if let Ok(stream) = stream {
                    let (reader, writer) = stream.into_split();
                    tcp.set_reader(reader).await;
                    tcp.set_writer(writer).await;
                    if let Some(another_tcp) = self.tcp(&addr) {
                        {
                            let mut reader = tcp.reader().await;
                            reader.take();
                        }

                        let writer = {
                            let mut writer = tcp.writer().await;
                            writer.take()
                        };

                        if let Some(mut writer) = writer {
                            let _ =
                                tokio::time::timeout(Duration::from_millis(100), writer.shutdown())
                                    .await;
                        }
                        return Ok((another_tcp, false));
                    }
                    tcp.set_connected(true);
                    Ok((tcp, true))
                } else {
                    Err(String::from(
                        "Error: failed to create TCP stream with {ip}:{port}",
                    ))
                }
            })
            .await
            {
                Ok(Ok((tcp, is_new))) => {
                    if is_new {
                        let tcp_listener = Arc::clone(&tcp);
                        tcp_listener.start_to_listen().await;

                        let tcp_writer = Arc::clone(&tcp);
                        tcp_writer.start_to_write();

                        let tcp_check_alive = Arc::clone(&tcp);
                        tcp_check_alive.start_check_alive().await;
                    }
                    return Ok(tcp);
                }
                Ok(Err(_)) => {
                    return Err("".to_string());
                }
                Err(_) => {
                    return Err("".to_string());
                }
            }
        }
    }
}
