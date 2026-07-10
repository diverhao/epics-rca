use crate::pva_message::cmd::{AppCmd, CtrlCmd, PvaCmd};

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
            MsgSeg::MidOfSeg => 0x30,
            MsgSeg::LastOfSeg => 0x20,
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
            0x20 => MsgSeg::LastOfSeg,
            0x30 => MsgSeg::MidOfSeg,
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
pub enum PvaHeaderData {
    ApplicationPayloadSize(u32),
    ControlData(i32),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PvaHeader {
    magic: u8,
    version: u8,
    flags: MsgFlags,
    cmd: PvaCmd,
    data: PvaHeaderData,
}

impl PvaHeader {
    pub fn new_application(
        seg_type: MsgSeg,
        src: MsgSrc,
        endian: MsgEndian,
        cmd: AppCmd,
        payload_size: u32,
    ) -> PvaHeader {
        PvaHeader {
            magic: PVA_MAGIC,
            version: PVA_VERSION,
            flags: MsgFlags {
                msg_type: MsgType::Application,
                seg_type,
                src,
                endian,
            },
            cmd: PvaCmd::App(cmd),
            data: PvaHeaderData::ApplicationPayloadSize(payload_size),
        }
    }

    pub fn new_control(src: MsgSrc, endian: MsgEndian, cmd: CtrlCmd, data: i32) -> PvaHeader {
        PvaHeader {
            magic: PVA_MAGIC,
            version: PVA_VERSION,
            flags: MsgFlags {
                msg_type: MsgType::Control,
                seg_type: MsgSeg::NotSeg,
                src,
                endian,
            },
            cmd: PvaCmd::Ctrl(cmd),
            data: PvaHeaderData::ControlData(data),
        }
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

    pub fn data(&self) -> PvaHeaderData {
        self.data
    }

    pub fn payload_size(&self) -> Option<u32> {
        match self.data {
            PvaHeaderData::ApplicationPayloadSize(size) => Some(size),
            PvaHeaderData::ControlData(_) => None,
        }
    }

    pub fn control_data(&self) -> Option<i32> {
        match self.data {
            PvaHeaderData::ApplicationPayloadSize(_) => None,
            PvaHeaderData::ControlData(data) => Some(data),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.magic != PVA_MAGIC {
            return Err(format!(
                "Error: Invalid PVA magic 0x{:02X}; expected 0x{PVA_MAGIC:02X}",
                self.magic
            ));
        }

        if self.version != PVA_VERSION {
            return Err(format!(
                "Error: Invalid PVA version {}; expected {PVA_VERSION}",
                self.version
            ));
        }

        match (self.flags.msg_type, self.cmd, self.data) {
            (MsgType::Application, PvaCmd::App(_), PvaHeaderData::ApplicationPayloadSize(_)) => {}
            (MsgType::Control, PvaCmd::Ctrl(_), PvaHeaderData::ControlData(_)) => {
                if self.flags.seg_type != MsgSeg::NotSeg {
                    return Err(String::from(
                        "Error: PVA control message cannot be segmented",
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "Error: Inconsistent PVA header message type, command {}, and data {:?}",
                    self.cmd, self.data
                ));
            }
        }

        Ok(())
    }

    pub fn to_buf(self: &Self) -> Result<Vec<u8>, String> {
        self.validate()?;

        let mut buf = Vec::with_capacity(PVA_HEADER_SIZE);
        buf.push(self.magic);
        buf.push(self.version);
        buf.push(self.flags.to_u8());
        buf.push(self.cmd.to_u8());

        match (self.data, self.flags.endian) {
            (PvaHeaderData::ApplicationPayloadSize(size), MsgEndian::Little) => {
                buf.extend_from_slice(&size.to_le_bytes())
            }
            (PvaHeaderData::ApplicationPayloadSize(size), MsgEndian::Big) => {
                buf.extend_from_slice(&size.to_be_bytes())
            }
            (PvaHeaderData::ControlData(data), MsgEndian::Little) => {
                buf.extend_from_slice(&data.to_le_bytes())
            }
            (PvaHeaderData::ControlData(data), MsgEndian::Big) => {
                buf.extend_from_slice(&data.to_be_bytes())
            }
        }

        Ok(buf)
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
        if version != PVA_VERSION {
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

        let data = match (flags.msg_type, flags.endian) {
            (MsgType::Application, MsgEndian::Little) => {
                PvaHeaderData::ApplicationPayloadSize(u32::from_le_bytes([
                    buf[4], buf[5], buf[6], buf[7],
                ]))
            }
            (MsgType::Application, MsgEndian::Big) => {
                PvaHeaderData::ApplicationPayloadSize(u32::from_be_bytes([
                    buf[4], buf[5], buf[6], buf[7],
                ]))
            }
            (MsgType::Control, MsgEndian::Little) => {
                PvaHeaderData::ControlData(i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]))
            }
            (MsgType::Control, MsgEndian::Big) => {
                PvaHeaderData::ControlData(i32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]))
            }
        };

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
