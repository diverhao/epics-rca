use crate::ca::ca_cmd::CaCmd;
use crate::ca::header::CaHeader;
use crate::channel::channel::ChannelState;
use crate::context::context::get_context;
use crate::udp::udp::UDP;
use ::log::debug;
use ::log::error;
use std::fmt;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};


pub const MAX_UDP_SEND: usize = 1024;

pub enum SearchReplyFlag {
    DoReply = 0x0a,
    DontReply = 0x05,
}

pub const CA_MINOR_VERSION: u32 = 13;

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

pub fn current_hostname_bytes() -> Vec<u8> {
    #[cfg(unix)]
    {
        unsafe extern "C" {
            fn gethostname(name: *mut std::os::raw::c_char, len: usize) -> i32;
        }

        let mut hostname = [0u8; 256];
        if unsafe { gethostname(hostname.as_mut_ptr().cast(), hostname.len()) } == 0 {
            let len = hostname
                .iter()
                .position(|&byte| byte == 0)
                .unwrap_or(hostname.len());
            return hostname[..len].to_vec();
        }
    }

    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_default()
        .into_bytes()
}

pub struct CaMsg {
    header: CaHeader,
    payload: Vec<u8>,
    src: Option<SocketAddr>,
    dest: Vec<SocketAddr>,
}

impl std::fmt::Display for CaMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.header)
    }
}

impl CaMsg {
    // ------------- from and to buffer ----------------

    pub fn to_buf(self: &Self) -> Vec<u8> {
        let mut buf: Vec<u8> = vec![];
        buf.extend_from_slice(&self.header().to_buf());
        buf.extend_from_slice(self.payload());
        buf
    }

    /**
     * Decode a Channel Access message buffer, the buffer is modified
     */
    pub fn from_buf(
        buf: &mut Vec<u8>,
        src: Option<SocketAddr>,
        dest: Vec<SocketAddr>,
    ) -> Vec<CaMsg> {
        let mut msgs: Vec<CaMsg> = vec![];
        loop {
            // determine the Channel Access message size
            let msg = {
                match CaHeader::from_buf(buf) {
                    Ok(ca_header) => {
                        // everything is OK
                        let header_size = ca_header.size();
                        let payload_size = ca_header.payload_size;
                        let msg_len = header_size + payload_size;
                        let payload = buf[header_size as usize..msg_len as usize].to_vec();

                        buf.drain(..msg_len as usize);
                        let dest = dest.clone();
                        let msg = CaMsg {
                            header: ca_header,
                            payload,
                            src,
                            dest,
                        };
                        msgs.push(msg);
                    }
                    Err(reason) => {
                        if reason.starts_with("Error") {
                            // something wrong with the buffer data, clear the buffer
                            buf.clear();
                            break;
                        } else {
                            // Warning
                            // the remaining buffer is not long enough, stop decoding
                            // do not clear the buffer
                            return msgs;
                        }
                    }
                }
            };
        }
        msgs
    }

    // ------------------- builders --------------------

    pub fn build_name_search(name: &str, cid: u32, dest: &Vec<SocketAddr>) -> Result<Self, String> {
        // build padded payload
        let mut payload: Vec<u8> = vec![];
        let name_bytes = name.as_bytes();
        payload.extend_from_slice(name_bytes);
        payload.push(0);
        let payload_size = pad_payload(&mut payload); // if payload too large, return None

        match payload_size {
            Some(payload_size) => {
                // build CA header
                let header = CaHeader {
                    cmd: CaCmd::CaProtoSearch,
                    payload_size: payload_size,
                    data_type: SearchReplyFlag::DontReply as u16,
                    data_count: CA_MINOR_VERSION as u32,
                    param1: cid,
                    param2: cid,
                };
                Ok(CaMsg {
                    header: header,
                    payload: payload,
                    src: None,
                    dest: dest.clone(),
                })
            }
            None => Err("Payload size too large".to_string()),
        }
    }

    pub fn build_version(dest: &Vec<SocketAddr>) -> CaMsg {
        let header = CaHeader {
            cmd: CaCmd::CaProtoVersion,
            payload_size: 0,
            data_type: 0,
            data_count: CA_MINOR_VERSION as u32,
            param1: 0,
            param2: 0,
        };
        CaMsg {
            header: header,
            payload: vec![],
            src: None,
            dest: dest.clone(),
        }
    }

    pub fn build_client_name(dest: &Vec<SocketAddr>) -> Result<Self, String> {
        let mut payload = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_default()
            .into_bytes();
        payload.push(0);
        let payload_size = pad_payload(&mut payload);

        match payload_size {
            Some(payload_size) => {
                let header = CaHeader {
                    cmd: CaCmd::CaProtoClientName,
                    payload_size,
                    data_type: 0,
                    data_count: 0,
                    param1: 0,
                    param2: 0,
                };
                Ok(CaMsg {
                    header,
                    payload,
                    src: None,
                    dest: dest.clone(),
                })
            }
            None => Err("".to_string()),
        }
    }

    pub fn build_host_name(dest: &Vec<SocketAddr>) -> Result<CaMsg, String> {
        let mut payload = current_hostname_bytes();
        payload.push(0);
        let payload_size = pad_payload(&mut payload);

        match payload_size {
            Some(payload_size) => {
                let header = CaHeader {
                    cmd: CaCmd::CaProtoHostName,
                    payload_size,
                    data_type: 0,
                    data_count: 0,
                    param1: 0,
                    param2: 0,
                };
                Ok(CaMsg {
                    header,
                    payload,
                    src: None,
                    dest: dest.clone(),
                })
            }
            None => Err("".to_string()),
        }
    }

    pub fn build_create_chan(
        cname: &str,
        cid: u32,
        dest: &Vec<SocketAddr>,
    ) -> Result<CaMsg, String> {
        let mut payload = cname.to_string().into_bytes();
        payload.push(0);
        let payload_size = pad_payload(&mut payload);

        match payload_size {
            Some(payload_size) => {
                let header = CaHeader {
                    cmd: CaCmd::CaProtoCreateChan,
                    payload_size,
                    data_type: 0,
                    data_count: 0,
                    param1: cid,
                    param2: CA_MINOR_VERSION,
                };
                Ok(CaMsg {
                    header,
                    payload,
                    src: None,
                    dest: dest.clone(),
                })
            }
            None => Err("".to_string()),
        }
    }

    // -------------- getters --------------------
    pub fn header(self: &Self) -> &CaHeader {
        &self.header
    }

    pub fn src(self: &Self) -> &Option<SocketAddr> {
        &self.src
    }

    pub fn dest(self: &Self) -> &Vec<SocketAddr> {
        &self.dest
    }

    pub fn payload(self: &Self) -> &Vec<u8> {
        &self.payload
    }

    pub fn size(self: &Self) -> u32 {
        self.header().size() + self.payload().len() as u32
    }
}
