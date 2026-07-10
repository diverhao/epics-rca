use core::num;

use crate::pva_message::header::MsgEndian;

// ------------------- size -----------------

const PVA_SIZE_NULL: u8 = 0xff;
const PVA_SIZE_EXTENDED: u8 = 0xfe;
const PVA_SIZE_EXTENDED_MIN: usize = PVA_SIZE_EXTENDED as usize;
const PVA_SIZE_INT32_MAX: usize = i32::MAX as usize;

const NULL_TYPE_CODE: u8 = 0xff;
const ONLY_ID_TYPE_CODE: u8 = 0xfe;
const FULL_WITH_ID_TYPE_CODE: u8 = 0xfd;
const FULL_TAGGED_ID_TYPE_CODE: u8 = 0xfc;

// ------------ buffer tools for basic types ----------------

pub trait PvaElement {
    // actually append_to_buf()
    fn to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String>;

    fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String>
    where
        Self: Sized;
}

// N is compile-time constant
fn read_n_bytes<const N: usize>(
    buf: &[u8],
    offset: &mut usize,
    element_type: &str,
) -> Result<[u8; N], String> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| format!("Error: PVA {element_type} offset overflow"))?;

    if buf.len() < end {
        return Err(format!(
            "Warning: Remaining buffer too short for PVA {element_type}"
        ));
    }

    let mut bytes = [0_u8; N];
    bytes.copy_from_slice(&buf[*offset..end]);
    *offset = end;

    Ok(bytes)
}

impl PvaElement for usize {
    // actually append_to_buf()
    fn to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        if *self < PVA_SIZE_EXTENDED_MIN {
            buf.extend_from_slice(&vec![*self as u8]);
            return Ok(());
        }

        if *self >= PVA_SIZE_INT32_MAX {
            return Err(String::from(
                "Error: PVA 64-bit size encoding is not implemented",
            ));
        }

        let mut buf_size = Vec::with_capacity(5);
        buf_size.push(PVA_SIZE_EXTENDED);

        let size = i32::try_from(*self)
            .map_err(|_| String::from("Error: PVA size does not fit in i32"))?;
        match endian {
            MsgEndian::Little => buf_size.extend_from_slice(&size.to_le_bytes()),
            MsgEndian::Big => buf_size.extend_from_slice(&size.to_be_bytes()),
        }

        buf.extend_from_slice(&buf_size);
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<usize, String> {
        let first = match buf.get(*offset) {
            Some(first) => *first,
            None => {
                return Err(String::from(
                    "Warning: Remaining buffer too short for PVA size",
                ));
            }
        };

        let size = match first {
            PVA_SIZE_NULL => {
                *offset += 1;
                0 as usize
            }
            0..=253 => {
                *offset += 1;
                first as usize
            }
            PVA_SIZE_EXTENDED => {
                let end = offset
                    .checked_add(5)
                    .ok_or_else(|| String::from("Error: PVA size offset overflow"))?;

                if buf.len() < end {
                    return Err(String::from(
                        "Warning: Remaining buffer too short for extended PVA size",
                    ));
                }

                let size = match endian {
                    MsgEndian::Little => i32::from_le_bytes([
                        buf[*offset + 1],
                        buf[*offset + 2],
                        buf[*offset + 3],
                        buf[*offset + 4],
                    ]),
                    MsgEndian::Big => i32::from_be_bytes([
                        buf[*offset + 1],
                        buf[*offset + 2],
                        buf[*offset + 3],
                        buf[*offset + 4],
                    ]),
                };

                if size < 0 {
                    return Err(String::from("Error: PVA size is negative"));
                }

                if size == i32::MAX {
                    return Err(String::from(
                        "Error: PVA 64-bit size encoding is not implemented",
                    ));
                }

                let size = usize::try_from(size)
                    .map_err(|_| String::from("Error: PVA size does not fit in usize"))?;
                if size < PVA_SIZE_EXTENDED_MIN {
                    return Err(String::from("Error: Non-canonical extended PVA size"));
                }

                *offset = end;

                size
            }
        };

        Ok(size)
    }
}

