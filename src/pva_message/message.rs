use core::num;

use crate::pva_message::header::MsgEndian;

// ------------------- size -----------------

const PVA_SIZE_NULL: u8 = 0xff;
const PVA_SIZE_EXTENDED: u8 = 0xfe;
const PVA_SIZE_EXTENDED_MIN: usize = PVA_SIZE_EXTENDED as usize;
const PVA_SIZE_INT32_MAX: usize = i32::MAX as usize;

pub struct PvaSize {}

impl PvaSize {
    pub fn from_buf(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Option<usize>, String> {
        let first = match buf.get(*offset) {
            Some(first) => *first,
            None => {
                return Err(String::from(
                    "Warning: Remaining buffer too short for PVA size",
                ));
            }
        };

        match first {
            PVA_SIZE_NULL => {
                *offset += 1;
                Ok(None)
            }
            0..=253 => {
                *offset += 1;
                Ok(Some(first as usize))
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

                Ok(Some(size))
            }
        }
    }

    // read a PVA Size, returns Ok() only if the reading is successful and the size is not NULL
    fn from_buf_no_null(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<usize, String> {
        let size = PvaSize::from_buf(buf, offset, endian)?;
        let size = size.ok_or_else(|| String::from("Error: PVA type size is null"))?;

        Ok(size)
    }

    pub fn to_buf(size: usize, endian: MsgEndian) -> Result<Vec<u8>, String> {
        if size < PVA_SIZE_EXTENDED_MIN {
            return Ok(vec![size as u8]);
        }

        if size >= PVA_SIZE_INT32_MAX {
            return Err(String::from(
                "Error: PVA 64-bit size encoding is not implemented",
            ));
        }

        let mut buf = Vec::with_capacity(5);
        buf.push(PVA_SIZE_EXTENDED);

        let size =
            i32::try_from(size).map_err(|_| String::from("Error: PVA size does not fit in i32"))?;
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&size.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&size.to_be_bytes()),
        }

        Ok(buf)
    }

    pub fn null_to_buf() -> Vec<u8> {
        vec![PVA_SIZE_NULL]
    }
}

// ------------ buffer tools for basic types ----------------

pub trait PvaElement {
    fn append_to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String>;

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

impl PvaElement for bool {
    fn append_to_buf(&self, buf: &mut Vec<u8>, _endian: MsgEndian) -> Result<(), String> {
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
    fn append_to_buf(&self, buf: &mut Vec<u8>, _endian: MsgEndian) -> Result<(), String> {
        buf.push(*self);
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, _endian: MsgEndian) -> Result<Self, String> {
        Ok(read_n_bytes::<1>(buf, offset, "u8")?[0])
    }
}

impl PvaElement for i8 {
    fn append_to_buf(&self, buf: &mut Vec<u8>, _endian: MsgEndian) -> Result<(), String> {
        buf.push(*self as u8);
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, _endian: MsgEndian) -> Result<Self, String> {
        Ok(read_n_bytes::<1>(buf, offset, "i8")?[0] as i8)
    }
}

impl PvaElement for u16 {
    fn append_to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
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
    fn append_to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
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
    fn append_to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
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
    fn append_to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
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
    fn append_to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
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
    fn append_to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
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
    fn append_to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
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
    fn append_to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
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
    fn append_to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        buf.extend_from_slice(&PvaSize::to_buf(self.len(), endian)?);
        buf.extend_from_slice(self.as_bytes());
        Ok(())
    }

    fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String> {
        if *offset > buf.len() {
            return Err(String::from("Error: PVA string offset past end of buffer"));
        }

        let mut local_offset = *offset;
        let size = PvaSize::from_buf(buf, &mut local_offset, endian)?;
        let size = size.ok_or_else(|| String::from("Error: PVA string size is null"))?;
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

// ---------------- variable size array ---------------

pub struct PvaVarSizeArr<T: PvaElement> {
    pub arr: Vec<T>,
}

impl<T: PvaElement> PvaVarSizeArr<T> {
    pub fn arr(self: &Self) -> &[T] {
        &self.arr
    }

    pub fn arr_mut(self: &mut Self) -> &mut Vec<T> {
        &mut self.arr
    }

    pub fn to_buf(self: &Self, endian: MsgEndian) -> Result<Vec<u8>, String> {
        let mut buf = PvaSize::to_buf(self.arr().len(), endian)?;
        for element in self.arr() {
            element.append_to_buf(&mut buf, endian)?;
        }
        Ok(buf)
    }

    pub fn append_to_buf(
        self: &Self,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
    ) -> Result<usize, String> {
        let new_buf = match self.to_buf(endian) {
            Ok(new_buf) => new_buf,
            Err(err) => return Err(err),
        };
        buf.extend_from_slice(&new_buf);
        Ok(new_buf.len())
    }

    pub fn from_buf(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<PvaVarSizeArr<T>, String> {
        let offset_init = *offset;
        if offset_init > buf.len() {
            return Err("Error: PVA array offset past end of buffer".to_string());
        }

        let mut element_offset = offset_init;
        let size = PvaSize::from_buf(buf, &mut element_offset, endian)?;
        let size = size.ok_or_else(|| String::from("Error: PVA array size is null"))?;
        let mut arr: Vec<T> = Vec::with_capacity(size);

        for _ in 0..size {
            let element = T::from_buf(buf, &mut element_offset, endian)?;
            arr.push(element);
        }

        *offset = element_offset;

        Ok(PvaVarSizeArr { arr: arr })
    }
}

// --------------- PVA type --------------
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
    VariantUnion,                    // 0x82, 0b 100 00 010
    UnionVarSizeArray(PvaUnionType), // 0x89, 0b 100 01 001
    VariantUnionVarSizeArray,        // 0x8A, 0b 100 01 010
}

impl PvaType {
    /**
     * A struct buffer is composed of
     *
     * type code (u8) + ID string (PVA String) + number of fields (PVA Size) + array of fields
     *
     * where each field is composed of
     *
     * field name (PVA String) + field type (PVA Type)
     */
    pub fn from_buf_struct(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<PvaStructType, String> {
        // the 0x80 type code has been consumed

        // struct ID string, decode like variable size string
        let id = String::from_buf(buf, offset, endian)?;

        // number of fields, encoded as PvaSize
        let num_fields = PvaSize::from_buf_no_null(buf, offset, endian)?;

        // fields type: field name + pva type
        let mut fields: Vec<PvaFieldType> = vec![];
        for _ in 0..num_fields {
            // field name
            let name = String::from_buf(buf, offset, endian)?;

            // field type
            let pva_type = Self::from_buf(buf, offset, endian)?;

            fields.push(PvaFieldType {
                name: name,
                typ: pva_type,
            });
        }

        Ok(PvaStructType {
            id: id,
            fields: fields,
        })
    }

    /**
     * A regular union buffer is composed of
     *
     * type code (u8) + ID string (PVA String) + number of fields (PVA Size) + array of fields
     *
     * where each field is composed of
     *
     * field name (PVA String) + field type (PVA Type)
     */
    pub fn from_buf_union(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<PvaUnionType, String> {
        // the 0x81 type code has been consumed
        Self::from_buf_struct(buf, offset, endian)
    }

    pub fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String> {
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
            0x83 => PvaType::BoundString(PvaSize::from_buf_no_null(buf, offset, endian)?),

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

            0x82 => PvaType::VariantUnion,
            0x8A => PvaType::VariantUnionVarSizeArray,

            0x10 => PvaType::BooleanBoundArray(PvaSize::from_buf_no_null(buf, offset, endian)?),
            0x18 => PvaType::BooleanFixArray(PvaSize::from_buf_no_null(buf, offset, endian)?),

            0x30 => PvaType::ByteBoundArray(PvaSize::from_buf_no_null(buf, offset, endian)?),
            0x38 => PvaType::ByteFixArray(PvaSize::from_buf_no_null(buf, offset, endian)?),

            0x31 => PvaType::ShortBoundArray(PvaSize::from_buf_no_null(buf, offset, endian)?),
            0x39 => PvaType::ShortFixArray(PvaSize::from_buf_no_null(buf, offset, endian)?),

            0x32 => PvaType::IntBoundArray(PvaSize::from_buf_no_null(buf, offset, endian)?),
            0x3A => PvaType::IntFixArray(PvaSize::from_buf_no_null(buf, offset, endian)?),

            0x33 => PvaType::LongBoundArray(PvaSize::from_buf_no_null(buf, offset, endian)?),
            0x3B => PvaType::LongFixArray(PvaSize::from_buf_no_null(buf, offset, endian)?),

            0x34 => PvaType::UByteBoundArray(PvaSize::from_buf_no_null(buf, offset, endian)?),
            0x3C => PvaType::UByteFixArray(PvaSize::from_buf_no_null(buf, offset, endian)?),

            0x35 => PvaType::UShortBoundArray(PvaSize::from_buf_no_null(buf, offset, endian)?),
            0x3D => PvaType::UShortFixArray(PvaSize::from_buf_no_null(buf, offset, endian)?),

            0x36 => PvaType::UIntBoundArray(PvaSize::from_buf_no_null(buf, offset, endian)?),
            0x3E => PvaType::UIntFixArray(PvaSize::from_buf_no_null(buf, offset, endian)?),

            0x37 => PvaType::ULongBoundArray(PvaSize::from_buf_no_null(buf, offset, endian)?),
            0x3F => PvaType::ULongFixArray(PvaSize::from_buf_no_null(buf, offset, endian)?),

            0x52 => PvaType::FloatBoundArray(PvaSize::from_buf_no_null(buf, offset, endian)?),
            0x5A => PvaType::FloatFixArray(PvaSize::from_buf_no_null(buf, offset, endian)?),

            0x53 => PvaType::DoubleBoundArray(PvaSize::from_buf_no_null(buf, offset, endian)?),
            0x5B => PvaType::DoubleFixArray(PvaSize::from_buf_no_null(buf, offset, endian)?),

            0x70 => PvaType::StringBoundArray(PvaSize::from_buf_no_null(buf, offset, endian)?),
            0x78 => PvaType::StringFixArray(PvaSize::from_buf_no_null(buf, offset, endian)?),

            0x80 => PvaType::Structure(PvaType::from_buf_struct(buf, offset, endian)?),
            0x88 => PvaType::StructureVarSizeArray(PvaType::from_buf_struct(buf, offset, endian)?),

            0x81 => PvaType::Union(PvaType::from_buf_union(buf, offset, endian)?),
            0x89 => PvaType::UnionVarSizeArray(PvaType::from_buf_union(buf, offset, endian)?),

            _ => return Err(format!("Error: Invalid PVA type code 0x{code:02X}")),
        };

        Ok(pva_type)
    }
}

pub struct PvaStructType {
    pub id: String,                // e.g. timeStamp_t
    pub fields: Vec<PvaFieldType>, // e.g. [{name: "secondsPastEpoch", typ: PvaType::Long}, {name: "nanoSeconds", typ: PvaType::Int}, ]
}

pub struct PvaFieldType {
    pub name: String,
    pub typ: PvaType,
}

pub type PvaUnionType = PvaStructType;

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
