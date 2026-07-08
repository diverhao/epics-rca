use crate::ca::cmd::CaCmd;
use crate::ca::header::CaHeader;
use crate::channel::dbr::{ChannelAccessRights, ChannelSeverity, ChannelState, ChannelStatus};
use crate::channel::dbr::{DbrType, DbrValue};
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
const MAX_CA_PAYLOAD_SIZE: usize = (u32::MAX as usize) & !7;

pub fn pad_payload(payload: &mut Vec<u8>) -> u32 {
    let padding = (8 - payload.len() % 8) % 8;
    let padded_len = match payload.len().checked_add(padding) {
        Some(padded_len) if padded_len <= MAX_CA_PAYLOAD_SIZE => padded_len,
        _ => {
            error!("Payload size too large, truncate to {MAX_CA_PAYLOAD_SIZE} bytes");
            payload.truncate(MAX_CA_PAYLOAD_SIZE);
            MAX_CA_PAYLOAD_SIZE
        }
    };

    payload.resize(padded_len, 0);
    padded_len as u32
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
        writeln!(f, "{}", self.header)?;

        let payload_len = self.payload.len();
        let show_len = payload_len.min(80);
        writeln!(f, "Payload: {payload_len} bytes, showing {show_len}")?;

        for (line, chunk) in self.payload[..show_len].chunks(16).enumerate() {
            write!(f, "  {:04x}  ", line * 16)?;

            for i in 0..16 {
                if let Some(byte) = chunk.get(i) {
                    write!(f, "{byte:02x} ")?;
                } else {
                    write!(f, "   ")?;
                }

                if i == 7 {
                    write!(f, " ")?;
                }
            }

            write!(f, " ")?;
            for byte in chunk {
                let byte = *byte;
                let ch = if byte.is_ascii_graphic() || byte == b' ' {
                    byte as char
                } else {
                    '.'
                };
                write!(f, "{ch}")?;
            }
            writeln!(f)?;
        }

        Ok(())
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
        is_tcp: bool,
    ) -> Vec<CaMsg> {
        let mut msgs: Vec<CaMsg> = vec![];
        loop {
            // determine the Channel Access message size
            let msg = {
                match CaHeader::from_buf(buf) {
                    Ok(ca_header) => {
                        // everything is OK
                        let header_size = ca_header.size() as usize;
                        let payload_size = ca_header.payload_size as usize;
                        let msg_len = match header_size.checked_add(payload_size) {
                            Some(msg_len) => msg_len,
                            None => {
                                buf.clear();
                                break;
                            }
                        };
                        let payload = buf[header_size..msg_len].to_vec();

                        buf.drain(..msg_len);
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
                            if !is_tcp {
                                buf.clear();
                            }
                            return msgs;
                        }
                    }
                }
            };
        }
        msgs
    }

    // ------------------- builders --------------------

    pub fn build_name_search(name: &str, cid: u32, dest: &Vec<SocketAddr>) -> Self {
        // build padded payload
        let mut payload: Vec<u8> = vec![];
        let name_bytes = name.as_bytes();
        payload.extend_from_slice(name_bytes);
        payload.push(0);
        let payload_size = pad_payload(&mut payload);

        // build CA header
        let header = CaHeader {
            cmd: CaCmd::CaProtoSearch,
            payload_size,
            data_type: SearchReplyFlag::DontReply as u16,
            data_count: CA_MINOR_VERSION as u32,
            param1: cid,
            param2: cid,
        };
        CaMsg {
            header,
            payload,
            src: None,
            dest: dest.clone(),
        }
    }

    pub fn build_echo(dest: &Vec<SocketAddr>) -> CaMsg {
        let header = CaHeader {
            cmd: CaCmd::CaProtoEcho,
            payload_size: 0,
            data_type: 0,
            data_count: 0,
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

    pub fn build_client_name(dest: &Vec<SocketAddr>) -> Self {
        let mut payload = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_default()
            .into_bytes();
        payload.push(0);
        let payload_size = pad_payload(&mut payload);

        let header = CaHeader {
            cmd: CaCmd::CaProtoClientName,
            payload_size,
            data_type: 0,
            data_count: 0,
            param1: 0,
            param2: 0,
        };
        CaMsg {
            header,
            payload,
            src: None,
            dest: dest.clone(),
        }
    }

    pub fn build_host_name(dest: &Vec<SocketAddr>) -> CaMsg {
        let mut payload = current_hostname_bytes();
        payload.push(0);
        let payload_size = pad_payload(&mut payload);

        let header = CaHeader {
            cmd: CaCmd::CaProtoHostName,
            payload_size,
            data_type: 0,
            data_count: 0,
            param1: 0,
            param2: 0,
        };
        CaMsg {
            header,
            payload,
            src: None,
            dest: dest.clone(),
        }
    }

    pub fn build_create_chan(cname: &str, cid: u32, dest: &Vec<SocketAddr>) -> CaMsg {
        let mut payload = cname.to_string().into_bytes();
        payload.push(0);
        let payload_size = pad_payload(&mut payload);

        let header = CaHeader {
            cmd: CaCmd::CaProtoCreateChan,
            payload_size,
            data_type: 0,
            data_count: 0,
            param1: cid,
            param2: CA_MINOR_VERSION,
        };
        CaMsg {
            header,
            payload,
            src: None,
            dest: dest.clone(),
        }
    }

    pub fn build_read_notify(
        dbr_type: DbrType,
        data_count: u32,
        sid: u32,
        ioid: u32,
        dest: &Vec<SocketAddr>,
    ) -> CaMsg {
        let dbr_type = dbr_type as u16;
        let header = CaHeader {
            cmd: CaCmd::CaProtoReadNotify,
            payload_size: 0,
            data_type: dbr_type,
            data_count: data_count,
            param1: sid,
            param2: ioid,
        };
        CaMsg {
            header: header,
            payload: vec![],
            src: None,
            dest: dest.clone(),
        }
    }

    pub fn build_event_add(
        dbr_type: DbrType,
        data_count: u32,
        sid: u32,
        subid: u32,
        dest: &Vec<SocketAddr>,
    ) -> CaMsg {
        let dbr_type = dbr_type as u16;
        let header = CaHeader {
            cmd: CaCmd::CaProtoEventAdd,
            payload_size: 16,
            data_type: dbr_type,
            data_count: data_count,
            param1: sid,
            param2: subid,
        };
        CaMsg {
            header: header,
            // only report value change
            payload: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0],
            src: None,
            dest: dest.clone(),
        }
    }

    pub fn build_event_cancel(
        dbr_type: DbrType,
        data_count: u32,
        sid: u32,
        subid: u32,
        dest: &Vec<SocketAddr>,
    ) -> CaMsg {
        let dbr_type = dbr_type as u16;
        let header = CaHeader {
            cmd: CaCmd::CaProtoEventCancel,
            payload_size: 0,
            data_type: dbr_type,
            data_count: data_count,
            param1: sid,
            param2: subid,
        };
        CaMsg {
            header: header,
            payload: vec![],
            src: None,
            dest: dest.clone(),
        }
    }

    pub fn build_clear_channel(sid: u32, cid: u32, dest: &Vec<SocketAddr>) -> CaMsg {
        let header = CaHeader {
            cmd: CaCmd::CaProtoClearChannel,
            payload_size: 0,
            data_type: 0,
            data_count: 0,
            param1: sid,
            param2: cid,
        };
        CaMsg {
            header: header,
            payload: vec![],
            src: None,
            dest: dest.clone(),
        }
    }

    pub fn build_event_off(dest: &Vec<SocketAddr>) -> CaMsg {
        let header = CaHeader {
            cmd: CaCmd::CaProtoEventsOff,
            payload_size: 0,
            data_type: 0,
            data_count: 0,
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

    pub fn build_event_on(dest: &Vec<SocketAddr>) -> CaMsg {
        let header = CaHeader {
            cmd: CaCmd::CaProtoEventsOn,
            payload_size: 0,
            data_type: 0,
            data_count: 0,
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
