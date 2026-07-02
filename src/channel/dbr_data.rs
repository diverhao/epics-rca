use crate::channel::channel::Channel;
use crate::channel::dbr::{ChannelSeverity, ChannelStatus, DbrType, DbrValue};
use std::{fmt, sync::Arc};

pub type DbrArray<T> = Arc<Vec<T>>;

#[derive(Clone, Debug)]
pub struct PlainData<T> {
    pub value: DbrArray<T>,
}

pub type StringData = PlainData<String>;
pub type ShortData = PlainData<i16>;
pub type FloatData = PlainData<f32>;
pub type EnumData = PlainData<u16>;
pub type CharData = PlainData<u8>;
pub type LongData = PlainData<i32>;
pub type DoubleData = PlainData<f64>;
pub type PutAcktData = PlainData<u16>;
pub type PutAcksData = PlainData<u16>;
pub type ClassNameData = PlainData<String>;

#[derive(Copy, Clone, Debug)]
pub struct AlarmMeta {
    pub status: ChannelStatus,
    pub severity: ChannelSeverity,
}

#[derive(Clone, Debug)]
pub struct StsData<T> {
    pub value: DbrArray<T>,
    pub status: ChannelStatus,
    pub severity: ChannelSeverity,
}

pub type StsStringData = StsData<String>;
pub type StsShortData = StsData<i16>;
pub type StsFloatData = StsData<f32>;
pub type StsEnumData = StsData<u16>;
pub type StsCharData = StsData<u8>;
pub type StsLongData = StsData<i32>;
pub type StsDoubleData = StsData<f64>;
pub type GrStringData = StsData<String>;
pub type CtrlStringData = StsData<String>;

#[derive(Clone, Debug)]
pub struct StsAckStringData {
    pub value: DbrArray<String>,
    pub status: ChannelStatus,
    pub severity: ChannelSeverity,
    pub ackt: Option<u16>,
    pub acks: Option<u16>,
}

#[derive(Clone, Debug)]
pub struct TimeData<T> {
    pub value: DbrArray<T>,
    pub status: ChannelStatus,
    pub severity: ChannelSeverity,
    pub seconds_since_epoch: i32,
    pub nano_seconds: u32,
}

pub type TimeStringData = TimeData<String>;
pub type TimeShortData = TimeData<i16>;
pub type TimeFloatData = TimeData<f32>;
pub type TimeEnumData = TimeData<u16>;
pub type TimeCharData = TimeData<u8>;
pub type TimeLongData = TimeData<i32>;
pub type TimeDoubleData = TimeData<f64>;

#[derive(Clone, Debug)]
pub struct GrData<T> {
    pub value: DbrArray<T>,
    pub status: ChannelStatus,
    pub severity: ChannelSeverity,
    pub units: String,
    pub upper_display_limit: i16,
    pub lower_display_limit: i16,
    pub upper_alarm_limit: i16,
    pub upper_warning_limit: i16,
    pub lower_warning_limit: i16,
    pub lower_alarm_limit: i16,
}

pub type GrShortData = GrData<i16>;
pub type GrCharData = GrData<u8>;
pub type GrLongData = GrData<i32>;

#[derive(Clone, Debug)]
pub struct GrPrecisionData<T> {
    pub value: DbrArray<T>,
    pub status: ChannelStatus,
    pub severity: ChannelSeverity,
    pub precision: i16,
    pub padding: i16,
    pub units: String,
    pub upper_display_limit: i16,
    pub lower_display_limit: i16,
    pub upper_alarm_limit: i16,
    pub upper_warning_limit: i16,
    pub lower_warning_limit: i16,
    pub lower_alarm_limit: i16,
}

pub type GrFloatData = GrPrecisionData<f32>;
pub type GrDoubleData = GrPrecisionData<f64>;

#[derive(Clone, Debug)]
pub struct GrEnumData {
    pub value: DbrArray<u16>,
    pub status: ChannelStatus,
    pub severity: ChannelSeverity,
    pub number_of_string_used: i16,
    pub strings: [String; 16],
}

#[derive(Clone, Debug)]
pub struct CtrlData<T> {
    pub value: DbrArray<T>,
    pub status: ChannelStatus,
    pub severity: ChannelSeverity,
    pub units: String,
    pub upper_display_limit: i16,
    pub lower_display_limit: i16,
    pub upper_alarm_limit: i16,
    pub upper_warning_limit: i16,
    pub lower_warning_limit: i16,
    pub lower_alarm_limit: i16,
    pub upper_control_limit: Option<i16>,
    pub lower_control_limit: Option<i16>,
}

