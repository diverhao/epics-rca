use crate::ca::message::CaMsg;
use crate::ca::message_handler::handle_tcp_msgs;
use crate::context::context::get_context;
use log::debug;
use log::error;
use log::warn;
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
use tokio::time::{self, Duration};

pub struct TCP {
    reader: Arc<Mutex<Option<OwnedReadHalf>>>,
    writer: Arc<Mutex<Option<OwnedWriteHalf>>>,
    read_task: Mutex<Option<JoinHandle<()>>>,
    connected: AtomicBool,
    addr: SocketAddr,
    cids: RwLock<Vec<u32>>,
    check_alive_task: Mutex<Option<JoinHandle<()>>>,
    alive: AtomicBool,
}

impl TCP {
    pub async fn new(addr: SocketAddr) -> Result<Self, String> {
        let ip = addr.ip();
        let port = addr.port();
        let addr = format!("{ip}:{port}");
        let stream = TcpStream::connect(addr).await;
        if let Ok(stream) = stream {
            let addr = stream.peer_addr().map_err(|err| err.to_string())?;
            let (reader, writer) = stream.into_split();
            let tcp = TCP {
                reader: Arc::new(Mutex::new(Some(reader))),
                writer: Arc::new(Mutex::new(Some(writer))),
                read_task: Mutex::new(None),
                connected: AtomicBool::new(true),
                addr: addr,
                cids: RwLock::new(vec![]),
                check_alive_task: Mutex::new(None),
                alive: AtomicBool::new(true),
            };
            Ok(tcp)
        } else {
            Err(String::from(
                "Error: failed to create TCP stream with {ip}:{port}",
            ))
        }
    }

