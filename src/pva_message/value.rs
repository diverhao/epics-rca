// ---------------------- ?? ----------------------------------

use crate::pva_message::{
    header::MsgEndian,
    primitive::{PvaElement, PvaSize},
    typ::{
        PvaStructType,
        PvaType::{self, Boolean},
        PvaUnionType,
    },
};

pub enum PvaValue {
    Boolean(bool),       // 0x00, 0b 000 00 000
    Byte(i8),            // 0x20, 0b 001 00 000
    Short(i16),          // 0x21, 0b 001 00 001
    Int(i32),            // 0x22, 0b 001 00 010
    Long(i64),           // 0x23, 0b 001 00 011
    UByte(u8),           // 0x24, 0b 001 00 100
    UShort(u16),         // 0x25, 0b 001 00 101
    UInt(u32),           // 0x26, 0b 001 00 110
    ULong(u64),          // 0x27, 0b 001 00 111
    Float(f32),          // 0x42, 0b 010 00 010
    Double(f64),         // 0x43, 0b 010 00 011
    String(String),      // 0x60, 0b 011 00 000, size is in value
    BoundString(String), // 0x83, 0b 100 00 011, bound is in type, not in value, size is in value

    BooleanVarSizeArray(Vec<bool>), // 0x08, 0b 000 01 000, size is in value
    BooleanBoundArray(Vec<bool>), // 0x10, 0b 000 10 000, bound is in type, not in value, size is in value
    BooleanFixArray(Vec<bool>),   // 0x18, 0b 000 11 000, size is in type, not in value

    ByteVarSizeArray(Vec<i8>), // 0x28, 0b 001 01 000, size is in value
    ByteBoundArray(Vec<i8>), // 0x30, 0b 001 10 000, bound is in type, not in value, size is in value
    ByteFixArray(Vec<i8>),   // 0x38, 0b 001 11 000, size is in type, not in value

    ShortVarSizeArray(Vec<i16>), // 0x29, 0b 001 01 001, size is in value
    ShortBoundArray(Vec<i16>), // 0x31, 0b 001 10 001 bound is in type, not in value, size is in value
    ShortFixArray(Vec<i16>),   // 0x39, 0b 001 11 001 size is in type, not in value

    IntVarSizeArray(Vec<i32>), // 0x2A, 0b 001 01 010, size is in value
    IntBoundArray(Vec<i32>), // 0x32, 0b 001 10 010 bound is in type, not in value, size is in value
    IntFixArray(Vec<i32>),   // 0x3A, 0b 001 11 010 size is in type, not in value

    LongVarSizeArray(Vec<i64>), // 0x2B, 0b 001 01 011, size is in value
    LongBoundArray(Vec<i64>), // 0x33, 0b 001 10 011 bound is in type, not in value, size is in value
    LongFixArray(Vec<i64>),   // 0x3B, 0b 001 11 011 size is in type, not in value

    UByteVarSizeArray(Vec<u8>), // 0x2C, 0b 001 01 100, size is in value
    UByteBoundArray(Vec<u8>), // 0x34, 0b 001 10 100 bound is in type, not in value, size is in value
    UByteFixArray(Vec<u8>),   // 0x3C, 0b 001 11 100 size is in type, not in value

    UShortVarSizeArray(Vec<u16>), // 0x2D, 0b 001 01 101, size is in value
    UShortBoundArray(Vec<u16>), // 0x35, 0b 001 10 101 bound is in type, not in value, size is in value
    UShortFixArray(Vec<u16>),   // 0x3D, 0b 001 11 101 size is in type, not in value

    UIntVarSizeArray(Vec<u32>), // 0x2E, 0b 001 01 110, size is in value
    UIntBoundArray(Vec<u32>), // 0x36, 0b 001 10 110 bound is in type, not in value, size is in value
    UIntFixArray(Vec<u32>),   // 0x3E, 0b 001 11 110 size is in type, not in value

    ULongVarSizeArray(Vec<u64>), // 0x2F, 0b 001 01 111, size is in value
    ULongBoundArray(Vec<u64>), // 0x37, 0b 001 10 111 bound is in type, not in value, size is in value
    ULongFixArray(Vec<u64>),   // 0x3F, 0b 001 11 111 size is in type, not in value

