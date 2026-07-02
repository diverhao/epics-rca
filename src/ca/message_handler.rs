use crate::ca::cmd::CaCmd;
use crate::ca::header::CaHeader;
use crate::ca::message::{CA_MINOR_VERSION, CaMsg, SearchReplyFlag};
use crate::channel::dbr::{ChannelAccessRights, ChannelSeverity, ChannelState, ChannelStatus};
use crate::channel::dbr::{DbrType, DbrValue};
use crate::channel::monitor::{Monitor, MonitorState};
use crate::channel::{self, monitor};
use crate::context::context::get_context;
use crate::udp::udp::UDP;
use ::log::error;
use ::log::{debug, warn};
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/**
 * Handle Channel Access messages
 */
pub fn handle_udp_msgs(src: &SocketAddr, msgs: Vec<CaMsg>) {
    for msg in msgs {
        handle_udp_msg(src, msg);
    }
}

pub fn handle_tcp_msgs(src: &SocketAddr, msgs: Vec<CaMsg>) {
    for msg in msgs {
        handle_tcp_msg(src, msg);
    }
}

pub fn handle_udp_msg(src: &SocketAddr, msg: CaMsg) {
    let cmd = msg.header().cmd;
    debug!("\nReceived from {src}: {msg}");
    match cmd {
        CaCmd::CaProtoVersion => handle_ca_proto_version(msg),
        CaCmd::CaProtoSearch => handle_ca_proto_search(msg),
        CaCmd::CaProtoNotFound => handle_ca_proto_not_found(msg),
        CaCmd::CaProtoRsrvIsUp => handle_ca_proto_rsrv_is_up(msg),
        CaCmd::CaRepeaterConfirm => handle_ca_repeater_confirm(msg),
        CaCmd::CaRepeaterRegister => handle_ca_repeater_register(msg),
        _ => {}
    }
}

