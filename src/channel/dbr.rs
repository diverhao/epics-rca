use crate::channel::channel::Channel;
use crate::channel::dbr_data::{
    CtrlData, CtrlEnumData, CtrlPrecisionData, DbrData, GrData, GrEnumData, GrPrecisionData,
    PlainData, StsAckStringData, StsData, TimeData,
};
use std::sync::Arc;

const DBR_STS_STRING_VALUE_OFFSET: u32 = 4;
const DBR_STS_SHORT_VALUE_OFFSET: u32 = 4;
const DBR_STS_FLOAT_VALUE_OFFSET: u32 = 4;
const DBR_STS_ENUM_VALUE_OFFSET: u32 = 4;
const DBR_STS_CHAR_VALUE_OFFSET: u32 = 5;
const DBR_STS_LONG_VALUE_OFFSET: u32 = 4;
const DBR_STS_DOUBLE_VALUE_OFFSET: u32 = 8;
const DBR_TIME_STRING_VALUE_OFFSET: u32 = 12;
const DBR_TIME_SHORT_VALUE_OFFSET: u32 = 14;
const DBR_TIME_FLOAT_VALUE_OFFSET: u32 = 12;
const DBR_TIME_ENUM_VALUE_OFFSET: u32 = 14;
const DBR_TIME_CHAR_VALUE_OFFSET: u32 = 15;
const DBR_TIME_LONG_VALUE_OFFSET: u32 = 12;
const DBR_TIME_DOUBLE_VALUE_OFFSET: u32 = 16;
const DBR_GR_STRING_VALUE_OFFSET: u32 = 4;
const DBR_GR_SHORT_VALUE_OFFSET: u32 = 24;
const DBR_GR_FLOAT_VALUE_OFFSET: u32 = 40;
const DBR_GR_ENUM_VALUE_OFFSET: u32 = 422;
const DBR_GR_CHAR_VALUE_OFFSET: u32 = 19;
const DBR_GR_LONG_VALUE_OFFSET: u32 = 36;
const DBR_GR_DOUBLE_VALUE_OFFSET: u32 = 64;
const DBR_CTRL_STRING_VALUE_OFFSET: u32 = 4;
const DBR_CTRL_SHORT_VALUE_OFFSET: u32 = 28;
const DBR_CTRL_FLOAT_VALUE_OFFSET: u32 = 48;
const DBR_CTRL_ENUM_VALUE_OFFSET: u32 = 422;
const DBR_CTRL_CHAR_VALUE_OFFSET: u32 = 21;
const DBR_CTRL_LONG_VALUE_OFFSET: u32 = 44;
const DBR_CTRL_DOUBLE_VALUE_OFFSET: u32 = 80;
const DBR_STSACK_STRING_VALUE_OFFSET: u32 = 8;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ChannelState {
    NameSearching, // start to send CA_PROTO_SEARCH
    NameFound, // after get CA_PROTO_SEARCH reply, next try to establish tcp connection with server
    TcpConnected, // after tcp connected, next send handshake packets: CA_PROTO_VERSION, CA_PROTO_CLIENT_NAME, CA_PROTO_HOST_NAME, CA_PROTO_CREATE_CHAN
    Created, // after CA_PROTO_CREATE_CHAN reply received, after which we can read/write the channel
    Destroyed, // no more name search, release all resources
}

#[derive(Copy, Clone, Debug)]
pub enum ChannelAccessRights {
    None,      // Neither read nor write
    Read,      // Read access only
    ReadWrite, // Both read and write
}

#[derive(Copy, Clone, Debug)]
pub enum ChannelSeverity {
    NoAlarm,
    Minor,
    Major,
    Invalid,
}