impl PvaElement for bool {
    fn to_buf(&self, buf: &mut Vec<u8>, _endian: MsgEndian) -> Result<(), String> {
        buf.push(u8::from(*self));
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, _endian: MsgEndian) -> Result<Self, String> {
        let mut local_offset = *offset;
        let byte = read_n_bytes::<1>(buf, &mut local_offset, "boolean")?[0];
        let value = match byte {
            0 => false,
            1 => true,
            _ => return Err(String::from("Error: Invalid PVA boolean value")),
        };
        *offset = local_offset;

        Ok(value)
    }
}

impl PvaElement for u8 {
    fn to_buf(&self, buf: &mut Vec<u8>, _endian: MsgEndian) -> Result<(), String> {
        buf.push(*self);
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, _endian: MsgEndian) -> Result<Self, String> {
        Ok(read_n_bytes::<1>(buf, offset, "u8")?[0])
    }
}

impl PvaElement for i8 {
    fn to_buf(&self, buf: &mut Vec<u8>, _endian: MsgEndian) -> Result<(), String> {
        buf.push(*self as u8);
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, _endian: MsgEndian) -> Result<Self, String> {
        Ok(read_n_bytes::<1>(buf, offset, "i8")?[0] as i8)
    }
}

impl PvaElement for u16 {
    fn to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String> {
        let bytes = read_n_bytes::<2>(buf, offset, "u16")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }
}

impl PvaElement for i16 {
    fn to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String> {
        let bytes = read_n_bytes::<2>(buf, offset, "i16")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }
}

impl PvaElement for u32 {
    fn to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String> {
        let bytes = read_n_bytes::<4>(buf, offset, "u32")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }
}

impl PvaElement for i32 {
    fn to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String> {
        let bytes = read_n_bytes::<4>(buf, offset, "i32")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }
}

impl PvaElement for u64 {
    fn to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String> {
        let bytes = read_n_bytes::<8>(buf, offset, "u64")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }
}

impl PvaElement for i64 {
    fn to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String> {
        let bytes = read_n_bytes::<8>(buf, offset, "i64")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }
}

impl PvaElement for f32 {
    fn to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String> {
        let bytes = read_n_bytes::<4>(buf, offset, "f32")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }
}

impl PvaElement for f64 {
    fn to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String> {
        let bytes = read_n_bytes::<8>(buf, offset, "f64")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }
}

impl PvaElement for String {
    fn to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        self.len().to_buf(buf, endian)?;
        buf.extend_from_slice(self.as_bytes());
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String> {
        if *offset > buf.len() {
            return Err(String::from("Error: PVA string offset past end of buffer"));
        }

        let mut local_offset = *offset;
        let size = usize::from_buf(buf, &mut local_offset, endian)?;
        let start = local_offset;
        let end = start
            .checked_add(size)
            .ok_or_else(|| String::from("Error: PVA string offset overflow"))?;

        if buf.len() < end {
            return Err(String::from(
                "Warning: Remaining buffer too short for PVA string",
            ));
        }

        let value = String::from_utf8(buf[start..end].to_vec())
            .map_err(|_| String::from("Error: PVA string is not valid UTF-8"))?;
        local_offset = end;
        *offset = local_offset;

        Ok(value)
    }
}

// --------------- PVA type --------------
#[derive(Debug, Clone)]
pub enum PvaType {
    Boolean,            // 0x00, 0b 000 00 000
    Byte,               // 0x20, 0b 001 00 000
    Short,              // 0x21, 0b 001 00 001
    Int,                // 0x22, 0b 001 00 010
    Long,               // 0x23, 0b 001 00 011
    UByte,              // 0x24, 0b 001 00 100
    UShort,             // 0x25, 0b 001 00 101
    UInt,               // 0x26, 0b 001 00 110
    ULong,              // 0x27, 0b 001 00 111
    Float,              // 0x42, 0b 010 00 010
    Double,             // 0x43, 0b 010 00 011
    String,             // 0x60, 0b 011 00 000
    BoundString(usize), // 0x83, 0b 100 00 011

