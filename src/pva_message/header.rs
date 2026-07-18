use crate::pva_message::cmd::{PvaCmd, PvaCtrlCmd};

const PVA_MAGIC: u8 = 0xca;
const PVA_VERSION: u8 = 0x02;
pub const PVA_HEADER_SIZE: usize = 8;

// -------------- header flags ---------------
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MsgType {
    Application, // bit 0 = 0
    Control,     // bit 0 = 1
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MsgSeg {
    NotSeg,     // bit (5, 4) = (0, 0)
    FirstOfSeg, // bit (5, 4) = (0, 1)
    MidOfSeg,   // bit (5, 4) = (1, 1)
    LastOfSeg,  // bit (5, 4) = (1, 0)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MsgOrigin {
    Client, // bit 6 = 0
    Server, // bit 6 = 1
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MsgEndian {
    Little, // bit 7 = 0
    Big,    // bit 7 = 1
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MsgFlags {
    pub msg_type: MsgType,
    pub seg_type: MsgSeg,
    pub origin: MsgOrigin,
    pub endian: MsgEndian,
}

impl MsgFlags {
    pub fn to_u8(self) -> u8 {
        let msg_type = match self.msg_type {
            MsgType::Application => 0x00,
            MsgType::Control => 0x01,
        };

        let seg_type = match self.seg_type {
            MsgSeg::NotSeg => 0x00,
            MsgSeg::FirstOfSeg => 0x10,
            MsgSeg::MidOfSeg => 0x30,
            MsgSeg::LastOfSeg => 0x20,
        };

        let origin = match self.origin {
            MsgOrigin::Client => 0x00,
            MsgOrigin::Server => 0x40,
        };

        let endian = match self.endian {
            MsgEndian::Little => 0x00,
            MsgEndian::Big => 0x80,
        };

        msg_type | seg_type | origin | endian
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        if value & 0x0e != 0 {
            return None;
        }

        let msg_type = if value & 0x01 == 0 {
            MsgType::Application
        } else {
            MsgType::Control
        };

        let seg_type = match value & 0x30 {
            0x00 => MsgSeg::NotSeg,
            0x10 => MsgSeg::FirstOfSeg,
            0x20 => MsgSeg::LastOfSeg,
            0x30 => MsgSeg::MidOfSeg,
            _ => unreachable!(),
        };

        let origin = if value & 0x40 == 0 {
            MsgOrigin::Client
        } else {
            MsgOrigin::Server
        };

        let endian = if value & 0x80 == 0 {
            MsgEndian::Little
        } else {
            MsgEndian::Big
        };

        Some(Self {
            msg_type,
            seg_type,
            origin,
            endian,
        })
    }

    pub fn is_control(self) -> bool {
        self.msg_type == MsgType::Control
    }
}

// --------------- application header ----------------

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PvaHeader {
    magic: u8,
    version: u8,
    flags: MsgFlags,
    cmd: PvaCmd,
    payload_size: u32,
}

impl PvaHeader {
    pub fn new(
        seg_type: MsgSeg,
        origin: MsgOrigin,
        endian: MsgEndian,
        cmd: PvaCmd,
        payload_size: u32,
    ) -> Result<Self, String> {
        let header = Self {
            magic: PVA_MAGIC,
            version: PVA_VERSION,
            flags: MsgFlags {
                msg_type: MsgType::Application,
                seg_type,
                origin,
                endian,
            },
            cmd,
            payload_size,
        };
        header.validate()?;
        Ok(header)
    }

    pub fn magic(&self) -> u8 {
        self.magic
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn flags(&self) -> MsgFlags {
        self.flags
    }

    pub fn cmd(&self) -> PvaCmd {
        self.cmd
    }

    pub fn payload_size(&self) -> u32 {
        self.payload_size
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_common_header(self.magic, self.version)?;

        if self.flags.msg_type != MsgType::Application {
            return Err(String::from(
                "Error: PVA application header has the control-message flag set",
            ));
        }

        Ok(())
    }

    pub fn to_buf(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        Ok(encode_header(
            self.magic,
            self.version,
            self.flags,
            self.cmd.to_u8(),
            self.payload_size,
        ))
    }

    pub fn from_buf(buf: &[u8]) -> Result<Self, String> {
        let (magic, version, flags) = decode_header_prefix(buf)?;
        if flags.msg_type != MsgType::Application {
            return Err(String::from(
                "Error: Expected a PVA application header, received a control header",
            ));
        }

        let cmd = PvaCmd::from_u8(buf[3])
            .ok_or_else(|| String::from("Error: Invalid PVA application command"))?;
        let payload_size = decode_u32(&buf[4..8], flags.endian);
        let header = Self {
            magic,
            version,
            flags,
            cmd,
            payload_size,
        };
        header.validate()?;
        Ok(header)
    }
}

// --------------- control header ----------------

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PvaCtrlHeader {
    magic: u8,
    version: u8,
    flags: MsgFlags,
    cmd: PvaCtrlCmd,
    data: u32,
}

impl PvaCtrlHeader {
    pub fn new(
        origin: MsgOrigin,
        endian: MsgEndian,
        cmd: PvaCtrlCmd,
        data: u32,
    ) -> Result<Self, String> {
        let header = Self {
            magic: PVA_MAGIC,
            version: PVA_VERSION,
            flags: MsgFlags {
                msg_type: MsgType::Control,
                seg_type: MsgSeg::NotSeg,
                origin,
                endian,
            },
            cmd,
            data,
        };
        header.validate()?;
        Ok(header)
    }

    pub fn magic(&self) -> u8 {
        self.magic
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn flags(&self) -> MsgFlags {
        self.flags
    }

    pub fn cmd(&self) -> PvaCtrlCmd {
        self.cmd
    }

    pub fn data(&self) -> u32 {
        self.data
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_common_header(self.magic, self.version)?;

        if self.flags.msg_type != MsgType::Control {
            return Err(String::from(
                "Error: PVA control header does not have the control-message flag set",
            ));
        }
        if self.flags.seg_type != MsgSeg::NotSeg {
            return Err(String::from(
                "Error: PVA control message cannot be segmented",
            ));
        }
        Ok(())
    }

    pub fn to_buf(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        Ok(encode_header(
            self.magic,
            self.version,
            self.flags,
            self.cmd.to_u8(),
            self.data,
        ))
    }

    pub fn from_buf(buf: &[u8]) -> Result<Self, String> {
        let (magic, version, flags) = decode_header_prefix(buf)?;
        if flags.msg_type != MsgType::Control {
            return Err(String::from(
                "Error: Expected a PVA control header, received an application header",
            ));
        }

        let cmd = PvaCtrlCmd::from_u8(buf[3])
            .ok_or_else(|| String::from("Error: Invalid PVA control command"))?;
        let data = decode_u32(&buf[4..8], flags.endian);
        let header = Self {
            magic,
            version,
            flags,
            cmd,
            data,
        };
        header.validate()?;
        Ok(header)
    }
}

fn validate_common_header(magic: u8, version: u8) -> Result<(), String> {
    if magic != PVA_MAGIC {
        return Err(format!(
            "Error: Invalid PVA magic 0x{magic:02X}; expected 0x{PVA_MAGIC:02X}"
        ));
    }
    if version != PVA_VERSION {
        return Err(format!(
            "Error: Invalid PVA version {version}; expected {PVA_VERSION}"
        ));
    }
    Ok(())
}

fn decode_header_prefix(buf: &[u8]) -> Result<(u8, u8, MsgFlags), String> {
    if buf.len() < PVA_HEADER_SIZE {
        return Err(String::from(
            "Warning: Remaining buffer too short for PVA header",
        ));
    }

    let magic = buf[0];
    let version = buf[1];
    validate_common_header(magic, version)?;
    let flags =
        MsgFlags::from_u8(buf[2]).ok_or_else(|| String::from("Error: Invalid PVA header flags"))?;
    Ok((magic, version, flags))
}

fn decode_u32(buf: &[u8], endian: MsgEndian) -> u32 {
    let bytes = [buf[0], buf[1], buf[2], buf[3]];
    match endian {
        MsgEndian::Little => u32::from_le_bytes(bytes),
        MsgEndian::Big => u32::from_be_bytes(bytes),
    }
}

fn encode_header(magic: u8, version: u8, flags: MsgFlags, cmd: u8, value: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(PVA_HEADER_SIZE);
    buf.push(magic);
    buf.push(version);
    buf.push(flags.to_u8());
    buf.push(cmd);
    match flags.endian {
        MsgEndian::Little => buf.extend_from_slice(&value.to_le_bytes()),
        MsgEndian::Big => buf.extend_from_slice(&value.to_be_bytes()),
    }
    buf
}
