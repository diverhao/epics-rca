use core::num;
use std::collections::HashMap;
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use crate::ca_message::message::{MAX_UDP_SEND, current_hostname_bytes};
use crate::pva_message::bit_set::BitSet;
use crate::pva_message::complex::{PvaFieldType, PvaStructType, PvaStructValue};
use crate::pva_message::pv_request::{parse_put_get_pv_request, parse_pv_request};
use crate::pva_message::typ::PvaType;
use crate::pva_message::type_registry::PvaTypeRegistry;
use crate::pva_message::value::PvaValue;
use crate::pva_message::{
    cmd::{PvaCmd, PvaCtrlCmd},
    header::{MsgEndian, MsgOrigin, MsgSeg, PVA_HEADER_SIZE, PvaCtrlHeader, PvaHeader},
    primitive::PvaElement,
};

pub enum PvaStatus {
    Ok,
    Warning { msg: String, call_tree: String },
    Error { msg: String, call_tree: String },
    Fatal { msg: String, call_tree: String },
}

impl PvaStatus {
    pub fn is_success(self: &Self) -> bool {
        match self {
            PvaStatus::Ok => true,
            PvaStatus::Warning { msg, call_tree } => true,
            _ => false,
        }
    }
}

pub enum PvaAuthnz {
    Ca,
    Anonymous,
}

const MAX_PVA_PAYLOAD_SIZE: usize = u32::MAX as usize;
const EPICS_PVA_MAX_ARRAY_BYTES: usize = 0x10000;
const MAX_TCP_RECV: usize = 0x10000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PvaMsg {
    header: PvaHeader,
    src: Option<SocketAddr>,
    dest: Vec<SocketAddr>,
    payload: Vec<u8>,
}