    BooleanVarSizeArray,      // 0x08, 0b 000 01 000
    BooleanBoundArray(usize), // 0x10, 0b 000 10 000
    BooleanFixArray(usize),   // 0x18, 0b 000 11 000

    ByteVarSizeArray,      // 0x28, 0b 001 01 000
    ByteBoundArray(usize), // 0x30, 0b 001 10 000
    ByteFixArray(usize),   // 0x38, 0b 001 11 000

    ShortVarSizeArray,      // 0x29, 0b 001 01 001
    ShortBoundArray(usize), // 0x31, 0b 001 10 001
    ShortFixArray(usize),   // 0x39, 0b 001 11 001

    IntVarSizeArray,      // 0x2A, 0b 001 01 010
    IntBoundArray(usize), // 0x32, 0b 001 10 010
    IntFixArray(usize),   // 0x3A, 0b 001 11 010

    LongVarSizeArray,      // 0x2B, 0b 001 01 011
    LongBoundArray(usize), // 0x33, 0b 001 10 011
    LongFixArray(usize),   // 0x3B, 0b 001 11 011

    UByteVarSizeArray,      // 0x2C, 0b 001 01 100
    UByteBoundArray(usize), // 0x34, 0b 001 10 100
    UByteFixArray(usize),   // 0x3C, 0b 001 11 100

    UShortVarSizeArray,      // 0x2D, 0b 001 01 101
    UShortBoundArray(usize), // 0x35, 0b 001 10 101
    UShortFixArray(usize),   // 0x3D, 0b 001 11 101

    UIntVarSizeArray,      // 0x2E, 0b 001 01 110
    UIntBoundArray(usize), // 0x36, 0b 001 10 110
    UIntFixArray(usize),   // 0x3E, 0b 001 11 110

    ULongVarSizeArray,      // 0x2F, 0b 001 01 111
    ULongBoundArray(usize), // 0x37, 0b 001 10 111
    ULongFixArray(usize),   // 0x3F, 0b 001 11 111

    FloatVarSizeArray,      // 0x4A, 0b 010 01 010
    FloatBoundArray(usize), // 0x52, 0b 010 10 010
    FloatFixArray(usize),   // 0x5A, 0b 010 11 010

    DoubleVarSizeArray,      // 0x4B, 0b 010 01 011
    DoubleBoundArray(usize), // 0x53, 0b 010 10 011
    DoubleFixArray(usize),   // 0x5B, 0b 010 11 011

    StringVarSizeArray,      // 0x68, 0b 011 01 000
    StringBoundArray(usize), // 0x70, 0b 011 10 000
    StringFixArray(usize),   // 0x78, 0b 011 11 000

    Structure(PvaStructType),             // 0x80, 0b 100 00 000
    StructureVarSizeArray(PvaStructType), // 0x88, 0b 100 01 000

    Union(PvaUnionType),             // 0x81, 0b 100 00 001
    UnionVarSizeArray(PvaUnionType), // 0x89, 0b 100 01 001

    VariantUnion,             // 0x82, 0b 100 00 010
    VariantUnionVarSizeArray, // 0x8A, 0b 100 01 010
}

impl PvaType {
    // a wrapper
    pub fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<PvaType, String> {
        // do not consume the first byte
        let form = match buf.first() {
            Some(form) => *form,
            None => return Err("Buffer empty".to_string()),
        };

        match form {
            // todo: what to do? how to name it
            NULL_TYPE_CODE => return Err("".to_string()),

            ONLY_ID_TYPE_CODE => {
                // todo: look for type from registry
                return Err("".to_string());
            }
            // type ID and type def
            FULL_WITH_ID_TYPE_CODE => {
                // consume 0xFD
                u8::from_buf(buf, offset, endian)?;
                // type ID
                let id = i16::from_buf(buf, offset, endian)?;
                // todo: register type
                return Self::from_buf_body(buf, offset, endian);
            }
            // not implemented
            FULL_TAGGED_ID_TYPE_CODE => {
                return Err("FULL_TAGGED_ID_TYPE_CODE not implemented".to_string());
            }
            // direct type def
            _ => {
                return Self::from_buf_body(buf, offset, endian);
            }
        };
    }

