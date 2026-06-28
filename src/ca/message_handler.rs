use crate::ca::ca_cmd::CaCmd;
use crate::ca::header::CaHeader;
use crate::ca::message::{CA_MINOR_VERSION, CaMsg, SearchReplyFlag};
use crate::channel;
use crate::channel::channel::{ChannelAccessRights, ChannelState, DbrType};
use crate::context::context::get_context;
use crate::udp::udp::UDP;
use ::log::debug;
use ::log::error;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/**
 * Handle Channel Access messages
 */
pub async fn handle_udp_msgs(src: &SocketAddr, msgs: &Vec<CaMsg>) {
    for msg in msgs {
        handle_tcp_msg(src, msg).await;
    }
}

pub async fn handle_tcp_msgs(src: &SocketAddr, msgs: &Vec<CaMsg>) {
    for msg in msgs {
        handle_udp_msg(src, msg).await;
    }
}

pub async fn handle_tcp_msg(src: &SocketAddr, msg: &CaMsg) {
    let header = msg.header();
    debug!("\nReceived from {src}: {msg}");
    match header.cmd {
        CaCmd::CaProtoVersion => handle_ca_proto_version(msg),
        CaCmd::CaProtoSearch => handle_ca_proto_search(msg).await,
        CaCmd::CaProtoNotFound => handle_ca_proto_not_found(msg),
        CaCmd::CaProtoEcho => handle_ca_proto_echo(msg),
        CaCmd::CaProtoRsrvIsUp => handle_ca_proto_rsrv_is_up(msg),
        CaCmd::CaRepeaterConfirm => handle_ca_repeater_confirm(msg),
        CaCmd::CaRepeaterRegister => handle_ca_repeater_register(msg),
        _ => {}
    }
}

pub async fn handle_udp_msg(src: &SocketAddr, msg: &CaMsg) {
    let header = msg.header();
    debug!("\nReceived from {src}: {msg}");
    match header.cmd {
        CaCmd::CaProtoEventAdd => handle_ca_proto_event_add(msg),
        CaCmd::CaProtoEventCancel => handle_ca_proto_event_cancel(msg),
        CaCmd::CaProtoRead => handle_ca_proto_read(msg),
        CaCmd::CaProtoWrite => handle_ca_proto_write(msg),
        CaCmd::CaProtoSnapshot => handle_ca_proto_snapshot(msg),
        CaCmd::CaProtoBuild => handle_ca_proto_build(msg),
        CaCmd::CaProtoEventsOff => handle_ca_proto_events_off(msg),
        CaCmd::CaProtoEventsOn => handle_ca_proto_events_on(msg),
        CaCmd::CaProtoReadSync => handle_ca_proto_read_sync(msg),
        CaCmd::CaProtoError => handle_ca_proto_error(msg),
        CaCmd::CaProtoClearChannel => handle_ca_proto_clear_channel(msg),
        CaCmd::CaProtoReadNotify => handle_ca_proto_read_notify(msg),
        CaCmd::CaProtoReadBuild => handle_ca_proto_read_build(msg),
        CaCmd::CaProtoCreateChan => handle_ca_proto_create_chan(msg),
        CaCmd::CaProtoWriteNotify => handle_ca_proto_write_notify(msg),
        CaCmd::CaProtoClientName => handle_ca_proto_client_name(msg),
        CaCmd::CaProtoHostName => handle_ca_proto_host_name(msg),
        CaCmd::CaProtoAccessRights => handle_ca_proto_access_rights(msg),
        CaCmd::CaProtoSignal => handle_ca_proto_signal(msg),
        CaCmd::CaProtoCreateChFail => handle_ca_proto_create_ch_fail(msg),
        CaCmd::CaProtoServerDisconn => handle_ca_proto_server_disconn(msg),
        _ => {}
    }
}

fn handle_ca_proto_version(msg: &CaMsg) {
    // do nothing
}

pub async fn handle_ca_proto_search(msg: &CaMsg) {
    // find the channel from search id (cid)
    let src = msg.src();
    match src {
        Some(src) => {
            let search_id = msg.header().param2;
            let server_port = msg.header().data_type;
            let context = get_context();
            let channels = context.channels();
            // find the channel
            let channel = channels.channel_by_cid(search_id);
            match channel {
                Some(channel) => {
                    let state = channel.state();
                    if state != ChannelState::NeverConnected && state != ChannelState::NameSearching
                    {
                        error!(
                            "Channel must be at NeverConnected or NameSearching state for CA_PROTO_SEARCH"
                        );
                        return;
                    }
                    // update state and search counter
                    channel.set_state(ChannelState::NameFound);
                    channel.reset_search_counter();

                    let server_addr = SocketAddr::new(src.ip(), server_port);
                    channel.connect(server_addr).await;
                }
                None => {
                    // channel not found by cid
                    return;
                }
            }
        }
        None => {
            return;
        }
    }
}

