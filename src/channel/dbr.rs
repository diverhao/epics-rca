use crate::channel::channel::Channel;

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
    NeverConnected, // initial state
    NameSearching,  // start to send CA_PROTO_SEARCH
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
    String(Vec<String>),
    // Int,
    Short(Vec<i16>),
    Float(Vec<f32>),
    Enum(Vec<u16>),
    Char(Vec<u8>),
    Long(Vec<i32>),
    Double(Vec<f64>),
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

impl Channel {
    pub fn update_from_payload_buf(&self, buf: &Vec<u8>, num_elem: u32, typ: DbrType) {
        // update Channel.meta
        match typ {
            DbrType::StsString
            | DbrType::StsShort
            | DbrType::StsFloat
            | DbrType::StsEnum
            | DbrType::StsChar
            | DbrType::StsLong
            | DbrType::StsDouble
            | DbrType::StsAckString
            | DbrType::GrString
            | DbrType::CtrlString => self.update_sts_meta_from_buf(buf),
            DbrType::TimeString
            | DbrType::TimeShort
            | DbrType::TimeFloat
            | DbrType::TimeEnum
            | DbrType::TimeChar
            | DbrType::TimeLong
            | DbrType::TimeDouble => self.update_time_meta_from_buf(buf),
            DbrType::GrShort => self.update_gr_short_meta_from_buf(buf),
            DbrType::CtrlShort => self.update_ctrl_int_meta_from_buf(buf),
            DbrType::GrFloat => self.update_gr_float_meta_from_buf(buf),
            DbrType::CtrlFloat => self.update_ctrl_float_meta_from_buf(buf),
            DbrType::CtrlEnum | DbrType::GrEnum => self.update_gr_enum_meta_from_buf(buf),
            DbrType::GrChar => self.update_gr_char_meta_from_buf(buf),
            DbrType::CtrlChar => self.update_ctrl_char_meta_from_buf(buf),
            DbrType::GrLong => self.update_gr_long_meta_from_buf(buf),
            DbrType::CtrlLong => self.update_ctrl_long_meta_from_buf(buf),
            DbrType::GrDouble => self.update_gr_double_meta_from_buf(buf),
            DbrType::CtrlDouble => self.update_ctrl_double_meta_from_buf(buf),
            _ => {}
        }

        // update Channel.value
        let value = match typ {
            DbrType::String => Self::buf_to_string(&buf, 0, num_elem),
            DbrType::Short => Self::buf_to_short(&buf, 0, num_elem),
            DbrType::Float => Self::buf_to_float(&buf, 0, num_elem),
            DbrType::Enum => Self::buf_to_enum(&buf, 0, num_elem),
            DbrType::Char => Self::buf_to_char(&buf, 0, num_elem),
            DbrType::Long => Self::buf_to_long(&buf, 0, num_elem),
            DbrType::Double => Self::buf_to_double(&buf, 0, num_elem),
            DbrType::StsString => Self::buf_to_string(&buf, DBR_STS_STRING_VALUE_OFFSET, num_elem),
            DbrType::StsShort => Self::buf_to_short(&buf, DBR_STS_SHORT_VALUE_OFFSET, num_elem),
            DbrType::StsFloat => Self::buf_to_float(&buf, DBR_STS_FLOAT_VALUE_OFFSET, num_elem),
            DbrType::StsEnum => Self::buf_to_enum(&buf, DBR_STS_ENUM_VALUE_OFFSET, num_elem),
            DbrType::StsChar => Self::buf_to_char(&buf, DBR_STS_CHAR_VALUE_OFFSET, num_elem),
            DbrType::StsLong => Self::buf_to_long(&buf, DBR_STS_LONG_VALUE_OFFSET, num_elem),
            DbrType::StsDouble => Self::buf_to_double(&buf, DBR_STS_DOUBLE_VALUE_OFFSET, num_elem),
            DbrType::TimeString => {
                Self::buf_to_string(&buf, DBR_TIME_STRING_VALUE_OFFSET, num_elem)
            }
            DbrType::TimeShort => Self::buf_to_short(&buf, DBR_TIME_SHORT_VALUE_OFFSET, num_elem),
            DbrType::TimeFloat => Self::buf_to_float(&buf, DBR_TIME_FLOAT_VALUE_OFFSET, num_elem),
            DbrType::TimeEnum => Self::buf_to_enum(&buf, DBR_TIME_ENUM_VALUE_OFFSET, num_elem),
            DbrType::TimeChar => Self::buf_to_char(&buf, DBR_TIME_CHAR_VALUE_OFFSET, num_elem),
            DbrType::TimeLong => Self::buf_to_long(&buf, DBR_TIME_LONG_VALUE_OFFSET, num_elem),
            DbrType::TimeDouble => {
                Self::buf_to_double(&buf, DBR_TIME_DOUBLE_VALUE_OFFSET, num_elem)
            }
            DbrType::GrString => Self::buf_to_string(&buf, DBR_GR_STRING_VALUE_OFFSET, num_elem),
            DbrType::GrShort => Self::buf_to_short(&buf, DBR_GR_SHORT_VALUE_OFFSET, num_elem),
            DbrType::GrFloat => Self::buf_to_float(&buf, DBR_GR_FLOAT_VALUE_OFFSET, num_elem),
            DbrType::GrEnum => Self::buf_to_enum(&buf, DBR_GR_ENUM_VALUE_OFFSET, num_elem),
            DbrType::GrChar => Self::buf_to_char(&buf, DBR_GR_CHAR_VALUE_OFFSET, num_elem),
            DbrType::GrLong => Self::buf_to_long(&buf, DBR_GR_LONG_VALUE_OFFSET, num_elem),
            DbrType::GrDouble => Self::buf_to_double(&buf, DBR_GR_DOUBLE_VALUE_OFFSET, num_elem),
            DbrType::CtrlString => {
                Self::buf_to_string(&buf, DBR_CTRL_STRING_VALUE_OFFSET, num_elem)
            }
            DbrType::CtrlShort => Self::buf_to_short(&buf, DBR_CTRL_SHORT_VALUE_OFFSET, num_elem),
            DbrType::CtrlFloat => Self::buf_to_float(&buf, DBR_CTRL_FLOAT_VALUE_OFFSET, num_elem),
            DbrType::CtrlEnum => Self::buf_to_enum(&buf, DBR_CTRL_ENUM_VALUE_OFFSET, num_elem),
            DbrType::CtrlChar => Self::buf_to_char(&buf, DBR_CTRL_CHAR_VALUE_OFFSET, num_elem),
            DbrType::CtrlLong => Self::buf_to_long(&buf, DBR_CTRL_LONG_VALUE_OFFSET, num_elem),
            DbrType::CtrlDouble => {
                Self::buf_to_double(&buf, DBR_CTRL_DOUBLE_VALUE_OFFSET, num_elem)
            }
            DbrType::PutAckt => Self::buf_to_enum(&buf, 0, num_elem),
            DbrType::PutAcks => Self::buf_to_enum(&buf, 0, num_elem),
            DbrType::StsAckString => {
                Self::buf_to_string(&buf, DBR_STSACK_STRING_VALUE_OFFSET, num_elem)
            }
            DbrType::ClassName => Self::buf_to_string(&buf, 0, num_elem),
        };

        if let Ok(value) = value {
            self.set_value(Some(value));
        } else {
            self.set_value(None);
        }
    }

