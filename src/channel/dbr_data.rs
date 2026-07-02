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
            DbrData::String(data) => fmt_plain_data(f, "DBR_STRING", data),
            DbrData::Short(data) => fmt_plain_data(f, "DBR_SHORT", data),
            DbrData::Float(data) => fmt_plain_data(f, "DBR_FLOAT", data),
            DbrData::Enum(data) => fmt_plain_data(f, "DBR_ENUM", data),
            DbrData::Char(data) => fmt_plain_data(f, "DBR_CHAR", data),
            DbrData::Long(data) => fmt_plain_data(f, "DBR_LONG", data),
            DbrData::Double(data) => fmt_plain_data(f, "DBR_DOUBLE", data),
            DbrData::StsString(data) => fmt_sts_data(f, "DBR_STS_STRING", data),
            DbrData::StsShort(data) => fmt_sts_data(f, "DBR_STS_SHORT", data),
            DbrData::StsFloat(data) => fmt_sts_data(f, "DBR_STS_FLOAT", data),
            DbrData::StsEnum(data) => fmt_sts_data(f, "DBR_STS_ENUM", data),
            DbrData::StsChar(data) => fmt_sts_data(f, "DBR_STS_CHAR", data),
            DbrData::StsLong(data) => fmt_sts_data(f, "DBR_STS_LONG", data),
            DbrData::StsDouble(data) => fmt_sts_data(f, "DBR_STS_DOUBLE", data),
            DbrData::TimeString(data) => fmt_time_data(f, "DBR_TIME_STRING", data),
            DbrData::TimeShort(data) => fmt_time_data(f, "DBR_TIME_SHORT", data),
            DbrData::TimeFloat(data) => fmt_time_data(f, "DBR_TIME_FLOAT", data),
            DbrData::TimeEnum(data) => fmt_time_data(f, "DBR_TIME_ENUM", data),
            DbrData::TimeChar(data) => fmt_time_data(f, "DBR_TIME_CHAR", data),
            DbrData::TimeLong(data) => fmt_time_data(f, "DBR_TIME_LONG", data),
            DbrData::TimeDouble(data) => fmt_time_data(f, "DBR_TIME_DOUBLE", data),
            DbrData::GrString(data) => fmt_sts_data(f, "DBR_GR_STRING", data),
            DbrData::GrShort(data) => fmt_gr_data(f, "DBR_GR_SHORT", data),
            DbrData::GrFloat(data) => fmt_gr_precision_data(f, "DBR_GR_FLOAT", data),
            DbrData::GrEnum(data) => fmt_gr_enum_data(f, "DBR_GR_ENUM", data),
            DbrData::GrChar(data) => fmt_gr_data(f, "DBR_GR_CHAR", data),
            DbrData::GrLong(data) => fmt_gr_data(f, "DBR_GR_LONG", data),
            DbrData::GrDouble(data) => fmt_gr_precision_data(f, "DBR_GR_DOUBLE", data),
            DbrData::CtrlString(data) => fmt_sts_data(f, "DBR_CTRL_STRING", data),
            DbrData::CtrlShort(data) => fmt_ctrl_data(f, "DBR_CTRL_SHORT", data),
            DbrData::CtrlFloat(data) => fmt_ctrl_precision_data(f, "DBR_CTRL_FLOAT", data),
            DbrData::CtrlEnum(data) => fmt_ctrl_enum_data(f, "DBR_CTRL_ENUM", data),
            DbrData::CtrlChar(data) => fmt_ctrl_data(f, "DBR_CTRL_CHAR", data),
            DbrData::CtrlLong(data) => fmt_ctrl_data(f, "DBR_CTRL_LONG", data),
            DbrData::CtrlDouble(data) => fmt_ctrl_precision_data(f, "DBR_CTRL_DOUBLE", data),
            DbrData::PutAckt(data) => fmt_plain_data(f, "DBR_PUT_ACKT", data),
            DbrData::PutAcks(data) => fmt_plain_data(f, "DBR_PUT_ACKS", data),
            DbrData::StsAckString(data) => fmt_sts_ack_string_data(f, "DBR_STSACK_STRING", data),
            DbrData::ClassName(data) => fmt_plain_data(f, "DBR_CLASS_NAME", data),
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