impl fmt::Display for PvaMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let flags = self.header.flags();

        writeln!(f, "\nHeader:")?;
        writeln!(f, "  {:<14}: 0x{:02x}", "Magic", self.header.magic())?;
        writeln!(f, "  {:<14}: {}", "Version", self.header.version())?;
        writeln!(f, "  {:<14}: {:?}", "Message Type", flags.msg_type)?;
        writeln!(f, "  {:<14}: {:?}", "Segmentation", flags.seg_type)?;
        writeln!(f, "  {:<14}: {:?}", "Origin", flags.origin)?;
        writeln!(f, "  {:<14}: {:?}", "Endian", flags.endian)?;
        writeln!(f, "  {:<14}: {}", "Command", self.header.cmd())?;
        writeln!(
            f,
            "  {:<14}: {}",
            "Payload Size",
            self.header.payload_size()
        )?;

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
                let ch = if byte.is_ascii_graphic() || *byte == b' ' {
                    *byte as char
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

impl PvaMsg {
    pub fn new(
        seg_type: MsgSeg,
        origin: MsgOrigin,
        src: Option<SocketAddr>,
        dest: Vec<SocketAddr>,
        endian: MsgEndian,
        cmd: PvaCmd,
        payload: Vec<u8>,
    ) -> Result<Self, String> {
        let payload_size = u32::try_from(payload.len())
            .map_err(|_| String::from("Error: PVA payload is larger than u32::MAX"))?;

        Ok(Self {
            header: PvaHeader::new(seg_type, origin, endian, cmd, payload_size)?,
            src,
            dest,
            payload,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        self.header.validate()?;
        let expected = usize::try_from(self.header.payload_size())
            .map_err(|_| String::from("Error: PVA payload size does not fit in usize"))?;
        if self.payload.len() != expected {
            return Err(format!(
                "Error: PVA payload length {} does not match header size {expected}",
                self.payload.len()
            ));
        }

        Ok(())
    }

    pub fn to_buf(&self) -> Result<Vec<u8>, String> {
        self.validate()?;

        let mut buf = Vec::with_capacity(PVA_HEADER_SIZE + self.payload.len());
        buf.extend_from_slice(&self.header.to_buf()?);
        buf.extend_from_slice(&self.payload);
        Ok(buf)
    }

    /**
     * Decode all complete PVA application messages in `buf`.
     *
     * Complete messages are removed from the front of the buffer. An
     * incomplete trailing message is retained for TCP and discarded for UDP.
     */
    pub fn from_buf(
        buf: &mut Vec<u8>,
        src: Option<SocketAddr>,
        dest: Vec<SocketAddr>,
        is_tcp: bool,
    ) -> Vec<Self> {
        let mut messages = Vec::new();

        loop {
            if buf.is_empty() {
                break;
            }

            let header = match PvaHeader::from_buf(buf) {
                Ok(header) => header,
                Err(reason) => {
                    if reason.starts_with("Error") || !is_tcp {
                        buf.clear();
                    }
                    break;
                }
            };

            let payload_size = match usize::try_from(header.payload_size()) {
                Ok(payload_size) => payload_size,
                Err(_) => {
                    buf.clear();
                    break;
                }
            };
            let message_size = match PVA_HEADER_SIZE.checked_add(payload_size) {
                Some(message_size) => message_size,
                None => {
                    buf.clear();
                    break;
                }
            };

            if buf.len() < message_size {
                if !is_tcp {
                    buf.clear();
                }
                break;
            }

            let message = Self {
                header,
                src,
                dest: dest.clone(),
                payload: buf[PVA_HEADER_SIZE..message_size].to_vec(),
            };
            buf.drain(..message_size);
            messages.push(message);
        }

        messages
    }

    // ---------------- getter -----------------------

    pub fn header(&self) -> &PvaHeader {
        &self.header
    }

    pub fn src(&self) -> &Option<SocketAddr> {
        &self.src
    }

    pub fn dest(&self) -> &Vec<SocketAddr> {
        &self.dest
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PvaCtrlMsg {
    header: PvaCtrlHeader,
}

impl PvaCtrlMsg {
    pub fn new(
        origin: MsgOrigin,
        endian: MsgEndian,
        cmd: PvaCtrlCmd,
        data: u32,
    ) -> Result<Self, String> {
        Ok(Self {
            header: PvaCtrlHeader::new(origin, endian, cmd, data)?,
        })
    }

    pub fn to_buf(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        self.header.to_buf()
    }

    pub fn from_buf(buf: &[u8], offset: &mut usize) -> Result<Self, String> {
        let header_end = offset
            .checked_add(PVA_HEADER_SIZE)
            .ok_or_else(|| String::from("Error: PVA control message header offset overflow"))?;
        if header_end > buf.len() {
            return Err(String::from(
                "Warning: Remaining buffer too short for PVA control message header",
            ));
        }

        let message = Self {
            header: PvaCtrlHeader::from_buf(&buf[*offset..header_end])?,
        };
        message.validate()?;
        *offset = header_end;
        Ok(message)
    }

    pub fn header(&self) -> &PvaCtrlHeader {
        &self.header
    }

    pub fn validate(&self) -> Result<(), String> {
        self.header.validate()
    }
}

pub fn build_connection_validation(
    authnz: PvaAuthnz,
    endian: MsgEndian,
    pva_type_registry: &mut PvaTypeRegistry,
) -> Result<Vec<u8>, String> {
    // struct connectionValidationResponse {
    //     int clientReceiveBufferSize;
    //     short clientIntrospectionRegistryMaxSize;
    //     short connectionQos;
    //     string authNZ;
    //     FieldDesc dataIF;
    //     PVField data;
    // };

    // payload buf
    let mut buf: Vec<u8> = vec![];

    // maximum TCP receive buffer size
    let tcp_buf_size: u32 = MAX_TCP_RECV as u32;
    tcp_buf_size.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // introspection registry size
    let type_registry_size: i16 = 0x7fff;
    type_registry_size.to_buf(&PvaType::Short, &mut buf, endian)?;

    // quality of service priority, 0
    let qos: i16 = 0;
    qos.to_buf(&PvaType::Short, &mut buf, endian)?;

    // authentication, "ca" or "anonymous"
    match authnz {
        PvaAuthnz::Anonymous => {
            "anonymous"
                .to_string()
                .to_buf(&PvaType::String, &mut buf, endian)?;
            PvaType::Null.to_buf(&mut buf, endian, pva_type_registry)?;
        }
        PvaAuthnz::Ca => {
            "ca".to_string()
                .to_buf(&PvaType::String, &mut buf, endian)?;
            // append type
            let typ = Arc::new(PvaType::Struct(Arc::new(PvaStructType {
                id: "structure".to_string(),
                fields: vec![
                    Arc::new(PvaFieldType {
                        name: "user".to_string(),
                        typ: Arc::new(PvaType::String),
                    }),
                    Arc::new(PvaFieldType {
                        name: "host".to_string(),
                        typ: Arc::new(PvaType::String),
                    }),
                ],
            })));

            let username = std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "nobody".to_string());
            let hostname_bytes = current_hostname_bytes();
            let hostname = if hostname_bytes.is_empty() {
                "invalidhost.".to_string()
            } else {
                String::from_utf8_lossy(&hostname_bytes).into_owned()
            };
            let value = PvaValue::Struct(PvaStructValue {
                fields: vec![PvaValue::String(username), PvaValue::String(hostname)],
            });

            typ.to_buf(&mut buf, endian, pva_type_registry)?;
            value.to_buf(typ, &mut buf, endian, pva_type_registry)?;
        }
    }

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::ConnectionValidation,
        buf,
    )?
    .to_buf()
}

pub fn build_echo(endian: MsgEndian) -> Result<Vec<u8>, String> {
    // no size
    // struct echoRequest {
    //     byte[] somePayload;
    // };
    let header = PvaHeader::new(MsgSeg::NotSeg, MsgOrigin::Client, endian, PvaCmd::Echo, 0)?;
    return header.to_buf();
}

pub fn build_search(
    search_seq_id: u32,
    endian: MsgEndian,
    response_addr: SocketAddr,
    channel_names_cids: &Vec<(String, u32)>,
) -> Result<Vec<u8>, String> {
    // reject 0-size channel search
    if channel_names_cids.len() == 0 {
        return Err("Cannot build 0 size PVA search buffer".to_string());
    }

    // this is a udp packet, no type registry
    let pva_type_registry: &mut PvaTypeRegistry = &mut PvaTypeRegistry {
        types: HashMap::new(),
    };

    // payload
    // struct searchRequest {
    //     int searchSequenceID;
    //     byte flags
    //     byte[3] reserved;
    //     byte[16] responseAddress;
    //     short responsePort;
    //     string[] protocols;
    //     struct {
    //         int searchInstanceID;
    //         string channelName;
    //     } channels[];
    // };

    // payload buffer
    let mut buf: Vec<u8> = vec![];

    // search sequence ID
    search_seq_id.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // flags, no reply if not found, broadcast only
    // bit 0: 1 means require reply if not found, 0 means no reply required
    // bit 7: 1 means unicast, 0 means broadcast or multicast
    let flags: u8 = 0b_0000_0000;
    flags.to_buf(&PvaType::UByte, &mut buf, endian)?;

    // fixed size array, byte[3]
    let pva_value: PvaValue = PvaValue::ByteFixArray(vec![0, 0, 0]);
    pva_value.to_buf(
        Arc::new(PvaType::ByteFixArray(3)),
        &mut buf,
        endian,
        pva_type_registry,
    )?;

    // ipv6 representation of address
    let addr_arr = match response_addr.ip() {
        IpAddr::V4(ipv4) => ipv4.to_ipv6_mapped().octets(),
        IpAddr::V6(ipv6) => ipv6.octets(),
    }
    .to_vec();
    let addr = PvaValue::UByteFixArray(addr_arr);
    addr.to_buf(
        Arc::new(PvaType::UByteFixArray(16)),
        &mut buf,
        endian,
        pva_type_registry,
    )?;

    // port
    let port = response_addr.port();
    PvaValue::UShort(port).to_buf(
        Arc::new(PvaType::UShort),
        &mut buf,
        endian,
        pva_type_registry,
    )?;

    // protocols, fixed ["tcp"]
    let protocols = vec!["tcp".to_string()];
    PvaValue::StringVarSizeArray(protocols).to_buf(
        Arc::new(PvaType::StringVarSizeArray),
        &mut buf,
        endian,
        pva_type_registry,
    )?;

    // channel names
    let struct_typ = Arc::new(PvaType::Struct(Arc::new(PvaStructType {
        id: "".to_string(),
        fields: vec![
            Arc::new(PvaFieldType {
                name: "searchInstanceID".to_string(),
                typ: Arc::new(PvaType::UInt),
            }),
            Arc::new(PvaFieldType {
                name: "channelName".to_string(),
                typ: Arc::new(PvaType::String),
            }),
        ],
    })));

    // number of names, u16 (short) type, not PVA size!
    let num_names = match u16::try_from(channel_names_cids.len()) {
        Ok(num_names) => num_names,
        Err(_) => return Err("number of names overflow".to_string()),
    };
    num_names.to_buf(&PvaType::UShort, &mut buf, endian)?;

    // name struct values, they are not PVA structure variable size array!
    for (name, cid) in channel_names_cids {
        let struct_value = PvaStructValue {
            fields: vec![PvaValue::UInt(*cid), PvaValue::String(name.clone())],
        };
        PvaValue::Struct(struct_value).to_buf(
            Arc::clone(&struct_typ),
            &mut buf,
            endian,
            pva_type_registry,
        )?;
    }

    // header, we need to know payload first
    let payload_size = match u32::try_from(buf.len()) {
        Ok(payload_size) => payload_size,
        Err(_) => return Err("payload size overflow".to_string()),
    };
    if payload_size as usize + PVA_HEADER_SIZE > MAX_UDP_SEND {
        return Err("Packet size overflow".to_string());
    }

    let header = PvaHeader::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        endian,
        PvaCmd::Search,
        payload_size,
    )?;
    let mut header_buf = header.to_buf()?;
    header_buf.extend_from_slice(&buf);
    return Ok(header_buf);
}

pub fn build_create_channel(
    name: String,
    cid: u32,
    endian: MsgEndian,
    pva_type_registry: &mut PvaTypeRegistry,
) -> Result<Vec<u8>, String> {
    // validate channel name
    if name.len() > 500 || name.len() == 0 {
        return Err("Channel name too long or 0 size".to_string());
    }
    // struct createChannelRequest {
    //     short count;
    //     struct {
    //         int clientChannelID;
    //         string channelName;
    //     } channels[];
    // };
    let mut buf: Vec<u8> = vec![];

    // count, fixed to be 1
    let count: i16 = 1;
    count.to_buf(&PvaType::Short, &mut buf, endian)?;

    // channel type
    let typ = Arc::new(PvaType::Struct(Arc::new(PvaStructType {
        id: "structure".to_string(),
        fields: vec![
            Arc::new(PvaFieldType {
                name: "clientChannelID".to_string(),
                typ: Arc::new(PvaType::UInt),
            }),
            Arc::new(PvaFieldType {
                name: "channelName".to_string(),
                typ: Arc::new(PvaType::String),
            }),
        ],
    })));
    // only one name allowed
    let struct_value = PvaValue::Struct(PvaStructValue {
        fields: vec![PvaValue::UInt(cid), PvaValue::String(name)],
    });
    struct_value.to_buf(Arc::clone(&typ), &mut buf, endian, pva_type_registry)?;

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::CreateChannel,
        buf,
    )?
    .to_buf()
}

pub fn build_destroy_channel(cid: u32, sid: u32, endian: MsgEndian) -> Result<Vec<u8>, String> {
    // struct destroyChannelRequest {
    //     int serverChannelID;
    //     int clientChannelID;
    // };
    let mut buf: Vec<u8> = vec![];

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // cid
    cid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // message with header
    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::DestroyChannel,
        buf,
    )?
    .to_buf()
}

