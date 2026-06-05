#[derive(Debug)]
pub enum ChannelState {
    NeverConnected,
    Connected,
    Disconnected,
    Destroyed,
}

pub enum ChannelAccessRights {
    None,        // Neither read nor write
    Read,        // Read access only
    ReadWrite    // Both read and write
}

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

// severity

// status



pub struct Channel {
    pub name: String,
    pub state: ChannelState,
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Channel {{ name: {}, state: {:?} }}", self.name, self.state)
    }
}

impl Channel {
    pub fn new(name: &str) -> Self {
        let channel = Channel {
            name: name.to_string(),
            state: ChannelState::NeverConnected,
        };
        channel
    }



}