use crate::ca::ca_cmd::CaCmd;
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


#[derive(Debug)]
pub struct CaHeader {
    pub cmd: CaCmd,
    pub payload_size: u32, // or 2 bytes
    pub data_type: u16,
    pub data_count: u32, // or 2 bytes
    pub param1: u32,
    pub param2: u32,
}

impl fmt::Display for CaHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [payload_size, data_type, data_count, param1, param2] = self.field_names();

        writeln!(f, "Header:")?;
        writeln!(f, "  {:<14}: {}", "cmd", self.cmd)?;
        writeln!(f, "  {:<14}: {}", payload_size, self.payload_size)?;
        writeln!(f, "  {:<14}: {}", data_type, self.data_type)?;
        writeln!(f, "  {:<14}: {}", data_count, self.data_count)?;
        writeln!(f, "  {:<14}: {}", param1, self.param1)?;
        write!(f, "  {:<14}: {}", param2, self.param2)
    }
}

impl CaHeader {
    fn field_names(&self) -> [&'static str; 5] {
        use CaCmd::*;

        match self.cmd {
            CaProtoVersion => ["Reserved", "Priority", "Version", "Reserved", "Reserved"],
            CaProtoSearch => [
                "Payload Size",
                "TCP Port",
                "Data Count",
                "SID or IP",
                "SearchID",
            ],
            CaProtoNotFound => ["Reserved", "Reply Flag", "Version", "SearchID", "SearchID"],
            CaProtoEcho => ["Reserved", "Reserved", "Reserved", "Reserved", "Reserved"],
            CaProtoRsrvIsUp => ["Reserved", "Version", "Server Port", "BeaconID", "Address"],
            CaRepeaterConfirm => [
                "Reserved",
                "Reserved",
                "Reserved",
                "Reserved",
                "Repeater Address",
            ],
            CaRepeaterRegister => [
                "Reserved",
                "Reserved",
                "Reserved",
                "Reserved",
                "Client IP Address",
            ],
            CaProtoEventAdd => [
                "Payload Size",
                "Data Type",
                "Data Count",
                "SID",
                "SubscriptionID",
            ],
            CaProtoEventCancel => [
                "Payload Size",
                "Data Type",
                "Data Count",
                "SID",
                "SubscriptionID",
            ],
            CaProtoRead | CaProtoReadNotify => {
                ["Payload Size", "Data Type", "Data Count", "SID", "IOID"]
            }
            CaProtoWriteNotify => ["Payload Size", "Data Type", "Data Count", "Status", "IOID"],
            CaProtoClearChannel => ["Payload Size", "Reserved", "Reserved", "SID", "CID"],
            CaProtoCreateChan => ["Payload Size", "Data Type", "Data Count", "CID", "SID"],
            CaProtoClientName => [
                "Payload Size",
                "Reserved",
                "Reserved",
                "Reserved",
                "Reserved",
            ],
            CaProtoHostName => [
                "Payload Size",
                "Reserved",
                "Reserved",
                "Reserved",
                "Reserved",
            ],
            CaProtoAccessRights => [
                "Payload Size",
                "Reserved",
                "Reserved",
                "CID",
                "Access Rights",
            ],
            CaProtoCreateChFail => ["Payload Size", "Reserved", "Reserved", "CID", "Reserved"],
            CaProtoServerDisconn => ["Payload Size", "Reserved", "Reserved", "CID", "Reserved"],
            _ => [
                "Payload Size",
                "Data Type",
                "Data Count",
                "Parameter 1",
                "Parameter 2",
            ],
        }
    }

    pub fn to_buf(self: &Self) -> Vec<u8> {
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
    pub fn from_buf(buf: &Vec<u8>) -> Result<CaHeader, String> {
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