fn fmt_indent(f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
    for _ in 0..level {
        f.write_str("  ")?;
    }
    Ok(())
}

fn fmt_begin_object(f: &mut fmt::Formatter<'_>, name: &str) -> fmt::Result {
    writeln!(f, "{{")?;
    fmt_field_debug(f, 1, "type", &name, true)
}

fn fmt_end_object(f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("}")
}

fn fmt_comma_newline(f: &mut fmt::Formatter<'_>, comma: bool) -> fmt::Result {
    if comma { writeln!(f, ",") } else { writeln!(f) }
}

fn fmt_field_debug<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    level: usize,
    key: &str,
    value: &T,
    comma: bool,
) -> fmt::Result {
    fmt_indent(f, level)?;
    write!(f, "\"{key}\": {value:?}")?;
    fmt_comma_newline(f, comma)
}

fn fmt_field_display<T: fmt::Display>(
    f: &mut fmt::Formatter<'_>,
    level: usize,
    key: &str,
    value: T,
    comma: bool,
) -> fmt::Result {
    fmt_indent(f, level)?;
    write!(f, "\"{key}\": {value}")?;
    fmt_comma_newline(f, comma)
}

fn fmt_field_option_i16(
    f: &mut fmt::Formatter<'_>,
    level: usize,
    key: &str,
    value: Option<i16>,
    comma: bool,
) -> fmt::Result {
    fmt_indent(f, level)?;
    write!(f, "\"{key}\": ")?;
    match value {
        Some(value) => write!(f, "{value}")?,
        None => f.write_str("null")?,
    }
    fmt_comma_newline(f, comma)
}

fn fmt_array<T: fmt::Debug>(f: &mut fmt::Formatter<'_>, values: &[T], level: usize) -> fmt::Result {
    if values.is_empty() {
        return f.write_str("[]");
    }

    writeln!(f, "[")?;

    let shown = values.len().min(DISPLAY_LIMIT);
    let has_more = values.len() > shown;
    for (i, value) in values.iter().take(DISPLAY_LIMIT).enumerate() {
        fmt_indent(f, level + 1)?;
        write!(f, "{value:?}")?;
        if i + 1 < shown || has_more {
            writeln!(f, ",")?;
        } else {
            writeln!(f)?;
        }
    }

    if has_more {
        fmt_indent(f, level + 1)?;
        writeln!(f, "\"... ({} more)\"", values.len() - shown)?;
    }

    fmt_indent(f, level)?;
    f.write_str("]")
}

fn fmt_field_array<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    level: usize,
    key: &str,
    values: &[T],
    comma: bool,
) -> fmt::Result {
    fmt_indent(f, level)?;
    write!(f, "\"{key}\": ")?;
    fmt_array(f, values, level)?;
    fmt_comma_newline(f, comma)
}

fn fmt_status_severity(
    f: &mut fmt::Formatter<'_>,
    status: ChannelStatus,
    severity: ChannelSeverity,
) -> fmt::Result {
    fmt_field_debug(f, 1, "status", &channel_status_name(status), true)?;
    fmt_field_debug(f, 1, "severity", &channel_severity_name(severity), true)
}

fn channel_severity_name(severity: ChannelSeverity) -> &'static str {
    match severity {
        ChannelSeverity::NoAlarm => "NO_ALARM",
        ChannelSeverity::Minor => "MINOR",
        ChannelSeverity::Major => "MAJOR",
        ChannelSeverity::Invalid => "INVALID",
    }
}

fn channel_status_name(status: ChannelStatus) -> &'static str {
    match status {
        ChannelStatus::NoAlarm => "NO_ALARM",
        ChannelStatus::Read => "READ",
        ChannelStatus::Write => "WRITE",
        ChannelStatus::Hihi => "HIHI",
        ChannelStatus::High => "HIGH",
        ChannelStatus::Lolo => "LOLO",
        ChannelStatus::Low => "LOW",
        ChannelStatus::State => "STATE",
        ChannelStatus::Cos => "COS",
        ChannelStatus::Comm => "COMM",
        ChannelStatus::Timeout => "TIMEOUT",
        ChannelStatus::HwLimit => "HWLIMIT",
        ChannelStatus::Calc => "CALC",
        ChannelStatus::Scan => "SCAN",
        ChannelStatus::Link => "LINK",
        ChannelStatus::Soft => "SOFT",
        ChannelStatus::BadSub => "BAD_SUB",
        ChannelStatus::Udf => "UDF",
        ChannelStatus::Disable => "DISABLE",
        ChannelStatus::Simm => "SIMM",
        ChannelStatus::ReadAccess => "READ_ACCESS",
        ChannelStatus::WriteAccess => "WRITE_ACCESS",
    }
}