    FloatVarSizeArray(Vec<f32>), // 0x4A, 0b 010 01 010, size is in value
    FloatBoundArray(Vec<f32>), // 0x52, 0b 010 10 010 bound is in type, not in value, size is in value
    FloatFixArray(Vec<f32>),   // 0x5A, 0b 010 11 010 size is in type, not in value

    DoubleVarSizeArray(Vec<f64>), // 0x4B, 0b 010 01 011, size is in value
    DoubleBoundArray(Vec<f64>), // 0x53, 0b 010 10 011 bound is in type, not in value, size is in value
    DoubleFixArray(Vec<f64>),   // 0x5B, 0b 010 11 011 size is in type, not in value

    StringVarSizeArray(Vec<String>), // 0x68, 0b 011 01 000, size is in value
    StringBoundArray(Vec<String>), // 0x70, 0b 011 10 000 bound is in type, not in value, size is in value
    StringFixArray(Vec<String>),   // 0x78, 0b 011 11 000 size is in type, not in value

    Struct(PvaStructValue),                          // 0x80, 0b 100 00 000
    StructVarSizeArray(Vec<Option<PvaStructValue>>), // 0x88, 0b 100 01 000

    Union(PvaUnionValue),                          // 0x81, 0b 100 00 001
    UnionVarSizeArray(Vec<Option<PvaUnionValue>>), // 0x89, 0b 100 01 001

    // todo: need to implement
    VariantUnion,             // 0x82, 0b 100 00 010
    VariantUnionVarSizeArray, // 0x8A, 0b 100 01 010
}

pub struct PvaStructValue {
    fields: Vec<PvaValue>,
}

pub struct PvaUnionValue {
    index: usize,
    field: Box<PvaValue>,
}

impl PvaElement for PvaStructValue {
    fn to_buf(
        self: &Self,
        typ: &PvaType,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
    ) -> Result<(), String> {
        let typ = match typ {
            PvaType::Struct(typ) => typ,
            _ => return Err("PVA struct value encoding requires PvaType::Struct".to_string()),
        };

        if self.fields.len() != typ.fields.len() {
            return Err(format!(
                "PVA struct value field count {} does not match type field count {}",
                self.fields.len(),
                typ.fields.len()
            ));
        }

        for (field, field_type) in self.fields.iter().zip(&typ.fields) {
            field.to_buf(field_type.typ.clone(), buf, endian)?;
        }
        Ok(())
    }

    // struct requires a type definition to decode the buffer
    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<PvaStructValue, String> {
        let typ: &PvaStructType = match typ {
            PvaType::Struct(typ) => typ,
            _ => return Err("PVA struct value decoding requires PvaType::Struct".to_string()),
        };

        let mut fields: Vec<PvaValue> = vec![];
        for field in &typ.fields {
            let value = PvaValue::from_buf(&field.typ, buf, offset, endian)?;
            fields.push(value);
        }
        Ok(PvaStructValue { fields: fields })
    }
}

impl PvaStructValue {
    fn var_array_from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Vec<Option<PvaStructValue>>, String> {
        let typ: &PvaStructType = match typ {
            PvaType::StructVarSizeArray(typ) => typ,
            _ => {
                return Err(
                    "PVA struct array decoding requires PvaType::StructVarSizeArray".to_string(),
                );
            }
        };

        let size = usize::from_buf(buf, offset, endian)?;
        let mut arr: Vec<Option<PvaStructValue>> = vec![];
        for ii in 0..size {
            let typ = typ.clone();
            // read existance byte
            let exist = bool::from_buf(&PvaType::Boolean, buf, offset, endian)?;
            if exist {
                arr.push(Some(PvaStructValue::from_buf(
                    &PvaType::Struct(typ),
                    buf,
                    offset,
                    endian,
                )?));
            } else {
                // do nothing
                arr.push(None);
            }
        }
        Ok(arr)
    }

    fn var_array_to_buf(
        typ: &PvaType,
        values: &[Option<PvaStructValue>],
        buf: &mut Vec<u8>,
        endian: MsgEndian,
    ) -> Result<(), String> {
        let typ: &PvaStructType = match typ {
            PvaType::StructVarSizeArray(typ) => typ,
            _ => {
                return Err(
                    "PVA struct array encoding requires PvaType::StructVarSizeArray".to_string(),
                );
            }
        };

        values.len().to_buf(buf, endian)?;
        for value in values {
            match value {
                Some(value) => {
                    let typ = typ.clone();
                    true.to_buf(&PvaType::Boolean, buf, endian)?;
                    value.to_buf(&PvaType::Struct(typ), buf, endian)?;
                }
                None => {
                    false.to_buf(&PvaType::Boolean, buf, endian)?;
                }
            }
        }
        Ok(())
    }
}

