use crate::ca::ca_cmd::CaCmd;
use crate::context::context::get_context;
use crate::udp::udp::UDP;
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

#[derive(Copy, Clone)]
pub enum CaSrc {
    Client,
    Server,
}

pub struct CaHeader {
    pub cmd: CaCmd,
    pub payload_size: u32, // or 2 bytes
    pub data_type: u16,
    pub data_count: u32, // or 2 bytes
    pub param1: u32,
    pub param2: u32,
    pub src: CaSrc,
}

impl std::fmt::Display for CaSrc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Client => "client",
            Self::Server => "server",
        })
    }
}

impl std::fmt::Display for CaHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [payload_size, data_type, data_count, param1, param2] = self.field_names();

        writeln!(f, "\nMessage header:")?;
        writeln!(f, "  {:<14}: {}", "src", self.src)?;
        writeln!(f, "  {:<14}: {}", "cmd", self.cmd)?;
        writeln!(f, "  {:<14}: {}", payload_size, self.payload_size)?;
        writeln!(f, "  {:<14}: {}", data_type, self.data_type)?;
        writeln!(f, "  {:<14}: {}", data_count, self.data_count)?;
        writeln!(f, "  {:<14}: {}", param1, self.param1)?;
        writeln!(f, "  {:<14}: {}", param2, self.param2)?;
        write!(f, "")
    }
}