pub fn build_get_init(
    sid: u32,
    ioid: u32,
    pv_request_str: String,
    endian: MsgEndian,
    pva_type_registry: &mut PvaTypeRegistry,
) -> Result<Vec<u8>, String> {
    // struct channelGetRequestInit {
    //     int serverChannelID;
    //     int requestID;
    //     byte subcommand = 0x08 for INIT;
    //     FieldDesc pvRequestIF;
    //     PVField pvRequest;
    // };
    let mut buf: Vec<u8> = vec![];

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x08
    (0x08 as u8).to_buf(&PvaType::UByte, &mut buf, endian)?;

    // pv request type
    let pv_request_node = parse_pv_request(&pv_request_str)?;
    let pv_request_type = PvaType::build_pv_request(&pv_request_node);
    pv_request_type.to_buf(&mut buf, endian, pva_type_registry)?;

    // pv request value
    let pv_request_value = PvaValue::build_pv_request(&pv_request_node);
    pv_request_value.to_buf(
        Arc::new(pv_request_type),
        &mut buf,
        endian,
        pva_type_registry,
    )?;

    // message with header
    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::Get,
        buf,
    )?
    .to_buf()
}

pub fn build_get(
    sid: u32,
    ioid: u32,
    endian: MsgEndian,
    destroy_get: bool,
) -> Result<Vec<u8>, String> {
    // struct channelGetRequest {
    //     int serverChannelID;
    //     int requestID;
    //     byte subcommand = 0x00 or 0x40 for GET; additional 0x10 mask for DESTROY;
    // };
    let mut buf: Vec<u8> = vec![];

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x00
    if destroy_get {
        ((0x40 | 0x10) as i8).to_buf(&PvaType::Byte, &mut buf, endian)?;
    } else {
        (0x00 as i8).to_buf(&PvaType::Byte, &mut buf, endian)?;
    }

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::Get,
        buf,
    )?
    .to_buf()
}

