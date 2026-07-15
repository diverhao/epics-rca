#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PvaCmd {
    Beacon,               // 0x00
    ConnectionValidation, // 0x01
    Echo,                 // 0x02
    Search,               // 0x03
    SearchResponse,       // 0x04
    Authnz,               // 0x05
    AclChange,            // 0x06
    CreateChannel,        // 0x07
    DestroyChannel,       // 0x08
    ConnectionValidated,  // 0x09
    Get,                  // 0x0A
    Put,                  // 0x0B
    PutGet,               // 0x0C
    Monitor,              // 0x0D
    Array,                // 0x0E
    DestroyRequest,       // 0x0F
    Process,              // 0x10
    GetField,             // 0x11
    Message,              // 0x12
    MultipleData,         // 0x13
    Rpc,                  // 0x14
    CancelRequest,        // 0x15
    OriginTag,            // 0x16
}

impl PvaCmd {
    pub fn to_u8(self) -> u8 {
        match self {
            Self::Beacon => 0x00,
            Self::ConnectionValidation => 0x01,
            Self::Echo => 0x02,
            Self::Search => 0x03,
            Self::SearchResponse => 0x04,
            Self::Authnz => 0x05,
            Self::AclChange => 0x06,
            Self::CreateChannel => 0x07,
            Self::DestroyChannel => 0x08,
            Self::ConnectionValidated => 0x09,
            Self::Get => 0x0a,
            Self::Put => 0x0b,
            Self::PutGet => 0x0c,
            Self::Monitor => 0x0d,
            Self::Array => 0x0e,
            Self::DestroyRequest => 0x0f,
            Self::Process => 0x10,
            Self::GetField => 0x11,
            Self::Message => 0x12,
            Self::MultipleData => 0x13,
            Self::Rpc => 0x14,
            Self::CancelRequest => 0x15,
            Self::OriginTag => 0x16,
        }
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::Beacon),
            0x01 => Some(Self::ConnectionValidation),
            0x02 => Some(Self::Echo),
            0x03 => Some(Self::Search),
            0x04 => Some(Self::SearchResponse),
            0x05 => Some(Self::Authnz),
            0x06 => Some(Self::AclChange),
            0x07 => Some(Self::CreateChannel),
            0x08 => Some(Self::DestroyChannel),
            0x09 => Some(Self::ConnectionValidated),
            0x0a => Some(Self::Get),
            0x0b => Some(Self::Put),
            0x0c => Some(Self::PutGet),
            0x0d => Some(Self::Monitor),
            0x0e => Some(Self::Array),
            0x0f => Some(Self::DestroyRequest),
            0x10 => Some(Self::Process),
            0x11 => Some(Self::GetField),
            0x12 => Some(Self::Message),
            0x13 => Some(Self::MultipleData),
            0x14 => Some(Self::Rpc),
            0x15 => Some(Self::CancelRequest),
            0x16 => Some(Self::OriginTag),
            _ => None,
        }
    }
}

impl std::fmt::Display for PvaCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Beacon => "CMD_BEACON",
            Self::ConnectionValidation => "CMD_CONNECTION_VALIDATION",
            Self::Echo => "CMD_ECHO",
            Self::Search => "CMD_SEARCH",
            Self::SearchResponse => "CMD_SEARCH_RESPONSE",
            Self::Authnz => "CMD_AUTHNZ",
            Self::AclChange => "CMD_ACL_CHANGE",
            Self::CreateChannel => "CMD_CREATE_CHANNEL",
            Self::DestroyChannel => "CMD_DESTROY_CHANNEL",
            Self::ConnectionValidated => "CMD_CONNECTION_VALIDATED",
            Self::Get => "CMD_GET",
            Self::Put => "CMD_PUT",
            Self::PutGet => "CMD_PUT_GET",
            Self::Monitor => "CMD_MONITOR",
            Self::Array => "CMD_ARRAY",
            Self::DestroyRequest => "CMD_DESTROY_REQUEST",
            Self::Process => "CMD_PROCESS",
            Self::GetField => "CMD_GET_FIELD",
            Self::Message => "CMD_MESSAGE",
            Self::MultipleData => "CMD_MULTIPLE_DATA",
            Self::Rpc => "CMD_RPC",
            Self::CancelRequest => "CMD_CANCEL_REQUEST",
            Self::OriginTag => "CMD_ORIGIN_TAG",
        })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PvaCtrlCmd {
    CtrlSetMarker, // 0x00
    CtrlAckMarker, // 0x01
    SetEndianess,  // 0x02
}

impl PvaCtrlCmd {
    pub fn to_u8(self) -> u8 {
        match self {
            Self::CtrlSetMarker => 0x00,
            Self::CtrlAckMarker => 0x01,
            Self::SetEndianess => 0x02,
        }
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::CtrlSetMarker),
            0x01 => Some(Self::CtrlAckMarker),
            0x02 => Some(Self::SetEndianess),
            _ => None,
        }
    }
}

impl std::fmt::Display for PvaCtrlCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CtrlSetMarker => "CMD_SET_MARKER",
            Self::CtrlAckMarker => "CMD_ACK_MARKER",
            Self::SetEndianess => "CMD_SET_ENDIANESS",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{PvaCmd, PvaCtrlCmd};

    #[test]
    fn decodes_overlapping_application_and_control_commands() {
        assert_eq!(PvaCmd::from_u8(0x00), Some(PvaCmd::Beacon));
        assert_eq!(PvaCtrlCmd::from_u8(0x00), Some(PvaCtrlCmd::CtrlSetMarker));
        assert_eq!(PvaCmd::from_u8(0x01), Some(PvaCmd::ConnectionValidation));
        assert_eq!(PvaCtrlCmd::from_u8(0x01), Some(PvaCtrlCmd::CtrlAckMarker));
        assert_eq!(PvaCmd::from_u8(0x02), Some(PvaCmd::Echo));
        assert_eq!(PvaCtrlCmd::from_u8(0x02), Some(PvaCtrlCmd::SetEndianess));
    }
}