    // ------------------- value ---------------

    fn value_range(
        buf: &Vec<u8>,
        start: u32,
        num_elem: u32,
        elem_size: usize,
    ) -> Result<std::ops::Range<usize>, String> {
        let start = start as usize;
        let num_elem = usize::try_from(num_elem).map_err(|_| "Element count too large")?;
        let len = num_elem
            .checked_mul(elem_size)
            .ok_or("Buffer length overflow")?;
        let end = start.checked_add(len).ok_or("Buffer length overflow")?;
        if buf.len() < end {
            return Err("Buffer length too short".to_string());
        }

        Ok(start..end)
    }

    fn buf_to_string(buf: &Vec<u8>, start: u32, num_elem: u32) -> Result<DbrValue, String> {
        const DBR_STRING_SIZE: usize = 40;

        let range = Self::value_range(buf, start, num_elem, DBR_STRING_SIZE)?;

        let mut strings = Vec::with_capacity(range.len() / DBR_STRING_SIZE);
        for chunk in buf[range].chunks_exact(DBR_STRING_SIZE) {
            let Some(string) = Self::fixed_c_string(chunk) else {
                return Err("Buffer contains invalid UTF-8".to_string());
            };
            strings.push(string);
        }

        Ok(DbrValue::String(strings))
    }

