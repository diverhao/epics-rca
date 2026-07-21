use crate::{
    ca_channel::dbr::ChannelState,
    context::context::get_context,
    pva_message::{
        cmd::PvaCmd,
        header::MsgEndian,
        message::{
            PvaAuthnz, PvaMsg, PvaStatus, build_connection_validation, build_create_channel,
        },
        primitive::{PvaElement, PvaSize},
        typ::PvaType,
        type_registry::PvaTypeRegistry,
        value::PvaValue,
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
pub fn handle_udp_pva_msgs(src: &SocketAddr, msgs: Vec<PvaMsg>) -> bool {
    for msg in msgs {
        if !handle_udp_pva_msg(src, msg) {
            return false;
        }
    }
    return true;
}

pub fn handle_tcp_pva_msgs(src: &SocketAddr, msgs: Vec<PvaMsg>) -> bool {
    for msg in msgs {
        if !handle_tcp_pva_msg(src, msg) {
            return false;
        }
    }
    true
}

pub fn handle_udp_pva_msg(src: &SocketAddr, msg: PvaMsg) -> bool {
    let cmd = msg.header().cmd();
    // dummy registry
    let mut pva_type_registry = PvaTypeRegistry::new();
    debug!("\nReceived from {src}: {msg}");
    match cmd {
        PvaCmd::SearchResponse => {
            if let Err(err) = handle_search_response(msg, src, &mut pva_type_registry) {
                error!("Failed to handle PVA search response from {src}: {err}");
                return false;
            }
        }
        _ => {}
    }
    return true;
}

pub fn handle_tcp_pva_msg(src: &SocketAddr, msg: PvaMsg) -> bool {
    let cmd = msg.header().cmd();
    debug!("\nReceived from {src}: {msg}");
    match cmd {
        PvaCmd::SetEndianess => {
            if let Err(err) = handle_set_endianess(msg, src) {
                error!("Failed to handle PVA Set Endianess from {src}: {err}");
                return false;
            }
        }
        PvaCmd::ConnectionValidation => {
            if let Err(err) = handle_connection_validation(msg, src) {
                error!("Failed to handle PVA Connection Validation from {src}: {err}");
                return false;
            }
        }
        PvaCmd::ConnectionValidated => {
            if let Err(err) = handle_connection_validated(msg, src) {
                error!("Failed to handle PVA Connection Validated from {src}: {err}");
                return false;
            }
        }

        PvaCmd::CreateChannel => {
            if let Err(err) = handle_create_channel(msg, src) {
                error!("Failed to handle PVA Create Channel from {src}: {err}");
                return false;
            }
        }
        _ => {}
    }
    true
}

// --------------------- handlers --------------------

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

    debug!(
        "\nSearchResponse {{\n\
         \x20   guid: {guid:02x?},\n\
         \x20   search_sequence_id: {search_seq_id},\n\
         \x20   server_address: {},\n\
         \x20   server_port: {port},\n\
         \x20   protocol: {protocol:?},\n\
         \x20   found: {found},\n\
         \x20   search_instance_ids: {cids:?},\n\
         }}",
        server_addr.ip()
    );

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
        if let Some(tcp) = context.pva_tcps().tcp(&server_addr) {
            // channel.connect_with_existing_tcp(tcp, server_addr);
            channel.set_state(ChannelState::TcpConnected, true);
            // add this channel to TCP
            let cid = channel.cid();
            tcp.add_cid(cid);
            // assign TCP to this channel
            channel.set_addr(Some(server_addr));
        } else {
            // try to connect tcp if not connected
            tokio::spawn(async move {
                channel.connect(server_addr).await;
            });
        }
    }

    Ok(())
}

pub fn handle_set_endianess(msg: PvaMsg, src: &SocketAddr) -> Result<(), String> {
    let endian = msg.header().flags().endian;

    let tcps = get_context().pva_tcps();
    let tcp = match tcps.tcp(src) {
        Some(tcp) => tcp,
        None => return Err("Failed to find TCP".to_string()),
    };
    debug!("Set TCP endian as {:?}", endian);
    tcp.set_pva_endian(endian);

    Ok(())
}

