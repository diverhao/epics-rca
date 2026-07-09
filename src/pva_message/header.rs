use crate::pva_message::cmd::PvaCmd;

const PVA_MAGIC: u8 = 0xca;
const PVA_VERSION: u8 = 0x02;
const PVA_HEADER_SIZE: usize = 8;

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
    MidOfSeg,   // bit (5, 4) = (1, 0)
    LastOfSeg,  // bit (5, 4) = (1, 1)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MsgSrc {
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
    pub src: MsgSrc,
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
            MsgSeg::MidOfSeg => 0x20,
            MsgSeg::LastOfSeg => 0x30,
        };

        let src = match self.src {
            MsgSrc::Client => 0x00,
            MsgSrc::Server => 0x40,
        };

        let endian = match self.endian {
            MsgEndian::Little => 0x00,
            MsgEndian::Big => 0x80,
        };

        msg_type | seg_type | src | endian
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
            0x20 => MsgSeg::MidOfSeg,
            0x30 => MsgSeg::LastOfSeg,
            _ => unreachable!(),
        };

        let src = if value & 0x40 == 0 {
            MsgSrc::Client
        } else {
            MsgSrc::Server
        };

        let endian = if value & 0x80 == 0 {
            MsgEndian::Little
        } else {
            MsgEndian::Big
        };

        Some(Self {
            msg_type,
            seg_type,
            src,
            endian,
        })
    }

    pub fn is_control(self) -> bool {
        self.msg_type == MsgType::Control
    }
}

// --------------- header ----------------

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PvaHeader {
    pub magic: u8,
    pub version: u8,
    pub flags: MsgFlags,
    pub cmd: PvaCmd,
    pub payload_size: u32,
}

impl PvaHeader {
    pub fn new(
        msg_type: MsgType,
        seg_type: MsgSeg,
        src: MsgSrc,
        endian: MsgEndian,
        cmd: PvaCmd,
        payload_size: u32,
    ) -> PvaHeader {
        debug_assert_eq!(msg_type == MsgType::Control, cmd.is_control());

        PvaHeader {
            magic: PVA_MAGIC,
            version: PVA_VERSION,
            flags: MsgFlags {
                msg_type,
                seg_type,
                src,
                endian,
            },
            cmd,
            payload_size,
        }
    }

    pub fn to_buf(self: &Self) -> Vec<u8> {
        debug_assert_eq!(self.flags.is_control(), self.cmd.is_control());

        let mut buf = Vec::with_capacity(PVA_HEADER_SIZE);
        buf.push(self.magic);
        buf.push(self.version);
        buf.push(self.flags.to_u8());
        buf.push(self.cmd.to_u8());

        match self.flags.endian {
            MsgEndian::Little => buf.extend_from_slice(&self.payload_size.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.payload_size.to_be_bytes()),
        }

        buf
    }

    pub fn from_buf(buf: &[u8]) -> Result<Self, String> {
        if buf.len() < PVA_HEADER_SIZE {
            return Err(String::from(
                "Warning: Remaining buffer too short for PVA header",
            ));
        }

        let magic = buf[0];
        if magic != PVA_MAGIC {
            return Err(String::from("Error: Invalid PVA magic"));
        }

        let version = buf[1];
        if version == 0 {
            return Err(String::from("Error: Invalid PVA version"));
        }

        let flags = match MsgFlags::from_u8(buf[2]) {
            Some(flags) => flags,
            None => return Err(String::from("Error: Invalid PVA header flags")),
        };

        let cmd = match PvaCmd::from_u8(flags.is_control(), buf[3]) {
            Some(cmd) => cmd,
            None => return Err(String::from("Error: Invalid PVA command")),
        };

        let payload_size = match flags.endian {
            MsgEndian::Little => u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
            MsgEndian::Big => u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]),
        };

        Ok(Self {
            magic,
            version,
            flags,
            cmd,
            payload_size,
        })
    }
}
