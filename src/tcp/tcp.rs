use crate::ca::message::CaMsg;
use crate::ca::message_handler::handle_tcp_msgs;
use log::debug;
use log::error;
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

pub struct TCP {
    reader: Arc<Mutex<OwnedReadHalf>>,
    writer: Arc<Mutex<OwnedWriteHalf>>,
    connected: AtomicBool,
    addr: SocketAddr,
    cids: RwLock<Vec<u32>>,
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
                reader: Arc::new(Mutex::new(reader)),
                writer: Arc::new(Mutex::new(writer)),
                connected: AtomicBool::new(true),
                addr: addr,
                cids: RwLock::new(vec![]),
            };

            Ok(tcp)
        } else {
            Err(String::from(
                "Error: failed to create TCP stream with {ip}:{port}",
            ))
        }
    }

    pub async fn start_to_listen(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut buf: Vec<u8> = vec![];
            // read up to 4 kB each time
            let mut buf_pending: [u8; 4096] = [0_u8; 4096];

            loop {
                let read_result = {
                    let mut reader = self.reader().await;
                    reader.read(&mut buf_pending).await
                };
                match read_result {
                    Ok(0) => {
                        // remote closed connection
                        self.connected.store(false, Ordering::Release);
                        break;
                    }
                    Ok(size) => {
                        debug!("Received {size} TCP bytes from {}", self.addr());
                        buf.extend_from_slice(&buf_pending[..size]);
                        let msgs = CaMsg::from_buf(&mut buf, Some(self.addr().clone()), vec![]);
                        let src = *self.addr();
                        handle_tcp_msgs(&src, msgs).await;
                    }
                    Err(err) => {
                        error!("{err}");
                        break;
                    }
                }
            }
        });
    }

    async fn send_buf(&self, buf: &Vec<u8>) {
        let mut writer = self.writer().await;
        if let Err(err) = writer.write_all(buf).await {
            self.connected.store(false, Ordering::Release);
            error!("{err}");
        }
    }

    pub async fn send_msgs(&self, msgs: Vec<CaMsg>) {
        let mut buf: Vec<u8> = vec![];
        for msg in msgs {
            debug!("\nSending TCP message {msg}");
            buf.extend_from_slice(&msg.to_buf());
        }
        self.send_buf(&buf).await;
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    async fn reader(self: &Self) -> MutexGuard<'_, OwnedReadHalf> {
        self.reader.lock().await
    }

    async fn writer(self: &Self) -> MutexGuard<'_, OwnedWriteHalf> {
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
        if let None = self.tcp(&addr) {
            let tcp = TCP::new(addr).await;
            match tcp {
                Ok(tcp) => {
                    let tcp = Arc::new(tcp);
                    self.tcps_mut().push(Arc::clone(&tcp));

                    let tcp_listener = Arc::clone(&tcp);
                    tcp_listener.start_to_listen().await;

                    Ok(Arc::clone(&tcp))
                }
                Err(err_msg) => {
                    error!("{err_msg}");
                    Err(err_msg)
                }
            }
        } else {
            // this tcp already exists
            debug!("TCP {addr} already exists");
            Err("TCP {addr} already exists".to_string())
        }
    }
}