pub fn build_put_init(
    sid: u32,
    ioid: u32,
    pv_request_str: String,
    endian: MsgEndian,
    pva_type_registry: &mut PvaTypeRegistry,
) -> Result<Vec<u8>, String> {
    // struct channelPutRequestInit {
    //     int serverChannelID;
    //     int requestID;
    //     byte subcommand = 0x08;
    //     FieldDesc pvRequestIF;
    //     PVField pvRequest;
    // };

    let mut buf: Vec<u8> = vec![];

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x08
    (0x08 as u8).to_buf(&PvaType::UByte, &mut buf, endian)?;

    // pv request type
    let pv_request_node = parse_pv_request(&pv_request_str)?;
    let pv_request_type = PvaType::build_pv_request(&pv_request_node);
    pv_request_type.to_buf(&mut buf, endian, pva_type_registry)?;

    // pv request value
    let pv_request_value = PvaValue::build_pv_request(&pv_request_node);
    pv_request_value.to_buf(
        Arc::new(pv_request_type),
        &mut buf,
        endian,
        pva_type_registry,
    )?;

    // message with header
    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::Put,
        buf,
    )?
    .to_buf()
}

pub fn build_put(
    sid: u32,
    ioid: u32,
    typ: Arc<PvaType>,
    field_paths: Vec<String>,
    value: PvaValue,
    pva_type_registry: &mut PvaTypeRegistry,
    endian: MsgEndian,
    destroy_upon_finish: bool,
) -> Result<Vec<u8>, String> {
    // struct channelPutRequest {
    //     int serverChannelID;
    //     int requestID;
    //     byte subcommand = 0x00 for PUT; 0x10 mask for DESTROY;
    //     BitSet toPutBitSet;
    //     PVField pvPutStructureData;
    // };
    let mut buf: Vec<u8> = vec![];

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x00
    if destroy_upon_finish {
        (0x10 as i8).to_buf(&PvaType::Byte, &mut buf, endian)?;
    } else {
        (0x00 as i8).to_buf(&PvaType::Byte, &mut buf, endian)?;
    }
    // bitset
    let bit_set = BitSet::from_field_paths(Arc::clone(&typ), field_paths)?;
    bit_set.to_buf(&mut buf, endian)?;

    //  partial data
    value.to_buf_with_bitset(typ, &bit_set, &mut buf, endian, pva_type_registry)?;

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::Put,
        buf,
    )?
    .to_buf()
}

