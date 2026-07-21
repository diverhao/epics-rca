use crate::ca_channel::ca_channel::CaChannel;
use crate::ca_channel::dbr::{ChannelAccessRights, ChannelSeverity, ChannelState, ChannelStatus};
use crate::ca_channel::dbr::{DbrType, DbrValue};
use crate::ca_channel::dbr_data::DbrData;
use crate::ca_channel::ca_monitor::{CaMonitor, MonitorState};
use crate::ca_channel::{self, ca_monitor};
use crate::ca_message::cmd::CaCmd;
use crate::ca_message::message::{CA_MINOR_VERSION, CaMsg, SearchReplyFlag};
use crate::context::context::get_context;
use crate::udp::udp::UDP;
use ::log::error;
use ::log::{debug, warn};
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/**
 * Handle Channel Access messages
 */
pub fn handle_udp_ca_msgs(src: &SocketAddr, msgs: Vec<CaMsg>) {
    for msg in msgs {
        handle_udp_msg(src, msg);
    }
}

pub fn handle_tcp_ca_msgs(src: &SocketAddr, msgs: Vec<CaMsg>) -> bool {
    for msg in msgs {
        if !handle_tcp_msg(src, msg) {
            return false;
        }
    }
    true
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

pub fn handle_tcp_msg(src: &SocketAddr, msg: CaMsg) -> bool {
    let cmd = msg.header().cmd;
    debug!("\nReceived from {src}: {msg}");
    match cmd {
        CaCmd::CaProtoEcho => handle_ca_proto_echo(msg),
        CaCmd::CaProtoEventAdd => return handle_ca_proto_event_add(msg),
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
        CaCmd::CaProtoReadNotify => return handle_ca_proto_read_notify(msg),
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
    true
}

fn handle_ca_proto_echo(msg: CaMsg) {
    println!("echo --------------------------------- received");
    // find the tcp and mark it alive
    let addr = msg.src().as_ref();
    match addr {
        Some(addr) => {
            let tcp = get_context().ca_tcps().tcp(addr);
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
    let channels = context.ca_channels();

    // find the channel
    let channel: Arc<CaChannel> = match channels.channel_by_cid(search_id) {
        Some(channel) => channel,
        None => return, // channel not found
    };

    let state = channel.state();
    if state != ChannelState::NameSearching {
        // duplicated search success message from server
        warn!(
            "Channel must be at NeverConnected or NameSearching state for CA_PROTO_SEARCH, but now is {:?}",
            state
        );
        return;
    }

    // update state and move channel from searching list to not_searching list
    channel.set_state(ChannelState::NameFound, true);

    // connect TCP (if not connected yet), and send handshake packets
    let server_addr = SocketAddr::new(src.ip(), server_port);
    if let Some(tcp) = context.ca_tcps().tcp(&server_addr) {
        // channel.connect_with_existing_tcp(tcp, server_addr);
        channel.set_state(ChannelState::TcpConnected, true);
        // add this channel to TCP
        let cid = channel.cid();
        tcp.add_cid(cid);
        // assign TCP to this channel
        channel.set_addr(Some(server_addr));
        // send handshake messages
        channel.send_connect_chan();
        return;
    } else {
        tokio::spawn(async move {
            channel.connect(server_addr).await;
        });
    }
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
    let channels = context.ca_channels();
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
    let Some(data_type) = DbrType::from_u16(header.data_type) else {
        return;
    };

    let cid = msg.header().param1;
    let sid = msg.header().param2;
    let data_count = msg.header().data_count;
    let context = get_context();
    let channels = context.ca_channels();
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
    channel.set_data_type_native(data_type);
    channel.set_data_count_native(data_count);
    // do it when everything is ready
    channel.set_state(ChannelState::Created, true);

    // send out CA_PROTO_EVENT_ADD if the monitor is started before this message
    if channel.monitor_state() == MonitorState::Starting {
        channel.send_monitor_add();
    };

    // notify IO for this channel, i.e. send CA_PROTO_READ_NOTIFY
    // and CA_PROTO_WRITE_NOTIFY if there were get() or put() started
    channel_io.get_step_2();
}

fn handle_ca_proto_read_notify(msg: CaMsg) -> bool {
    // tell the get() to proceed
    let ioid = msg.header().param2;

    let io = match get_context().ca_channels().io(ioid) {
        Some(io) => io,
        None => return true,
    };
    let cid = io.cid;
    let channel = match get_context().ca_channels().channel_by_cid(cid) {
        Some(channel) => channel,
        None => return true,
    };
    match channel.get_step_3(msg) {
        Ok(_dbr_data) => true,
        Err(reason) => {
            if reason.starts_with("DBR decode error:") {
                error!("Failed to decode CA_PROTO_READ_NOTIFY DBR payload: {reason}");
                false
            } else {
                true
            }
        }
    }
}

fn handle_ca_proto_event_add(msg: CaMsg) -> bool {
    let subid: u32 = msg.header().param2; // actually cid
    let cid = subid;
    let data_count = msg.header().data_count;
    let num_elem = msg.header().data_count;
    let data_type_num = msg.header().data_type;
    let data_type = match DbrType::from_u16(data_type_num) {
        Some(data_type) => data_type,
        None => {
            error!("Invalid CA_PROTO_EVENT_ADD DBR type {data_type_num}");
            return false;
        }
    };

    let channel = match get_context().ca_channels().channel_by_cid(subid) {
        Some(channel) => channel,
        None => return true, // cannot find channel in Channels registry
    };

    if channel.state() != ChannelState::Created {
        debug!("Channel not Created.");
        return true;
    }

    // must be Starting or Running
    if channel.monitor_state() == MonitorState::NotRunning {
        debug!("Monitor has been stopped");
        return true;
    }

    let callback_and_data = if let Some(callback) = channel.monitor_callback().clone() {
        // Decode the data before mutating monitor state. If this payload is malformed,
        // the current event is discarded and the TCP buffer will be flushed by the caller.
        match DbrData::from_buf(msg.payload(), data_type, data_count) {
            Ok(dbr_data) => Some((callback, dbr_data)),
            Err(reason) => {
                error!("Failed to decode CA_PROTO_EVENT_ADD DBR payload: {reason}");
                return false;
            }
        }
    } else {
        None
    };

    // update the monitor state each time
    if channel.monitor_state() == MonitorState::Starting {
        // for benchmark
        get_context()
            .ca_tcps()
            .running_monitor_count
            .fetch_add(1, Ordering::Relaxed);
        if get_context()
            .ca_tcps()
            .running_monitor_count
            .load(Ordering::Relaxed)
            == 100000
        {
            println!("OKOKOK");
            let start = get_context().ca_tcps().start;
            let elapsed = start.elapsed();

            println!("elapsed: {:?}", elapsed);
            println!("seconds: {:.3}", elapsed.as_secs_f64());
        }
        channel.set_monitor_state(MonitorState::Running);
        channel.set_monitor_data_count(data_count);
        channel.set_monitor_data_type(data_type);
    }

    // call callback
    // todo: ECA_XXX status
    if let Some((callback, dbr_data)) = callback_and_data {
        callback(cid, data_type, data_count, &dbr_data);
    } else {
        // no callback
    }
    true
}

fn handle_ca_proto_event_cancel(msg: CaMsg) {
    // do nothing
}

fn handle_ca_proto_clear_channel(_msg: CaMsg) {
    // do nothing
}

fn handle_ca_proto_create_ch_fail(msg: CaMsg) {
    let cid = msg.header().param1;
    if let Some(channel) = get_context().ca_channels().channel_by_cid(cid) {
        tokio::spawn(async move {
            channel.reconnect().await;
        });
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