pub type CtrlShortData = CtrlData<i16>;
pub type CtrlCharData = CtrlData<u8>;
pub type CtrlLongData = CtrlData<i32>;

#[derive(Clone, Debug)]
pub struct CtrlPrecisionData<T> {
    pub value: DbrArray<T>,
    pub status: ChannelStatus,
    pub severity: ChannelSeverity,
    pub precision: i16,
    pub padding: i16,
    pub units: String,
    pub upper_display_limit: i16,
    pub lower_display_limit: i16,
    pub upper_alarm_limit: i16,
    pub upper_warning_limit: i16,
    pub lower_warning_limit: i16,
    pub lower_alarm_limit: i16,
    pub upper_control_limit: Option<i16>,
    pub lower_control_limit: Option<i16>,
}

pub type CtrlFloatData = CtrlPrecisionData<f32>;
pub type CtrlDoubleData = CtrlPrecisionData<f64>;

#[derive(Clone, Debug)]
pub struct CtrlEnumData {
    pub value: DbrArray<u16>,
    pub status: ChannelStatus,
    pub severity: ChannelSeverity,
    pub number_of_string_used: i16,
    pub strings: [String; 16],
}

#[derive(Clone, Debug)]
pub enum DbrData {
    String(StringData),
    Short(ShortData),
    Float(FloatData),
    Enum(EnumData),
    Char(CharData),
    Long(LongData),
    Double(DoubleData),
    StsString(StsStringData),
    StsShort(StsShortData),
    StsFloat(StsFloatData),
    StsEnum(StsEnumData),
    StsChar(StsCharData),
    StsLong(StsLongData),
    StsDouble(StsDoubleData),
    TimeString(TimeStringData),
    TimeShort(TimeShortData),
    TimeFloat(TimeFloatData),
    TimeEnum(TimeEnumData),
    TimeChar(TimeCharData),
    TimeLong(TimeLongData),
    TimeDouble(TimeDoubleData),
    GrString(GrStringData),
    GrShort(GrShortData),
    GrFloat(GrFloatData),
    GrEnum(GrEnumData),
    GrChar(GrCharData),
    GrLong(GrLongData),
    GrDouble(GrDoubleData),
    CtrlString(CtrlStringData),
    CtrlShort(CtrlShortData),
    CtrlFloat(CtrlFloatData),
    CtrlEnum(CtrlEnumData),
    CtrlChar(CtrlCharData),
    CtrlLong(CtrlLongData),
    CtrlDouble(CtrlDoubleData),
    PutAckt(PutAcktData),
    PutAcks(PutAcksData),
    StsAckString(StsAckStringData),
    ClassName(ClassNameData),
}