    // real decoder, the part after type code and type id
    pub fn from_buf_body(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Self, String> {
        let code = u8::from_buf(buf, offset, endian)?;

        let pva_type = match code {
            0x00 => PvaType::Boolean,
            0x20 => PvaType::Byte,
            0x21 => PvaType::Short,
            0x22 => PvaType::Int,
            0x23 => PvaType::Long,
            0x24 => PvaType::UByte,
            0x25 => PvaType::UShort,
            0x26 => PvaType::UInt,
            0x27 => PvaType::ULong,
            0x42 => PvaType::Float,
            0x43 => PvaType::Double,
            0x60 => PvaType::String,
            0x83 => PvaType::BoundString(usize::from_buf(buf, offset, endian)?),

            0x08 => PvaType::BooleanVarSizeArray,
            0x28 => PvaType::ByteVarSizeArray,
            0x29 => PvaType::ShortVarSizeArray,
            0x2A => PvaType::IntVarSizeArray,
            0x2B => PvaType::LongVarSizeArray,
            0x2C => PvaType::UByteVarSizeArray,
            0x2D => PvaType::UShortVarSizeArray,
            0x2E => PvaType::UIntVarSizeArray,
            0x2F => PvaType::ULongVarSizeArray,
            0x4A => PvaType::FloatVarSizeArray,
            0x4B => PvaType::DoubleVarSizeArray,
            0x68 => PvaType::StringVarSizeArray,

            0x10 => PvaType::BooleanBoundArray(usize::from_buf(buf, offset, endian)?),
            0x18 => PvaType::BooleanFixArray(usize::from_buf(buf, offset, endian)?),

            0x30 => PvaType::ByteBoundArray(usize::from_buf(buf, offset, endian)?),
            0x38 => PvaType::ByteFixArray(usize::from_buf(buf, offset, endian)?),

            0x31 => PvaType::ShortBoundArray(usize::from_buf(buf, offset, endian)?),
            0x39 => PvaType::ShortFixArray(usize::from_buf(buf, offset, endian)?),

            0x32 => PvaType::IntBoundArray(usize::from_buf(buf, offset, endian)?),
            0x3A => PvaType::IntFixArray(usize::from_buf(buf, offset, endian)?),

            0x33 => PvaType::LongBoundArray(usize::from_buf(buf, offset, endian)?),
            0x3B => PvaType::LongFixArray(usize::from_buf(buf, offset, endian)?),

            0x34 => PvaType::UByteBoundArray(usize::from_buf(buf, offset, endian)?),
            0x3C => PvaType::UByteFixArray(usize::from_buf(buf, offset, endian)?),

            0x35 => PvaType::UShortBoundArray(usize::from_buf(buf, offset, endian)?),
            0x3D => PvaType::UShortFixArray(usize::from_buf(buf, offset, endian)?),

            0x36 => PvaType::UIntBoundArray(usize::from_buf(buf, offset, endian)?),
            0x3E => PvaType::UIntFixArray(usize::from_buf(buf, offset, endian)?),

            0x37 => PvaType::ULongBoundArray(usize::from_buf(buf, offset, endian)?),
            0x3F => PvaType::ULongFixArray(usize::from_buf(buf, offset, endian)?),

            0x52 => PvaType::FloatBoundArray(usize::from_buf(buf, offset, endian)?),
            0x5A => PvaType::FloatFixArray(usize::from_buf(buf, offset, endian)?),

            0x53 => PvaType::DoubleBoundArray(usize::from_buf(buf, offset, endian)?),
            0x5B => PvaType::DoubleFixArray(usize::from_buf(buf, offset, endian)?),

            0x70 => PvaType::StringBoundArray(usize::from_buf(buf, offset, endian)?),
            0x78 => PvaType::StringFixArray(usize::from_buf(buf, offset, endian)?),

            0x80 => {
                // retract by 1 for 0x80
                *offset -= 1;
                PvaType::Structure(PvaStructType::from_buf(buf, offset, endian)?)
            }
            0x88 => PvaType::StructureVarSizeArray(PvaStructType::from_buf(buf, offset, endian)?),

            0x81 => {
                // retract by 1 for 0x81
                *offset -= 1;
                PvaType::Union(PvaUnionType::from_buf(buf, offset, endian)?)
            }
            0x89 => PvaType::UnionVarSizeArray(PvaUnionType::from_buf(buf, offset, endian)?),

            0x82 | 0x8A => {
                return Err(format!(
                    "Error: PVA variant union type code 0x{code:02X} is not implemented"
                ));
            }

            _ => return Err(format!("Error: Invalid PVA type code 0x{code:02X}")),
        };

        Ok(pva_type)
    }

    // actually append_to_buf()
    pub fn to_buf(self: &Self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match self {
            Self::Boolean => buf.push(0x00),
            Self::Byte => buf.push(0x20),
            Self::Short => buf.push(0x21),
            Self::Int => buf.push(0x22),
            Self::Long => buf.push(0x23),
            Self::UByte => buf.push(0x24),
            Self::UShort => buf.push(0x25),
            Self::UInt => buf.push(0x26),
            Self::ULong => buf.push(0x27),
            Self::Float => buf.push(0x42),
            Self::Double => buf.push(0x43),
            Self::String => buf.push(0x60),
            Self::BoundString(bound) => {
                buf.push(0x83);
                bound.to_buf(buf, endian)?;
            }

            Self::BooleanVarSizeArray => buf.push(0x08),
            Self::BooleanBoundArray(bound) => {
                buf.push(0x10);
                bound.to_buf(buf, endian)?;
            }
            Self::BooleanFixArray(len) => {
                buf.push(0x18);
                len.to_buf(buf, endian)?;
            }

            Self::ByteVarSizeArray => buf.push(0x28),
            Self::ByteBoundArray(bound) => {
                buf.push(0x30);
                bound.to_buf(buf, endian)?;
            }
            Self::ByteFixArray(len) => {
                buf.push(0x38);
                len.to_buf(buf, endian)?;
            }

            Self::ShortVarSizeArray => buf.push(0x29),
            Self::ShortBoundArray(bound) => {
                buf.push(0x31);
                bound.to_buf(buf, endian)?;
            }
            Self::ShortFixArray(len) => {
                buf.push(0x39);
                len.to_buf(buf, endian)?;
            }

            Self::IntVarSizeArray => buf.push(0x2A),
            Self::IntBoundArray(bound) => {
                buf.push(0x32);
                bound.to_buf(buf, endian)?;
            }
            Self::IntFixArray(len) => {
                buf.push(0x3A);
                len.to_buf(buf, endian)?;
            }

            Self::LongVarSizeArray => buf.push(0x2B),
            Self::LongBoundArray(bound) => {
                buf.push(0x33);
                bound.to_buf(buf, endian)?;
            }
            Self::LongFixArray(len) => {
                buf.push(0x3B);
                len.to_buf(buf, endian)?;
            }

            Self::UByteVarSizeArray => buf.push(0x2C),
            Self::UByteBoundArray(bound) => {
                buf.push(0x34);
                bound.to_buf(buf, endian)?;
            }
            Self::UByteFixArray(len) => {
                buf.push(0x3C);
                len.to_buf(buf, endian)?;
            }

            Self::UShortVarSizeArray => buf.push(0x2D),
            Self::UShortBoundArray(bound) => {
                buf.push(0x35);
                bound.to_buf(buf, endian)?;
            }
            Self::UShortFixArray(len) => {
                buf.push(0x3D);
                len.to_buf(buf, endian)?;
            }

            Self::UIntVarSizeArray => buf.push(0x2E),
            Self::UIntBoundArray(bound) => {
                buf.push(0x36);
                bound.to_buf(buf, endian)?;
            }
            Self::UIntFixArray(len) => {
                buf.push(0x3E);
                len.to_buf(buf, endian)?;
            }

            Self::ULongVarSizeArray => buf.push(0x2F),
            Self::ULongBoundArray(bound) => {
                buf.push(0x37);
                bound.to_buf(buf, endian)?;
            }
            Self::ULongFixArray(len) => {
                buf.push(0x3F);
                len.to_buf(buf, endian)?;
            }

            Self::FloatVarSizeArray => buf.push(0x4A),
            Self::FloatBoundArray(bound) => {
                buf.push(0x52);
                bound.to_buf(buf, endian)?;
            }
            Self::FloatFixArray(len) => {
                buf.push(0x5A);
                len.to_buf(buf, endian)?;
            }

            Self::DoubleVarSizeArray => buf.push(0x4B),
            Self::DoubleBoundArray(bound) => {
                buf.push(0x53);
                bound.to_buf(buf, endian)?;
            }
            Self::DoubleFixArray(len) => {
                buf.push(0x5B);
                len.to_buf(buf, endian)?;
            }

            Self::StringVarSizeArray => buf.push(0x68),
            Self::StringBoundArray(bound) => {
                buf.push(0x70);
                bound.to_buf(buf, endian)?;
            }
            Self::StringFixArray(len) => {
                buf.push(0x78);
                len.to_buf(buf, endian)?;
            }

            Self::Structure(typ) => {
                typ.to_buf(buf, endian)?;
            }
            Self::StructureVarSizeArray(typ) => {
                // append 0x88
                buf.push(0x88);
                typ.to_buf(buf, endian)?;
            }

            Self::Union(typ) => {
                typ.to_buf(buf, endian)?;
            }
            Self::UnionVarSizeArray(typ) => {
                // append 0x89
                buf.push(0x89);
                typ.to_buf(buf, endian)?;
            }

            Self::VariantUnion | Self::VariantUnionVarSizeArray => {
                return Err("Variant Union type encoding has not been implemented".to_string());
            }
        }
        Ok(())
    }
}

// ---------------- struct type -------------

#[derive(Debug, Clone)]
pub struct PvaStructType {
    pub id: String,                // e.g. timeStamp_t
    pub fields: Vec<PvaFieldType>, // e.g. [{name: "secondsPastEpoch", typ: PvaType::Long}, {name: "nanoSeconds", typ: PvaType::Int}, ]
}

impl PvaStructType {
    pub fn to_buf(self: &Self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        // code 0x80
        buf.push(0x80);

        // struct ID string
        self.id.to_buf(buf, endian)?;

        // number of fields
        self.fields.len().to_buf(buf, endian)?;

        // field types
        for field_type in &self.fields {
            field_type.to_buf(buf, endian)?;
        }

        Ok(())
    }

