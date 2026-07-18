use crate::env::env::Env;
use crate::env::env::EnvType;
use crate::udp::udp::UDP;
use ::log::debug;
use ::log::error;
use ::log::info;
use core::net::{IpAddr, SocketAddr};
use tokio::net::UdpSocket;

pub fn parse_ca_pva_addr_list(env: &Env) -> (Vec<SocketAddr>, Vec<SocketAddr>) {
    let (ca_broadcast_port, pva_broadcast_port) = parse_ca_pva_broadcast_port(env);

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
                Ok(ip_addr) => SocketAddr::new(ip_addr, ca_broadcast_port),
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
                Ok(ip_addr) => SocketAddr::new(ip_addr, pva_broadcast_port),
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

fn parse_ca_pva_broadcast_port(env: &Env) -> (u16, u16) {
    const DEFAULT_CA_SERVER_PORT: u16 = 5064;
    const DEFAULT_PVA_BROADCAST_PORT: u16 = 5076;

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
    let pva_server_port = match env.get_env("EPICS_PVA_BROADCAST_PORT") {
        Some(EnvType::Integer(port)) => match u16::try_from(*port) {
            Ok(port) => port,
            Err(err) => {
                error!(
                    "EPICS_PVA_BROADCAST_PORT is outside the valid u16 port range: {}: {}. Use default {}.",
                    port, err, DEFAULT_PVA_BROADCAST_PORT
                );
                DEFAULT_PVA_BROADCAST_PORT
            }
        },
        Some(value) => {
            error!(
                "EPICS_PVA_BROADCAST_PORT is not an integer: {:?}. Use default {}.",
                value, DEFAULT_PVA_BROADCAST_PORT
            );
            DEFAULT_PVA_BROADCAST_PORT
        }
        None => {
            error!(
                "EPICS_PVA_BROADCAST_PORT is not set. Use default {}.",
                DEFAULT_PVA_BROADCAST_PORT
            );
            DEFAULT_PVA_BROADCAST_PORT
        }
    };
    (ca_server_port, pva_server_port)
}