impl ChannelSeverity {
    pub(crate) fn from_i16(value: i16) -> Option<Self> {
        match value {
            0 => Some(ChannelSeverity::NoAlarm),
            1 => Some(ChannelSeverity::Minor),
            2 => Some(ChannelSeverity::Major),
            3 => Some(ChannelSeverity::Invalid),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ChannelStatus {
    NoAlarm,
    Read,
    Write,
    Hihi,
    High,
    Lolo,
    Low,
    State,
    Cos,
    Comm,
    Timeout,
    HwLimit,
    Calc,
    Scan,
    Link,
    Soft,
    BadSub,
    Udf,
    Disable,
    Simm,
    ReadAccess,
    WriteAccess,
}

impl ChannelStatus {
    pub(crate) fn from_i16(value: i16) -> Option<Self> {
        match value {
            0 => Some(ChannelStatus::NoAlarm),
            1 => Some(ChannelStatus::Read),
            2 => Some(ChannelStatus::Write),
            3 => Some(ChannelStatus::Hihi),
            4 => Some(ChannelStatus::High),
            5 => Some(ChannelStatus::Lolo),
            6 => Some(ChannelStatus::Low),
            7 => Some(ChannelStatus::State),
            8 => Some(ChannelStatus::Cos),
            9 => Some(ChannelStatus::Comm),
            10 => Some(ChannelStatus::Timeout),
            11 => Some(ChannelStatus::HwLimit),
            12 => Some(ChannelStatus::Calc),
            13 => Some(ChannelStatus::Scan),
            14 => Some(ChannelStatus::Link),
            15 => Some(ChannelStatus::Soft),
            16 => Some(ChannelStatus::BadSub),
            17 => Some(ChannelStatus::Udf),
            18 => Some(ChannelStatus::Disable),
            19 => Some(ChannelStatus::Simm),
            20 => Some(ChannelStatus::ReadAccess),
            21 => Some(ChannelStatus::WriteAccess),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum DbrValue {
    String(Arc<Vec<String>>),
    // Int,
    Short(Arc<Vec<i16>>),
    Float(Arc<Vec<f32>>),
    Enum(Arc<Vec<u16>>),
    Char(Arc<Vec<u8>>),
    Long(Arc<Vec<i32>>),
    Double(Arc<Vec<f64>>),
}

impl DbrValue {
    const DISPLAY_LIMIT: usize = 100;

    fn fmt_vec<T: std::fmt::Debug>(
        f: &mut std::fmt::Formatter<'_>,
        name: &str,
        values: &[T],
    ) -> std::fmt::Result {
        write!(f, "{name}([")?;

        let shown = values.len().min(Self::DISPLAY_LIMIT);
        for (i, value) in values.iter().take(Self::DISPLAY_LIMIT).enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{value:?}")?;
        }

        if values.len() > shown {
            if shown > 0 {
                f.write_str(", ")?;
            }
            write!(f, "... ({} more)", values.len() - shown)?;
        }

        f.write_str("])")
    }
}

impl std::fmt::Display for DbrValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbrValue::String(values) => Self::fmt_vec(f, "String", values),
            DbrValue::Short(values) => Self::fmt_vec(f, "Short", values),
            DbrValue::Float(values) => Self::fmt_vec(f, "Float", values),
            DbrValue::Enum(values) => Self::fmt_vec(f, "Enum", values),
            DbrValue::Char(values) => Self::fmt_vec(f, "Char", values),
            DbrValue::Long(values) => Self::fmt_vec(f, "Long", values),
            DbrValue::Double(values) => Self::fmt_vec(f, "Double", values),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum DbrType {
    String,
    // Int,
    Short,
    Float,
    Enum,
    Char,
    Long,
    Double,

    StsString,
    // StsInt,
    StsShort,
    StsFloat,
    StsEnum,
    StsChar,
    StsLong,
    StsDouble,

    TimeString,
    // TimeInt,
    TimeShort,
    TimeFloat,
    TimeEnum,
    TimeChar,
    TimeLong,
    TimeDouble,

    GrString,
    // GrInt,
    GrShort,
    GrFloat,
    GrEnum,
    GrChar,
    GrLong,
    GrDouble,

    CtrlString,
    // CtrlInt,
    CtrlShort,
    CtrlFloat,
    CtrlEnum,
    CtrlChar,
    CtrlLong,
    CtrlDouble,

    PutAckt,
    PutAcks,
    StsAckString,
    ClassName,
}

impl DbrType {
    pub(crate) fn from_u16(value: u16) -> Option<Self> {
        match value {
            0 => Some(DbrType::String),
            1 => Some(DbrType::Short),
            2 => Some(DbrType::Float),
            3 => Some(DbrType::Enum),
            4 => Some(DbrType::Char),
            5 => Some(DbrType::Long),
            6 => Some(DbrType::Double),
            7 => Some(DbrType::StsString),
            8 => Some(DbrType::StsShort),
            9 => Some(DbrType::StsFloat),
            10 => Some(DbrType::StsEnum),
            11 => Some(DbrType::StsChar),
            12 => Some(DbrType::StsLong),
            13 => Some(DbrType::StsDouble),
            14 => Some(DbrType::TimeString),
            15 => Some(DbrType::TimeShort),
            16 => Some(DbrType::TimeFloat),
            17 => Some(DbrType::TimeEnum),
            18 => Some(DbrType::TimeChar),
            19 => Some(DbrType::TimeLong),
            20 => Some(DbrType::TimeDouble),
            21 => Some(DbrType::GrString),
            22 => Some(DbrType::GrShort),
            23 => Some(DbrType::GrFloat),
            24 => Some(DbrType::GrEnum),
            25 => Some(DbrType::GrChar),
            26 => Some(DbrType::GrLong),
            27 => Some(DbrType::GrDouble),
            28 => Some(DbrType::CtrlString),
            29 => Some(DbrType::CtrlShort),
            30 => Some(DbrType::CtrlFloat),
            31 => Some(DbrType::CtrlEnum),
            32 => Some(DbrType::CtrlChar),
            33 => Some(DbrType::CtrlLong),
            34 => Some(DbrType::CtrlDouble),
            35 => Some(DbrType::PutAckt),
            36 => Some(DbrType::PutAcks),
            37 => Some(DbrType::StsAckString),
            38 => Some(DbrType::ClassName),
            _ => None,
        }
    }
}

impl DbrData {
    pub fn from_buf(buf: &Vec<u8>, data_type: DbrType, data_count: u32) -> Result<Self, String> {
        match data_type {
            DbrType::String => Ok(DbrData::String(PlainData {
                value: Self::parse_string(&buf, 0, data_count)?,
            })),
            DbrType::Short => Ok(DbrData::Short(PlainData {
                value: Self::parse_i16(&buf, 0, data_count)?,
            })),
            DbrType::Float => Ok(DbrData::Float(PlainData {
                value: Self::parse_f32(&buf, 0, data_count)?,
            })),
            DbrType::Enum => Ok(DbrData::Enum(PlainData {
                value: Self::parse_u16(&buf, 0, data_count)?,
            })),
            DbrType::Char => Ok(DbrData::Char(PlainData {
                value: Self::parse_u8(&buf, 0, data_count)?,
            })),
            DbrType::Long => Ok(DbrData::Long(PlainData {
                value: Self::parse_i32(&buf, 0, data_count)?,
            })),
            DbrType::Double => Ok(DbrData::Double(PlainData {
                value: Self::parse_f64(&buf, 0, data_count)?,
            })),
            DbrType::StsString => Ok(DbrData::StsString(Self::sts_data(
                &buf,
                Self::parse_string(&buf, DBR_STS_STRING_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::StsShort => Ok(DbrData::StsShort(Self::sts_data(
                &buf,
                Self::parse_i16(&buf, DBR_STS_SHORT_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::StsFloat => Ok(DbrData::StsFloat(Self::sts_data(
                &buf,
                Self::parse_f32(&buf, DBR_STS_FLOAT_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::StsEnum => Ok(DbrData::StsEnum(Self::sts_data(
                &buf,
                Self::parse_u16(&buf, DBR_STS_ENUM_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::StsChar => Ok(DbrData::StsChar(Self::sts_data(
                &buf,
                Self::parse_u8(&buf, DBR_STS_CHAR_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::StsLong => Ok(DbrData::StsLong(Self::sts_data(
                &buf,
                Self::parse_i32(&buf, DBR_STS_LONG_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::StsDouble => Ok(DbrData::StsDouble(Self::sts_data(
                &buf,
                Self::parse_f64(&buf, DBR_STS_DOUBLE_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::TimeString => Ok(DbrData::TimeString(Self::time_data(
                &buf,
                Self::parse_string(&buf, DBR_TIME_STRING_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::TimeShort => Ok(DbrData::TimeShort(Self::time_data(
                &buf,
                Self::parse_i16(&buf, DBR_TIME_SHORT_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::TimeFloat => Ok(DbrData::TimeFloat(Self::time_data(
                &buf,
                Self::parse_f32(&buf, DBR_TIME_FLOAT_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::TimeEnum => Ok(DbrData::TimeEnum(Self::time_data(
                &buf,
                Self::parse_u16(&buf, DBR_TIME_ENUM_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::TimeChar => Ok(DbrData::TimeChar(Self::time_data(
                &buf,
                Self::parse_u8(&buf, DBR_TIME_CHAR_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::TimeLong => Ok(DbrData::TimeLong(Self::time_data(
                &buf,
                Self::parse_i32(&buf, DBR_TIME_LONG_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::TimeDouble => Ok(DbrData::TimeDouble(Self::time_data(
                &buf,
                Self::parse_f64(&buf, DBR_TIME_DOUBLE_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::GrString => Ok(DbrData::GrString(Self::sts_data(
                &buf,
                Self::parse_string(&buf, DBR_GR_STRING_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::GrShort => Ok(DbrData::GrShort(Self::gr_i16_data(
                &buf,
                DBR_GR_SHORT_VALUE_OFFSET as usize,
                data_count,
            )?)),
            DbrType::GrFloat => Ok(DbrData::GrFloat(Self::gr_f32_data(
                &buf,
                DBR_GR_FLOAT_VALUE_OFFSET as usize,
                data_count,
            )?)),
            DbrType::GrEnum => Ok(DbrData::GrEnum(Self::gr_enum_data(
                &buf,
                DBR_GR_ENUM_VALUE_OFFSET as usize,
                data_count,
            )?)),
            DbrType::GrChar => Ok(DbrData::GrChar(Self::gr_u8_data(
                &buf,
                DBR_GR_CHAR_VALUE_OFFSET as usize,
                data_count,
            )?)),
            DbrType::GrLong => Ok(DbrData::GrLong(Self::gr_i32_data(
                &buf,
                DBR_GR_LONG_VALUE_OFFSET as usize,
                data_count,
            )?)),
            DbrType::GrDouble => Ok(DbrData::GrDouble(Self::gr_f64_data(
                &buf,
                DBR_GR_DOUBLE_VALUE_OFFSET as usize,
                data_count,
            )?)),
            DbrType::CtrlString => Ok(DbrData::CtrlString(Self::sts_data(
                &buf,
                Self::parse_string(&buf, DBR_CTRL_STRING_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::CtrlShort => Ok(DbrData::CtrlShort(Self::ctrl_i16_data(
                &buf,
                DBR_CTRL_SHORT_VALUE_OFFSET as usize,
                data_count,
            )?)),
            DbrType::CtrlFloat => Ok(DbrData::CtrlFloat(Self::ctrl_f32_data(
                &buf,
                DBR_CTRL_FLOAT_VALUE_OFFSET as usize,
                data_count,
            )?)),
            DbrType::CtrlEnum => Ok(DbrData::CtrlEnum(Self::ctrl_enum_data(
                &buf,
                DBR_CTRL_ENUM_VALUE_OFFSET as usize,
                data_count,
            )?)),
            DbrType::CtrlChar => Ok(DbrData::CtrlChar(Self::ctrl_u8_data(
                &buf,
                DBR_CTRL_CHAR_VALUE_OFFSET as usize,
                data_count,
            )?)),
            DbrType::CtrlLong => Ok(DbrData::CtrlLong(Self::ctrl_i32_data(
                &buf,
                DBR_CTRL_LONG_VALUE_OFFSET as usize,
                data_count,
            )?)),
            DbrType::CtrlDouble => Ok(DbrData::CtrlDouble(Self::ctrl_f64_data(
                &buf,
                DBR_CTRL_DOUBLE_VALUE_OFFSET as usize,
                data_count,
            )?)),
            DbrType::PutAckt => Ok(DbrData::PutAckt(PlainData {
                value: Self::parse_u16(&buf, 0, data_count)?,
            })),
            DbrType::PutAcks => Ok(DbrData::PutAcks(PlainData {
                value: Self::parse_u16(&buf, 0, data_count)?,
            })),
            DbrType::StsAckString => Ok(DbrData::StsAckString(Self::sts_ack_string_data(
                &buf,
                Self::parse_string(&buf, DBR_STSACK_STRING_VALUE_OFFSET as usize, data_count)?,
            )?)),
            DbrType::ClassName => Ok(DbrData::ClassName(PlainData {
                value: Self::parse_string(&buf, 0, data_count)?,
            })),
        }
    }

    pub fn from_buf_u16(buf: &Vec<u8>, data_type: u16, data_count: u32) -> Result<Self, String> {
        let data_type =
            DbrType::from_u16(data_type).ok_or_else(|| format!("Invalid DBR type {data_type}"))?;
        Self::from_buf(&buf, data_type, data_count)
    }

    fn value_range(
        buf: &[u8],
        start: usize,
        data_count: u32,
        elem_size: usize,
    ) -> Result<std::ops::Range<usize>, String> {
        let data_count =
            usize::try_from(data_count).map_err(|_| "Element count too large".to_string())?;
        let len = data_count
            .checked_mul(elem_size)
            .ok_or_else(|| "Buffer length overflow".to_string())?;
        let end = start
            .checked_add(len)
            .ok_or_else(|| "Buffer length overflow".to_string())?;

        if buf.len() < end {
            return Err(format!(
                "Buffer length too short: need {end} bytes, got {}",
                buf.len()
            ));
        }

        Ok(start..end)
    }

    fn fixed_c_string(buf: &[u8]) -> Result<String, String> {
        let end = buf.iter().position(|&byte| byte == 0).unwrap_or(buf.len());
        std::str::from_utf8(&buf[..end])
            .map(str::to_string)
            .map_err(|_| "Buffer contains invalid UTF-8".to_string())
    }

    fn parse_string(buf: &[u8], start: usize, data_count: u32) -> Result<Arc<Vec<String>>, String> {
        const DBR_STRING_SIZE: usize = 40;
        let range = Self::value_range(buf, start, data_count, DBR_STRING_SIZE)?;

        let mut values = Vec::with_capacity(range.len() / DBR_STRING_SIZE);
        for chunk in buf[range].chunks_exact(DBR_STRING_SIZE) {
            values.push(Self::fixed_c_string(chunk)?);
        }
        Ok(Arc::new(values))
    }

    fn parse_i16(buf: &[u8], start: usize, data_count: u32) -> Result<Arc<Vec<i16>>, String> {
        let range = Self::value_range(buf, start, data_count, 2)?;
        let mut values = Vec::with_capacity(range.len() / 2);
        for chunk in buf[range].chunks_exact(2) {
            values.push(i16::from_be_bytes([chunk[0], chunk[1]]));
        }
        Ok(Arc::new(values))
    }

    fn parse_u16(buf: &[u8], start: usize, data_count: u32) -> Result<Arc<Vec<u16>>, String> {
        let range = Self::value_range(buf, start, data_count, 2)?;
        let mut values = Vec::with_capacity(range.len() / 2);
        for chunk in buf[range].chunks_exact(2) {
            values.push(u16::from_be_bytes([chunk[0], chunk[1]]));
        }
        Ok(Arc::new(values))
    }

    fn parse_f32(buf: &[u8], start: usize, data_count: u32) -> Result<Arc<Vec<f32>>, String> {
        let range = Self::value_range(buf, start, data_count, 4)?;
        let mut values = Vec::with_capacity(range.len() / 4);
        for chunk in buf[range].chunks_exact(4) {
            values.push(f32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        Ok(Arc::new(values))
    }

    fn parse_u8(buf: &[u8], start: usize, data_count: u32) -> Result<Arc<Vec<u8>>, String> {
        let range = Self::value_range(buf, start, data_count, 1)?;
        Ok(Arc::new(buf[range].to_vec()))
    }

    fn parse_i32(buf: &[u8], start: usize, data_count: u32) -> Result<Arc<Vec<i32>>, String> {
        let range = Self::value_range(buf, start, data_count, 4)?;
        let mut values = Vec::with_capacity(range.len() / 4);
        for chunk in buf[range].chunks_exact(4) {
            values.push(i32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        Ok(Arc::new(values))
    }

    fn parse_f64(buf: &[u8], start: usize, data_count: u32) -> Result<Arc<Vec<f64>>, String> {
        let range = Self::value_range(buf, start, data_count, 8)?;
        let mut values = Vec::with_capacity(range.len() / 8);
        for chunk in buf[range].chunks_exact(8) {
            values.push(f64::from_be_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]));
        }
        Ok(Arc::new(values))
    }

    fn require_len(buf: &[u8], len: usize) -> Result<(), String> {
        if buf.len() < len {
            return Err(format!(
                "Buffer length too short: need {len} bytes, got {}",
                buf.len()
            ));
        }
        Ok(())
    }

    fn i16_at(buf: &[u8], offset: usize) -> Result<i16, String> {
        Self::require_len(buf, offset + 2)?;
        Ok(i16::from_be_bytes([buf[offset], buf[offset + 1]]))
    }

    fn u16_at(buf: &[u8], offset: usize) -> Result<u16, String> {
        Self::require_len(buf, offset + 2)?;
        Ok(u16::from_be_bytes([buf[offset], buf[offset + 1]]))
    }

    fn i32_at(buf: &[u8], offset: usize) -> Result<i32, String> {
        Self::require_len(buf, offset + 4)?;
        Ok(i32::from_be_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]))
    }

    fn u32_at(buf: &[u8], offset: usize) -> Result<u32, String> {
        Self::require_len(buf, offset + 4)?;
        Ok(u32::from_be_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]))
    }

    fn f32_at(buf: &[u8], offset: usize) -> Result<f32, String> {
        Self::require_len(buf, offset + 4)?;
        Ok(f32::from_be_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]))
    }

    fn f64_at(buf: &[u8], offset: usize) -> Result<f64, String> {
        Self::require_len(buf, offset + 8)?;
        Ok(f64::from_be_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
            buf[offset + 4],
            buf[offset + 5],
            buf[offset + 6],
            buf[offset + 7],
        ]))
    }

    fn alarm(buf: &[u8]) -> Result<(ChannelStatus, ChannelSeverity), String> {
        let status = ChannelStatus::from_i16(Self::i16_at(buf, 0)?)
            .ok_or_else(|| "Invalid DBR alarm status".to_string())?;
        let severity = ChannelSeverity::from_i16(Self::i16_at(buf, 2)?)
            .ok_or_else(|| "Invalid DBR alarm severity".to_string())?;
        Ok((status, severity))
    }

    fn sts_data<T>(buf: &[u8], value: Arc<Vec<T>>) -> Result<StsData<T>, String> {
        let (status, severity) = Self::alarm(buf)?;
        Ok(StsData {
            value,
            status,
            severity,
        })
    }

    fn sts_ack_string_data(
        buf: &[u8],
        value: Arc<Vec<String>>,
    ) -> Result<StsAckStringData, String> {
        let (status, severity) = Self::alarm(buf)?;
        Ok(StsAckStringData {
            value,
            status,
            severity,
            ackt: Some(Self::u16_at(buf, 4)?),
            acks: Some(Self::u16_at(buf, 6)?),
        })
    }

    fn time_data<T>(buf: &[u8], value: Arc<Vec<T>>) -> Result<TimeData<T>, String> {
        const POSIX_TIME_AT_EPICS_EPOCH: u32 = 631_152_000;

        let (status, severity) = Self::alarm(buf)?;
        let seconds_since_epoch = Self::u32_at(buf, 4)?
            .checked_add(POSIX_TIME_AT_EPICS_EPOCH)
            .and_then(|seconds| i32::try_from(seconds).ok())
            .ok_or_else(|| "DBR timestamp out of range".to_string())?;
        let nano_seconds = Self::u32_at(buf, 8)?;

        Ok(TimeData {
            value,
            status,
            severity,
            seconds_since_epoch,
            nano_seconds,
        })
    }

    fn gr_i16_data(
        buf: &[u8],
        value_offset: usize,
        data_count: u32,
    ) -> Result<GrData<i16>, String> {
        let (status, severity) = Self::alarm(buf)?;
        Ok(GrData {
            value: Self::parse_i16(buf, value_offset, data_count)?,
            status,
            severity,
            units: Self::fixed_c_string(Self::range(buf, 4, 12)?)?,
            upper_display_limit: Self::i16_at(buf, 12)?,
            lower_display_limit: Self::i16_at(buf, 14)?,
            upper_alarm_limit: Self::i16_at(buf, 16)?,
            upper_warning_limit: Self::i16_at(buf, 18)?,
            lower_warning_limit: Self::i16_at(buf, 20)?,
            lower_alarm_limit: Self::i16_at(buf, 22)?,
        })
    }

    fn gr_u8_data(buf: &[u8], value_offset: usize, data_count: u32) -> Result<GrData<u8>, String> {
        let (status, severity) = Self::alarm(buf)?;
        Self::require_len(buf, value_offset)?;
        Ok(GrData {
            value: Self::parse_u8(buf, value_offset, data_count)?,
            status,
            severity,
            units: Self::fixed_c_string(Self::range(buf, 4, 12)?)?,
            upper_display_limit: buf[12] as i16,
            lower_display_limit: buf[13] as i16,
            upper_alarm_limit: buf[14] as i16,
            upper_warning_limit: buf[15] as i16,
            lower_warning_limit: buf[16] as i16,
            lower_alarm_limit: buf[17] as i16,
        })
    }

    fn gr_i32_data(
        buf: &[u8],
        value_offset: usize,
        data_count: u32,
    ) -> Result<GrData<i32>, String> {
        let (status, severity) = Self::alarm(buf)?;
        Ok(GrData {
            value: Self::parse_i32(buf, value_offset, data_count)?,
            status,
            severity,
            units: Self::fixed_c_string(Self::range(buf, 4, 12)?)?,
            upper_display_limit: Self::i32_at(buf, 12)? as i16,
            lower_display_limit: Self::i32_at(buf, 16)? as i16,
            upper_alarm_limit: Self::i32_at(buf, 20)? as i16,
            upper_warning_limit: Self::i32_at(buf, 24)? as i16,
            lower_warning_limit: Self::i32_at(buf, 28)? as i16,
            lower_alarm_limit: Self::i32_at(buf, 32)? as i16,
        })
    }

    fn gr_f32_data(
        buf: &[u8],
        value_offset: usize,
        data_count: u32,
    ) -> Result<GrPrecisionData<f32>, String> {
        let (status, severity) = Self::alarm(buf)?;
        Ok(GrPrecisionData {
            value: Self::parse_f32(buf, value_offset, data_count)?,
            status,
            severity,
            precision: Self::i16_at(buf, 4)?,
            padding: Self::i16_at(buf, 6)?,
            units: Self::fixed_c_string(Self::range(buf, 8, 16)?)?,
            upper_display_limit: Self::f32_at(buf, 16)? as i16,
            lower_display_limit: Self::f32_at(buf, 20)? as i16,
            upper_alarm_limit: Self::f32_at(buf, 24)? as i16,
            upper_warning_limit: Self::f32_at(buf, 28)? as i16,
            lower_warning_limit: Self::f32_at(buf, 32)? as i16,
            lower_alarm_limit: Self::f32_at(buf, 36)? as i16,
        })
    }

    fn gr_f64_data(
        buf: &[u8],
        value_offset: usize,
        data_count: u32,
    ) -> Result<GrPrecisionData<f64>, String> {
        let (status, severity) = Self::alarm(buf)?;
        Ok(GrPrecisionData {
            value: Self::parse_f64(buf, value_offset, data_count)?,
            status,
            severity,
            precision: Self::i16_at(buf, 4)?,
            padding: Self::i16_at(buf, 6)?,
            units: Self::fixed_c_string(Self::range(buf, 8, 16)?)?,
            upper_display_limit: Self::f64_at(buf, 16)? as i16,
            lower_display_limit: Self::f64_at(buf, 24)? as i16,
            upper_alarm_limit: Self::f64_at(buf, 32)? as i16,
            upper_warning_limit: Self::f64_at(buf, 40)? as i16,
            lower_warning_limit: Self::f64_at(buf, 48)? as i16,
            lower_alarm_limit: Self::f64_at(buf, 56)? as i16,
        })
    }

    fn gr_enum_data(
        buf: &[u8],
        value_offset: usize,
        data_count: u32,
    ) -> Result<GrEnumData, String> {
        let (status, severity) = Self::alarm(buf)?;
        let (number_of_string_used, strings) = Self::enum_strings(buf)?;
        Ok(GrEnumData {
            value: Self::parse_u16(buf, value_offset, data_count)?,
            status,
            severity,
            number_of_string_used,
            strings,
        })
    }

    fn ctrl_i16_data(
        buf: &[u8],
        value_offset: usize,
        data_count: u32,
    ) -> Result<CtrlData<i16>, String> {
        let gr = Self::gr_i16_data(buf, value_offset, data_count)?;
        Ok(CtrlData {
            value: gr.value,
            status: gr.status,
            severity: gr.severity,
            units: gr.units,
            upper_display_limit: gr.upper_display_limit,
            lower_display_limit: gr.lower_display_limit,
            upper_alarm_limit: gr.upper_alarm_limit,
            upper_warning_limit: gr.upper_warning_limit,
            lower_warning_limit: gr.lower_warning_limit,
            lower_alarm_limit: gr.lower_alarm_limit,
            upper_control_limit: Some(Self::i16_at(buf, 24)?),
            lower_control_limit: Some(Self::i16_at(buf, 26)?),
        })
    }

    fn ctrl_u8_data(
        buf: &[u8],
        value_offset: usize,
        data_count: u32,
    ) -> Result<CtrlData<u8>, String> {
        let gr = Self::gr_u8_data(buf, value_offset, data_count)?;
        Ok(CtrlData {
            value: gr.value,
            status: gr.status,
            severity: gr.severity,
            units: gr.units,
            upper_display_limit: gr.upper_display_limit,
            lower_display_limit: gr.lower_display_limit,
            upper_alarm_limit: gr.upper_alarm_limit,
            upper_warning_limit: gr.upper_warning_limit,
            lower_warning_limit: gr.lower_warning_limit,
            lower_alarm_limit: gr.lower_alarm_limit,
            upper_control_limit: Some(buf[18] as i16),
            lower_control_limit: Some(buf[19] as i16),
        })
    }

    fn ctrl_i32_data(
        buf: &[u8],
        value_offset: usize,
        data_count: u32,
    ) -> Result<CtrlData<i32>, String> {
        let gr = Self::gr_i32_data(buf, value_offset, data_count)?;
        Ok(CtrlData {
            value: gr.value,
            status: gr.status,
            severity: gr.severity,
            units: gr.units,
            upper_display_limit: gr.upper_display_limit,
            lower_display_limit: gr.lower_display_limit,
            upper_alarm_limit: gr.upper_alarm_limit,
            upper_warning_limit: gr.upper_warning_limit,
            lower_warning_limit: gr.lower_warning_limit,
            lower_alarm_limit: gr.lower_alarm_limit,
            upper_control_limit: Some(Self::i32_at(buf, 36)? as i16),
            lower_control_limit: Some(Self::i32_at(buf, 40)? as i16),
        })
    }

    fn ctrl_f32_data(
        buf: &[u8],
        value_offset: usize,
        data_count: u32,
    ) -> Result<CtrlPrecisionData<f32>, String> {
        let gr = Self::gr_f32_data(buf, value_offset, data_count)?;
        Ok(CtrlPrecisionData {
            value: gr.value,
            status: gr.status,
            severity: gr.severity,
            precision: gr.precision,
            padding: gr.padding,
            units: gr.units,
            upper_display_limit: gr.upper_display_limit,
            lower_display_limit: gr.lower_display_limit,
            upper_alarm_limit: gr.upper_alarm_limit,
            upper_warning_limit: gr.upper_warning_limit,
            lower_warning_limit: gr.lower_warning_limit,
            lower_alarm_limit: gr.lower_alarm_limit,
            upper_control_limit: Some(Self::f32_at(buf, 40)? as i16),
            lower_control_limit: Some(Self::f32_at(buf, 44)? as i16),
        })
    }

    fn ctrl_f64_data(
        buf: &[u8],
        value_offset: usize,
        data_count: u32,
    ) -> Result<CtrlPrecisionData<f64>, String> {
        let gr = Self::gr_f64_data(buf, value_offset, data_count)?;
        Ok(CtrlPrecisionData {
            value: gr.value,
            status: gr.status,
            severity: gr.severity,
            precision: gr.precision,
            padding: gr.padding,
            units: gr.units,
            upper_display_limit: gr.upper_display_limit,
            lower_display_limit: gr.lower_display_limit,
            upper_alarm_limit: gr.upper_alarm_limit,
            upper_warning_limit: gr.upper_warning_limit,
            lower_warning_limit: gr.lower_warning_limit,
            lower_alarm_limit: gr.lower_alarm_limit,
            upper_control_limit: Some(Self::f64_at(buf, 64)? as i16),
            lower_control_limit: Some(Self::f64_at(buf, 72)? as i16),
        })
    }

    fn ctrl_enum_data(
        buf: &[u8],
        value_offset: usize,
        data_count: u32,
    ) -> Result<CtrlEnumData, String> {
        let (status, severity) = Self::alarm(buf)?;
        let (number_of_string_used, strings) = Self::enum_strings(buf)?;
        Ok(CtrlEnumData {
            value: Self::parse_u16(buf, value_offset, data_count)?,
            status,
            severity,
            number_of_string_used,
            strings,
        })
    }

    fn enum_strings(buf: &[u8]) -> Result<(i16, Vec<String>), String> {
        const DBR_ENUM_STRING_COUNT: usize = 16;
        const DBR_ENUM_STRING_SIZE: usize = 26;
        const DBR_ENUM_STRING_START: usize = 6;

        let number_of_string_used = Self::i16_at(buf, 4)?;
        if !(0..=DBR_ENUM_STRING_COUNT as i16).contains(&number_of_string_used) {
            return Err(format!("Invalid enum string count {number_of_string_used}"));
        }

        Self::require_len(buf, DBR_GR_ENUM_VALUE_OFFSET as usize)?;
        let mut strings = Vec::with_capacity(number_of_string_used as usize);
        for i in 0..number_of_string_used as usize {
            let start = DBR_ENUM_STRING_START + i * DBR_ENUM_STRING_SIZE;
            let end = start + DBR_ENUM_STRING_SIZE;
            strings.push(Self::fixed_c_string(Self::range(buf, start, end)?)?);
        }

        Ok((number_of_string_used, strings))
    }

    fn range(buf: &[u8], start: usize, end: usize) -> Result<&[u8], String> {
        Self::require_len(buf, end)?;
        Ok(&buf[start..end])
    }
}

impl Channel {
    
    // ----------------- dbr type conversion -------
    pub fn data_type_native_as_time(self: &Self) -> DbrType {
        match self.data_type_native() {
            DbrType::String
            | DbrType::StsString
            | DbrType::TimeString
            | DbrType::GrString
            | DbrType::CtrlString
            | DbrType::StsAckString
            | DbrType::ClassName => DbrType::TimeString,
            DbrType::Short
            | DbrType::StsShort
            | DbrType::TimeShort
            | DbrType::GrShort
            | DbrType::CtrlShort => DbrType::TimeShort,
            DbrType::Float
            | DbrType::StsFloat
            | DbrType::TimeFloat
            | DbrType::GrFloat
            | DbrType::CtrlFloat => DbrType::TimeFloat,
            DbrType::Enum
            | DbrType::StsEnum
            | DbrType::TimeEnum
            | DbrType::GrEnum
            | DbrType::CtrlEnum
            | DbrType::PutAckt
            | DbrType::PutAcks => DbrType::TimeEnum,
            DbrType::Char
            | DbrType::StsChar
            | DbrType::TimeChar
            | DbrType::GrChar
            | DbrType::CtrlChar => DbrType::TimeChar,
            DbrType::Long
            | DbrType::StsLong
            | DbrType::TimeLong
            | DbrType::GrLong
            | DbrType::CtrlLong => DbrType::TimeLong,
            DbrType::Double
            | DbrType::StsDouble
            | DbrType::TimeDouble
            | DbrType::GrDouble
            | DbrType::CtrlDouble => DbrType::TimeDouble,
        }
    }

    pub fn data_type_native_as_sts(self: &Self) -> DbrType {
        match self.data_type_native() {
            DbrType::String
            | DbrType::StsString
            | DbrType::TimeString
            | DbrType::GrString
            | DbrType::CtrlString
            | DbrType::StsAckString
            | DbrType::ClassName => DbrType::StsString,
            DbrType::Short
            | DbrType::StsShort
            | DbrType::TimeShort
            | DbrType::GrShort
            | DbrType::CtrlShort => DbrType::StsShort,
            DbrType::Float
            | DbrType::StsFloat
            | DbrType::TimeFloat
            | DbrType::GrFloat
            | DbrType::CtrlFloat => DbrType::StsFloat,
            DbrType::Enum
            | DbrType::StsEnum
            | DbrType::TimeEnum
            | DbrType::GrEnum
            | DbrType::CtrlEnum
            | DbrType::PutAckt
            | DbrType::PutAcks => DbrType::StsEnum,
            DbrType::Char
            | DbrType::StsChar
            | DbrType::TimeChar
            | DbrType::GrChar
            | DbrType::CtrlChar => DbrType::StsChar,
            DbrType::Long
            | DbrType::StsLong
            | DbrType::TimeLong
            | DbrType::GrLong
            | DbrType::CtrlLong => DbrType::StsLong,
            DbrType::Double
            | DbrType::StsDouble
            | DbrType::TimeDouble
            | DbrType::GrDouble
            | DbrType::CtrlDouble => DbrType::StsDouble,
        }
    }

    pub fn data_type_native_as_gr(self: &Self) -> DbrType {
        match self.data_type_native() {
            DbrType::String
            | DbrType::StsString
            | DbrType::TimeString
            | DbrType::GrString
            | DbrType::CtrlString
            | DbrType::StsAckString
            | DbrType::ClassName => DbrType::GrString,
            DbrType::Short
            | DbrType::StsShort
            | DbrType::TimeShort
            | DbrType::GrShort
            | DbrType::CtrlShort => DbrType::GrShort,
            DbrType::Float
            | DbrType::StsFloat
            | DbrType::TimeFloat
            | DbrType::GrFloat
            | DbrType::CtrlFloat => DbrType::GrFloat,
            DbrType::Enum
            | DbrType::StsEnum
            | DbrType::TimeEnum
            | DbrType::GrEnum
            | DbrType::CtrlEnum
            | DbrType::PutAckt
            | DbrType::PutAcks => DbrType::GrEnum,
            DbrType::Char
            | DbrType::StsChar
            | DbrType::TimeChar
            | DbrType::GrChar
            | DbrType::CtrlChar => DbrType::GrChar,
            DbrType::Long
            | DbrType::StsLong
            | DbrType::TimeLong
            | DbrType::GrLong
            | DbrType::CtrlLong => DbrType::GrLong,
            DbrType::Double
            | DbrType::StsDouble
            | DbrType::TimeDouble
            | DbrType::GrDouble
            | DbrType::CtrlDouble => DbrType::GrDouble,
        }
    }

    pub fn data_type_native_as_ctrl(self: &Self) -> DbrType {
        match self.data_type_native() {
            DbrType::String
            | DbrType::StsString
            | DbrType::TimeString
            | DbrType::GrString
            | DbrType::CtrlString
            | DbrType::StsAckString
            | DbrType::ClassName => DbrType::CtrlString,
            DbrType::Short
            | DbrType::StsShort
            | DbrType::TimeShort
            | DbrType::GrShort
            | DbrType::CtrlShort => DbrType::CtrlShort,
            DbrType::Float
            | DbrType::StsFloat
            | DbrType::TimeFloat
            | DbrType::GrFloat
            | DbrType::CtrlFloat => DbrType::CtrlFloat,
            DbrType::Enum
            | DbrType::StsEnum
            | DbrType::TimeEnum
            | DbrType::GrEnum
            | DbrType::CtrlEnum
            | DbrType::PutAckt
            | DbrType::PutAcks => DbrType::CtrlEnum,
            DbrType::Char
            | DbrType::StsChar
            | DbrType::TimeChar
            | DbrType::GrChar
            | DbrType::CtrlChar => DbrType::CtrlChar,
            DbrType::Long
            | DbrType::StsLong
            | DbrType::TimeLong
            | DbrType::GrLong
            | DbrType::CtrlLong => DbrType::CtrlLong,
            DbrType::Double
            | DbrType::StsDouble
            | DbrType::TimeDouble
            | DbrType::GrDouble
            | DbrType::CtrlDouble => DbrType::CtrlDouble,
        }
    }
}