    pub fn from_buf(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<PvaStructType, String> {
        // consume and verify 0x80
        let code = u8::from_buf(buf, offset, endian)?;

        // struct ID string, decode like variable size string
        let id = String::from_buf(buf, offset, endian)?;

        // number of fields, encoded as PvaSize
        let num_fields = usize::from_buf(buf, offset, endian)?;

        // fields type: field name + pva type
        let mut fields: Vec<PvaFieldType> = vec![];
        for _ in 0..num_fields {
            let field_type = PvaFieldType::from_buf(buf, offset, endian)?;
            fields.push(field_type);
        }

        Ok(PvaStructType {
            id: id,
            fields: fields,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PvaFieldType {
    pub name: String,
    pub typ: PvaType,
}

impl PvaFieldType {
    pub fn to_buf(self: &Self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        let name = &self.name;
        name.to_buf(buf, endian)?;

        let typ = &self.typ;
        typ.to_buf(buf, endian)?;
        Ok(())
    }

    pub fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String> {
        // field name
        let name = String::from_buf(buf, offset, endian)?;

        // field type
        let typ = PvaType::from_buf(buf, offset, endian)?;

        Ok(PvaFieldType {
            name: name,
            typ: typ,
        })
    }
}

// ---------------- union type -------------

// exactly the same as PvaStructType except the to_buf
#[derive(Debug, Clone)]
pub struct PvaUnionType {
    pub id: String,                // e.g. timeStamp_t
    pub fields: Vec<PvaFieldType>, // e.g. [{name: "secondsPastEpoch", typ: PvaType::Long}, {name: "nanoSeconds", typ: PvaType::Int}, ]
}

impl PvaUnionType {
    pub fn to_buf(self: &Self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        // code 0x81
        buf.push(0x81);

        // union ID string
        self.id.to_buf(buf, endian)?;

        // number of fields
        self.fields.len().to_buf(buf, endian)?;

        // field types
        for field_type in &self.fields {
            field_type.to_buf(buf, endian)?;
        }

        Ok(())
    }

    pub fn from_buf(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<PvaUnionType, String> {
        // consume 0x81
        let code = u8::from_buf(buf, offset, endian)?;

        // union ID string, decode like variable size string
        let id = String::from_buf(buf, offset, endian)?;

        // number of fields, encoded as PvaSize
        let num_fields = usize::from_buf(buf, offset, endian)?;

        // fields type: field name + pva type
        let mut fields: Vec<PvaFieldType> = vec![];
        for _ in 0..num_fields {
            let field_type = PvaFieldType::from_buf(buf, offset, endian)?;
            fields.push(field_type);
        }

        Ok(PvaUnionType {
            id: id,
            fields: fields,
        })
    }
}

// --------------- PVA value --------------
pub enum PvaValue {
    Boolean(bool),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    UByte(u8),
    UShort(u16),
    UInt(u32),
    ULong(u64),
    Float(f32),
    Double(f64),
    String(String),
    BoundString(String),

    BooleanVarSizeArray(Vec<bool>),
    BooleanBoundArray(Vec<bool>),
    BooleanFixArray(Vec<bool>),

    ByteVarSizeArray(Vec<i8>),
    ByteBoundArray(Vec<i8>),
    ByteFixArray(Vec<i8>),

    ShortVarSizeArray(Vec<i16>),
    ShortBoundArray(Vec<i16>),
    ShortFixArray(Vec<i16>),

    IntVarSizeArray(Vec<i32>),
    IntBoundArray(Vec<i32>),
    IntFixArray(Vec<i32>),

    LongVarSizeArray(Vec<i64>),
    LongBoundArray(Vec<i64>),
    LongFixArray(Vec<i64>),

    UByteVarSizeArray(Vec<u8>),
    UByteBoundArray(Vec<u8>),
    UByteFixArray(Vec<u8>),

    UShortVarSizeArray(Vec<u16>),
    UShortBoundArray(Vec<u16>),
    UShortFixArray(Vec<u16>),

    UIntVarSizeArray(Vec<u32>),
    UIntBoundArray(Vec<u32>),
    UIntFixArray(Vec<u32>),

    ULongVarSizeArray(Vec<u64>),
    ULongBoundArray(Vec<u64>),
    ULongFixArray(Vec<u64>),

    FloatVarSizeArray(Vec<f32>),
    FloatBoundArray(Vec<f32>),
    FloatFixArray(Vec<f32>),

    DoubleVarSizeArray(Vec<f64>),
    DoubleBoundArray(Vec<f64>),
    DoubleFixArray(Vec<f64>),

    StringVarSizeArray(Vec<String>),
    StringBoundArray(Vec<String>),
    StringFixArray(Vec<String>),

    Structure(PvaStructureValue),
    StructureVarSizeArray(Vec<PvaStructureValue>),

    Union(PvaUnionValue),
    VariantUnion(PvaUnionValue),
    UnionVarSizeArray(Vec<PvaUnionValue>),
    VariantUnionVarSizeArray(Vec<PvaUnionValue>),
}

pub type PvaStructureValue = Vec<PvaValue>;

pub enum PvaUnionValue {
    Null,
    Selected {
        index: Option<usize>,
        name: Option<String>,
        value: Box<PvaValue>,
    },
}
