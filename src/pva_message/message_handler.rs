use crate::{
    ca_message::cmd::CaCmd,
    pva_message::{cmd::PvaCmd, header::MsgEndian, message::PvaMsg, typ::PvaType, value::PvaValue},
};
use log::debug;
use std::net::{IpAddr, Ipv4Addr};
use std::{
    fmt::Error,
    net::{Ipv6Addr, SocketAddr},
    sync::Arc,
};

/**
 * Handle Channel Access messages
 */
pub fn handle_udp_msgs(src: &SocketAddr, msgs: Vec<PvaMsg>) {
    for msg in msgs {
        handle_udp_msg(src, msg);
    }
}

pub fn handle_tcp_msgs(src: &SocketAddr, msgs: Vec<PvaMsg>) -> bool {
    for msg in msgs {
        if handle_tcp_msg(src, msg) {
            return false;
        }
    }
    true
}

pub fn handle_udp_msg(src: &SocketAddr, msg: PvaMsg) {
    let cmd = msg.header().cmd();
    debug!("\nReceived from {src}: {msg}");
    match cmd {
        PvaCmd::SearchResponse => {}
        _ => {}
    }
}

pub fn handle_search_response(
    msg: PvaMsg,
    src: &SocketAddr,
    endian: MsgEndian,
    pva_type_registry: &mut super::type_registry::PvaTypeRegistry,
) -> Result<(), String> {
    // struct searchResponse {
    //     byte[12] guid;
    //     int searchSequenceID;
    //     byte[16] serverAddress;
    //     short serverPort;
    //     string protocol;
    //     boolean found;
    //     int[] searchInstanceIDs;
    // };

    // skip guid
    let mut offset = 0;
    let buf = msg.payload();
    let guid: [u8; 12] = match buf[offset..offset + 12].try_into() {
        Ok(guid) => guid,
        Err(_) => return Err("".to_string()),
    };
    offset = offset + 12;

    // search sequence ID
    let search_seq_id = match PvaValue::from_buf(
        Arc::new(PvaType::Int),
        buf,
        &mut offset,
        endian,
        pva_type_registry,
    )? {
        PvaValue::Int(id) => id,
        _ => return Err("".to_string()),
    };

    let ip: &[u8; 16] = buf
        .get(offset..offset + 16)
        .ok_or_else(|| "PVA search response is too short for server address".to_string())?
        .try_into()
        .map_err(|_| "PVA server address must contain exactly 16 bytes".to_string())?;
    offset += 16;

    // server TCP port
    let port = match PvaValue::from_buf(
        Arc::new(PvaType::UShort),
        buf,
        &mut offset,
        endian,
        pva_type_registry,
    )? {
        PvaValue::UShort(port) => port,
        _ => return Err("".to_string()),
    };

    // the tcp address and port for this channel
    let tcp_socket_addr = decode_pva_socket_addr(ip, port, src.ip());

    // protocol
    let protocol = match PvaValue::from_buf(
        Arc::new(PvaType::String),
        buf,
        &mut offset,
        endian,
        pva_type_registry,
    )? {
        PvaValue::String(protocol) => protocol,
        _ => return Err("".to_string()),
    };

    // found
    let found = match PvaValue::from_buf(Arc::new(PvaType::Boolean), buf, &mut offset, endian, pva_type_registry)? {
        PvaValue::Boolean(found) => found,
        _ => return Err("".to_string())
    };

    // cids
    let mut cids: Vec<u32> = vec![];
    let count = match PvaValue::from_buf(Arc::new(PvaType::UShort), buf, &mut offset, endian, pva_type_registry)? {
        PvaValue::UShort(count) => count,
        _ => return Err("".to_string())
    };
    for ii in 0..count {
        let cid = match PvaValue::from_buf(Arc::new(PvaType::UInt), buf, &mut offset, endian, pva_type_registry)? {
            PvaValue::UInt(cid) => cid,
            _ => return Err("".to_string())
        };
        cids.push(cid);
    }



    Ok(())
}

fn decode_pva_socket_addr(address: &[u8; 16], port: u16, source_ip: IpAddr) -> SocketAddr {
    let ipv6 = Ipv6Addr::from(*address);

    let mut ip = match ipv6.to_ipv4_mapped() {
        Some(ipv4) => IpAddr::V4(ipv4),
        None => IpAddr::V6(ipv6),
    };

    // PVA permits an unspecified address, meaning use the
    // source address of the UDP search-response packet.
    if ip.is_unspecified() {
        ip = source_ip;
    }

    SocketAddr::new(ip, port)
}

pub fn handle_tcp_msg(src: &SocketAddr, msg: PvaMsg) -> bool {
    let cmd = msg.header().cmd();
    debug!("\nReceived from {src}: {msg}");
    match cmd {
        // PvaCmd::
        // CaCmd::CaProtoEcho => handle_ca_proto_echo(msg),
        // CaCmd::CaProtoEventAdd => return handle_ca_proto_event_add(msg),
        // CaCmd::CaProtoEventCancel => handle_ca_proto_event_cancel(msg),
        // CaCmd::CaProtoRead => handle_ca_proto_read(msg),
        // CaCmd::CaProtoWrite => handle_ca_proto_write(msg),
        // CaCmd::CaProtoSnapshot => handle_ca_proto_snapshot(msg),
        // CaCmd::CaProtoBuild => handle_ca_proto_build(msg),
        // CaCmd::CaProtoEventsOff => handle_ca_proto_events_off(msg),
        // CaCmd::CaProtoEventsOn => handle_ca_proto_events_on(msg),
        // CaCmd::CaProtoReadSync => handle_ca_proto_read_sync(msg),
        // CaCmd::CaProtoError => handle_ca_proto_error(msg),
        // CaCmd::CaProtoClearChannel => handle_ca_proto_clear_channel(msg),
        // CaCmd::CaProtoReadNotify => return handle_ca_proto_read_notify(msg),
        // CaCmd::CaProtoReadBuild => handle_ca_proto_read_build(msg),
        // CaCmd::CaProtoCreateChan => handle_ca_proto_create_chan(msg),
        // CaCmd::CaProtoWriteNotify => handle_ca_proto_write_notify(msg),
        // CaCmd::CaProtoClientName => handle_ca_proto_client_name(msg),
        // CaCmd::CaProtoHostName => handle_ca_proto_host_name(msg),
        // CaCmd::CaProtoAccessRights => handle_ca_proto_access_rights(msg),
        // CaCmd::CaProtoSignal => handle_ca_proto_signal(msg),
        // CaCmd::CaProtoCreateChFail => handle_ca_proto_create_ch_fail(msg),
        // CaCmd::CaProtoServerDisconn => handle_ca_proto_server_disconn(msg),
        _ => {}
    }
    true
}