// GET the value from a PUT channle
pub fn build_get_from_put(
    sid: u32,
    ioid: u32,
    endian: MsgEndian,
    destroy_upon_finish: bool,
) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![];
    // struct channelGetPutRequest {
    //     int serverChannelID;
    //     int requestID;
    //     byte subcommand = 0x40;
    // };

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x40 and 0x10
    if destroy_upon_finish {
        (0x50 as u8).to_buf(&PvaType::UByte, &mut buf, endian)?;
    } else {
        (0x40 as u8).to_buf(&PvaType::UByte, &mut buf, endian)?;
    }

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::Put,
        buf,
    )?
    .to_buf()
}

// put then get
// practically the same as build_put()
pub fn build_put_get_init(
    sid: u32,
    ioid: u32,
    put_pv_request_str: String,
    get_pv_request_str: String,
    endian: MsgEndian,
    pva_type_registry: &mut PvaTypeRegistry,
) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![];

    // struct channelPutGetRequestInit {
    //     int serverChannelID;
    //     int requestID;
    //     byte subcommand = 0x08;
    //     FieldDesc pvRequestIF;
    //     PVField pvRequest;
    // };

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x08
    (0x08 as u8).to_buf(&PvaType::UByte, &mut buf, endian)?;

    // pv request type
    let pv_request_node = parse_put_get_pv_request(&put_pv_request_str, &get_pv_request_str)?;
    let pv_request_type = PvaType::build_pv_request(&pv_request_node);
    pv_request_type.to_buf(&mut buf, endian, pva_type_registry)?;

    // pv request value
    let pv_request_value = PvaValue::build_pv_request(&pv_request_node);
    pv_request_value.to_buf(
        Arc::new(pv_request_type),
        &mut buf,
        endian,
        pva_type_registry,
    )?;

    // message with header
    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::PutGet,
        buf,
    )?
    .to_buf()
}

