use crate::channel::channel::DbrType;
use crate::context::context::get_context;
use crate::env::env::Env;
use crate::env::env::EnvType;
use ::log::debug;
use ::log::error;
use ::log::info;
use core::net::{IpAddr, SocketAddr};
use tokio::net::UdpSocket;

pub const MAX_UDP_SEND: usize = 1024;

#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum CaCmd {
    // Commands (TCP and UDP)
    CaProtoVersion = 0x0000,
    CaProtoSearch = 0x0006,
    CaProtoNotFound = 0x000e,
    CaProtoEcho = 0x0017,
    // Commands (UDP)
    CaProtoRsrvIsUp = 0x000d,
    CaRepeaterConfirm = 0x0011,
    CaRepeaterRegister = 0x0018,
    // Commands (TCP)
    CaProtoEventAdd = 0x0001,
    CaProtoEventCancel = 0x0002,
    CaProtoRead = 0x0003,
    CaProtoWrite = 0x0004,
    CaProtoSnapshot = 0x0005,
    CaProtoBuild = 0x0007,
    CaProtoEventsOff = 0x0008,
    CaProtoEventsOn = 0x0009,
    CaProtoReadSync = 0x000a,
    CaProtoError = 0x000b,
    CaProtoClearChannel = 0x000c,
    CaProtoReadNotify = 0x000f,
    CaProtoReadBuild = 0x0010,
    CaProtoCreateChan = 0x0012,
    CaProtoWriteNotify = 0x0013,
    CaProtoClientName = 0x0014,
    CaProtoHostName = 0x0015,
    CaProtoAccessRights = 0x0016,
    CaProtoSignal = 0x0019,
    CaProtoCreateChFail = 0x001a,
    CaProtoServerDisconn = 0x001b,
}

pub struct UDP {
    socket_v4: UdpSocket,
    socket_v6: UdpSocket,
    ca_addr_list: Vec<SocketAddr>,
    pva_addr_list: Vec<SocketAddr>,
}

