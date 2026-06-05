use tokio::net::UdpSocket;

#[repr(u16)]
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

pub struct UDP {
    pub socket_v4: UdpSocket,
    pub socket_v6: UdpSocket,
}

impl UDP {
    pub async fn new() {
        // bind to all interfaces for both v4 and v6
        // panic if fail
        let socket_v4: UdpSocket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        let socket_v6: UdpSocket = UdpSocket::bind("[::]:0").await.unwrap();
        let udp = UDP {
            socket_v4: socket_v4,
            socket_v6: socket_v6,
        };
    }

    /**
     * Send one UDP message to hosts defined in EPICS_CA_ADDR_LIST
     */
    pub fn send_ca(
        self: &Self,
        cmd: CaCmd,
        payload_size: u16,
        data_type: DbrType,
        data_count: u16,
        param1: u32,
        param2: u32,
    ) {
        let hosts = 
        // assemble message
        // 
    }
}