pub fn build_put_get(
    sid: u32,
    ioid: u32,
    typ: Arc<PvaType>,
    field_paths: Vec<String>,
    value: PvaValue,
    pva_type_registry: &mut PvaTypeRegistry,
    endian: MsgEndian,
    destroy_upon_finish: bool,
) -> Result<Vec<u8>, String> {
    // struct channelPutGetRequest {
    //     int serverChannelID;
    //     int requestID;
    //     byte subcommand = 0x00 for PUT_GET; 0x10 mask for DESTROY;
    //     BitSet toPutBitSet;
    //     PVField pvPutStructureData;
    // };

    let mut buf: Vec<u8> = vec![];

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x00
    if destroy_upon_finish {
        (0x10 as i8).to_buf(&PvaType::Byte, &mut buf, endian)?;
    } else {
        (0x00 as i8).to_buf(&PvaType::Byte, &mut buf, endian)?;
    }
    // bitset
    let bit_set = BitSet::from_field_paths(Arc::clone(&typ), field_paths)?;
    bit_set.to_buf(&mut buf, endian)?;

    //  partial data
    value.to_buf_with_bitset(typ, &bit_set, &mut buf, endian, pva_type_registry)?;

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::PutGet,
        buf,
    )?
    .to_buf()
}

// GET the value of pvPutStructureIF type from an existing PUT_GET channel
pub fn build_get_put_from_put_get(
    sid: u32,
    ioid: u32,
    endian: MsgEndian,
    destroy_upon_finish: bool,
) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![];
    // struct channelGetPutRequest {
    //     int serverChannelID;
    //     int requestID;
    //     byte subcommand = 0x80;
    // };

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x80
    if destroy_upon_finish {
        (0x90 as u8).to_buf(&PvaType::UByte, &mut buf, endian)?;
    } else {
        (0x80 as u8).to_buf(&PvaType::UByte, &mut buf, endian)?;
    }

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::PutGet,
        buf,
    )?
    .to_buf()
}