    fn buf_to_short(buf: &Vec<u8>, start: u32, num_elem: u32) -> Result<DbrValue, String> {
        let range = Self::value_range(buf, start, num_elem, 2)?;

        let mut shorts = Vec::with_capacity(range.len() / 2);
        for chunk in buf[range].chunks_exact(2) {
            shorts.push(i16::from_be_bytes([chunk[0], chunk[1]]));
        }

        Ok(DbrValue::Short(shorts))
    }

    fn buf_to_float(buf: &Vec<u8>, start: u32, num_elem: u32) -> Result<DbrValue, String> {
        let range = Self::value_range(buf, start, num_elem, 4)?;

        let mut floats = Vec::with_capacity(range.len() / 4);
        for chunk in buf[range].chunks_exact(4) {
            floats.push(f32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }

        Ok(DbrValue::Float(floats))
    }

    fn buf_to_enum(buf: &Vec<u8>, start: u32, num_elem: u32) -> Result<DbrValue, String> {
        let range = Self::value_range(buf, start, num_elem, 2)?;

        let mut enums = Vec::with_capacity(range.len() / 2);
        for chunk in buf[range].chunks_exact(2) {
            enums.push(u16::from_be_bytes([chunk[0], chunk[1]]));
        }

        Ok(DbrValue::Enum(enums))
    }

    fn buf_to_char(buf: &Vec<u8>, start: u32, num_elem: u32) -> Result<DbrValue, String> {
        let range = Self::value_range(buf, start, num_elem, 1)?;

        Ok(DbrValue::Char(buf[range].to_vec()))
    }

    fn buf_to_long(buf: &Vec<u8>, start: u32, num_elem: u32) -> Result<DbrValue, String> {
        let range = Self::value_range(buf, start, num_elem, 4)?;

        let mut longs = Vec::with_capacity(range.len() / 4);
        for chunk in buf[range].chunks_exact(4) {
            longs.push(i32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }

        Ok(DbrValue::Long(longs))
    }

    fn buf_to_double(buf: &Vec<u8>, start: u32, num_elem: u32) -> Result<DbrValue, String> {
        let range = Self::value_range(buf, start, num_elem, 8)?;

        let mut doubles = Vec::with_capacity(range.len() / 8);
        for chunk in buf[range].chunks_exact(8) {
            doubles.push(f64::from_be_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]));
        }

        Ok(DbrValue::Double(doubles))
    }

    // ------------------- meta ----------------

    fn fixed_c_string(buf: &[u8]) -> Option<String> {
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        std::str::from_utf8(&buf[..end]).ok().map(str::to_string)
    }

    fn update_sts_meta_from_buf(&self, buf: &Vec<u8>) {
        if buf.len() < 4 {
            return;
        }

        let status_code = i16::from_be_bytes([buf[0], buf[1]]);
        let severity_code = i16::from_be_bytes([buf[2], buf[3]]);

        let Some(status) = ChannelStatus::from_i16(status_code) else {
            return;
        };
        let Some(severity) = ChannelSeverity::from_i16(severity_code) else {
            return;
        };

        self.set_status(status);
        self.set_severity(severity);
    }