pub fn handle_connection_validation(msg: PvaMsg, src: &SocketAddr) -> Result<(), String> {
    let endian = msg.header().flags().endian;

    let tcps = get_context().pva_tcps();
    let tcp = match tcps.tcp(src) {
        Some(tcp) => tcp,
        None => return Err("Failed to find TCP".to_string()),
    };
    let mut registry = tcp.pva_types_in_mut();

    // struct connectionValidationRequest {
    //     int serverReceiveBufferSize;
    //     short serverIntrospectionRegistryMaxSize;
    //     string[] authNZ;
    // };

    let buf = msg.payload();
    let mut offset = 0;
    let server_receive_buffer_size = match PvaValue::from_buf(
        Arc::new(PvaType::UInt),
        buf,
        &mut offset,
        endian,
        &mut registry,
    )? {
        PvaValue::UInt(size) => size,
        _ => return Err("".to_string()),
    };
    let server_introspection_registry_max_size = match PvaValue::from_buf(
        Arc::new(PvaType::UShort),
        buf,
        &mut offset,
        endian,
        &mut registry,
    )? {
        PvaValue::UShort(size) => size,
        _ => return Err("".to_string()),
    };

    let authnz = match PvaValue::from_buf(
        Arc::new(PvaType::StringVarSizeArray),
        buf,
        &mut offset,
        endian,
        &mut registry,
    )? {
        PvaValue::StringVarSizeArray(strs) => strs,
        _ => return Err("".to_string()),
    };

    debug!(
        "\nConnectionValidationRequest {{\n\
         \x20   server_receive_buffer_size: {server_receive_buffer_size},\n\
         \x20   server_introspection_registry_max_size: {server_introspection_registry_max_size},\n\
         \x20   auth_nz: {authnz:?},\n\
         }}"
    );

    drop(registry); // we will use out registry

    // tell server the authentication choice, prefer CA
    let selected_auth = if authnz.contains(&"ca".to_string()) {
        PvaAuthnz::Ca
    } else if authnz.contains(&"anonymous".to_string()) {
        PvaAuthnz::Anonymous
    } else {
        return Err("IOC does not provide ca or anonymous authentication".to_string());
    };

    let reply_msg = {
        let mut registry = tcp.pva_types_out_mut();
        build_connection_validation(selected_auth, endian, &mut registry)?
    };
    tcp.send_pva_msgs(vec![reply_msg]);

    Ok(())
}

pub fn handle_connection_validated(msg: PvaMsg, src: &SocketAddr) -> Result<(), String> {
    // struct connectionValidated {
    //     Status status;
    // };
    let endian = msg.header().flags().endian;

    let tcps = get_context().pva_tcps();
    let tcp = match tcps.tcp(src) {
        Some(tcp) => tcp,
        None => return Err("Failed to find TCP".to_string()),
    };

    let mut registry = tcp.pva_types_in_mut();

    let buf = msg.payload();
    let mut offset = 0;

    let status = PvaStatus::from_buf(buf, &mut offset, endian, &mut registry)?;

    debug!("\nConnectionValidated {{\n\x20   status: {status},\n}}");

    if status.is_success() {
        println!("abc");
        // tell server to create channel
        let mut names_cids: Vec<(String, u32)> = vec![];
        for cid in tcp.cids().iter() {
            let channel = match get_context().pva_channels().channel_by_cid(*cid) {
                Some(channel) => channel,
                None => continue,
            };
            // make sure the channel is at right state
            match channel.state() {
                ChannelState::TcpConnected => {}
                _ => continue,
            };
            let name = channel.name().to_string();
            names_cids.push((name, *cid));
        }
        // outgoing registry
        let mut registry = tcp.pva_types_out_mut();
        let msg = build_create_channel(names_cids, endian, &mut registry)?;
        tcp.send_pva_msgs(vec![msg]);
    } else {
        // disconnect tcp (re-search channels)
        tcp.disconnect(true, true);
    }

    Ok(())
}

pub fn handle_create_channel(msg: PvaMsg, src: &SocketAddr) -> Result<(), String> {
    let endian = msg.header().flags().endian;

    let tcps = get_context().pva_tcps();
    let tcp = match tcps.tcp(src) {
        Some(tcp) => tcp,
        None => return Err("Failed to find TCP".to_string()),
    };
    let mut registry = tcp.pva_types_in_mut();

    // struct createChannelResponse {
    //     int clientChannelID;
    //     int serverChannelID;
    //     Status status;
    // };

    let buf = msg.payload();
    let mut offset = 0;
    let cid = match PvaValue::from_buf(
        Arc::new(PvaType::UInt),
        buf,
        &mut offset,
        endian,
        &mut registry,
    )? {
        PvaValue::UInt(cid) => cid,
        _ => return Err("".to_string()),
    };

    let sid = match PvaValue::from_buf(
        Arc::new(PvaType::UInt),
        buf,
        &mut offset,
        endian,
        &mut registry,
    )? {
        PvaValue::UInt(cid) => cid,
        _ => return Err("".to_string()),
    };

    let status = PvaStatus::from_buf(buf, &mut offset, endian, &mut registry)?;
    let channel = match get_context().pva_channels().channel_by_cid(cid) {
        Some(channel) => channel,
        _ => return Err("Failed to find channel".to_string()),
    };

    if status.is_success() {
        channel.set_sid(sid);
        channel.set_state(ChannelState::Created, false);
    } else {
        // re-search the channel
        channel.reconnect();
        return Err("Failed to create channel".to_string());
    }

    Ok(())
}

// ------------------------ helpers ----------------------

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

fn decode_string_array(
    buf: &[u8],
    offset: &mut usize,
    endian: MsgEndian,
    registry: &mut PvaTypeRegistry,
) -> Result<Vec<String>, String> {
    let mut strs: Vec<String> = vec![];
    let size = usize::from_buf(buf, offset, endian)?;

    println!("string array size {}", size);

    for ii in 0..size {
        let str =
            match PvaValue::from_buf(Arc::new(PvaType::String), buf, offset, endian, registry)? {
                PvaValue::String(str) => str,
                _ => return Err("".to_string()),
            };
        strs.push(str);
    }

    Ok(strs)
}