impl fmt::Display for DbrData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbrData::String(data) => fmt_plain_data(f, "String", data),
            DbrData::Short(data) => fmt_plain_data(f, "Short", data),
            DbrData::Float(data) => fmt_plain_data(f, "Float", data),
            DbrData::Enum(data) => fmt_plain_data(f, "Enum", data),
            DbrData::Char(data) => fmt_plain_data(f, "Char", data),
            DbrData::Long(data) => fmt_plain_data(f, "Long", data),
            DbrData::Double(data) => fmt_plain_data(f, "Double", data),
            DbrData::StsString(data) => fmt_sts_data(f, "StsString", data),
            DbrData::StsShort(data) => fmt_sts_data(f, "StsShort", data),
            DbrData::StsFloat(data) => fmt_sts_data(f, "StsFloat", data),
            DbrData::StsEnum(data) => fmt_sts_data(f, "StsEnum", data),
            DbrData::StsChar(data) => fmt_sts_data(f, "StsChar", data),
            DbrData::StsLong(data) => fmt_sts_data(f, "StsLong", data),
            DbrData::StsDouble(data) => fmt_sts_data(f, "StsDouble", data),
            DbrData::TimeString(data) => fmt_time_data(f, "TimeString", data),
            DbrData::TimeShort(data) => fmt_time_data(f, "TimeShort", data),
            DbrData::TimeFloat(data) => fmt_time_data(f, "TimeFloat", data),
            DbrData::TimeEnum(data) => fmt_time_data(f, "TimeEnum", data),
            DbrData::TimeChar(data) => fmt_time_data(f, "TimeChar", data),
            DbrData::TimeLong(data) => fmt_time_data(f, "TimeLong", data),
            DbrData::TimeDouble(data) => fmt_time_data(f, "TimeDouble", data),
            DbrData::GrString(data) => fmt_sts_data(f, "GrString", data),
            DbrData::GrShort(data) => fmt_gr_data(f, "GrShort", data),
            DbrData::GrFloat(data) => fmt_gr_precision_data(f, "GrFloat", data),
            DbrData::GrEnum(data) => fmt_gr_enum_data(f, "GrEnum", data),
            DbrData::GrChar(data) => fmt_gr_data(f, "GrChar", data),
            DbrData::GrLong(data) => fmt_gr_data(f, "GrLong", data),
            DbrData::GrDouble(data) => fmt_gr_precision_data(f, "GrDouble", data),
            DbrData::CtrlString(data) => fmt_sts_data(f, "CtrlString", data),
            DbrData::CtrlShort(data) => fmt_ctrl_data(f, "CtrlShort", data),
            DbrData::CtrlFloat(data) => fmt_ctrl_precision_data(f, "CtrlFloat", data),
            DbrData::CtrlEnum(data) => fmt_ctrl_enum_data(f, "CtrlEnum", data),
            DbrData::CtrlChar(data) => fmt_ctrl_data(f, "CtrlChar", data),
            DbrData::CtrlLong(data) => fmt_ctrl_data(f, "CtrlLong", data),
            DbrData::CtrlDouble(data) => fmt_ctrl_precision_data(f, "CtrlDouble", data),
            DbrData::PutAckt(data) => fmt_plain_data(f, "PutAckt", data),
            DbrData::PutAcks(data) => fmt_plain_data(f, "PutAcks", data),
            DbrData::StsAckString(data) => fmt_sts_ack_string_data(f, "StsAckString", data),
            DbrData::ClassName(data) => fmt_plain_data(f, "ClassName", data),
        }
    }
}

impl Channel {
    pub fn dbr_data(self: &Self, dbr_type: DbrType) -> Option<DbrData> {
        let value = self.value().clone()?;

        match dbr_type {
            DbrType::String => Some(DbrData::String(PlainData {
                value: into_string(value)?,
            })),
            DbrType::Short => Some(DbrData::Short(PlainData {
                value: into_short(value)?,
            })),
            DbrType::Float => Some(DbrData::Float(PlainData {
                value: into_float(value)?,
            })),
            DbrType::Enum => Some(DbrData::Enum(PlainData {
                value: into_enum(value)?,
            })),
            DbrType::Char => Some(DbrData::Char(PlainData {
                value: into_char(value)?,
            })),
            DbrType::Long => Some(DbrData::Long(PlainData {
                value: into_long(value)?,
            })),
            DbrType::Double => Some(DbrData::Double(PlainData {
                value: into_double(value)?,
            })),
            DbrType::StsString => Some(DbrData::StsString(sts_data(self, into_string(value)?))),
            DbrType::StsShort => Some(DbrData::StsShort(sts_data(self, into_short(value)?))),
            DbrType::StsFloat => Some(DbrData::StsFloat(sts_data(self, into_float(value)?))),
            DbrType::StsEnum => Some(DbrData::StsEnum(sts_data(self, into_enum(value)?))),
            DbrType::StsChar => Some(DbrData::StsChar(sts_data(self, into_char(value)?))),
            DbrType::StsLong => Some(DbrData::StsLong(sts_data(self, into_long(value)?))),
            DbrType::StsDouble => Some(DbrData::StsDouble(sts_data(self, into_double(value)?))),
            DbrType::TimeString => Some(DbrData::TimeString(time_data(self, into_string(value)?))),
            DbrType::TimeShort => Some(DbrData::TimeShort(time_data(self, into_short(value)?))),
            DbrType::TimeFloat => Some(DbrData::TimeFloat(time_data(self, into_float(value)?))),
            DbrType::TimeEnum => Some(DbrData::TimeEnum(time_data(self, into_enum(value)?))),
            DbrType::TimeChar => Some(DbrData::TimeChar(time_data(self, into_char(value)?))),
            DbrType::TimeLong => Some(DbrData::TimeLong(time_data(self, into_long(value)?))),
            DbrType::TimeDouble => Some(DbrData::TimeDouble(time_data(self, into_double(value)?))),
            DbrType::GrString => Some(DbrData::GrString(sts_data(self, into_string(value)?))),
            DbrType::GrShort => Some(DbrData::GrShort(gr_data(self, into_short(value)?))),
            DbrType::GrFloat => Some(DbrData::GrFloat(gr_precision_data(
                self,
                into_float(value)?,
            ))),
            DbrType::GrEnum => Some(DbrData::GrEnum(gr_enum_data(self, into_enum(value)?))),
            DbrType::GrChar => Some(DbrData::GrChar(gr_data(self, into_char(value)?))),
            DbrType::GrLong => Some(DbrData::GrLong(gr_data(self, into_long(value)?))),
            DbrType::GrDouble => Some(DbrData::GrDouble(gr_precision_data(
                self,
                into_double(value)?,
            ))),
            DbrType::CtrlString => Some(DbrData::CtrlString(sts_data(self, into_string(value)?))),
            DbrType::CtrlShort => Some(DbrData::CtrlShort(ctrl_data(self, into_short(value)?))),
            DbrType::CtrlFloat => Some(DbrData::CtrlFloat(ctrl_precision_data(
                self,
                into_float(value)?,
            ))),
            DbrType::CtrlEnum => Some(DbrData::CtrlEnum(ctrl_enum_data(self, into_enum(value)?))),
            DbrType::CtrlChar => Some(DbrData::CtrlChar(ctrl_data(self, into_char(value)?))),
            DbrType::CtrlLong => Some(DbrData::CtrlLong(ctrl_data(self, into_long(value)?))),
            DbrType::CtrlDouble => Some(DbrData::CtrlDouble(ctrl_precision_data(
                self,
                into_double(value)?,
            ))),
            DbrType::PutAckt => Some(DbrData::PutAckt(PlainData {
                value: into_enum(value)?,
            })),
            DbrType::PutAcks => Some(DbrData::PutAcks(PlainData {
                value: into_enum(value)?,
            })),
            DbrType::StsAckString => Some(DbrData::StsAckString(sts_ack_string_data(
                self,
                into_string(value)?,
            ))),
            DbrType::ClassName => Some(DbrData::ClassName(PlainData {
                value: into_string(value)?,
            })),
        }
    }
}