fn fmt_plain_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &PlainData<T>,
) -> fmt::Result {
    fmt_begin_object(f, name)?;
    fmt_field_array(f, 1, "value", data.value.as_slice(), false)?;
    fmt_end_object(f)
}

fn fmt_sts_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &StsData<T>,
) -> fmt::Result {
    fmt_begin_object(f, name)?;
    fmt_status_severity(f, data.status, data.severity)?;
    fmt_field_array(f, 1, "value", data.value.as_slice(), false)?;
    fmt_end_object(f)
}

fn fmt_sts_ack_string_data(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &StsAckStringData,
) -> fmt::Result {
    fmt_begin_object(f, name)?;
    fmt_status_severity(f, data.status, data.severity)?;
    fmt_field_option_i16(f, 1, "ackt", data.ackt.map(|value| value as i16), true)?;
    fmt_field_option_i16(f, 1, "acks", data.acks.map(|value| value as i16), true)?;
    fmt_field_array(f, 1, "value", data.value.as_slice(), false)?;
    fmt_end_object(f)
}

fn fmt_time_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &TimeData<T>,
) -> fmt::Result {
    fmt_begin_object(f, name)?;
    fmt_status_severity(f, data.status, data.severity)?;
    fmt_field_display(f, 1, "seconds_since_epoch", data.seconds_since_epoch, true)?;
    fmt_field_display(f, 1, "nano_seconds", data.nano_seconds, true)?;
    fmt_field_debug(
        f,
        1,
        "time_utc",
        &unix_time_to_utc_string(data.seconds_since_epoch, data.nano_seconds),
        true,
    )?;
    fmt_field_array(f, 1, "value", data.value.as_slice(), false)?;
    fmt_end_object(f)
}

fn unix_time_to_utc_string(seconds_since_epoch: i32, nano_seconds: u32) -> String {
    let seconds_since_epoch = seconds_since_epoch as i64;
    let days = seconds_since_epoch.div_euclid(86_400);
    let seconds_of_day = seconds_since_epoch.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = seconds_of_day % 3_600 / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{nano_seconds:09}Z")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i64, u32, u32) {
    let days = days_since_unix_epoch + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    (year, month as u32, day as u32)
}

fn fmt_gr_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &GrData<T>,
) -> fmt::Result {
    fmt_begin_object(f, name)?;
    fmt_status_severity(f, data.status, data.severity)?;
    fmt_field_debug(f, 1, "units", &data.units, true)?;
    fmt_field_display(f, 1, "upper_display_limit", data.upper_display_limit, true)?;
    fmt_field_display(f, 1, "lower_display_limit", data.lower_display_limit, true)?;
    fmt_field_display(f, 1, "upper_alarm_limit", data.upper_alarm_limit, true)?;
    fmt_field_display(f, 1, "upper_warning_limit", data.upper_warning_limit, true)?;
    fmt_field_display(f, 1, "lower_warning_limit", data.lower_warning_limit, true)?;
    fmt_field_display(f, 1, "lower_alarm_limit", data.lower_alarm_limit, true)?;
    fmt_field_array(f, 1, "value", data.value.as_slice(), false)?;
    fmt_end_object(f)
}

fn fmt_gr_precision_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &GrPrecisionData<T>,
) -> fmt::Result {
    fmt_begin_object(f, name)?;
    fmt_status_severity(f, data.status, data.severity)?;
    fmt_field_display(f, 1, "precision", data.precision, true)?;
    fmt_field_display(f, 1, "padding", data.padding, true)?;
    fmt_field_debug(f, 1, "units", &data.units, true)?;
    fmt_field_display(f, 1, "upper_display_limit", data.upper_display_limit, true)?;
    fmt_field_display(f, 1, "lower_display_limit", data.lower_display_limit, true)?;
    fmt_field_display(f, 1, "upper_alarm_limit", data.upper_alarm_limit, true)?;
    fmt_field_display(f, 1, "upper_warning_limit", data.upper_warning_limit, true)?;
    fmt_field_display(f, 1, "lower_warning_limit", data.lower_warning_limit, true)?;
    fmt_field_display(f, 1, "lower_alarm_limit", data.lower_alarm_limit, true)?;
    fmt_field_array(f, 1, "value", data.value.as_slice(), false)?;
    fmt_end_object(f)
}

