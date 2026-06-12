use crate::ca::ca_cmd::CaCmd;
use crate::context::context::get_context;
use ::log::debug;
use ::log::error;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub const MAX_UDP_SEND: usize = 1024;

pub enum SearchReplyFlag {
    DoReply = 0x0a,
    DontReply = 0x05,
}

pub const CA_MINOR_VERSION: usize = 13;

fn pad_payload(payload: &mut Vec<u8>) -> Option<u32> {
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

pub async fn send_ver_ca() {
    let context = get_context();
    let udp = context.udp();
    let buf = build_version_buf();
    udp.send(&buf).await;
}

pub struct CaHeader {
    pub cmd: CaCmd,
    pub payload_size: u32, // or 2 bytes
    pub data_type: u16,
    pub data_count: u32, // or 2 bytes
    pub param1: u32,
    pub param2: u32,
}

impl std::fmt::Display for CaHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CaHeader {{ cmd: {}, payload_size: {}, data_type: {}, data_count: {}, param1: {}, param2: {} }}",
            self.cmd, self.payload_size, self.data_type, self.data_count, self.param1, self.param2
        )
    }
}

impl CaHeader {
    fn encode(self: &Self) -> Vec<u8> {
        let mut use_extended_header = false;
        // use extended header if payload size > 16368 bytes
        if self.payload_size > 0x3ff0 {
            debug!("UDP payload size larger than 16368 bytes, use extended header");
            use_extended_header = true;
        }

        if !use_extended_header {
            let mut buf: Vec<u8> = Vec::with_capacity(16);
            buf.extend_from_slice(&(self.cmd as u16).to_be_bytes());
            buf.extend_from_slice(&(self.payload_size as u16).to_be_bytes());
            buf.extend_from_slice(&(self.data_type as u16).to_be_bytes()); // 2 bytes
            buf.extend_from_slice(&(self.data_count as u16).to_be_bytes()); // 2 bytes
            buf.extend_from_slice(&self.param1.to_be_bytes());
            buf.extend_from_slice(&self.param2.to_be_bytes());
            buf
        } else {
            let mut buf: Vec<u8> = Vec::with_capacity(24);
            buf.extend_from_slice(&(self.cmd as u16).to_be_bytes());
            buf.extend_from_slice(&(0xffff as u16).to_be_bytes());
            buf.extend_from_slice(&(self.data_type as u16).to_be_bytes());
            buf.extend_from_slice(&(0x0000 as u16).to_be_bytes());
            buf.extend_from_slice(&self.param1.to_be_bytes());
            buf.extend_from_slice(&self.param2.to_be_bytes());
            buf.extend_from_slice(&self.payload_size.to_be_bytes());
            buf.extend_from_slice(&self.data_count.to_be_bytes());
            buf
        }
    }

    /**
     * Decode Channel Access UDP message
     */
    pub fn decode(buf: &RwLockReadGuard<Vec<u8>>) -> Result<CaHeader, String> {
        if buf.len() < 16 {
            return Err(String::from("Remaining buffer too short"));
        }

        let extended_header = buf[2] == 0xff && buf[3] == 0xff;

        if extended_header {
            if buf.len() < 24 {
                return Err(String::from("Remaining buffer too short for an extended header"));
            }
        }

        // Channel Access always use big endian
        let cmd = u16::from_be_bytes([buf[0], buf[1]]);
        let data_type = u16::from_be_bytes([buf[4], buf[5]]);
        let param1 = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let param2 = u32::from_be_bytes([buf[12], buf[13], buf[14], buf[15]]);

        let (payload_size, data_count) = if extended_header {
            (
                u32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]]),
                u32::from_be_bytes([buf[20], buf[21], buf[22], buf[23]]),
            )
        } else {
            (
                u32::from(u16::from_be_bytes([buf[2], buf[3]])),
                u32::from(u16::from_be_bytes([buf[6], buf[7]])),
            )
        };

        match CaCmd::try_from(cmd) {
            Ok(cmd) => Ok(CaHeader {
                cmd,
                payload_size,
                data_type,
                data_count,
                param1,
                param2,
            }),
            Err(_) => Err(String::from("Failed to decode header")),
        }
    }

    pub fn is_extended(self: &Self) -> bool {
        self.payload_size > 0x3ff0
    }
}

pub fn build_name_search_buf(name: &str, cid: u32) -> Option<Vec<u8>> {
    // build padded payload
    let mut payload: Vec<u8> = vec![];
    let name_bytes = name.as_bytes();
    payload.extend_from_slice(name_bytes);
    payload.push(0);
    pad_payload(&mut payload)?; // if payload too large, return None

    // build CA header
    let mut buf = CaHeader {
        cmd: CaCmd::CaProtoSearch,
        payload_size: payload.len() as u32,
        data_type: SearchReplyFlag::DontReply as u16,
        data_count: CA_MINOR_VERSION as u32,
        param1: cid,
        param2: cid,
    }
    .encode();

    // join header and payload
    buf.extend_from_slice(&payload);

    Some(buf)
}

pub fn build_version_buf() -> Vec<u8> {
    CaHeader {
        cmd: CaCmd::CaProtoVersion,
        payload_size: 0,
        data_type: 1,
        data_count: CA_MINOR_VERSION as u32,
        param1: 1,
        param2: 0,
    }
    .encode()
}

pub fn decode_udp_ca(buf: &[u8; MAX_UDP_SEND]) {
    let context = get_context();
    let udp = context.udp();
    {
        let mut buf_mut = udp.buf_mut();
        buf_mut.extend_from_slice(buf);
    }
    let mut offset: u32 = 0;
    let buf = udp.buf();

    loop {
        match CaHeader::decode(&buf) {
            Ok(ca_header) => {
                let payload_size = ca_header.payload_size;
                // todo: parse the payload, may invole the asyn

                // trim buf
                if ca_header.is_extended() {
                    offset = payload_size + 24 + payload_size;
                } else {
                    offset = payload_size + 16 + payload_size;
                }
                if offset >= buf.len().try_into().unwrap() {
                    return;
                } else {
                    // consume the data
                    let mut buf_mut = udp.buf_mut();
                    buf_mut.drain(..(offset as usize));
                }
            }
            Err(_) => {
                // disgard remaining data in buf
                break;
            }
        }
    }
}