impl CaHeader {
    fn field_names(&self) -> [&'static str; 5] {
        use CaCmd::*;
        use CaSrc::*;

        match (self.cmd, self.src) {
            (CaProtoVersion, Client) => [
                "Payload Size",
                "Priority",
                "Version",
                "Reserved",
                "Reserved",
            ],
            (CaProtoVersion, Server) => ["Reserved", "Priority", "Version", "Reserved", "Reserved"],
            (CaProtoSearch, Client) => ["Payload Size", "Reply", "Version", "SearchID", "SearchID"],
            (CaProtoSearch, Server) => [
                "Payload Size",
                "TCP Port",
                "Data Count",
                "SID or IP",
                "SearchID",
            ],
            (CaProtoNotFound, _) => ["Reserved", "Reply Flag", "Version", "SearchID", "SearchID"],
            (CaProtoEcho, _) => ["Reserved", "Reserved", "Reserved", "Reserved", "Reserved"],
            (CaProtoRsrvIsUp, _) => ["Reserved", "Version", "Server Port", "BeaconID", "Address"],
            (CaRepeaterConfirm, _) => [
                "Reserved",
                "Reserved",
                "Reserved",
                "Reserved",
                "Repeater Address",
            ],
            (CaRepeaterRegister, _) => [
                "Reserved",
                "Reserved",
                "Reserved",
                "Reserved",
                "Client IP Address",
            ],
            (CaProtoEventAdd, Client) => [
                "Payload Size",
                "Data Type",
                "Data Count",
                "SID",
                "SubscriptionID",
            ],
            (CaProtoEventAdd, Server) => [
                "Payload Size",
                "Data Type",
                "Data Count",
                "SID",
                "SubscriptionID",
            ],
            (CaProtoEventCancel, Client) => [
                "Payload Size",
                "Data Type",
                "Data Count",
                "SID",
                "SubscriptionID",
            ],
            (CaProtoEventCancel, Server) => [
                "Payload Size",
                "Data Type",
                "Data Count",
                "SID",
                "SubscriptionID",
            ],
            (CaProtoRead, Client) | (CaProtoReadNotify, Client) => {
                ["Payload Size", "Data Type", "Data Count", "SID", "IOID"]
            }
            (CaProtoRead, Server) | (CaProtoReadNotify, Server) => {
                ["Payload Size", "Data Type", "Data Count", "SID", "IOID"]
            }
            (CaProtoWrite, Client) | (CaProtoWriteNotify, Client) => {
                ["Payload Size", "Data Type", "Data Count", "SID", "IOID"]
            }
            (CaProtoWriteNotify, Server) => {
                ["Payload Size", "Data Type", "Data Count", "Status", "IOID"]
            }
            (CaProtoClearChannel, Client) => ["Payload Size", "Reserved", "Reserved", "SID", "CID"],
            (CaProtoClearChannel, Server) => ["Payload Size", "Reserved", "Reserved", "SID", "CID"],
            (CaProtoCreateChan, Client) => [
                "Payload Size",
                "Reserved",
                "Reserved",
                "CID",
                "Client Version",
            ],
            (CaProtoCreateChan, Server) => {
                ["Payload Size", "Data Type", "Data Count", "CID", "SID"]
            }
            (CaProtoClientName, _) => [
                "Payload Size",
                "Reserved",
                "Reserved",
                "Reserved",
                "Reserved",
            ],
            (CaProtoHostName, _) => [
                "Payload Size",
                "Reserved",
                "Reserved",
                "Reserved",
                "Reserved",
            ],
            (CaProtoAccessRights, _) => [
                "Payload Size",
                "Reserved",
                "Reserved",
                "CID",
                "Access Rights",
            ],
            (CaProtoCreateChFail, _) => ["Payload Size", "Reserved", "Reserved", "CID", "Reserved"],
            (CaProtoServerDisconn, _) => {
                ["Payload Size", "Reserved", "Reserved", "CID", "Reserved"]
            }
            _ => [
                "Payload Size",
                "Data Type",
                "Data Count",
                "Parameter 1",
                "Parameter 2",
            ],
        }
    }

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
    pub fn decode(buf: &Vec<u8>) -> Result<CaHeader, String> {
        let mut header_size: u32 = 16;

        if buf.len() < header_size as usize {
            return Err(String::from("Warning: Remaining buffer too short"));
        }

        let extended_header = buf[2] == 0xff && buf[3] == 0xff;

        if extended_header {
            header_size = 24;
            if buf.len() < header_size as usize {
                return Err(String::from(
                    "Warning: Remaining buffer too short for an extended header",
                ));
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

        if header_size + payload_size > buf.len() as u32 {
            Err(String::from(
                "Warning: Remaining buffer too short for header and payload",
            ))
        } else {
            match CaCmd::try_from(cmd) {
                Ok(cmd) => Ok(CaHeader {
                    cmd,
                    payload_size,
                    data_type,
                    data_count,
                    param1,
                    param2,
                    src: CaSrc::Server,
                }),
                Err(_) => Err(String::from("Error: Failed to decode header")),
            }
        }
    }

    fn is_extended(self: &Self) -> bool {
        self.payload_size > 0x3ff0
    }

    pub fn size(self: &Self) -> u32 {
        if self.is_extended() { 24 } else { 16 }
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
    let header = CaHeader {
        cmd: CaCmd::CaProtoSearch,
        payload_size: payload.len() as u32,
        data_type: SearchReplyFlag::DontReply as u16,
        data_count: CA_MINOR_VERSION as u32,
        param1: cid,
        param2: cid,
        src: CaSrc::Client,
    };
    debug!("{header}");

    let mut buf = header.encode();
    // join header and payload
    buf.extend_from_slice(&payload);

    Some(buf)
}

pub fn build_version_buf() -> Vec<u8> {
    let header = CaHeader {
        cmd: CaCmd::CaProtoVersion,
        payload_size: 0,
        data_type: 1,
        data_count: CA_MINOR_VERSION as u32,
        param1: 1,
        param2: 0,
        src: CaSrc::Client,
    };
    debug!("{header}");
    header.encode()
}

/**
 * Decode a Channel Access message
 */
pub fn decode_ca(buf: &mut Vec<u8>) {
    loop {
        // determine the Channel Access message size
        let msg_data = {
            match CaHeader::decode(buf) {
                Ok(ca_header) => {
                    // everything is OK
                    let header_size = ca_header.size();
                    let payload_size = ca_header.payload_size;
                    let msg_len = header_size + payload_size;
                    let payload = buf[header_size as usize..msg_len as usize].to_vec();
                    Ok((ca_header, payload))
                }
                Err(reason) => {
                    if reason.starts_with("Error") {
                        // something wrong with the buffer data, clear the buffer
                        Err(String::from("Error"))
                    } else {
                        // the remaining buffer is not long enough, stop decoding
                        return;
                    }
                }
            }
        };

        if let Ok((ca_header, payload)) = msg_data {
            // consume the message buffer
            {
                buf.drain(..(ca_header.size() as usize + payload.len()));
            }
            // todo parse the message using ca_header and payload
            debug!("{}", ca_header);
        } else {
            // something is wrong, clear the whole buffer
            buf.clear();
            return;
        }
    }
}
