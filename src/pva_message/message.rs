use crate::pva_message::{
    cmd::{AppCmd, CtrlCmd, PvaCmd},
    header::{MsgEndian, MsgSeg, MsgSrc, PVA_HEADER_SIZE, PvaHeader, PvaHeaderData},
};

const MAX_PVA_PAYLOAD_SIZE: usize = i32::MAX as usize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PvaMessage {
    header: PvaHeader,
    payload: Vec<u8>,
}

impl PvaMessage {
    pub fn new_application(
        seg_type: MsgSeg,
        src: MsgSrc,
        endian: MsgEndian,
        cmd: AppCmd,
        payload: Vec<u8>,
    ) -> Result<Self, String> {
        let payload_size = i32::try_from(payload.len())
            .map_err(|_| String::from("Error: PVA payload is larger than i32::MAX"))?;

        Ok(Self {
            header: PvaHeader::new_application(seg_type, src, endian, cmd, payload_size)?,
            payload,
        })
    }

    pub fn new_control(src: MsgSrc, endian: MsgEndian, cmd: CtrlCmd, data: i32) -> Self {
        Self {
            header: PvaHeader::new_control(src, endian, cmd, data),
            payload: Vec::new(),
        }
    }

    pub fn header(&self) -> &PvaHeader {
        &self.header
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }

    pub fn validate(&self) -> Result<(), String> {
        self.header.validate()?;

        match self.header.data() {
            PvaHeaderData::ApplicationPayloadSize(size) => {
                let expected = usize::try_from(size)
                    .map_err(|_| String::from("Error: PVA payload size does not fit in usize"))?;
                if self.payload.len() != expected {
                    return Err(format!(
                        "Error: PVA payload length {} does not match header size {expected}",
                        self.payload.len()
                    ));
                }
            }
            PvaHeaderData::ControlData(_) => {
                if !self.payload.is_empty() {
                    return Err(String::from(
                        "Error: PVA control message cannot have a payload",
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn to_buf(&self) -> Result<Vec<u8>, String> {
        self.validate()?;

        let mut buf = Vec::with_capacity(PVA_HEADER_SIZE + self.payload.len());
        buf.extend_from_slice(&self.header.to_buf()?);
        buf.extend_from_slice(&self.payload);
        Ok(buf)
    }

    pub fn from_buf(buf: &[u8], offset: &mut usize) -> Result<Self, String> {
        let header_end = offset
            .checked_add(PVA_HEADER_SIZE)
            .ok_or_else(|| String::from("Error: PVA message header offset overflow"))?;
        if header_end > buf.len() {
            return Err(String::from(
                "Warning: Remaining buffer too short for PVA message header",
            ));
        }

        let header = PvaHeader::from_buf(&buf[*offset..header_end])?;
        let message_end = match header.data() {
            PvaHeaderData::ApplicationPayloadSize(size) => {
                let payload_size = usize::try_from(size)
                    .map_err(|_| String::from("Error: PVA payload size does not fit in usize"))?;
                header_end
                    .checked_add(payload_size)
                    .ok_or_else(|| String::from("Error: PVA message payload offset overflow"))?
            }
            PvaHeaderData::ControlData(_) => header_end,
        };

        if message_end > buf.len() {
            return Err(format!(
                "Warning: Remaining buffer too short for PVA message: need {message_end} bytes, have {}",
                buf.len()
            ));
        }

        let message = Self {
            header,
            payload: buf[header_end..message_end].to_vec(),
        };
        message.validate()?;
        *offset = message_end;
        Ok(message)
    }
}

#[derive(Debug)]
struct PendingSegments {
    src: MsgSrc,
    endian: MsgEndian,
    cmd: AppCmd,
    payload: Vec<u8>,
}

#[derive(Debug)]
pub struct PvaMessageReassembler {
    pending: Option<PendingSegments>,
    max_payload_size: usize,
}

impl PvaMessageReassembler {
    pub fn new(max_payload_size: usize) -> Self {
        Self {
            pending: None,
            max_payload_size,
        }
    }

    pub fn has_pending_message(&self) -> bool {
        self.pending.is_some()
    }

    pub fn reset(&mut self) {
        self.pending = None;
    }

    pub fn push(&mut self, message: PvaMessage) -> Result<Option<PvaMessage>, String> {
        message.validate()?;

        let flags = message.header.flags();
        let cmd = match message.header.cmd() {
            PvaCmd::Ctrl(_) => return Ok(Some(message)),
            PvaCmd::App(cmd) => cmd,
        };

        match flags.seg_type {
            MsgSeg::NotSeg => {
                if self.pending.is_some() {
                    return Err(String::from(
                        "Error: Received an unsegmented PVA application message while segmented reassembly is pending",
                    ));
                }
                self.ensure_size(0, message.payload.len())?;
                Ok(Some(message))
            }
            MsgSeg::FirstOfSeg => {
                if self.pending.is_some() {
                    return Err(String::from(
                        "Error: Received a first PVA segment while another segmented message is pending",
                    ));
                }
                self.ensure_size(0, message.payload.len())?;
                self.pending = Some(PendingSegments {
                    src: flags.src,
                    endian: flags.endian,
                    cmd,
                    payload: message.payload,
                });
                Ok(None)
            }
            MsgSeg::MidOfSeg => {
                self.append_segment(flags.src, flags.endian, cmd, &message.payload)?;
                Ok(None)
            }
            MsgSeg::LastOfSeg => {
                self.append_segment(flags.src, flags.endian, cmd, &message.payload)?;
                let pending = self
                    .pending
                    .take()
                    .ok_or_else(|| String::from("Error: Missing pending PVA segmented message"))?;
                let complete = PvaMessage::new_application(
                    MsgSeg::NotSeg,
                    pending.src,
                    pending.endian,
                    pending.cmd,
                    pending.payload,
                )?;
                Ok(Some(complete))
            }
        }
    }

    fn append_segment(
        &mut self,
        src: MsgSrc,
        endian: MsgEndian,
        cmd: AppCmd,
        payload: &[u8],
    ) -> Result<(), String> {
        let pending = self.pending.as_ref().ok_or_else(|| {
            String::from("Error: Received a non-first PVA segment without a first segment")
        })?;

        if (pending.src, pending.endian, pending.cmd) != (src, endian, cmd) {
            return Err(String::from(
                "Error: PVA segment source, endian, or command does not match the first segment",
            ));
        }

        self.ensure_size(pending.payload.len(), payload.len())?;
        self.pending
            .as_mut()
            .expect("pending PVA segments were checked above")
            .payload
            .extend_from_slice(payload);
        Ok(())
    }

    fn ensure_size(&self, current: usize, additional: usize) -> Result<(), String> {
        let total = current
            .checked_add(additional)
            .ok_or_else(|| String::from("Error: Reassembled PVA payload size overflow"))?;
        if total > self.max_payload_size {
            return Err(format!(
                "Error: Reassembled PVA payload size {total} exceeds configured limit {}",
                self.max_payload_size
            ));
        }
        if total > MAX_PVA_PAYLOAD_SIZE {
            return Err(String::from(
                "Error: Reassembled PVA payload is larger than i32::MAX",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pva_message::header::{MsgFlags, MsgType};

    #[test]
    fn frames_application_and_control_messages() {
        let application = PvaMessage::new_application(
            MsgSeg::NotSeg,
            MsgSrc::Server,
            MsgEndian::Little,
            AppCmd::Echo,
            vec![1, 2, 3],
        )
        .unwrap();
        let control =
            PvaMessage::new_control(MsgSrc::Server, MsgEndian::Big, CtrlCmd::SetMarker, -42);

        let mut buf = application.to_buf().unwrap();
        buf.extend_from_slice(&control.to_buf().unwrap());

        let mut offset = 0;
        assert_eq!(
            PvaMessage::from_buf(&buf, &mut offset).unwrap(),
            application
        );
        assert_eq!(offset, PVA_HEADER_SIZE + 3);
        assert_eq!(PvaMessage::from_buf(&buf, &mut offset).unwrap(), control);
        assert_eq!(offset, buf.len());
    }

    #[test]
    fn truncated_payload_does_not_advance_offset() {
        let message = PvaMessage::new_application(
            MsgSeg::NotSeg,
            MsgSrc::Server,
            MsgEndian::Little,
            AppCmd::Echo,
            vec![1, 2, 3],
        )
        .unwrap();
        let mut buf = message.to_buf().unwrap();
        buf.pop();

        let mut offset = 0;
        assert!(PvaMessage::from_buf(&buf, &mut offset).is_err());
        assert_eq!(offset, 0);
    }

    #[test]
    fn rejects_negative_application_payload_size() {
        let buf = [
            0xca,
            0x02,
            MsgFlags {
                msg_type: MsgType::Application,
                seg_type: MsgSeg::NotSeg,
                src: MsgSrc::Server,
                endian: MsgEndian::Little,
            }
            .to_u8(),
            AppCmd::Echo.to_u8(),
            0xff,
            0xff,
            0xff,
            0xff,
        ];

        let mut offset = 0;
        let error = PvaMessage::from_buf(&buf, &mut offset).unwrap_err();
        assert!(error.contains("payload size cannot be negative"));
        assert_eq!(offset, 0);

        let error = PvaHeader::new_application(
            MsgSeg::NotSeg,
            MsgSrc::Server,
            MsgEndian::Little,
            AppCmd::Echo,
            -1,
        )
        .unwrap_err();
        assert!(error.contains("payload size cannot be negative"));
    }

    #[test]
    fn reassembles_segments_and_allows_interleaved_control() {
        let first = PvaMessage::new_application(
            MsgSeg::FirstOfSeg,
            MsgSrc::Server,
            MsgEndian::Little,
            AppCmd::Get,
            vec![1, 2],
        )
        .unwrap();
        let middle = PvaMessage::new_application(
            MsgSeg::MidOfSeg,
            MsgSrc::Server,
            MsgEndian::Little,
            AppCmd::Get,
            vec![3],
        )
        .unwrap();
        let last = PvaMessage::new_application(
            MsgSeg::LastOfSeg,
            MsgSrc::Server,
            MsgEndian::Little,
            AppCmd::Get,
            vec![4, 5],
        )
        .unwrap();
        let control =
            PvaMessage::new_control(MsgSrc::Server, MsgEndian::Little, CtrlCmd::AckMarker, 17);

        let mut reassembler = PvaMessageReassembler::new(32);
        assert_eq!(reassembler.push(first).unwrap(), None);
        assert!(reassembler.has_pending_message());
        assert_eq!(reassembler.push(control.clone()).unwrap(), Some(control));
        assert_eq!(reassembler.push(middle).unwrap(), None);

        let complete = reassembler.push(last).unwrap().unwrap();
        assert_eq!(complete.header().flags().seg_type, MsgSeg::NotSeg);
        assert_eq!(complete.payload(), &[1, 2, 3, 4, 5]);
        assert!(!reassembler.has_pending_message());
    }

    #[test]
    fn rejects_invalid_segment_order_and_oversized_payload() {
        let middle = PvaMessage::new_application(
            MsgSeg::MidOfSeg,
            MsgSrc::Server,
            MsgEndian::Little,
            AppCmd::Get,
            vec![1],
        )
        .unwrap();
        let first = PvaMessage::new_application(
            MsgSeg::FirstOfSeg,
            MsgSrc::Server,
            MsgEndian::Little,
            AppCmd::Get,
            vec![1, 2, 3],
        )
        .unwrap();

        let mut reassembler = PvaMessageReassembler::new(2);
        assert!(reassembler.push(middle).is_err());
        assert!(reassembler.push(first).is_err());

        let reassembler = PvaMessageReassembler::new(usize::MAX);
        assert!(reassembler.ensure_size(MAX_PVA_PAYLOAD_SIZE, 1).is_err());
    }
}