impl PvaElement for PvaUnionValue {
    fn to_buf(
        self: &Self,
        typ: &PvaType,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
    ) -> Result<(), String> {
        let typ = match typ {
            PvaType::Union(typ) => typ,
            _ => return Err("PVA union value encoding requires PvaType::Union".to_string()),
        };

        self.index.to_buf(buf, endian)?;
        let field_type = typ
            .fields
            .get(self.index)
            .ok_or_else(|| format!("Error: PVA union choice {} is out of range", self.index))?;
        self.field.to_buf(field_type.typ.clone(), buf, endian)
    }

    // union requires a type definition to decode the buffer
    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<PvaUnionValue, String> {
        let typ: &PvaUnionType = match typ {
            PvaType::Union(typ) => typ,
            _ => return Err("PVA union value decoding requires PvaType::Union".to_string()),
        };

        // choice value
        let index = usize::from_buf(buf, offset, endian)?;
        // one value
        let field_type = typ
            .fields
            .get(index)
            .ok_or_else(|| format!("Error: PVA union choice {index} is out of range"))?;

        let field = PvaValue::from_buf(&field_type.typ, buf, offset, endian)?;

        Ok(PvaUnionValue {
            index: index,
            field: Box::new(field),
        })
    }
}
impl PvaUnionValue {
    fn var_array_from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Vec<Option<PvaUnionValue>>, String> {
        let typ = match typ {
            PvaType::UnionVarSizeArray(typ) => typ,
            _ => {
                return Err(
                    "PVA union array decoding requires PvaType::UnionVarSizeArray".to_string(),
                );
            }
        };

        let size = usize::from_buf(buf, offset, endian)?;
        let mut arr: Vec<Option<PvaUnionValue>> = vec![];
        for ii in 0..size {
            // read exitence byte
            let exist = bool::from_buf(&PvaType::Boolean, buf, offset, endian)?;
            if exist {
                let typ = typ.clone();
                arr.push(Some(PvaUnionValue::from_buf(
                    &PvaType::Union(typ),
                    buf,
                    offset,
                    endian,
                )?));
            } else {
                arr.push(None);
            }
        }
        Ok(arr)
    }

    fn var_array_to_buf(
        typ: &PvaType,
        values: &[Option<PvaUnionValue>],
        buf: &mut Vec<u8>,
        endian: MsgEndian,
    ) -> Result<(), String> {
        let typ = match typ {
            PvaType::UnionVarSizeArray(typ) => typ,
            _ => {
                return Err(
                    "PVA union array encoding requires PvaType::UnionVarSizeArray".to_string(),
                );
            }
        };

        values.len().to_buf(buf, endian)?;
        for value in values {
            match value {
                Some(value) => {
                    let typ = typ.clone();
                    // write 1 to indicate this element exists
                    true.to_buf(&PvaType::Boolean, buf, endian)?;
                    value.to_buf(&PvaType::Union(typ), buf, endian)?;
                }
                None => {
                    false.to_buf(&PvaType::Boolean, buf, endian)?;
                }
            }
        }
        Ok(())
    }
}

