use log::error;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf, ReadHalf};
use tokio::sync::{Mutex, MutexGuard};

pub struct TCP {
    reader: Arc<Mutex<OwnedReadHalf>>,
    writer: Arc<Mutex<OwnedWriteHalf>>,
}

impl TCP {
    pub async fn new(ip: &str, port: u16) -> Result<Self, String> {
        let addr = format!("{ip}:{port}");
        let stream = TcpStream::connect(addr).await;
        if let Ok(stream) = stream {
            let (reader, writer) = stream.into_split();
            Ok(TCP {
                reader: Arc::new(Mutex::new(reader)),
                writer: Arc::new(Mutex::new(writer)),
            })
        } else {
            Err(String::from(
                "Error: failed to create TCP stream with {ip}:{port}",
            ))
        }
    }

    pub async fn start_to_listen(self: &Self) {
        let mut buf: Vec<u8> = vec![];
        let mut reader = self.reader().await;
        // read up to 4 kB each time
        let mut buf_pending: [u8; 4096] = [0_u8; 4096];

        loop {
            match reader.read(&mut buf_pending).await {
                Ok(0) => {
                    // remote closed connection
                    break;
                }
                Ok(size) => {
                    buf.extend_from_slice(&buf_pending[..size]);
                    // todo: decode incoming TCP data here
                }
                Err(err) => {
                    error!("{err}");
                    break;
                }
            }
        }
    }

    pub async fn send(&self, buf: &Vec<u8>) {
        let mut writer = self.writer().await;
        if let Err(err) = writer.write_all(buf).await {
            error!("{err}");
        }
    }

    async fn reader(self: &Self) -> MutexGuard<'_, OwnedReadHalf> {
        self.reader.lock().await
    }

    async fn writer(self: &Self) -> MutexGuard<'_, OwnedWriteHalf> {
        self.writer.lock().await
    }
}