fn fmt_gr_enum_data(f: &mut fmt::Formatter<'_>, name: &str, data: &GrEnumData) -> fmt::Result {
    fmt_begin_object(f, name)?;
    fmt_status_severity(f, data.status, data.severity)?;
    fmt_field_display(
        f,
        1,
        "number_of_string_used",
        data.number_of_string_used,
        true,
    )?;
    fmt_field_array(f, 1, "strings", &data.strings, true)?;
    fmt_field_array(f, 1, "value", data.value.as_slice(), false)?;
    fmt_end_object(f)
}

fn fmt_ctrl_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &CtrlData<T>,
) -> fmt::Result {
    fmt_begin_object(f, name)?;
    fmt_status_severity(f, data.status, data.severity)?;
    fmt_field_debug(f, 1, "units", &data.units, true)?;
    fmt_field_display(f, 1, "upper_display_limit", data.upper_display_limit, true)?;
    fmt_field_display(f, 1, "lower_display_limit", data.lower_display_limit, true)?;
    fmt_field_display(f, 1, "upper_alarm_limit", data.upper_alarm_limit, true)?;
    fmt_field_display(f, 1, "upper_warning_limit", data.upper_warning_limit, true)?;
    fmt_field_display(f, 1, "lower_warning_limit", data.lower_warning_limit, true)?;
    fmt_field_display(f, 1, "lower_alarm_limit", data.lower_alarm_limit, true)?;
    fmt_field_option_i16(f, 1, "upper_control_limit", data.upper_control_limit, true)?;
    fmt_field_option_i16(f, 1, "lower_control_limit", data.lower_control_limit, true)?;
    fmt_field_array(f, 1, "value", data.value.as_slice(), false)?;
    fmt_end_object(f)
}

fn fmt_ctrl_precision_data<T: fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    data: &CtrlPrecisionData<T>,
) -> fmt::Result {
    fmt_begin_object(f, name)?;
    fmt_status_severity(f, data.status, data.severity)?;
    fmt_field_display(f, 1, "precision", data.precision, true)?;
    fmt_field_display(f, 1, "padding", data.padding, true)?;
    fmt_field_debug(f, 1, "units", &data.units, true)?;
    fmt_field_display(f, 1, "upper_display_limit", data.upper_display_limit, true)?;
    fmt_field_display(f, 1, "lower_display_limit", data.lower_display_limit, true)?;
    fmt_field_display(f, 1, "upper_alarm_limit", data.upper_alarm_limit, true)?;
    fmt_field_display(f, 1, "upper_warning_limit", data.upper_warning_limit, true)?;
    fmt_field_display(f, 1, "lower_warning_limit", data.lower_warning_limit, true)?;
    fmt_field_display(f, 1, "lower_alarm_limit", data.lower_alarm_limit, true)?;
    fmt_field_option_i16(f, 1, "upper_control_limit", data.upper_control_limit, true)?;
    fmt_field_option_i16(f, 1, "lower_control_limit", data.lower_control_limit, true)?;
    fmt_field_array(f, 1, "value", data.value.as_slice(), false)?;
    fmt_end_object(f)
}

fn fmt_ctrl_enum_data(f: &mut fmt::Formatter<'_>, name: &str, data: &CtrlEnumData) -> fmt::Result {
    fmt_begin_object(f, name)?;
    fmt_status_severity(f, data.status, data.severity)?;
    fmt_field_display(
        f,
        1,
        "number_of_string_used",
        data.number_of_string_used,
        true,
    )?;
    fmt_field_array(f, 1, "strings", &data.strings, true)?;
    fmt_field_array(f, 1, "value", data.value.as_slice(), false)?;
    fmt_end_object(f)
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
