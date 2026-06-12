
#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum CaCmd {
    // Commands (TCP and UDP)
    CaProtoVersion = 0x0000,
    CaProtoSearch = 0x0006,
    CaProtoNotFound = 0x000e,
    CaProtoEcho = 0x0017,
    // Commands (UDP)
    CaProtoRsrvIsUp = 0x000d,
    CaRepeaterConfirm = 0x0011,
    CaRepeaterRegister = 0x0018,
    // Commands (TCP)
    CaProtoEventAdd = 0x0001,
    CaProtoEventCancel = 0x0002,
    CaProtoRead = 0x0003,
    CaProtoWrite = 0x0004,
    CaProtoSnapshot = 0x0005,
    CaProtoBuild = 0x0007,
    CaProtoEventsOff = 0x0008,
    CaProtoEventsOn = 0x0009,
    CaProtoReadSync = 0x000a,
    CaProtoError = 0x000b,
    CaProtoClearChannel = 0x000c,
    CaProtoReadNotify = 0x000f,
    CaProtoReadBuild = 0x0010,
    CaProtoCreateChan = 0x0012,
    CaProtoWriteNotify = 0x0013,
    CaProtoClientName = 0x0014,
    CaProtoHostName = 0x0015,
    CaProtoAccessRights = 0x0016,
    CaProtoSignal = 0x0019,
    CaProtoCreateChFail = 0x001a,
    CaProtoServerDisconn = 0x001b,
}

impl std::fmt::Display for CaCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CaProtoVersion => "CA_PROTO_VERSION",
            Self::CaProtoSearch => "CA_PROTO_SEARCH",
            Self::CaProtoNotFound => "CA_PROTO_NOT_FOUND",
            Self::CaProtoEcho => "CA_PROTO_ECHO",
            Self::CaProtoRsrvIsUp => "CA_PROTO_RSRV_IS_UP",
            Self::CaRepeaterConfirm => "CA_REPEATER_CONFIRM",
            Self::CaRepeaterRegister => "CA_REPEATER_REGISTER",
            Self::CaProtoEventAdd => "CA_PROTO_EVENT_ADD",
            Self::CaProtoEventCancel => "CA_PROTO_EVENT_CANCEL",
            Self::CaProtoRead => "CA_PROTO_READ",
            Self::CaProtoWrite => "CA_PROTO_WRITE",
            Self::CaProtoSnapshot => "CA_PROTO_SNAPSHOT",
            Self::CaProtoBuild => "CA_PROTO_BUILD",
            Self::CaProtoEventsOff => "CA_PROTO_EVENTS_OFF",
            Self::CaProtoEventsOn => "CA_PROTO_EVENTS_ON",
            Self::CaProtoReadSync => "CA_PROTO_READ_SYNC",
            Self::CaProtoError => "CA_PROTO_ERROR",
            Self::CaProtoClearChannel => "CA_PROTO_CLEAR_CHANNEL",
            Self::CaProtoReadNotify => "CA_PROTO_READ_NOTIFY",
            Self::CaProtoReadBuild => "CA_PROTO_READ_BUILD",
            Self::CaProtoCreateChan => "CA_PROTO_CREATE_CHAN",
            Self::CaProtoWriteNotify => "CA_PROTO_WRITE_NOTIFY",
            Self::CaProtoClientName => "CA_PROTO_CLIENT_NAME",
            Self::CaProtoHostName => "CA_PROTO_HOST_NAME",
            Self::CaProtoAccessRights => "CA_PROTO_ACCESS_RIGHTS",
            Self::CaProtoSignal => "CA_PROTO_SIGNAL",
            Self::CaProtoCreateChFail => "CA_PROTO_CREATE_CH_FAIL",
            Self::CaProtoServerDisconn => "CA_PROTO_SERVER_DISCONN",
        })
    }
}

impl TryFrom<u16> for CaCmd {
    type Error = u16;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0x0000 => Ok(CaCmd::CaProtoVersion),
            0x0001 => Ok(CaCmd::CaProtoEventAdd),
            0x0002 => Ok(CaCmd::CaProtoEventCancel),
            0x0003 => Ok(CaCmd::CaProtoRead),
            0x0004 => Ok(CaCmd::CaProtoWrite),
            0x0005 => Ok(CaCmd::CaProtoSnapshot),
            0x0006 => Ok(CaCmd::CaProtoSearch),
            0x0007 => Ok(CaCmd::CaProtoBuild),
            0x0008 => Ok(CaCmd::CaProtoEventsOff),
            0x0009 => Ok(CaCmd::CaProtoEventsOn),
            0x000a => Ok(CaCmd::CaProtoReadSync),
            0x000b => Ok(CaCmd::CaProtoError),
            0x000c => Ok(CaCmd::CaProtoClearChannel),
            0x000d => Ok(CaCmd::CaProtoRsrvIsUp),
            0x000e => Ok(CaCmd::CaProtoNotFound),
            0x000f => Ok(CaCmd::CaProtoReadNotify),
            0x0010 => Ok(CaCmd::CaProtoReadBuild),
            0x0011 => Ok(CaCmd::CaRepeaterConfirm),
            0x0012 => Ok(CaCmd::CaProtoCreateChan),
            0x0013 => Ok(CaCmd::CaProtoWriteNotify),
            0x0014 => Ok(CaCmd::CaProtoClientName),
            0x0015 => Ok(CaCmd::CaProtoHostName),
            0x0016 => Ok(CaCmd::CaProtoAccessRights),
            0x0017 => Ok(CaCmd::CaProtoEcho),
            0x0018 => Ok(CaCmd::CaRepeaterRegister),
            0x0019 => Ok(CaCmd::CaProtoSignal),
            0x001a => Ok(CaCmd::CaProtoCreateChFail),
            0x001b => Ok(CaCmd::CaProtoServerDisconn),
            unknown => Err(unknown),
        }
    }
}