const DISPLAY_LIMIT: usize = 100;

fn fmt_array<T: fmt::Debug>(f: &mut fmt::Formatter<'_>, values: &DbrArray<T>) -> fmt::Result {
    f.write_str("[")?;

    let shown = values.len().min(DISPLAY_LIMIT);
    for (i, value) in values.iter().take(DISPLAY_LIMIT).enumerate() {
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

    f.write_str("]")
}

fn fmt_plain_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &PlainData<T>,
) -> fmt::Result {
    write!(f, "{name} {{ value: ")?;
    fmt_array(f, &data.value)?;
    f.write_str(" }")
}

fn fmt_sts_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &StsData<T>,
) -> fmt::Result {
    write!(
        f,
        "{name} {{ status: {:?}, severity: {:?}, value: ",
        data.status, data.severity
    )?;
    fmt_array(f, &data.value)?;
    f.write_str(" }")
}

fn fmt_sts_ack_string_data(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &StsAckStringData,
) -> fmt::Result {
    write!(
        f,
        "{name} {{ status: {:?}, severity: {:?}, ackt: {:?}, acks: {:?}, value: ",
        data.status, data.severity, data.ackt, data.acks
    )?;
    fmt_array(f, &data.value)?;
    f.write_str(" }")
}

fn fmt_time_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &TimeData<T>,
) -> fmt::Result {
    write!(
        f,
        "{name} {{ status: {:?}, severity: {:?}, seconds_since_epoch: {}, nano_seconds: {}, value: ",
        data.status, data.severity, data.seconds_since_epoch, data.nano_seconds
    )?;
    fmt_array(f, &data.value)?;
    f.write_str(" }")
}

fn fmt_gr_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &GrData<T>,
) -> fmt::Result {
    write!(
        f,
        "{name} {{ status: {:?}, severity: {:?}, units: {:?}, upper_display_limit: {}, lower_display_limit: {}, upper_alarm_limit: {}, upper_warning_limit: {}, lower_warning_limit: {}, lower_alarm_limit: {}, value: ",
        data.status,
        data.severity,
        data.units,
        data.upper_display_limit,
        data.lower_display_limit,
        data.upper_alarm_limit,
        data.upper_warning_limit,
        data.lower_warning_limit,
        data.lower_alarm_limit
    )?;
    fmt_array(f, &data.value)?;
    f.write_str(" }")
}