    fn update_time_meta_from_buf(&self, buf: &Vec<u8>) {
        const POSIX_TIME_AT_EPICS_EPOCH: u32 = 631_152_000;

        if buf.len() < 12 {
            return;
        }

        let status_code = i16::from_be_bytes([buf[0], buf[1]]);
        let severity_code = i16::from_be_bytes([buf[2], buf[3]]);

        let Some(status) = ChannelStatus::from_i16(status_code) else {
            return;
        };
        let Some(severity) = ChannelSeverity::from_i16(severity_code) else {
            return;
        };

        let Some(seconds_since_epoch) = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]])
            .checked_add(POSIX_TIME_AT_EPICS_EPOCH)
            .and_then(|seconds| i32::try_from(seconds).ok())
        else {
            return;
        };
        let nano_seconds = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);

        self.set_status(status);
        self.set_severity(severity);
        self.set_seconds_since_epoch(seconds_since_epoch);
        self.set_nano_seconds(nano_seconds);
    }

    fn update_gr_short_meta_from_buf(&self, buf: &Vec<u8>) {
        if buf.len() < DBR_GR_SHORT_VALUE_OFFSET as usize {
            return;
        }

        let status_code = i16::from_be_bytes([buf[0], buf[1]]);
        let severity_code = i16::from_be_bytes([buf[2], buf[3]]);

        let Some(status) = ChannelStatus::from_i16(status_code) else {
            return;
        };
        let Some(severity) = ChannelSeverity::from_i16(severity_code) else {
            return;
        };

        let Some(units) = Self::fixed_c_string(&buf[4..12]) else {
            return;
        };

        self.set_status(status);
        self.set_severity(severity);
        self.set_units(units);
        self.set_upper_display_limit(i16::from_be_bytes([buf[12], buf[13]]));
        self.set_lower_display_limit(i16::from_be_bytes([buf[14], buf[15]]));
        self.set_upper_alarm_limit(i16::from_be_bytes([buf[16], buf[17]]));
        self.set_upper_warning_limit(i16::from_be_bytes([buf[18], buf[19]]));
        self.set_lower_warning_limit(i16::from_be_bytes([buf[20], buf[21]]));
        self.set_lower_alarm_limit(i16::from_be_bytes([buf[22], buf[23]]));
    }

    fn update_gr_float_meta_from_buf(&self, buf: &Vec<u8>) {
        if buf.len() < DBR_GR_FLOAT_VALUE_OFFSET as usize {
            return;
        }

        let status_code = i16::from_be_bytes([buf[0], buf[1]]);
        let severity_code = i16::from_be_bytes([buf[2], buf[3]]);

        let Some(status) = ChannelStatus::from_i16(status_code) else {
            return;
        };
        let Some(severity) = ChannelSeverity::from_i16(severity_code) else {
            return;
        };

        let Some(units) = Self::fixed_c_string(&buf[8..16]) else {
            return;
        };

        self.set_status(status);
        self.set_severity(severity);
        self.set_precision(i16::from_be_bytes([buf[4], buf[5]]));
        self.set_padding(i16::from_be_bytes([buf[6], buf[7]]));
        self.set_units(units);
        self.set_upper_display_limit(
            f32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]]) as i16
        );
        self.set_lower_display_limit(
            f32::from_be_bytes([buf[20], buf[21], buf[22], buf[23]]) as i16
        );
        self.set_upper_alarm_limit(f32::from_be_bytes([buf[24], buf[25], buf[26], buf[27]]) as i16);
        self.set_upper_warning_limit(
            f32::from_be_bytes([buf[28], buf[29], buf[30], buf[31]]) as i16
        );
        self.set_lower_warning_limit(
            f32::from_be_bytes([buf[32], buf[33], buf[34], buf[35]]) as i16
        );
        self.set_lower_alarm_limit(f32::from_be_bytes([buf[36], buf[37], buf[38], buf[39]]) as i16);
    }

    fn update_gr_double_meta_from_buf(&self, buf: &Vec<u8>) {
        if buf.len() < DBR_GR_DOUBLE_VALUE_OFFSET as usize {
            return;
        }

        let status_code = i16::from_be_bytes([buf[0], buf[1]]);
        let severity_code = i16::from_be_bytes([buf[2], buf[3]]);

        let Some(status) = ChannelStatus::from_i16(status_code) else {
            return;
        };
        let Some(severity) = ChannelSeverity::from_i16(severity_code) else {
            return;
        };

        let Some(units) = Self::fixed_c_string(&buf[8..16]) else {
            return;
        };

        self.set_status(status);
        self.set_severity(severity);
        self.set_precision(i16::from_be_bytes([buf[4], buf[5]]));
        self.set_padding(i16::from_be_bytes([buf[6], buf[7]]));
        self.set_units(units);
        self.set_upper_display_limit(f64::from_be_bytes([
            buf[16], buf[17], buf[18], buf[19], buf[20], buf[21], buf[22], buf[23],
        ]) as i16);
        self.set_lower_display_limit(f64::from_be_bytes([
            buf[24], buf[25], buf[26], buf[27], buf[28], buf[29], buf[30], buf[31],
        ]) as i16);
        self.set_upper_alarm_limit(f64::from_be_bytes([
            buf[32], buf[33], buf[34], buf[35], buf[36], buf[37], buf[38], buf[39],
        ]) as i16);
        self.set_upper_warning_limit(f64::from_be_bytes([
            buf[40], buf[41], buf[42], buf[43], buf[44], buf[45], buf[46], buf[47],
        ]) as i16);
        self.set_lower_warning_limit(f64::from_be_bytes([
            buf[48], buf[49], buf[50], buf[51], buf[52], buf[53], buf[54], buf[55],
        ]) as i16);
        self.set_lower_alarm_limit(f64::from_be_bytes([
            buf[56], buf[57], buf[58], buf[59], buf[60], buf[61], buf[62], buf[63],
        ]) as i16);
    }

    fn update_gr_enum_meta_from_buf(&self, buf: &Vec<u8>) {
        const DBR_ENUM_STRING_COUNT: usize = 16;
        const DBR_ENUM_STRING_SIZE: usize = 26;

        if buf.len() < DBR_GR_ENUM_VALUE_OFFSET as usize {
            return;
        }

        let status_code = i16::from_be_bytes([buf[0], buf[1]]);
        let severity_code = i16::from_be_bytes([buf[2], buf[3]]);

        let Some(status) = ChannelStatus::from_i16(status_code) else {
            return;
        };
        let Some(severity) = ChannelSeverity::from_i16(severity_code) else {
            return;
        };

        let number_of_string_used = i16::from_be_bytes([buf[4], buf[5]]);
        if !(0..=DBR_ENUM_STRING_COUNT as i16).contains(&number_of_string_used) {
            return;
        }

        let mut invalid_string = false;
        let strings = std::array::from_fn(|i| {
            let start = 6 + i * DBR_ENUM_STRING_SIZE;
            let end = start + DBR_ENUM_STRING_SIZE;
            match Self::fixed_c_string(&buf[start..end]) {
                Some(string) => string,
                None => {
                    invalid_string = true;
                    String::new()
                }
            }
        });
        if invalid_string {
            return;
        }

        self.set_status(status);
        self.set_severity(severity);
        self.set_number_of_string_used(number_of_string_used);
        self.set_strings(strings);
    }

    fn update_gr_char_meta_from_buf(&self, buf: &Vec<u8>) {
        if buf.len() < DBR_GR_CHAR_VALUE_OFFSET as usize {
            return;
        }

        let status_code = i16::from_be_bytes([buf[0], buf[1]]);
        let severity_code = i16::from_be_bytes([buf[2], buf[3]]);

        let Some(status) = ChannelStatus::from_i16(status_code) else {
            return;
        };
        let Some(severity) = ChannelSeverity::from_i16(severity_code) else {
            return;
        };

        let Some(units) = Self::fixed_c_string(&buf[4..12]) else {
            return;
        };

        self.set_status(status);
        self.set_severity(severity);
        self.set_units(units);
        self.set_upper_display_limit(buf[12] as i16);
        self.set_lower_display_limit(buf[13] as i16);
        self.set_upper_alarm_limit(buf[14] as i16);
        self.set_upper_warning_limit(buf[15] as i16);
        self.set_lower_warning_limit(buf[16] as i16);
        self.set_lower_alarm_limit(buf[17] as i16);
    }

    fn update_gr_long_meta_from_buf(&self, buf: &Vec<u8>) {
        if buf.len() < DBR_GR_LONG_VALUE_OFFSET as usize {
            return;
        }

        let status_code = i16::from_be_bytes([buf[0], buf[1]]);
        let severity_code = i16::from_be_bytes([buf[2], buf[3]]);

        let Some(status) = ChannelStatus::from_i16(status_code) else {
            return;
        };
        let Some(severity) = ChannelSeverity::from_i16(severity_code) else {
            return;
        };

        let Some(units) = Self::fixed_c_string(&buf[4..12]) else {
            return;
        };

        self.set_status(status);
        self.set_severity(severity);
        self.set_units(units);
        self.set_upper_display_limit(
            i32::from_be_bytes([buf[12], buf[13], buf[14], buf[15]]) as i16
        );
        self.set_lower_display_limit(
            i32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]]) as i16
        );
        self.set_upper_alarm_limit(i32::from_be_bytes([buf[20], buf[21], buf[22], buf[23]]) as i16);
        self.set_upper_warning_limit(
            i32::from_be_bytes([buf[24], buf[25], buf[26], buf[27]]) as i16
        );
        self.set_lower_warning_limit(
            i32::from_be_bytes([buf[28], buf[29], buf[30], buf[31]]) as i16
        );
        self.set_lower_alarm_limit(i32::from_be_bytes([buf[32], buf[33], buf[34], buf[35]]) as i16);
    }

    fn update_ctrl_int_meta_from_buf(&self, buf: &Vec<u8>) {
        if buf.len() < DBR_CTRL_SHORT_VALUE_OFFSET as usize {
            return;
        }

        let status_code = i16::from_be_bytes([buf[0], buf[1]]);
        let severity_code = i16::from_be_bytes([buf[2], buf[3]]);

        let Some(status) = ChannelStatus::from_i16(status_code) else {
            return;
        };
        let Some(severity) = ChannelSeverity::from_i16(severity_code) else {
            return;
        };

        let Some(units) = Self::fixed_c_string(&buf[4..12]) else {
            return;
        };

        self.set_status(status);
        self.set_severity(severity);
        self.set_units(units);
        self.set_upper_display_limit(i16::from_be_bytes([buf[12], buf[13]]));
        self.set_lower_display_limit(i16::from_be_bytes([buf[14], buf[15]]));
        self.set_upper_alarm_limit(i16::from_be_bytes([buf[16], buf[17]]));
        self.set_upper_warning_limit(i16::from_be_bytes([buf[18], buf[19]]));
        self.set_lower_warning_limit(i16::from_be_bytes([buf[20], buf[21]]));
        self.set_lower_alarm_limit(i16::from_be_bytes([buf[22], buf[23]]));
    }
    fn update_ctrl_float_meta_from_buf(&self, buf: &Vec<u8>) {
        if buf.len() < DBR_CTRL_FLOAT_VALUE_OFFSET as usize {
            return;
        }

        let status_code = i16::from_be_bytes([buf[0], buf[1]]);
        let severity_code = i16::from_be_bytes([buf[2], buf[3]]);

        let Some(status) = ChannelStatus::from_i16(status_code) else {
            return;
        };
        let Some(severity) = ChannelSeverity::from_i16(severity_code) else {
            return;
        };

        let Some(units) = Self::fixed_c_string(&buf[8..16]) else {
            return;
        };

        self.set_status(status);
        self.set_severity(severity);
        self.set_precision(i16::from_be_bytes([buf[4], buf[5]]));
        self.set_padding(i16::from_be_bytes([buf[6], buf[7]]));
        self.set_units(units);
        self.set_upper_display_limit(
            f32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]]) as i16
        );
        self.set_lower_display_limit(
            f32::from_be_bytes([buf[20], buf[21], buf[22], buf[23]]) as i16
        );
        self.set_upper_alarm_limit(f32::from_be_bytes([buf[24], buf[25], buf[26], buf[27]]) as i16);
        self.set_upper_warning_limit(
            f32::from_be_bytes([buf[28], buf[29], buf[30], buf[31]]) as i16
        );
        self.set_lower_warning_limit(
            f32::from_be_bytes([buf[32], buf[33], buf[34], buf[35]]) as i16
        );
        self.set_lower_alarm_limit(f32::from_be_bytes([buf[36], buf[37], buf[38], buf[39]]) as i16);
    }
    fn update_ctrl_double_meta_from_buf(&self, buf: &Vec<u8>) {
        if buf.len() < DBR_CTRL_DOUBLE_VALUE_OFFSET as usize {
            return;
        }

        let status_code = i16::from_be_bytes([buf[0], buf[1]]);
        let severity_code = i16::from_be_bytes([buf[2], buf[3]]);

        let Some(status) = ChannelStatus::from_i16(status_code) else {
            return;
        };
        let Some(severity) = ChannelSeverity::from_i16(severity_code) else {
            return;
        };

        let Some(units) = Self::fixed_c_string(&buf[8..16]) else {
            return;
        };

        self.set_status(status);
        self.set_severity(severity);
        self.set_precision(i16::from_be_bytes([buf[4], buf[5]]));
        self.set_padding(i16::from_be_bytes([buf[6], buf[7]]));
        self.set_units(units);
        self.set_upper_display_limit(f64::from_be_bytes([
            buf[16], buf[17], buf[18], buf[19], buf[20], buf[21], buf[22], buf[23],
        ]) as i16);
        self.set_lower_display_limit(f64::from_be_bytes([
            buf[24], buf[25], buf[26], buf[27], buf[28], buf[29], buf[30], buf[31],
        ]) as i16);
        self.set_upper_alarm_limit(f64::from_be_bytes([
            buf[32], buf[33], buf[34], buf[35], buf[36], buf[37], buf[38], buf[39],
        ]) as i16);
        self.set_upper_warning_limit(f64::from_be_bytes([
            buf[40], buf[41], buf[42], buf[43], buf[44], buf[45], buf[46], buf[47],
        ]) as i16);
        self.set_lower_warning_limit(f64::from_be_bytes([
            buf[48], buf[49], buf[50], buf[51], buf[52], buf[53], buf[54], buf[55],
        ]) as i16);
        self.set_lower_alarm_limit(f64::from_be_bytes([
            buf[56], buf[57], buf[58], buf[59], buf[60], buf[61], buf[62], buf[63],
        ]) as i16);
    }
    fn update_ctrl_long_meta_from_buf(&self, buf: &Vec<u8>) {
        if buf.len() < DBR_CTRL_LONG_VALUE_OFFSET as usize {
            return;
        }

        let status_code = i16::from_be_bytes([buf[0], buf[1]]);
        let severity_code = i16::from_be_bytes([buf[2], buf[3]]);

        let Some(status) = ChannelStatus::from_i16(status_code) else {
            return;
        };
        let Some(severity) = ChannelSeverity::from_i16(severity_code) else {
            return;
        };

        let Some(units) = Self::fixed_c_string(&buf[4..12]) else {
            return;
        };

        self.set_status(status);
        self.set_severity(severity);
        self.set_units(units);
        self.set_upper_display_limit(
            i32::from_be_bytes([buf[12], buf[13], buf[14], buf[15]]) as i16
        );
        self.set_lower_display_limit(
            i32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]]) as i16
        );
        self.set_upper_alarm_limit(i32::from_be_bytes([buf[20], buf[21], buf[22], buf[23]]) as i16);
        self.set_upper_warning_limit(
            i32::from_be_bytes([buf[24], buf[25], buf[26], buf[27]]) as i16
        );
        self.set_lower_warning_limit(
            i32::from_be_bytes([buf[28], buf[29], buf[30], buf[31]]) as i16
        );
        self.set_lower_alarm_limit(i32::from_be_bytes([buf[32], buf[33], buf[34], buf[35]]) as i16);
    }
    fn update_ctrl_char_meta_from_buf(&self, buf: &Vec<u8>) {
        if buf.len() < DBR_CTRL_CHAR_VALUE_OFFSET as usize {
            return;
        }

        let status_code = i16::from_be_bytes([buf[0], buf[1]]);
        let severity_code = i16::from_be_bytes([buf[2], buf[3]]);

        let Some(status) = ChannelStatus::from_i16(status_code) else {
            return;
        };
        let Some(severity) = ChannelSeverity::from_i16(severity_code) else {
            return;
        };

        let Some(units) = Self::fixed_c_string(&buf[4..12]) else {
            return;
        };

        self.set_status(status);
        self.set_severity(severity);
        self.set_units(units);
        self.set_upper_display_limit(buf[12] as i16);
        self.set_lower_display_limit(buf[13] as i16);
        self.set_upper_alarm_limit(buf[14] as i16);
        self.set_upper_warning_limit(buf[15] as i16);
        self.set_lower_warning_limit(buf[16] as i16);
        self.set_lower_alarm_limit(buf[17] as i16);
    }

    // ----------------- dbr type conversion -------
    pub fn dbr_type_native_to_time(self: &Self) -> DbrType {
        match self.dbr_type_native() {
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

    pub fn dbr_type_native_to_sts(self: &Self) -> DbrType {
        match self.dbr_type_native() {
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

    pub fn dbr_type_native_to_gr(self: &Self) -> DbrType {
        match self.dbr_type_native() {
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

    pub fn dbr_type_native_to_ctrl(self: &Self) -> DbrType {
        match self.dbr_type_native() {
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
