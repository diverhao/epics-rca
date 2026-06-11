use crate::udp::udp::CaCmd;
use ::log::debug;
use ::log::error;

pub const MAX_UDP_SEND: usize = 1024;

pub enum SearchReplyFlag {
    DoReply = 0x0a,
    DontReply = 0x05,
}

pub const CA_MINOR_VERSION: usize = 13;

pub fn pad_payload(payload: &mut Vec<u8>) -> Option<u32> {
    // resize payload
    let padding = (8 - payload.len() % 8) % 8;
    payload.resize(payload.len() + padding, 0);

    // make sure the payload size fits in u32 data
    let payload_size: u32 = if let Ok(payload_size) = payload.len().try_into() {
        payload_size
    } else {
        error!("Payload size too large");
        return None;
    };
    Some(payload_size)
}

pub fn build_ca_header(
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

pub fn build_name_search_buf_ca(name: &str, cid: u32) -> Option<Vec<u8>> {
    // build payload
    let mut payload: Vec<u8> = vec![];
    let name_bytes = name.as_bytes();
    payload.extend_from_slice(name_bytes);
    payload.push(0);
    pad_payload(&mut payload)?; // if payload too large, return None

    // build CA header
    let mut buf = build_ca_header(
        CaCmd::CaProtoSearch,
        payload.len() as u32,
        SearchReplyFlag::DontReply as u32,
        CA_MINOR_VERSION as u32,
        cid,
        cid,
    );

    // join header and payload
    buf.extend_from_slice(&payload);

    Some(buf)
}
