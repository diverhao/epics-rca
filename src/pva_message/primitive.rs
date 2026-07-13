use crate::pva_message::{header::MsgEndian, typ::PvaType};

const PVA_SIZE_NULL: u8 = 0xff;
const PVA_SIZE_EXTENDED: u8 = 0xfe;
const PVA_SIZE_EXTENDED_MIN: usize = PVA_SIZE_EXTENDED as usize;
const PVA_SIZE_INT32_MAX: usize = i32::MAX as usize;

// ------------ buffer tools for basic types ----------------

pub trait PvaElement {
    // actually append_to_buf()
    fn to_buf(&self, typ: &PvaType, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String>;

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Self, String>
    where
        Self: Sized;

    fn default_typ() -> Option<PvaType>
    where
        Self: Sized,
    {
        None
    }
}

pub trait PvaSize {
    // actually append_to_buf()
    fn to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String>;

    fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<Self, String>
    where
        Self: Sized;
}

// N is compile-time constant
pub fn read_n_bytes<const N: usize>(
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

impl PvaSize for usize {
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
                return Err("Failed to decode null (0xff) to a number".to_string());
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
    fn to_buf(&self, typ: &PvaType, buf: &mut Vec<u8>, _endian: MsgEndian) -> Result<(), String> {
        match typ {
            PvaType::Boolean => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        buf.push(u8::from(*self));
        Ok(())
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        _endian: MsgEndian,
    ) -> Result<Self, String> {
        match typ {
            PvaType::Boolean => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
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

    fn default_typ() -> Option<PvaType> {
        Some(PvaType::Boolean)
    }
}

impl PvaElement for u8 {
    fn to_buf(&self, typ: &PvaType, buf: &mut Vec<u8>, _endian: MsgEndian) -> Result<(), String> {
        match typ {
            PvaType::UByte => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        buf.push(*self);
        Ok(())
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        _endian: MsgEndian,
    ) -> Result<Self, String> {
        match typ {
            PvaType::UByte => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        Ok(read_n_bytes::<1>(buf, offset, "u8")?[0])
    }

    fn default_typ() -> Option<PvaType> {
        Some(PvaType::UByte)
    }
}

impl PvaElement for i8 {
    fn to_buf(&self, typ: &PvaType, buf: &mut Vec<u8>, _endian: MsgEndian) -> Result<(), String> {
        match typ {
            PvaType::Byte => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        buf.push(*self as u8);
        Ok(())
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        _endian: MsgEndian,
    ) -> Result<Self, String> {
        match typ {
            PvaType::Byte => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        Ok(read_n_bytes::<1>(buf, offset, "i8")?[0] as i8)
    }

    fn default_typ() -> Option<PvaType> {
        Some(PvaType::Byte)
    }
}

impl PvaElement for u16 {
    fn to_buf(&self, typ: &PvaType, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match typ {
            PvaType::UShort => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Self, String> {
        match typ {
            PvaType::UShort => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        let bytes = read_n_bytes::<2>(buf, offset, "u16")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }

    fn default_typ() -> Option<PvaType> {
        Some(PvaType::UShort)
    }
}

impl PvaElement for i16 {
    fn to_buf(&self, typ: &PvaType, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match typ {
            PvaType::Short => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Self, String> {
        match typ {
            PvaType::Short => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        let bytes = read_n_bytes::<2>(buf, offset, "i16")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }

    fn default_typ() -> Option<PvaType> {
        Some(PvaType::Short)
    }
}

impl PvaElement for u32 {
    fn to_buf(&self, typ: &PvaType, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match typ {
            PvaType::UInt => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Self, String> {
        match typ {
            PvaType::UInt => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        let bytes = read_n_bytes::<4>(buf, offset, "u32")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }

    fn default_typ() -> Option<PvaType> {
        Some(PvaType::UInt)
    }
}

impl PvaElement for i32 {
    fn to_buf(&self, typ: &PvaType, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match typ {
            PvaType::Int => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Self, String> {
        match typ {
            PvaType::Int => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        let bytes = read_n_bytes::<4>(buf, offset, "i32")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }

    fn default_typ() -> Option<PvaType> {
        Some(PvaType::Int)
    }
}

impl PvaElement for u64 {
    fn to_buf(&self, typ: &PvaType, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match typ {
            PvaType::ULong => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Self, String> {
        match typ {
            PvaType::ULong => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        let bytes = read_n_bytes::<8>(buf, offset, "u64")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }

    fn default_typ() -> Option<PvaType> {
        Some(PvaType::ULong)
    }
}

impl PvaElement for i64 {
    fn to_buf(&self, typ: &PvaType, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match typ {
            PvaType::Long => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Self, String> {
        match typ {
            PvaType::Long => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        let bytes = read_n_bytes::<8>(buf, offset, "i64")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }

    fn default_typ() -> Option<PvaType> {
        Some(PvaType::Long)
    }
}

impl PvaElement for f32 {
    fn to_buf(&self, typ: &PvaType, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match typ {
            PvaType::Float => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Self, String> {
        match typ {
            PvaType::Float => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        let bytes = read_n_bytes::<4>(buf, offset, "f32")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }

    fn default_typ() -> Option<PvaType> {
        Some(PvaType::Float)
    }
}

impl PvaElement for f64 {
    fn to_buf(&self, typ: &PvaType, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match typ {
            PvaType::Double => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        match endian {
            MsgEndian::Little => buf.extend_from_slice(&self.to_le_bytes()),
            MsgEndian::Big => buf.extend_from_slice(&self.to_be_bytes()),
        }
        Ok(())
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Self, String> {
        match typ {
            PvaType::Double => {}
            _ => return Err("PvaElement type not matched".to_string()),
        };
        let bytes = read_n_bytes::<8>(buf, offset, "f64")?;
        let value = match endian {
            MsgEndian::Little => Self::from_le_bytes(bytes),
            MsgEndian::Big => Self::from_be_bytes(bytes),
        };

        Ok(value)
    }

    fn default_typ() -> Option<PvaType> {
        Some(PvaType::Double)
    }
}

impl PvaElement for String {
    fn to_buf(&self, typ: &PvaType, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        match typ {
            PvaType::String => {}
            PvaType::BoundString(bound) => {
                if self.len() > *bound {
                    return Err(format!(
                        "PVA bound string length {} exceeds bound {}",
                        self.len(),
                        bound
                    ));
                }
            }
            _ => return Err("PvaElement type not matched".to_string()),
        };
        self.len().to_buf(buf, endian)?;
        buf.extend_from_slice(self.as_bytes());
        Ok(())
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Self, String> {
        let bound = match typ {
            PvaType::String => None,
            PvaType::BoundString(bound) => Some(*bound),
            _ => return Err("PvaElement type not matched".to_string()),
        };

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
        if let Some(bound) = bound {
            if value.len() > bound {
                return Err(format!(
                    "PVA bound string length {} exceeds bound {}",
                    value.len(),
                    bound
                ));
            }
        }
        local_offset = end;
        *offset = local_offset;

        Ok(value)
    }

    fn default_typ() -> Option<PvaType> {
        Some(PvaType::String)
    }
}

// -------------- array helpers -----------------------

pub fn var_size_array_to_buf<T: PvaElement>(
    values: &[T],
    buf: &mut Vec<u8>,
    endian: MsgEndian,
) -> Result<(), String> {
    let element_typ =
        T::default_typ().ok_or_else(|| "PVA array element type is not known".to_string())?;
    values.len().to_buf(buf, endian)?;
    for value in values {
        value.to_buf(&element_typ, buf, endian)?;
    }
    Ok(())
}

pub fn bounded_array_to_buf<T: PvaElement>(
    bound: usize,
    values: &[T],
    buf: &mut Vec<u8>,
    endian: MsgEndian,
) -> Result<(), String> {
    if values.len() > bound {
        return Err("Bounded array oversize".to_string());
    }
    let element_typ =
        T::default_typ().ok_or_else(|| "PVA array element type is not known".to_string())?;
    values.len().to_buf(buf, endian)?;
    for value in values {
        value.to_buf(&element_typ, buf, endian)?;
    }
    Ok(())
}

pub fn fixed_array_to_buf<T: PvaElement>(
    size: usize,
    values: &[T],
    buf: &mut Vec<u8>,
    endian: MsgEndian,
) -> Result<(), String> {
    if values.len() != size {
        return Err("Fixed size array not match".to_string());
    }
    let element_typ =
        T::default_typ().ok_or_else(|| "PVA array element type is not known".to_string())?;
    for value in values {
        value.to_buf(&element_typ, buf, endian)?;
    }
    Ok(())
}

pub fn array_from_buf<T: PvaElement>(
    len: usize,
    buf: &[u8],
    offset: &mut usize,
    endian: MsgEndian,
) -> Result<Vec<T>, String> {
    let element_typ =
        T::default_typ().ok_or_else(|| "PVA array element type is not known".to_string())?;
    let mut values = vec![];
    for _ in 0..len {
        values.push(T::from_buf(&element_typ, buf, offset, endian)?);
    }
    Ok(values)
}

pub fn var_array_from_buf<T: PvaElement>(
    buf: &[u8],
    offset: &mut usize,
    endian: MsgEndian,
) -> Result<Vec<T>, String> {
    let len = usize::from_buf(buf, offset, endian)?;
    array_from_buf(len, buf, offset, endian)
}

pub fn bound_array_from_buf<T: PvaElement>(
    bound: usize,
    element_type: &str,
    buf: &[u8],
    offset: &mut usize,
    endian: MsgEndian,
) -> Result<Vec<T>, String> {
    let len = usize::from_buf(buf, offset, endian)?;
    if len > bound {
        return Err(format!(
            "Error: PVA {element_type} bounded array length {len} exceeds bound {bound}"
        ));
    }
    array_from_buf(len, buf, offset, endian)
}
