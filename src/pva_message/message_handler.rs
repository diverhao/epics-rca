use std::os::unix::net::SocketAddr;

use crate::pva_message::message::PvaMsg;



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
        if !handle_tcp_msg(src, msg) {
            return false;
        }
    }
    true
}


pub fn handle_udp_msg(src: &SocketAddr, msg: PvaMsg) {
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
