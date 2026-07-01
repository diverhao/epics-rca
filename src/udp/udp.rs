use crate::ca::header::CaHeader;
use crate::ca::message::CaMsg;
use crate::ca::message::MAX_UDP_SEND;
use crate::ca::message_handler::handle_udp_msgs;
use crate::env::env::Env;
use crate::udp::addr_list::parse_ca_pva_addr_list;
use ::log::debug;
use ::log::error;
use ::log::info;
use core::net::SocketAddr;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::usize::MAX;
use tokio::net::UdpSocket;

pub struct UDP {
    socket_v4: Arc<UdpSocket>,
    socket_v6: Arc<UdpSocket>,
    ca_addr_list: Vec<SocketAddr>,
    pva_addr_list: Vec<SocketAddr>,
}

impl UDP {
    /**
     * Creates a UDP transport with IPv4 and IPv6 sockets bound to ephemeral
     * ports on all interfaces, using CA and PVA search addresses from `env`.
     *
     * Panics if either socket cannot be bound.
     */
    pub async fn new(env: &Env) -> Self {
        // bind to all interfaces for both v4 and v6
        // panic if fail
        let socket_v4: Arc<UdpSocket> = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
        let socket_v6: Arc<UdpSocket> = Arc::new(UdpSocket::bind("[::]:0").await.unwrap());
        let (ca_addr_list, pva_addr_list) = parse_ca_pva_addr_list(&env);
        let port_v4 = socket_v4
            .local_addr()
            .expect("IPv4 UDP socket should have a local address")
            .port();
        let port_v6 = socket_v6
            .local_addr()
            .expect("IPv6 UDP socket should have a local address")
            .port();

        info!(
            "Creating UDP with IPv4 bound to port {} on all network interfaces",
            port_v4
        );
        info!(
            "Creating UDP with IPv6 bound to port {} on all network interfaces",
            port_v6
        );
        info!("CA name search addresses: {:?}", ca_addr_list);
        info!("PVA name search addresses: {:?}", pva_addr_list);
        UDP {
            socket_v4,
            socket_v6,
            ca_addr_list,
            pva_addr_list,
        }
    }

    pub fn start_to_listen(self: Arc<Self>) {
        let socket_v4 = Arc::clone(self.socket_v4());
        let socket_v6 = Arc::clone(self.socket_v6());
        let udp_v4 = Arc::clone(&self);
        let udp_v6 = Arc::clone(&self);
        tokio::spawn(async move {
            let mut buf: Vec<u8> = vec![0_u8; MAX_UDP_SEND];
            loop {
                match socket_v4.recv_from(&mut buf).await {
                    Ok((size, remote_socket)) => {
                        let mut buf: Vec<u8> = buf[..size].to_vec();
                        let msgs = CaMsg::from_buf(&mut buf, Some(remote_socket), vec![]);
                        handle_udp_msgs(&remote_socket, msgs);
                    }
                    Err(err) => {
                        error!("Error receving UDP, {:?}", err);
                    }
                }
            }
        });
        tokio::spawn(async move {
            let mut buf = [0_u8; MAX_UDP_SEND];
            loop {
                match socket_v6.recv_from(&mut buf).await {
                    Ok((size, remote_socket)) => {
                        let mut buf: Vec<u8> = buf[..size].to_vec();
                        let msgs = CaMsg::from_buf(&mut buf, Some(remote_socket), vec![]);
                        handle_udp_msgs(&remote_socket, msgs);
                    }
                    Err(err) => {
                        error!("Error receving UDP, {:?}", err);
                    }
                }
            }
        });
    }

    // -------------- network ----------------------

    async fn send_buf(self: &Self, buf: &Vec<u8>) {
        // send to addresses in EPICS_CA_ADDR_LIST
        for socket_addr in self.ca_addr_list() {
            match socket_addr {
                SocketAddr::V4(_) => {
                    let sent = self.socket_v4().send_to(buf, socket_addr).await;
                    match sent {
                        Ok(bytes) => {
                            // debug!("Sent out {} bytes of UDP data to {:?}", bytes, socket_addr);
                        }
                        Err(err) => {
                            error!("Failed to send UDP data to {:?}: {}", socket_addr, err);
                        }
                    }
                }
                SocketAddr::V6(_) => {
                    let sent = self.socket_v6().send_to(buf, socket_addr).await;
                    match sent {
                        Ok(bytes) => {
                            // debug!("Sent out {} bytes of UDP data to {:?}", bytes, socket_addr);
                        }
                        Err(err) => {
                            error!("Failed to send UDP data to {:?}: {}", socket_addr, err);
                        }
                    }
                }
            }
        }
    }

    pub async fn send_msgs(self: &Self, msgs: &Vec<CaMsg>) {
        let mut buf: Vec<u8> = vec![];
        for msg in msgs {
            debug!("\nSending UDP message {msg}");
            if buf.len() as u32 + msg.size() > MAX_UDP_SEND as u32 {
                self.send_buf(&buf).await;
                buf.clear();
            }
            buf.extend_from_slice(&msg.to_buf());
        }
        self.send_buf(&buf).await;
    }

    // ----------------- getters -----------------

    pub fn port_v4(self: &Self) -> u16 {
        self.socket_v4().local_addr().unwrap().port()
    }

    pub fn port_v6(self: &Self) -> u16 {
        self.socket_v6().local_addr().unwrap().port()
    }

    pub fn socket_v4(self: &Self) -> &Arc<UdpSocket> {
        &self.socket_v4
    }

    pub fn socket_v6(self: &Self) -> &Arc<UdpSocket> {
        &self.socket_v6
    }

    pub fn ca_addr_list(self: &Self) -> &Vec<SocketAddr> {
        &self.ca_addr_list
    }

    pub fn pva_addr_list(self: &Self) -> &Vec<SocketAddr> {
        &self.pva_addr_list
    }
}