// GET the value of pvGetStructureIF type from an existing PUT_GET channel
pub fn build_get_get_from_put_get(
    sid: u32,
    ioid: u32,
    endian: MsgEndian,
    destroy_upon_finish: bool,
) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![];
    // struct channelGetPutRequest {
    //     int serverChannelID;
    //     int requestID;
    //     byte subcommand = 0x40;
    // };

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x40
    if destroy_upon_finish {
        (0x50 as u8).to_buf(&PvaType::UByte, &mut buf, endian)?;
    } else {
        (0x40 as u8).to_buf(&PvaType::UByte, &mut buf, endian)?;
    }

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::PutGet,
        buf,
    )?
    .to_buf()
}

pub fn build_monitor_init(
    sid: u32,
    ioid: u32,
    endian: MsgEndian,
    pv_request_str: String,
    pva_type_registry: &mut PvaTypeRegistry,
) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![];

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x08
    (0x08_u8).to_buf(&PvaType::UByte, &mut buf, endian)?;

    // pv request type
    let pv_request_node = parse_pv_request(&pv_request_str)?;
    let pv_request_type = PvaType::build_pv_request(&pv_request_node);
    pv_request_type.to_buf(&mut buf, endian, pva_type_registry)?;

    // pv request value
    let pv_request_value = PvaValue::build_pv_request(&pv_request_node);
    pv_request_value.to_buf(
        Arc::new(pv_request_type),
        &mut buf,
        endian,
        pva_type_registry,
    )?;

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::Monitor,
        buf,
    )?
    .to_buf()
}

pub fn build_monitor_start(sid: u32, ioid: u32, endian: MsgEndian) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![];

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x44
    (0x44_u8).to_buf(&PvaType::UByte, &mut buf, endian)?;

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::Monitor,
        buf,
    )?
    .to_buf()
}

pub fn build_monitor_stop(
    sid: u32,
    ioid: u32,
    endian: MsgEndian,
    destroy: bool,
) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![];

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x04
    if destroy {
        (0x14_u8).to_buf(&PvaType::UByte, &mut buf, endian)?;
    } else {
        (0x04_u8).to_buf(&PvaType::UByte, &mut buf, endian)?;
    }

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::Monitor,
        buf,
    )?
    .to_buf()
}

pub fn build_destroy_request(sid: u32, ioid: u32, endian: MsgEndian) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![];
    // struct destroyRequest {
    //     int serverChannelID;
    //     int requestID;
    // };

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::DestroyRequest,
        buf,
    )?
    .to_buf()
}

// todo: CMD_ARRAY (0x0E)

pub fn build_process_init(
    sid: u32,
    ioid: u32,
    endian: MsgEndian,
    pv_request_str: String,
    pva_type_registry: &mut PvaTypeRegistry,
) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![];

    // struct channelProcessRequestInit {
    //     int serverChannelID;
    //     int requestID;
    //     byte subcommand = 0x08;
    //     FieldDesc pvRequestIF;
    //     [if serverStatusIF != NULL_TYPE_CODE] PVField pvRequest;
    // };

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x08
    (0x08_u8).to_buf(&PvaType::UByte, &mut buf, endian)?;

    // pv request type
    let pv_request_node = parse_pv_request(&pv_request_str)?;
    let pv_request_type = PvaType::build_pv_request(&pv_request_node);
    pv_request_type.to_buf(&mut buf, endian, pva_type_registry)?;

    // pv request value
    let pv_request_value = PvaValue::build_pv_request(&pv_request_node);
    pv_request_value.to_buf(
        Arc::new(pv_request_type),
        &mut buf,
        endian,
        pva_type_registry,
    )?;

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::Process,
        buf,
    )?
    .to_buf()
}

pub fn build_process(
    sid: u32,
    ioid: u32,
    endian: MsgEndian,
    destroy_upon_finish: bool,
) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![];

    // struct channelProcessRequest {
    //     int serverChannelID;
    //     int requestID;
    //     byte subcommand = 0x00 mask for PROCESS; 0x10 mask for DESTROY;
    // };

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x00
    if destroy_upon_finish {
        (0x10_u8).to_buf(&PvaType::UByte, &mut buf, endian)?;
    } else {
        (0x00_u8).to_buf(&PvaType::UByte, &mut buf, endian)?;
    }

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::Process,
        buf,
    )?
    .to_buf()
}