fn var_size_array_to_buf<T: PvaElement>(
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

fn bounded_array_to_buf<T: PvaElement>(
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

fn fixed_array_to_buf<T: PvaElement>(
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

fn array_from_buf<T: PvaElement>(
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

fn var_array_from_buf<T: PvaElement>(
    buf: &[u8],
    offset: &mut usize,
    endian: MsgEndian,
) -> Result<Vec<T>, String> {
    let len = usize::from_buf(buf, offset, endian)?;
    array_from_buf(len, buf, offset, endian)
}

fn bound_array_from_buf<T: PvaElement>(
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

impl PvaValue {
    // actually append_to_buf()
    // Container variants still compare PvaValue and PvaType here.
    pub fn to_buf(
        self: &Self,
        typ: PvaType,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
    ) -> Result<(), String> {
        match self {
            PvaValue::Boolean(value) => value.to_buf(&typ, buf, endian),
            PvaValue::Byte(value) => value.to_buf(&typ, buf, endian),
            PvaValue::Short(value) => value.to_buf(&typ, buf, endian),
            PvaValue::Int(value) => value.to_buf(&typ, buf, endian),
            PvaValue::Long(value) => value.to_buf(&typ, buf, endian),
            PvaValue::UByte(value) => value.to_buf(&typ, buf, endian),
            PvaValue::UShort(value) => value.to_buf(&typ, buf, endian),
            PvaValue::UInt(value) => value.to_buf(&typ, buf, endian),
            PvaValue::ULong(value) => value.to_buf(&typ, buf, endian),
            PvaValue::Float(value) => value.to_buf(&typ, buf, endian),
            PvaValue::Double(value) => value.to_buf(&typ, buf, endian),
            PvaValue::String(value) => match typ {
                PvaType::String => value.to_buf(&typ, buf, endian),
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },
            PvaValue::BoundString(value) => match typ {
                PvaType::BoundString(bound) => {
                    if value.len() > bound {
                        return Err(format!(
                            "PVA bound string length {} exceeds bound {}",
                            value.len(),
                            bound
                        ));
                    }
                    value.to_buf(&PvaType::BoundString(bound), buf, endian)
                }
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },

            PvaValue::BooleanVarSizeArray(values) => match typ {
                PvaType::BooleanVarSizeArray => var_size_array_to_buf(values, buf, endian),
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },
            PvaValue::BooleanBoundArray(values) => {
                let bound: usize = match typ {
                    PvaType::BooleanBoundArray(bound) => bound,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                bounded_array_to_buf(bound, values, buf, endian)
            }
            PvaValue::BooleanFixArray(values) => {
                let size: usize = match typ {
                    PvaType::BooleanFixArray(size) => size,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                fixed_array_to_buf(size, values, buf, endian)
            }

            PvaValue::ByteVarSizeArray(values) => match typ {
                PvaType::ByteVarSizeArray => var_size_array_to_buf(values, buf, endian),
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },
            PvaValue::ByteBoundArray(values) => {
                let bound: usize = match typ {
                    PvaType::ByteBoundArray(bound) => bound,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                bounded_array_to_buf(bound, values, buf, endian)
            }
            PvaValue::ByteFixArray(values) => {
                let size: usize = match typ {
                    PvaType::ByteFixArray(size) => size,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                fixed_array_to_buf(size, values, buf, endian)
            }

            PvaValue::ShortVarSizeArray(values) => match typ {
                PvaType::ShortVarSizeArray => var_size_array_to_buf(values, buf, endian),
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },
            PvaValue::ShortBoundArray(values) => {
                let bound: usize = match typ {
                    PvaType::ShortBoundArray(bound) => bound,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                bounded_array_to_buf(bound, values, buf, endian)
            }
            PvaValue::ShortFixArray(values) => {
                let size: usize = match typ {
                    PvaType::ShortFixArray(size) => size,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                fixed_array_to_buf(size, values, buf, endian)
            }

            PvaValue::IntVarSizeArray(values) => match typ {
                PvaType::IntVarSizeArray => var_size_array_to_buf(values, buf, endian),
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },
            PvaValue::IntBoundArray(values) => {
                let bound: usize = match typ {
                    PvaType::IntBoundArray(bound) => bound,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                bounded_array_to_buf(bound, values, buf, endian)
            }
            PvaValue::IntFixArray(values) => {
                let size: usize = match typ {
                    PvaType::IntFixArray(size) => size,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                fixed_array_to_buf(size, values, buf, endian)
            }

            PvaValue::LongVarSizeArray(values) => match typ {
                PvaType::LongVarSizeArray => var_size_array_to_buf(values, buf, endian),
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },
            PvaValue::LongBoundArray(values) => {
                let bound: usize = match typ {
                    PvaType::LongBoundArray(bound) => bound,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                bounded_array_to_buf(bound, values, buf, endian)
            }
            PvaValue::LongFixArray(values) => {
                let size: usize = match typ {
                    PvaType::LongFixArray(size) => size,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                fixed_array_to_buf(size, values, buf, endian)
            }

            PvaValue::UByteVarSizeArray(values) => match typ {
                PvaType::UByteVarSizeArray => var_size_array_to_buf(values, buf, endian),
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },
            PvaValue::UByteBoundArray(values) => {
                let bound: usize = match typ {
                    PvaType::UByteBoundArray(bound) => bound,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                bounded_array_to_buf(bound, values, buf, endian)
            }
            PvaValue::UByteFixArray(values) => {
                let size: usize = match typ {
                    PvaType::UByteFixArray(size) => size,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                fixed_array_to_buf(size, values, buf, endian)
            }

            PvaValue::UShortVarSizeArray(values) => match typ {
                PvaType::UShortVarSizeArray => var_size_array_to_buf(values, buf, endian),
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },
            PvaValue::UShortBoundArray(values) => {
                let bound: usize = match typ {
                    PvaType::UShortBoundArray(bound) => bound,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                bounded_array_to_buf(bound, values, buf, endian)
            }
            PvaValue::UShortFixArray(values) => {
                let size: usize = match typ {
                    PvaType::UShortFixArray(size) => size,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                fixed_array_to_buf(size, values, buf, endian)
            }

            PvaValue::UIntVarSizeArray(values) => match typ {
                PvaType::UIntVarSizeArray => var_size_array_to_buf(values, buf, endian),
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },
            PvaValue::UIntBoundArray(values) => {
                let bound: usize = match typ {
                    PvaType::UIntBoundArray(bound) => bound,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                bounded_array_to_buf(bound, values, buf, endian)
            }
            PvaValue::UIntFixArray(values) => {
                let size: usize = match typ {
                    PvaType::UIntFixArray(size) => size,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                fixed_array_to_buf(size, values, buf, endian)
            }

            PvaValue::ULongVarSizeArray(values) => match typ {
                PvaType::ULongVarSizeArray => var_size_array_to_buf(values, buf, endian),
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },
            PvaValue::ULongBoundArray(values) => {
                let bound: usize = match typ {
                    PvaType::ULongBoundArray(bound) => bound,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                bounded_array_to_buf(bound, values, buf, endian)
            }
            PvaValue::ULongFixArray(values) => {
                let size: usize = match typ {
                    PvaType::ULongFixArray(size) => size,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                fixed_array_to_buf(size, values, buf, endian)
            }

            PvaValue::FloatVarSizeArray(values) => match typ {
                PvaType::FloatVarSizeArray => var_size_array_to_buf(values, buf, endian),
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },
            PvaValue::FloatBoundArray(values) => {
                let bound: usize = match typ {
                    PvaType::FloatBoundArray(bound) => bound,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                bounded_array_to_buf(bound, values, buf, endian)
            }
            PvaValue::FloatFixArray(values) => {
                let size: usize = match typ {
                    PvaType::FloatFixArray(size) => size,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                fixed_array_to_buf(size, values, buf, endian)
            }

            PvaValue::DoubleVarSizeArray(values) => match typ {
                PvaType::DoubleVarSizeArray => var_size_array_to_buf(values, buf, endian),
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },
            PvaValue::DoubleBoundArray(values) => {
                let bound: usize = match typ {
                    PvaType::DoubleBoundArray(bound) => bound,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                bounded_array_to_buf(bound, values, buf, endian)
            }
            PvaValue::DoubleFixArray(values) => {
                let size: usize = match typ {
                    PvaType::DoubleFixArray(size) => size,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                fixed_array_to_buf(size, values, buf, endian)
            }

            PvaValue::StringVarSizeArray(values) => match typ {
                PvaType::StringVarSizeArray => var_size_array_to_buf(values, buf, endian),
                _ => Err("PvaValue failed to encode: type not match".to_string()),
            },
            PvaValue::StringBoundArray(values) => {
                let bound: usize = match typ {
                    PvaType::StringBoundArray(bound) => bound,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                bounded_array_to_buf(bound, values, buf, endian)
            }
            PvaValue::StringFixArray(values) => {
                let size: usize = match typ {
                    PvaType::StringFixArray(size) => size,
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                fixed_array_to_buf(size, values, buf, endian)
            }

            PvaValue::Struct(value) => {
                match typ {
                    PvaType::Struct(_) => {}
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                value.to_buf(&typ, buf, endian)
            }

            PvaValue::StructVarSizeArray(values) => {
                match typ {
                    PvaType::StructVarSizeArray(_) => {}
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                PvaStructValue::var_array_to_buf(&typ, values, buf, endian)
            }

            PvaValue::Union(value) => {
                match typ {
                    PvaType::Union(_) => {}
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                value.to_buf(&typ, buf, endian)
            }

            PvaValue::UnionVarSizeArray(values) => {
                match typ {
                    PvaType::UnionVarSizeArray(_) => {}
                    _ => return Err("PvaValue failed to encode: type not match".to_string()),
                };
                PvaUnionValue::var_array_to_buf(&typ, values, buf, endian)
            }

            // todo: implement it
            PvaValue::VariantUnion | PvaValue::VariantUnionVarSizeArray => {
                return Err("Variant union value encoding not implemented".to_string());
            }
        }?;
        Ok(())
    }

    pub fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<Self, String> {
        let result = match typ {
            PvaType::Boolean => {
                PvaValue::Boolean(bool::from_buf(&PvaType::Boolean, buf, offset, endian)?)
            }
            PvaType::Byte => PvaValue::Byte(i8::from_buf(&PvaType::Byte, buf, offset, endian)?),
            PvaType::Short => PvaValue::Short(i16::from_buf(&PvaType::Short, buf, offset, endian)?),
            PvaType::Int => PvaValue::Int(i32::from_buf(&PvaType::Int, buf, offset, endian)?),
            PvaType::Long => PvaValue::Long(i64::from_buf(&PvaType::Long, buf, offset, endian)?),
            PvaType::UByte => PvaValue::UByte(u8::from_buf(&PvaType::UByte, buf, offset, endian)?),
            PvaType::UShort => {
                PvaValue::UShort(u16::from_buf(&PvaType::UShort, buf, offset, endian)?)
            }
            PvaType::UInt => PvaValue::UInt(u32::from_buf(&PvaType::UInt, buf, offset, endian)?),
            PvaType::ULong => PvaValue::ULong(u64::from_buf(&PvaType::ULong, buf, offset, endian)?),
            PvaType::Float => PvaValue::Float(f32::from_buf(&PvaType::Float, buf, offset, endian)?),
            PvaType::Double => {
                PvaValue::Double(f64::from_buf(&PvaType::Double, buf, offset, endian)?)
            }
            PvaType::String => {
                PvaValue::String(String::from_buf(&PvaType::String, buf, offset, endian)?)
            }
            PvaType::BoundString(bound) => {
                let value = String::from_buf(&PvaType::BoundString(*bound), buf, offset, endian)?;
                if value.len() > *bound {
                    return Err(format!(
                        "Error: PVA bound string length {} exceeds bound {}",
                        value.len(),
                        bound
                    ));
                }
                PvaValue::BoundString(value)
            }

            PvaType::BooleanVarSizeArray => {
                PvaValue::BooleanVarSizeArray(var_array_from_buf(buf, offset, endian)?)
            }
            PvaType::BooleanBoundArray(bound) => PvaValue::BooleanBoundArray(bound_array_from_buf(
                *bound, "boolean", buf, offset, endian,
            )?),
            PvaType::BooleanFixArray(len) => {
                PvaValue::BooleanFixArray(array_from_buf(*len, buf, offset, endian)?)
            }

            PvaType::ByteVarSizeArray => {
                PvaValue::ByteVarSizeArray(var_array_from_buf(buf, offset, endian)?)
            }
            PvaType::ByteBoundArray(bound) => {
                PvaValue::ByteBoundArray(bound_array_from_buf(*bound, "byte", buf, offset, endian)?)
            }
            PvaType::ByteFixArray(len) => {
                PvaValue::ByteFixArray(array_from_buf(*len, buf, offset, endian)?)
            }

            PvaType::ShortVarSizeArray => {
                PvaValue::ShortVarSizeArray(var_array_from_buf(buf, offset, endian)?)
            }
            PvaType::ShortBoundArray(bound) => PvaValue::ShortBoundArray(bound_array_from_buf(
                *bound, "short", buf, offset, endian,
            )?),
            PvaType::ShortFixArray(len) => {
                PvaValue::ShortFixArray(array_from_buf(*len, buf, offset, endian)?)
            }

            PvaType::IntVarSizeArray => {
                PvaValue::IntVarSizeArray(var_array_from_buf(buf, offset, endian)?)
            }
            PvaType::IntBoundArray(bound) => {
                PvaValue::IntBoundArray(bound_array_from_buf(*bound, "int", buf, offset, endian)?)
            }
            PvaType::IntFixArray(len) => {
                PvaValue::IntFixArray(array_from_buf(*len, buf, offset, endian)?)
            }

            PvaType::LongVarSizeArray => {
                PvaValue::LongVarSizeArray(var_array_from_buf(buf, offset, endian)?)
            }
            PvaType::LongBoundArray(bound) => {
                PvaValue::LongBoundArray(bound_array_from_buf(*bound, "long", buf, offset, endian)?)
            }
            PvaType::LongFixArray(len) => {
                PvaValue::LongFixArray(array_from_buf(*len, buf, offset, endian)?)
            }

            PvaType::UByteVarSizeArray => {
                PvaValue::UByteVarSizeArray(var_array_from_buf(buf, offset, endian)?)
            }
            PvaType::UByteBoundArray(bound) => PvaValue::UByteBoundArray(bound_array_from_buf(
                *bound, "ubyte", buf, offset, endian,
            )?),
            PvaType::UByteFixArray(len) => {
                PvaValue::UByteFixArray(array_from_buf(*len, buf, offset, endian)?)
            }

            PvaType::UShortVarSizeArray => {
                PvaValue::UShortVarSizeArray(var_array_from_buf(buf, offset, endian)?)
            }
            PvaType::UShortBoundArray(bound) => PvaValue::UShortBoundArray(bound_array_from_buf(
                *bound, "ushort", buf, offset, endian,
            )?),
            PvaType::UShortFixArray(len) => {
                PvaValue::UShortFixArray(array_from_buf(*len, buf, offset, endian)?)
            }

            PvaType::UIntVarSizeArray => {
                PvaValue::UIntVarSizeArray(var_array_from_buf(buf, offset, endian)?)
            }
            PvaType::UIntBoundArray(bound) => {
                PvaValue::UIntBoundArray(bound_array_from_buf(*bound, "uint", buf, offset, endian)?)
            }
            PvaType::UIntFixArray(len) => {
                PvaValue::UIntFixArray(array_from_buf(*len, buf, offset, endian)?)
            }

            PvaType::ULongVarSizeArray => {
                PvaValue::ULongVarSizeArray(var_array_from_buf(buf, offset, endian)?)
            }
            PvaType::ULongBoundArray(bound) => PvaValue::ULongBoundArray(bound_array_from_buf(
                *bound, "ulong", buf, offset, endian,
            )?),
            PvaType::ULongFixArray(len) => {
                PvaValue::ULongFixArray(array_from_buf(*len, buf, offset, endian)?)
            }

            PvaType::FloatVarSizeArray => {
                PvaValue::FloatVarSizeArray(var_array_from_buf(buf, offset, endian)?)
            }
            PvaType::FloatBoundArray(bound) => PvaValue::FloatBoundArray(bound_array_from_buf(
                *bound, "float", buf, offset, endian,
            )?),
            PvaType::FloatFixArray(len) => {
                PvaValue::FloatFixArray(array_from_buf(*len, buf, offset, endian)?)
            }

            PvaType::DoubleVarSizeArray => {
                PvaValue::DoubleVarSizeArray(var_array_from_buf(buf, offset, endian)?)
            }
            PvaType::DoubleBoundArray(bound) => PvaValue::DoubleBoundArray(bound_array_from_buf(
                *bound, "double", buf, offset, endian,
            )?),
            PvaType::DoubleFixArray(len) => {
                PvaValue::DoubleFixArray(array_from_buf(*len, buf, offset, endian)?)
            }

            PvaType::StringVarSizeArray => {
                PvaValue::StringVarSizeArray(var_array_from_buf(buf, offset, endian)?)
            }
            PvaType::StringBoundArray(bound) => PvaValue::StringBoundArray(bound_array_from_buf(
                *bound, "string", buf, offset, endian,
            )?),
            PvaType::StringFixArray(len) => {
                PvaValue::StringFixArray(array_from_buf(*len, buf, offset, endian)?)
            }

            PvaType::Struct(_) => {
                PvaValue::Struct(PvaStructValue::from_buf(&typ, buf, offset, endian)?)
            }

            PvaType::StructVarSizeArray(_) => PvaValue::StructVarSizeArray(
                PvaStructValue::var_array_from_buf(&typ, buf, offset, endian)?,
            ),

            PvaType::Union(_) => {
                PvaValue::Union(PvaUnionValue::from_buf(&typ, buf, offset, endian)?)
            }

            PvaType::UnionVarSizeArray(_) => PvaValue::UnionVarSizeArray(
                PvaUnionValue::var_array_from_buf(&typ, buf, offset, endian)?,
            ),

            PvaType::VariantUnion | PvaType::VariantUnionVarSizeArray => {
                return Err("Variant union value decoder not implemented".to_string());
            }
        };
        Ok(result)
    }
}
