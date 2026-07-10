use crate::pva_message::{header::MsgEndian, primitive::{PvaElement, PvaSize}, typ::{PvaStructType, PvaType, PvaUnionType}, type_registry::PvaTypeRegistry, value::PvaValue, value_validation::validate_pva_value_type};


pub struct PvaStructValue {
    pub(crate) fields: Vec<PvaValue>,
}

pub enum PvaUnionValue {
    Null,
    Selected { index: usize, field: Box<PvaValue> },
}

pub enum PvaVariantUnionValue {
    Null,
    Selected { typ: PvaType, value: Box<PvaValue> },
}

impl PvaElement for PvaVariantUnionValue {
    fn to_buf(
        self: &Self,
        typ: &PvaType,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
    ) -> Result<(), String> {
        match typ {
            PvaType::VariantUnion => {}
            _ => return Err("Must a variant union type".to_string()),
        };

        match self {
            PvaVariantUnionValue::Null => {
                buf.push(0xff);
                return Ok(());
            }
            PvaVariantUnionValue::Selected { typ, value } => {
                if matches!(typ, PvaType::Null) || matches!(value.as_ref(), PvaValue::Null) {
                    return Err(
                        "A selected PVA variant union cannot contain a null type or value"
                            .to_string(),
                    );
                }
                validate_pva_value_type(value, typ)?;

                // encode the type, no caching
                typ.to_buf(buf, endian)?;
                // encode the value
                value.to_buf(typ.clone(), buf, endian)?;
                return Ok(());
            }
        }
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<PvaVariantUnionValue, String> {
        match typ {
            PvaType::VariantUnion => {}
            _ => return Err("Must a variant union type".to_string()),
        };

        let first_byte = buf.get(*offset);
        let first_byte = match first_byte {
            Some(first_byte) => first_byte,
            None => return Err("First byte error".to_string()),
        };

        if *first_byte == 0xff {
            *offset = *offset + 1;
            return Ok(PvaVariantUnionValue::Null);
        } else {
            // decode type
            // use a dummy pva type registry
            let registry = &mut PvaTypeRegistry::new();
            let typ = PvaType::from_buf(buf, offset, endian, registry)?;
            // decode value
            let value = PvaValue::from_buf(&typ, buf, offset, endian)?;
            return Ok(PvaVariantUnionValue::Selected {
                typ,
                value: Box::new(value),
            });
        }
    }
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
    pub fn var_array_from_buf(
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

    pub fn var_array_to_buf(
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

        match self {
            PvaUnionValue::Null => {
                buf.push(0xff);
                return Ok(());
            }
            PvaUnionValue::Selected { index, field } => {
                let field_type = typ
                    .fields
                    .get(*index)
                    .ok_or_else(|| format!("Error: PVA union choice {} is out of range", index))?;
                index.to_buf(buf, endian)?;
                return field.to_buf(field_type.typ.clone(), buf, endian);
            }
        }
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

        // read the first byte
        match buf.get(*offset) {
            Some(first_byte) => {
                if *first_byte == 0xff {
                    *offset += 1;
                    return Ok(PvaUnionValue::Null);
                } else {
                    // choice value
                    let index = usize::from_buf(buf, offset, endian)?;
                    // one value
                    let field_type = typ.fields.get(index).ok_or_else(|| {
                        format!("Error: PVA union choice {index} is out of range")
                    })?;

                    let field = PvaValue::from_buf(&field_type.typ, buf, offset, endian)?;

                    return Ok(PvaUnionValue::Selected {
                        index: index,
                        field: Box::new(field),
                    });
                }
            }
            None => return Err("Remaining buffer too short for PVA union selector".to_string()),
        };
    }
}
impl PvaUnionValue {
    pub fn var_array_from_buf(
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

    pub fn var_array_to_buf(
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