    /**
     * Close tcp connection, also update the state and clean up the relationships.
     */
    pub async fn disconnect(&self, abort_read_task: bool, stop_check_alive_task: bool) {
        debug!("============== action 1 ------------");
        // Update connect state
        self.set_connected(false);

        // Stop self-checking alive, i.e. periodically sending out CA_PROTO_ECHO
        if stop_check_alive_task {
            self.stop_check_alive().await;
        }
        debug!("============== action 2 ------------");

        // Unregister from TCPs
        {
            get_context().tcps().remove_tcp(self.addr());
        }
        debug!("============== action 3 ------------");

        if let Some(handle) = self.read_task.lock().await.take() {
            // Cancel the tcp read loop even in reader.read(&mut buf_pending).await
            if abort_read_task {
                handle.abort();
            }
        }
        debug!("============== action 4 ------------");

        {
            // let mut writer = self.writer.lock().await;
            // if let Some(mut writer) = writer.take() {
            //     // move the writer out of Option
            //     let _ = writer.shutdown().await;
            // }
            if let Ok(mut writer_guard) = self.writer.try_lock() {
                if let Some(mut writer) = writer_guard.take() {
                    let _ =
                        tokio::time::timeout(Duration::from_millis(100), writer.shutdown()).await;
                }
            }
        }
        debug!("============== action 5 ------------");

        // {
        //     let mut reader = self.reader.lock().await;
        //     // move the actual reader out of Option and drop it in this scope
        //     let _ = reader.take();
        // }
        debug!("============== action 6 ------------");
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
                        debug!("Received {size} TCP bytes from {}", tcp.addr());
                        buf.extend_from_slice(&buf_pending[..size]);
                        let msgs = CaMsg::from_buf(&mut buf, Some(tcp.addr().clone()), vec![]);
                        let src = *tcp.addr();
                        handle_tcp_msgs(&src, msgs).await;
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
                    error!("{err}");
                    return Err(err.to_string());
                }
            },
            None => {
                // no writer
                return Ok(());
            }
        }
    }

    pub async fn send_msgs(&self, msgs: Vec<CaMsg>) -> Result<(), String> {
        if !self.is_connected() {
            return Err("TCP not connected".to_string());
        }

        let mut buf: Vec<u8> = vec![];
        for msg in msgs {
            debug!("\nSending TCP message {msg}");
            buf.extend_from_slice(&msg.to_buf());
        }
        match self.send_buf(&buf).await {
            Ok(_) => {
                return Ok(());
            }
            Err(error) => {
                return Err(error);
            }
        }
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
        println!("-------------- AAA ----------------");
        self.reconnect_channels().await;
        println!("-------------- BBB ----------------");
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

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
        let mut cids_mut = self.cids.write().unwrap();
        if !cids_mut.contains(&cid) {
            cids_mut.push(cid);
        } else {
            // do nothing
        }
    }

    pub fn remove_cid(self: &Self, cid: u32) {
        let mut cids_mut = self.cids.write().unwrap();
        if let Some(pos) = cids_mut.iter().position(|&existing| existing == cid) {
            cids_mut.swap_remove(pos);
        }
    }

    pub fn set_connected(self: &Self, connected: bool) {
        self.connected.store(connected, Ordering::Release);
    }

    pub fn cids(self: &Self) -> RwLockReadGuard<'_, Vec<u32>> {
        self.cids.read().unwrap()
    }

    pub fn cids_mut(self: &Self) -> RwLockWriteGuard<'_, Vec<u32>> {
        self.cids.write().unwrap()
    }


    pub async fn start_check_alive(self: Arc<Self>) {
        let mut task = self.check_alive_task.lock().await;
        if task.is_some() {
            return;
        }

        let tcp = Arc::clone(&self);
        let handle = tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(5));

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

                if tcp.alive() {
                    // will be reset to true when tcp receives the echo reply
                    tcp.set_alive(false);
                } else {
                    // TCP did not receive the echo reply in last 30 seconds
                    // Connection is broken, handle the failure
                    tcp.set_alive(true);
                    // tcp failed
                    tcp.handle_tcp_failure(true, false).await;
                    break;
                }

                // send CA_PROTO_ECHO
                let addr = tcp.addr().clone();
                let msg = CaMsg::build_echo(&vec![addr]);
                match tcp.send_msgs(vec![msg]).await {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        });
        *task = Some(handle);
    }

    pub async fn stop_check_alive(self: &Self) {
        if let Some(handle) = self.check_alive_task.lock().await.take() {
            handle.abort();
        }
    }

    // /**
    //  * Check if the tcp connection is alive when the tcp is connected.
    //  *
    //  * If the tcp is not connected, don't bother to check
    //  */
    // async fn check_alive(self: &Self) {
    //     if !self.is_connected() {
    //         return;
    //     }

    //     if self.alive() {
    //         // will be reset to true when tcp receives the echo reply
    //         self.set_alive(false);
    //     } else {
    //         // TCP did not receive the echo reply in last 30 seconds
    //         // Connection is broken, handle the failure
    //         self.set_alive(true);
    //         // abort read task because we are outside the read task
    //         self.handle_tcp_failure(true, false).await;
    //         return;
    //     }

    //     let addr = self.addr().clone();
    //     let msg = CaMsg::build_echo(&vec![addr]);
    //     match self.send_msgs(vec![msg]).await {
    //         Ok(_) => {}
    //         Err(_) => {}
    //     }
    // }

    pub fn set_alive(&self, alive: bool) {
        debug!("Set tcp alive =============================== to {}", alive);
        self.alive
            .store(alive, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn alive(self: &Self) -> bool {
        self.alive.load(Ordering::Relaxed)
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

    pub async fn create_tcp(self: &Self, addr: SocketAddr) -> Result<Arc<TCP>, String> {
        if let Some(tcp) = self.tcp(&addr) {
            // this tcp already exists
            debug!("TCP {addr} already exists");
            return Ok(tcp);
        } else {
            let tcp: Result<TCP, String> = TCP::new(addr).await;
            // check again if TCPs has one such TCP
            // in case another TCP with same addr is created during the above await
            if let Some(tcp) = self.tcp(&addr) {
                // the above newly created tcp is automatically dropped
                return Ok(tcp);
            }

            match tcp {
                Ok(tcp) => {
                    let tcp = Arc::new(tcp);
                    self.tcps_mut().push(Arc::clone(&tcp));

                    let tcp_listener = Arc::clone(&tcp);
                    tcp_listener.start_to_listen().await;

                    let tcp_check_alive = Arc::clone(&tcp);
                    tcp_check_alive.start_check_alive().await;

                    Ok(Arc::clone(&tcp))
                }
                Err(err_msg) => {
                    error!("{err_msg}");
                    Err(err_msg)
                }
            }
        }
    }
}