fn handle_ca_proto_access_rights(msg: &CaMsg) {
    // get access right
    let access_right_raw = msg.header().param2;
    let access_right = match access_right_raw & 0x03 {
        0 => ChannelAccessRights::None,
        1 => ChannelAccessRights::Read,
        _ => ChannelAccessRights::ReadWrite,
    };
    // get channel
    let cid = msg.header().param1;
    let context = get_context();
    let channels = context.channels();
    let channel = channels.channel_by_cid(cid);
    match channel {
        Some(channel) => {
            let state = channel.state();
            if state == ChannelState::TcpConnected {
                channel.set_access_right(access_right);
            } else {
                error!("Channel must be at TcpConnected state for CA_PROTO_ACCESS_RIGHTS");
            }
        }
        None => {}
    }
}

fn handle_ca_proto_create_chan(msg: &CaMsg) {
    let header = msg.header();
    let dbr_type = header.data_type;
    let dbr_type = match dbr_type {
        0 => DbrType::String,
        1 => DbrType::Short,
        2 => DbrType::Float,
        3 => DbrType::Enum,
        4 => DbrType::Char,
        5 => DbrType::Long,
        6 => DbrType::Double,
        7 => DbrType::StsString,
        8 => DbrType::StsShort,
        9 => DbrType::StsFloat,
        10 => DbrType::StsEnum,
        11 => DbrType::StsChar,
        12 => DbrType::StsLong,
        13 => DbrType::StsDouble,
        14 => DbrType::TimeString,
        15 => DbrType::TimeShort,
        16 => DbrType::TimeFloat,
        17 => DbrType::TimeEnum,
        18 => DbrType::TimeChar,
        19 => DbrType::TimeLong,
        20 => DbrType::TimeDouble,
        21 => DbrType::GrString,
        22 => DbrType::GrShort,
        23 => DbrType::GrFloat,
        24 => DbrType::GrEnum,
        25 => DbrType::GrChar,
        26 => DbrType::GrLong,
        27 => DbrType::GrDouble,
        28 => DbrType::CtrlString,
        29 => DbrType::CtrlShort,
        30 => DbrType::CtrlFloat,
        31 => DbrType::CtrlEnum,
        32 => DbrType::CtrlChar,
        33 => DbrType::CtrlLong,
        34 => DbrType::CtrlDouble,
        35 => DbrType::PutAckt,
        36 => DbrType::PutAcks,
        37 => DbrType::StsAckString,
        38 => DbrType::ClassName,
        _ => return,
    };

    // get channel
    let cid = msg.header().param1;
    let context = get_context();
    let channels = context.channels();
    let channel = channels.channel_by_cid(cid);
    match channel {
        Some(channel) => {
            let state = channel.state();
            if state != ChannelState::TcpConnected {
                error!("Channel must be at TcpConnected state for CA_PROTO_CREATE_CHAN");
                return;
            }
            channel.set_state(ChannelState::Created);
            channel.set_dbr_type_native(dbr_type);
        }
        None => {}
    }
}

fn handle_ca_proto_not_found(msg: &CaMsg) {}

fn handle_ca_proto_echo(msg: &CaMsg) {}

fn handle_ca_proto_rsrv_is_up(msg: &CaMsg) {}

fn handle_ca_repeater_confirm(msg: &CaMsg) {}

fn handle_ca_repeater_register(msg: &CaMsg) {}

fn handle_ca_proto_event_add(msg: &CaMsg) {}

fn handle_ca_proto_event_cancel(msg: &CaMsg) {}

fn handle_ca_proto_read(msg: &CaMsg) {}

fn handle_ca_proto_write(msg: &CaMsg) {}

fn handle_ca_proto_snapshot(msg: &CaMsg) {}

fn handle_ca_proto_build(msg: &CaMsg) {}

fn handle_ca_proto_events_off(msg: &CaMsg) {}

fn handle_ca_proto_events_on(msg: &CaMsg) {}

fn handle_ca_proto_read_sync(msg: &CaMsg) {}

fn handle_ca_proto_error(msg: &CaMsg) {}

fn handle_ca_proto_clear_channel(msg: &CaMsg) {}

fn handle_ca_proto_read_notify(msg: &CaMsg) {}

fn handle_ca_proto_read_build(msg: &CaMsg) {}

fn handle_ca_proto_write_notify(msg: &CaMsg) {}

fn handle_ca_proto_client_name(msg: &CaMsg) {}

fn handle_ca_proto_host_name(msg: &CaMsg) {}

fn handle_ca_proto_signal(msg: &CaMsg) {}

fn handle_ca_proto_create_ch_fail(msg: &CaMsg) {}

fn handle_ca_proto_server_disconn(msg: &CaMsg) {}