impl UDP {
    pub async fn new(env: &Env) -> Self {
        // bind to all interfaces for both v4 and v6
        // panic if fail
        let socket_v4: UdpSocket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        let socket_v6: UdpSocket = UdpSocket::bind("[::]:0").await.unwrap();
        let (ca_addr_list, pva_addr_list) = Self::parse_ca_pva_addr_list(&env);
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

    fn parse_ca_pva_addr_list(env: &Env) -> (Vec<SocketAddr>, Vec<SocketAddr>) {
        let (ca_server_port, pva_server_port) = UDP::parse_ca_pva_server_port(env);

        let mut ca_addr_list: Vec<SocketAddr> = vec![];
        let mut pva_addr_list: Vec<SocketAddr> = vec![];

        // create ca_addr_list
        let Some(EnvType::StringArray(epics_ca_addr_list)) =
            ({ env.get_env("EPICS_CA_ADDR_LIST").cloned() })
        else {
            error!("EPICS_CA_ADDR_LIST is not a string array");
            return (ca_addr_list, pva_addr_list);
        };

        for addr in epics_ca_addr_list {
            let socket_addr = match addr.parse::<SocketAddr>() {
                Ok(socket_addr) => socket_addr,
                Err(socket_addr_err) => match addr.parse::<IpAddr>() {
                    Ok(ip_addr) => SocketAddr::new(ip_addr, ca_server_port),
                    Err(ip_addr_err) => {
                        error!(
                            "Failed to parse EPICS_CA_ADDR_LIST address {} as socket address ({}) or IP address ({}).",
                            addr, socket_addr_err, ip_addr_err
                        );
                        continue;
                    }
                },
            };
            ca_addr_list.push(socket_addr);
        }

        // create pva_addr_list
        let Some(EnvType::StringArray(epics_pva_addr_list)) =
            ({ env.get_env("EPICS_PVA_ADDR_LIST").cloned() })
        else {
            error!("EPICS_PVA_ADDR_LIST is not a string array");
            return (ca_addr_list, pva_addr_list);
        };

        for addr in epics_pva_addr_list {
            let socket_addr = match addr.parse::<SocketAddr>() {
                Ok(socket_addr) => socket_addr,
                Err(socket_addr_err) => match addr.parse::<IpAddr>() {
                    Ok(ip_addr) => SocketAddr::new(ip_addr, pva_server_port),
                    Err(ip_addr_err) => {
                        error!(
                            "Failed to parse EPICS_PVA_ADDR_LIST address {} as socket address ({}) or IP address ({}).",
                            addr, socket_addr_err, ip_addr_err
                        );
                        continue;
                    }
                },
            };

            pva_addr_list.push(socket_addr);
        }

        (ca_addr_list, pva_addr_list)
    }

    fn parse_ca_pva_server_port(env: &Env) -> (u16, u16) {
        const DEFAULT_CA_SERVER_PORT: u16 = 5064;
        const DEFAULT_PVA_SERVER_PORT: u16 = 5076;

        let ca_server_port = match env.get_env("EPICS_CA_SERVER_PORT") {
            Some(EnvType::Integer(port)) => match u16::try_from(*port) {
                Ok(port) => port,
                Err(err) => {
                    error!(
                        "EPICS_CA_SERVER_PORT is outside the valid u16 port range: {}: {}. Use default {}.",
                        port, err, DEFAULT_CA_SERVER_PORT
                    );
                    DEFAULT_CA_SERVER_PORT
                }
            },
            Some(value) => {
                error!(
                    "EPICS_CA_SERVER_PORT is not an integer: {:?}. Use default {}.",
                    value, DEFAULT_CA_SERVER_PORT
                );
                DEFAULT_CA_SERVER_PORT
            }
            None => {
                error!(
                    "EPICS_CA_SERVER_PORT is not set. Use default {}.",
                    DEFAULT_CA_SERVER_PORT
                );
                DEFAULT_CA_SERVER_PORT
            }
        };
        let pva_server_port = match env.get_env("EPICS_PVA_SERVER_PORT") {
            Some(EnvType::Integer(port)) => match u16::try_from(*port) {
                Ok(port) => port,
                Err(err) => {
                    error!(
                        "EPICS_PVA_SERVER_PORT is outside the valid u16 port range: {}: {}. Use default {}.",
                        port, err, DEFAULT_PVA_SERVER_PORT
                    );
                    DEFAULT_PVA_SERVER_PORT
                }
            },
            Some(value) => {
                error!(
                    "EPICS_PVA_SERVER_PORT is not an integer: {:?}. Use default {}.",
                    value, DEFAULT_PVA_SERVER_PORT
                );
                DEFAULT_PVA_SERVER_PORT
            }
            None => {
                error!(
                    "EPICS_PVA_SERVER_PORT is not set. Use default {}.",
                    DEFAULT_PVA_SERVER_PORT
                );
                DEFAULT_PVA_SERVER_PORT
            }
        };
        (ca_server_port, pva_server_port)
    }

    // -------------- network ----------------------
    /**
     * Send one UDP message to all hosts defined in EPICS_CA_ADDR_LIST
     */
    pub async fn send_ca(
        self: &Self,
        cmd: CaCmd,
        data_type: u32,
        data_count: u32,
        param1: u32,
        param2: u32,
        mut payload: Vec<u8>,
    ) {
        // patch payload
        let padding = (8 - payload.len() % 8) % 8;
        payload.resize(payload.len() + padding, 0);

        // make sure the payload size fits in u32 data
        let payload_size: u32 = if let Ok(payload_size) = payload.len().try_into() {
            payload_size
        } else {
            error!("Payload size too large");
            return;
        };

        // create header buffer
        let mut buf =
            UDP::build_ca_header(cmd, payload_size, data_type, data_count, param1, param2);

        // combine header and payload
        buf.extend_from_slice(&payload);

        // send to addresses in EPICS_CA_ADDR_LIST
        for socket_addr in self.ca_addr_list() {
            debug!("Sending UDP data to {:?}", socket_addr);
            match socket_addr {
                SocketAddr::V4(_) => {
                    let sent = self.socket_v4().send_to(&buf, socket_addr).await;
                    match sent {
                        Ok(bytes) => {
                            debug!("Sent out {} bytes of UDP data to {:?}", bytes, socket_addr);
                        }
                        Err(err) => {
                            error!("Failed to send UDP data to {:?}: {}", socket_addr, err);
                        }
                    }
                }
                SocketAddr::V6(_) => {
                    let sent = self.socket_v6().send_to(&buf, socket_addr).await;
                    match sent {
                        Ok(bytes) => {
                            debug!("Sent out {} bytes of UDP data to {:?}", bytes, socket_addr);
                        }
                        Err(err) => {
                            error!("Failed to send UDP data to {:?}: {}", socket_addr, err);
                        }
                    }
                }
            }
        }
    }

    fn build_ca_header(
        cmd: CaCmd,        // 2 bytes
        payload_size: u32, // up to 4 bytes
        data_type: u32,    // up to 4 bytes
        data_count: u32,
        param1: u32,
        param2: u32,
    ) -> Vec<u8> {
        let mut use_extended_header = false;
        // use extended header if payload size > 16368 bytes
        if payload_size > 0x3ff0 {
            debug!("UDP payload size larger than 16368 bytes, use extended header");
            use_extended_header = true;
        }

        if use_extended_header {
            let mut buf: Vec<u8> = Vec::with_capacity(16);
            buf.extend_from_slice(&(cmd as u16).to_be_bytes());
            buf.extend_from_slice(&(payload_size as u16).to_be_bytes());
            buf.extend_from_slice(&(data_type as u16).to_be_bytes()); // 2 bytes
            buf.extend_from_slice(&(data_count as u16).to_be_bytes()); // 2 bytes
            buf.extend_from_slice(&param1.to_be_bytes());
            buf.extend_from_slice(&param2.to_be_bytes());
            buf
        } else {
            let mut buf: Vec<u8> = Vec::with_capacity(24);
            buf.extend_from_slice(&(cmd as u16).to_be_bytes());
            buf.extend_from_slice(&(0xffff as u16).to_be_bytes());
            buf.extend_from_slice(&(data_type as u16).to_be_bytes());
            buf.extend_from_slice(&(0x0000 as u16).to_be_bytes());
            buf.extend_from_slice(&param1.to_be_bytes());
            buf.extend_from_slice(&param2.to_be_bytes());
            buf.extend_from_slice(&payload_size.to_be_bytes());
            buf.extend_from_slice(&data_count.to_be_bytes());
            buf
        }
    }

    // ----------------- getters -----------------

    pub fn port_v4(self: &Self) -> u16 {
        self.socket_v4().local_addr().unwrap().port()
    }

    pub fn port_v6(self: &Self) -> u16 {
        self.socket_v6().local_addr().unwrap().port()
    }

    pub fn socket_v4(self: &Self) -> &UdpSocket {
        &self.socket_v4
    }

    pub fn socket_v6(self: &Self) -> &UdpSocket {
        &self.socket_v6
    }

    pub fn ca_addr_list(self: &Self) -> &Vec<SocketAddr> {
        &self.ca_addr_list
    }

    pub fn pva_addr_list(self: &Self) -> &Vec<SocketAddr> {
        &self.pva_addr_list
    }
}
