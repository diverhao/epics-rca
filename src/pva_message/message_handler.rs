use crate::{
    ca_channel::dbr::ChannelState,
    context::context::get_context,
    pva_message::{
        cmd::PvaCmd, message::PvaMsg, typ::PvaType, type_registry::PvaTypeRegistry, value::PvaValue,
    },
};
use log::{debug, error, warn};
use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::Arc,
};

/**
 * Handle Channel Access messages
 */
pub fn handle_udp_pva_msgs(src: &SocketAddr, msgs: Vec<PvaMsg>) {
    for msg in msgs {
        handle_udp_pva_msg(src, msg);
    }
}

pub fn handle_tcp_pva_msgs(src: &SocketAddr, msgs: Vec<PvaMsg>) -> bool {
    for msg in msgs {
        if handle_tcp_pva_msg(src, msg) {
            return false;
        }
    }
    true
}

pub fn handle_udp_pva_msg(src: &SocketAddr, msg: PvaMsg) {
    let cmd = msg.header().cmd();
    // dummy registry
    let mut pva_type_registry = PvaTypeRegistry::new();
    debug!("\nReceived from {src}: {msg}");
    match cmd {
        PvaCmd::SearchResponse => {
            if let Err(err) = handle_search_response(msg, src, &mut pva_type_registry) {
                error!("Failed to handle PVA search response from {src}: {err}");
            }
        }
        _ => {}
    }
}

pub fn handle_search_response(
    msg: PvaMsg,
    src: &SocketAddr,
    pva_type_registry: &mut PvaTypeRegistry,
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

    let endian = msg.header().flags().endian;

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
    let server_addr = decode_pva_socket_addr(ip, port, src.ip());

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
    let found = match PvaValue::from_buf(
        Arc::new(PvaType::Boolean),
        buf,
        &mut offset,
        endian,
        pva_type_registry,
    )? {
        PvaValue::Boolean(found) => found,
        _ => return Err("".to_string()),
    };

    // cids
    let mut cids: Vec<u32> = vec![];
    let count = match PvaValue::from_buf(
        Arc::new(PvaType::UShort),
        buf,
        &mut offset,
        endian,
        pva_type_registry,
    )? {
        PvaValue::UShort(count) => count,
        _ => return Err("".to_string()),
    };
    for ii in 0..count {
        let cid = match PvaValue::from_buf(
            Arc::new(PvaType::UInt),
            buf,
            &mut offset,
            endian,
            pva_type_registry,
        )? {
            PvaValue::UInt(cid) => cid,
            _ => return Err("".to_string()),
        };
        cids.push(cid);
    }

    if found == false && protocol == "tcp" {
        return Err("Not found".to_string());
    }

    // update channel from above info
    for cid in cids {
        let context = get_context();
        let channels = context.pva_channels();

        // find the channel
        let channel = match channels.channel_by_cid(cid) {
            Some(channel) => channel,
            None => continue, // channel not found
        };

        let state = channel.state();
        if state != ChannelState::NameSearching {
            // duplicated search success message from server
            warn!(
                "Channel must be at NeverConnected or NameSearching state for CA_PROTO_SEARCH, but now is {:?}",
                state
            );
            continue;
        }

        // update state and move channel from searching list to not_searching list
        channel.set_state(ChannelState::NameFound, true);

        // connect TCP (if not connected yet), and send handshake packets
        if let Some(tcp) = context.tcps().tcp(&server_addr) {
            // channel.connect_with_existing_tcp(tcp, server_addr);
            channel.set_state(ChannelState::TcpConnected, true);
            // add this channel to TCP
            let cid = channel.cid();
            tcp.add_cid(cid);
            // assign TCP to this channel
            channel.set_addr(Some(server_addr));
            // todo: send handshake messages
            channel.send_connect_chan();
        } else {
            // todo: try to connect tcp if not connected
            tokio::spawn(async move {
                channel.connect(server_addr).await;
            });
        }
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

pub fn handle_tcp_pva_msg(src: &SocketAddr, msg: PvaMsg) -> bool {
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