pub fn handle_tcp_msg(src: &SocketAddr, msg: CaMsg) {
    let cmd = msg.header().cmd;
    debug!("\nReceived from {src}: {msg}");
    match cmd {
        CaCmd::CaProtoEcho => handle_ca_proto_echo(msg),
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

fn handle_ca_proto_echo(msg: CaMsg) {
    // find the tcp and mark it alive
    let addr = msg.src().as_ref();
    match addr {
        Some(addr) => {
            let tcp = get_context().tcps().tcp(addr);
            match tcp {
                Some(tcp) => {
                    tcp.set_alive(true);
                }
                None => {}
            }
        }
        None => {}
    }
}

fn handle_ca_proto_version(_msg: CaMsg) {
    // do nothing
}

pub fn handle_ca_proto_search(msg: CaMsg) {
    // extract info from message
    let src = match msg.src() {
        Some(src) => src,
        None => return,
    };
    let search_id = msg.header().param2;
    let server_port = msg.header().data_type;
    let context = get_context();
    let channels = context.channels();

    // find the channel
    let channel = match channels.channel_by_cid(search_id) {
        Some(channel) => channel,
        None => return, // channel not found
    };

    let state = channel.state();
    if state != ChannelState::NeverConnected && state != ChannelState::NameSearching {
        error!("Channel must be at NeverConnected or NameSearching state for CA_PROTO_SEARCH");
        return;
    }

    // update state
    channel.set_state(ChannelState::NameFound, true);

    // connect TCP (if not connected yet), and send handshake packets
    let server_addr = SocketAddr::new(src.ip(), server_port);
    tokio::spawn(async move {
        channel.connect(server_addr).await;
    });
}

fn handle_ca_proto_access_rights(msg: CaMsg) {
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
    let channel = match channels.channel_by_cid(cid) {
        Some(channel) => channel,
        None => {
            // cannot find channel in Channels registry, may be destroyed
            // do nothing
            return;
        }
    };
    channel.set_access_right(access_right);
}

fn handle_ca_proto_create_chan(msg: CaMsg) {
    let header = msg.header();
    let Some(dbr_type) = DbrType::from_u16(header.data_type) else {
        return;
    };

    let cid = msg.header().param1;
    let sid = msg.header().param2;
    let data_count = msg.header().data_count;
    let context = get_context();
    let channels = context.channels();
    let channel = match channels.channel_by_cid(cid) {
        Some(channel) => channel,
        None => return, // cannot find channel in Channels registry, may have been destroyed
    };
    let channel_io = Arc::clone(&channel);
    let state = channel.state();
    if state != ChannelState::TcpConnected {
        error!("Channel must be at TcpConnected state for CA_PROTO_CREATE_CHAN");
        return;
    }
    channel.set_sid(sid);
    channel.set_dbr_type_native(dbr_type);
    channel.set_data_count_native(data_count);
    // do it when everything is ready
    channel.set_state(ChannelState::Created, true);

    // send out CA_PROTO_EVENT_ADD if the monitor is started before this message
    if channel.monitor_state() == MonitorState::Starting {
        tokio::spawn(async move {
            channel.send_monitor_add().await;
        });
    };

    // notify IO for this channel, i.e. send CA_PROTO_READ_NOTIFY
    // and CA_PROTO_WRITE_NOTIFY if there were get() or put() started
    tokio::spawn(async move {
        channel_io.get_step_2().await;
    });
}

fn handle_ca_proto_read_notify(msg: CaMsg) {
    // tell the get() to proceed
    let ioid = msg.header().param2;

    let io = match get_context().channels().io(ioid) {
        Some(io) => io,
        None => return,
    };
    let cid = io.cid;
    let channel = match get_context().channels().channel_by_cid(cid) {
        Some(channel) => channel,
        None => return,
    };
    channel.get_step_3(msg);
}

fn handle_ca_proto_event_add(msg: CaMsg) {
    let subid: u32 = msg.header().param2; // actually cid
    let data_count = msg.header().data_count;
    let num_elem = msg.header().data_count;
    let dbr_type_num = msg.header().data_type;
    let dbr_type = match DbrType::from_u16(dbr_type_num) {
        Some(dbr_type) => dbr_type,
        None => return,
    };
    let channel = match get_context().channels().channel_by_cid(subid) {
        Some(channel) => channel,
        None => return, // cannot find channel in Channels registry
    };

    if channel.state() != ChannelState::Created {
        debug!("Channel not Created.");
        return;
    }

    // must be Starting or Running
    if channel.monitor_state() == MonitorState::NotRunning {
        debug!("Monitor has been stopped");
        return;
    }

    // update value and meta first
    channel.update_value(msg.payload(), num_elem, dbr_type);

    // update the monitor state each time
    channel.set_monitor_state(MonitorState::Running);
    channel.set_monitor_data_count(data_count);
    channel.set_monitor_data_type(dbr_type);

    // call callback, it is already set
    channel.call_monitor_callback();
}

fn handle_ca_proto_event_cancel(msg: CaMsg) {
    // do nothing
}

fn handle_ca_proto_clear_channel(_msg: CaMsg) {
    // do nothing
}

fn handle_ca_proto_create_ch_fail(msg: CaMsg) {
    let cid = msg.header().param1;
    if let Some(channel) = get_context().channels().channel_by_cid(cid) {
        tokio::spawn(async move { channel.reconnect().await });
    }
}

fn handle_ca_proto_server_disconn(msg: CaMsg) {
    handle_ca_proto_create_ch_fail(msg);
}

fn handle_ca_proto_error(_msg: CaMsg) {
    // do nothing
}

// ---------- need to implement ---------

fn handle_ca_proto_write_notify(_msg: CaMsg) {}

fn handle_ca_proto_rsrv_is_up(_msg: CaMsg) {}

fn handle_ca_repeater_confirm(_msg: CaMsg) {}

fn handle_ca_repeater_register(_msg: CaMsg) {}

// ------------- server does not reply, not implemented or deprecated --------------

fn handle_ca_proto_not_found(_msg: CaMsg) {
    // server replies only when CA_PROTO_SEARCH has DO_REPLY bit
    // not implemented
}

fn handle_ca_proto_read(_msg: CaMsg) {
    // deprecated
}

fn handle_ca_proto_snapshot(_msg: CaMsg) {
    // deprecated
}

fn handle_ca_proto_build(_msg: CaMsg) {
    // deprecated
}

fn handle_ca_proto_read_sync(_msg: CaMsg) {
    // deprecated
}

fn handle_ca_proto_read_build(_msg: CaMsg) {
    // deprecated
}

fn handle_ca_proto_client_name(_msg: CaMsg) {
    // server does not reply this message
}

fn handle_ca_proto_host_name(_msg: CaMsg) {
    // server does not reply this message
}

fn handle_ca_proto_signal(_msg: CaMsg) {
    // deprecated
}

fn handle_ca_proto_write(_msg: CaMsg) {
    // server does not reply
}

fn handle_ca_proto_events_off(_msg: CaMsg) {
    // server does not reply
}

fn handle_ca_proto_events_on(_msg: CaMsg) {
    // server does not reply
}
