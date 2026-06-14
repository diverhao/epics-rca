use tokio::io;
use tokio::net::TcpStream;

pub struct TCP {
    stream: TcpStream,
}

impl TCP {
    pub async fn new(ip: &str, port: u16) -> Result<Self> {
        let addr = format!("{ip}:{port}");
        let stream = TcpStream::connect(addr).await;
        if let Ok(stream) = stream {
            Ok(TCP { stream })
        } else {
            Err(String::from(
                "Error: failed to create TCP stream with {ip}:{port}",
            ))
        }
    }

    pub async fn start_to_listen(self: &Self) {}

    pub async fn send(self: &Self, buf: &Vec<u8>) {
        self.stream.write_all(buf).await;
    }
}