pub fn build_get_field(
    sid: u32,
    ioid: u32,
    endian: MsgEndian,
    sub_field_name: String,
) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![];
    // struct channelGetFieldRequest {
    //     int serverChannelID;
    //     int requestID;
    //     string subFieldName;  // entire record if empty
    // };

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // sub field name
    sub_field_name.to_buf(&PvaType::String, &mut buf, endian)?;

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::GetField,
        buf,
    )?
    .to_buf()
}

// only server to client: CMD_MESSAGE (0x12)

// depracated: CMD_MULTIPLE_DATA (0x13)

pub fn build_rpc_init(
    sid: u32,
    ioid: u32,
    endian: MsgEndian,
    pva_type_registry: &mut PvaTypeRegistry,
) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![];

    // struct channelRPCRequestInit {
    //     int serverChannelID;
    //     int requestID;
    //     byte subcommand = 0x08;
    //     FieldDesc pvRequestIF;
    //     PVField pvRequest;
    // };

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x08
    (0x08_i8).to_buf(&PvaType::Byte, &mut buf, endian)?;

    // pv request type
    let pv_request_str = String::from(""); // empty is enough
    let pv_request_node = parse_pv_request(&pv_request_str)?;
    let pv_request_type = PvaType::build_pv_request(&pv_request_node);
    pv_request_type.to_buf(&mut buf, endian, pva_type_registry)?;

    // pv request value
    let pv_request_value = PvaValue::build_pv_request(&pv_request_node);
    pv_request_value.to_buf(
        Arc::new(pv_request_type),
        &mut buf,
        endian,
        pva_type_registry,
    )?;

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::Rpc,
        buf,
    )?
    .to_buf()
}

pub fn build_rpc(
    sid: u32,
    ioid: u32,
    endian: MsgEndian,
    pva_type_registry: &mut PvaTypeRegistry,
    input_type: PvaType,
    input_value: PvaValue,
    destroy_upon_finish: bool,
) -> Result<Vec<u8>, String> {
    if let PvaType::Struct(_) = input_type {
    } else {
        return Err("RPC input argument type must be PvaType::Struct".to_string());
    }

    let mut buf: Vec<u8> = vec![];

    // struct channelRPCRequest {
    //     int serverChannelID;
    //     int requestID;
    //     byte subcommand = 0x00 mask for RPC; 0x10 mask for DESTROY;
    //     FieldDesc pvStructureIF;
    //     PVField pvStructureData;
    // };

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // 0x00
    if destroy_upon_finish {
        (0x10_i8).to_buf(&PvaType::Byte, &mut buf, endian)?;
    } else {
        (0x00_i8).to_buf(&PvaType::Byte, &mut buf, endian)?;
    }

    // input argument's type
    input_type.to_buf(&mut buf, endian, pva_type_registry)?;

    // input argument's value
    input_value.to_buf(Arc::new(input_type), &mut buf, endian, pva_type_registry)?;

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::Rpc,
        buf,
    )?
    .to_buf()
}

pub fn build_cancel_request(sid: u32, ioid: u32, endian: MsgEndian) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = vec![];
    // struct cancelRequest {
    //     int serverChannelID;
    //     int requestID;
    // };

    // sid
    sid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    // ioid
    ioid.to_buf(&PvaType::UInt, &mut buf, endian)?;

    PvaMsg::new(
        MsgSeg::NotSeg,
        MsgOrigin::Client,
        None,
        vec![],
        endian,
        PvaCmd::CancelRequest,
        buf,
    )?
    .to_buf()
}

// CMD_ORIGIN_TAG (0x16), not implement in epics-rca

// Mark Total Byte Sent (0x00), not used

// Acknowledge Total Bytes Received (0x01), not used

// Set byte order (0x02), from server to client

// Echo request (0x03)

// Echo response (0x04)