fn fmt_gr_precision_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &GrPrecisionData<T>,
) -> fmt::Result {
    write!(
        f,
        "{name} {{ status: {:?}, severity: {:?}, precision: {}, padding: {}, units: {:?}, upper_display_limit: {}, lower_display_limit: {}, upper_alarm_limit: {}, upper_warning_limit: {}, lower_warning_limit: {}, lower_alarm_limit: {}, value: ",
        data.status,
        data.severity,
        data.precision,
        data.padding,
        data.units,
        data.upper_display_limit,
        data.lower_display_limit,
        data.upper_alarm_limit,
        data.upper_warning_limit,
        data.lower_warning_limit,
        data.lower_alarm_limit
    )?;
    fmt_array(f, &data.value)?;
    f.write_str(" }")
}

fn fmt_gr_enum_data(f: &mut fmt::Formatter<'_>, name: &str, data: &GrEnumData) -> fmt::Result {
    write!(
        f,
        "{name} {{ status: {:?}, severity: {:?}, number_of_string_used: {}, strings: {:?}, value: ",
        data.status, data.severity, data.number_of_string_used, data.strings
    )?;
    fmt_array(f, &data.value)?;
    f.write_str(" }")
}

fn fmt_ctrl_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &CtrlData<T>,
) -> fmt::Result {
    write!(
        f,
        "{name} {{ status: {:?}, severity: {:?}, units: {:?}, upper_display_limit: {}, lower_display_limit: {}, upper_alarm_limit: {}, upper_warning_limit: {}, lower_warning_limit: {}, lower_alarm_limit: {}, upper_control_limit: {:?}, lower_control_limit: {:?}, value: ",
        data.status,
        data.severity,
        data.units,
        data.upper_display_limit,
        data.lower_display_limit,
        data.upper_alarm_limit,
        data.upper_warning_limit,
        data.lower_warning_limit,
        data.lower_alarm_limit,
        data.upper_control_limit,
        data.lower_control_limit
    )?;
    fmt_array(f, &data.value)?;
    f.write_str(" }")
}

fn fmt_ctrl_precision_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &CtrlPrecisionData<T>,
) -> fmt::Result {
    write!(
        f,
        "{name} {{ status: {:?}, severity: {:?}, precision: {}, padding: {}, units: {:?}, upper_display_limit: {}, lower_display_limit: {}, upper_alarm_limit: {}, upper_warning_limit: {}, lower_warning_limit: {}, lower_alarm_limit: {}, upper_control_limit: {:?}, lower_control_limit: {:?}, value: ",
        data.status,
        data.severity,
        data.precision,
        data.padding,
        data.units,
        data.upper_display_limit,
        data.lower_display_limit,
        data.upper_alarm_limit,
        data.upper_warning_limit,
        data.lower_warning_limit,
        data.lower_alarm_limit,
        data.upper_control_limit,
        data.lower_control_limit
    )?;
    fmt_array(f, &data.value)?;
    f.write_str(" }")
}

fn fmt_ctrl_enum_data(f: &mut fmt::Formatter<'_>, name: &str, data: &CtrlEnumData) -> fmt::Result {
    write!(
        f,
        "{name} {{ status: {:?}, severity: {:?}, number_of_string_used: {}, strings: {:?}, value: ",
        data.status, data.severity, data.number_of_string_used, data.strings
    )?;
    fmt_array(f, &data.value)?;
    f.write_str(" }")
}

fn sts_data<T>(channel: &Channel, value: DbrArray<T>) -> StsData<T> {
    StsData {
        value,
        status: channel.status(),
        severity: channel.severity(),
    }
}

fn sts_ack_string_data(channel: &Channel, value: DbrArray<String>) -> StsAckStringData {
    StsAckStringData {
        value,
        status: channel.status(),
        severity: channel.severity(),
        ackt: None,
        acks: None,
    }
}

fn time_data<T>(channel: &Channel, value: DbrArray<T>) -> TimeData<T> {
    TimeData {
        value,
        status: channel.status(),
        severity: channel.severity(),
        seconds_since_epoch: channel.seconds_since_epoch(),
        nano_seconds: channel.nano_seconds(),
    }
}

fn gr_data<T>(channel: &Channel, value: DbrArray<T>) -> GrData<T> {
    GrData {
        value,
        status: channel.status(),
        severity: channel.severity(),
        units: channel.units(),
        upper_display_limit: channel.upper_display_limit(),
        lower_display_limit: channel.lower_display_limit(),
        upper_alarm_limit: channel.upper_alarm_limit(),
        upper_warning_limit: channel.upper_warning_limit(),
        lower_warning_limit: channel.lower_warning_limit(),
        lower_alarm_limit: channel.lower_alarm_limit(),
    }
}

fn gr_precision_data<T>(channel: &Channel, value: DbrArray<T>) -> GrPrecisionData<T> {
    GrPrecisionData {
        value,
        status: channel.status(),
        severity: channel.severity(),
        precision: channel.precision(),
        padding: channel.padding(),
        units: channel.units(),
        upper_display_limit: channel.upper_display_limit(),
        lower_display_limit: channel.lower_display_limit(),
        upper_alarm_limit: channel.upper_alarm_limit(),
        upper_warning_limit: channel.upper_warning_limit(),
        lower_warning_limit: channel.lower_warning_limit(),
        lower_alarm_limit: channel.lower_alarm_limit(),
    }
}

fn gr_enum_data(channel: &Channel, value: DbrArray<u16>) -> GrEnumData {
    GrEnumData {
        value,
        status: channel.status(),
        severity: channel.severity(),
        number_of_string_used: channel.number_of_string_used(),
        strings: channel.strings(),
    }
}

fn ctrl_data<T>(channel: &Channel, value: DbrArray<T>) -> CtrlData<T> {
    CtrlData {
        value,
        status: channel.status(),
        severity: channel.severity(),
        units: channel.units(),
        upper_display_limit: channel.upper_display_limit(),
        lower_display_limit: channel.lower_display_limit(),
        upper_alarm_limit: channel.upper_alarm_limit(),
        upper_warning_limit: channel.upper_warning_limit(),
        lower_warning_limit: channel.lower_warning_limit(),
        lower_alarm_limit: channel.lower_alarm_limit(),
        upper_control_limit: None,
        lower_control_limit: None,
    }
}

fn ctrl_precision_data<T>(channel: &Channel, value: DbrArray<T>) -> CtrlPrecisionData<T> {
    CtrlPrecisionData {
        value,
        status: channel.status(),
        severity: channel.severity(),
        precision: channel.precision(),
        padding: channel.padding(),
        units: channel.units(),
        upper_display_limit: channel.upper_display_limit(),
        lower_display_limit: channel.lower_display_limit(),
        upper_alarm_limit: channel.upper_alarm_limit(),
        upper_warning_limit: channel.upper_warning_limit(),
        lower_warning_limit: channel.lower_warning_limit(),
        lower_alarm_limit: channel.lower_alarm_limit(),
        upper_control_limit: None,
        lower_control_limit: None,
    }
}

fn ctrl_enum_data(channel: &Channel, value: DbrArray<u16>) -> CtrlEnumData {
    CtrlEnumData {
        value,
        status: channel.status(),
        severity: channel.severity(),
        number_of_string_used: channel.number_of_string_used(),
        strings: channel.strings(),
    }
}

fn into_string(value: DbrValue) -> Option<DbrArray<String>> {
    match value {
        DbrValue::String(value) => Some(value),
        _ => None,
    }
}

fn into_short(value: DbrValue) -> Option<DbrArray<i16>> {
    match value {
        DbrValue::Short(value) => Some(value),
        _ => None,
    }
}

fn into_float(value: DbrValue) -> Option<DbrArray<f32>> {
    match value {
        DbrValue::Float(value) => Some(value),
        _ => None,
    }
}

fn into_enum(value: DbrValue) -> Option<DbrArray<u16>> {
    match value {
        DbrValue::Enum(value) => Some(value),
        _ => None,
    }
}

fn into_char(value: DbrValue) -> Option<DbrArray<u8>> {
    match value {
        DbrValue::Char(value) => Some(value),
        _ => None,
    }
}

fn into_long(value: DbrValue) -> Option<DbrArray<i32>> {
    match value {
        DbrValue::Long(value) => Some(value),
        _ => None,
    }
}

fn into_double(value: DbrValue) -> Option<DbrArray<f64>> {
    match value {
        DbrValue::Double(value) => Some(value),
        _ => None,
    }
}
