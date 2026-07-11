use crate::pva_message::{
    header::MsgEndian,
    primitive::{PvaElement, PvaSize},
    typ::PvaType,
    type_registry::PvaTypeRegistry,
    value::PvaValue,
    value_validation::validate_pva_value_type,
};

// ---------------- trait -------------------

pub trait PvaComplexType {
    // actually append_to_buf()
    fn to_buf(
        &self,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<(), String>;

    fn from_buf(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
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

pub trait PvaComplexValue {
    // actually append_to_buf()
    fn to_buf(
        &self,
        typ: &PvaType,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<(), String>;

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
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

// ---------------- struct type -------------

#[derive(Debug, Clone)]
pub struct PvaStructType {
    pub id: String,                // e.g. timeStamp_t
    pub fields: Vec<PvaFieldType>, // e.g. [{name: "secondsPastEpoch", typ: PvaType::Long}, {name: "nanoSeconds", typ: PvaType::Int}, ]
}

impl PvaComplexType for PvaStructType {
    fn to_buf(
        self: &Self,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<(), String> {
        // code 0x80
        buf.push(0x80);

        // struct ID string
        self.id.to_buf(&PvaType::String, buf, endian)?;

        // number of fields
        self.fields.len().to_buf(buf, endian)?;

        // field types
        for field_type in &self.fields {
            field_type.to_buf(buf, endian, registry)?;
        }

        Ok(())
    }

    fn from_buf(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<PvaStructType, String> {
        // consume and verify 0x80
        let code = u8::from_buf(&PvaType::UByte, buf, offset, endian)?;
        if code != 0x80 {
            return Err("Error decoding struct type, code is not 0x80".to_string());
        }

        // struct ID string, decode like variable size string
        let id = String::from_buf(&PvaType::String, buf, offset, endian)?;

        // number of fields, encoded as PvaSize
        let num_fields = usize::from_buf(buf, offset, endian)?;

        // fields type: field name + pva type
        let mut fields: Vec<PvaFieldType> = vec![];
        for _ in 0..num_fields {
            let field_type = PvaFieldType::from_buf(buf, offset, endian, registry)?;
            fields.push(field_type);
        }

        Ok(PvaStructType {
            id: id,
            fields: fields,
        })
    }
}

// ------------ struct value ----------------

pub struct PvaStructValue {
    pub(crate) fields: Vec<PvaValue>,
}

impl PvaComplexValue for PvaStructValue {
    fn to_buf(
        self: &Self,
        typ: &PvaType,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
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
            field.to_buf(field_type.typ.clone(), buf, endian, registry)?;
        }
        Ok(())
    }

    // struct requires a type definition to decode the buffer
    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<PvaStructValue, String> {
        let typ: &PvaStructType = match typ {
            PvaType::Struct(typ) => typ,
            _ => return Err("PVA struct value decoding requires PvaType::Struct".to_string()),
        };

        let mut fields: Vec<PvaValue> = vec![];
        for field in &typ.fields {
            let value = PvaValue::from_buf(&field.typ, buf, offset, endian, registry)?;
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
        registry: &mut PvaTypeRegistry,
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
                    registry,
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
        registry: &mut PvaTypeRegistry,
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
                    value.to_buf(&PvaType::Struct(typ), buf, endian, registry)?;
                }
                None => {
                    false.to_buf(&PvaType::Boolean, buf, endian)?;
                }
            }
        }
        Ok(())
    }
}

// ------------- struct/union field type --------------
#[derive(Debug, Clone)]
pub struct PvaFieldType {
    pub name: String,
    pub typ: PvaType,
}

impl PvaComplexType for PvaFieldType {
    fn to_buf(
        self: &Self,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<(), String> {
        let name = &self.name;
        name.to_buf(&PvaType::String, buf, endian)?;

        let typ = &self.typ;
        typ.to_buf(buf, endian, registry)?;
        Ok(())
    }

    fn from_buf(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<Self, String> {
        // field name
        let name = String::from_buf(&PvaType::String, buf, offset, endian)?;

        // field type
        let typ = PvaType::from_buf(buf, offset, endian, registry)?;

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

impl PvaComplexType for PvaUnionType {
    fn to_buf(
        self: &Self,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<(), String> {
        // code 0x81
        buf.push(0x81);

        // union ID string
        self.id.to_buf(&PvaType::String, buf, endian)?;

        // number of fields
        self.fields.len().to_buf(buf, endian)?;

        // field types
        for field_type in &self.fields {
            field_type.to_buf(buf, endian, registry)?;
        }

        Ok(())
    }

    fn from_buf(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<PvaUnionType, String> {
        // consume 0x81
        let code = u8::from_buf(&PvaType::UByte, buf, offset, endian)?;
        if code != 0x81 {
            return Err("Error decoding union type, code is not 0x81".to_string());
        }

        // union ID string, decode like variable size string
        let id = String::from_buf(&PvaType::String, buf, offset, endian)?;

        // number of fields, encoded as PvaSize
        let num_fields = usize::from_buf(buf, offset, endian)?;

        // fields type: field name + pva type
        let mut fields: Vec<PvaFieldType> = vec![];
        for _ in 0..num_fields {
            let field_type = PvaFieldType::from_buf(buf, offset, endian, registry)?;
            fields.push(field_type);
        }

        Ok(PvaUnionType {
            id: id,
            fields: fields,
        })
    }
}

// ------------ union value ----------------

pub enum PvaUnionValue {
    Null,
    Selected { index: usize, field: Box<PvaValue> },
}

impl PvaComplexValue for PvaUnionValue {
    fn to_buf(
        self: &Self,
        typ: &PvaType,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
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
                return field.to_buf(field_type.typ.clone(), buf, endian, registry);
            }
        }
    }

    // union requires a type definition to decode the buffer
    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<PvaUnionValue, String> {
        // 0x81 is already consumed

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

                    let field = PvaValue::from_buf(&field_type.typ, buf, offset, endian, registry)?;

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
        registry: &mut PvaTypeRegistry,
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
                    registry,
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
        registry: &mut PvaTypeRegistry,
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
                    value.to_buf(&PvaType::Union(typ), buf, endian, registry)?;
                }
                None => {
                    false.to_buf(&PvaType::Boolean, buf, endian)?;
                }
            }
        }
        Ok(())
    }
}

// ------------ variant union value ----------------

pub enum PvaVariantUnionValue {
    Null,
    Selected { typ: PvaType, value: Box<PvaValue> },
}

impl PvaComplexValue for PvaVariantUnionValue {
    fn to_buf(
        self: &Self,
        typ: &PvaType,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
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
                typ.to_buf(buf, endian, registry)?;
                // encode the value
                value.to_buf(typ.clone(), buf, endian, registry)?;
                return Ok(());
            }
        }
    }

    fn from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<PvaVariantUnionValue, String> {
        // 0x82 is already consumed

        // check input type
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
            // Null variant
            *offset = *offset + 1;
            return Ok(PvaVariantUnionValue::Null);
        } else {
            // decode type
            let typ = PvaType::from_buf(buf, offset, endian, registry)?;
            // decode value
            let value = PvaValue::from_buf(&typ, buf, offset, endian, registry)?;
            return Ok(PvaVariantUnionValue::Selected {
                typ,
                value: Box::new(value),
            });
        }
    }
}

impl PvaVariantUnionValue {
    pub fn var_array_from_buf(
        typ: &PvaType,
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<Vec<Option<PvaVariantUnionValue>>, String> {
        match typ {
            PvaType::VariantUnionVarSizeArray => {}
            _ => {
                return Err(
                    "PVA variant union array decoding requires PvaType::VariantUnionVarSizeArray"
                        .to_string(),
                );
            }
        };

        let size = usize::from_buf(buf, offset, endian)?;
        let mut arr: Vec<Option<PvaVariantUnionValue>> = vec![];
        for ii in 0..size {
            // read existance byte
            let exist = bool::from_buf(&PvaType::Boolean, buf, offset, endian)?;
            if exist {
                arr.push(Some(PvaVariantUnionValue::from_buf(
                    &PvaType::VariantUnion,
                    buf,
                    offset,
                    endian,
                    registry,
                )?));
            } else {
                // element does not exist
                arr.push(None);
            }
        }
        Ok(arr)
    }

    pub fn var_array_to_buf(
        typ: &PvaType,
        values: &[Option<PvaVariantUnionValue>],
        buf: &mut Vec<u8>,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<(), String> {
        match typ {
            PvaType::VariantUnionVarSizeArray => {}
            _ => {
                return Err(
                    "PVA variant union array encoding requires PvaType::VariantUnionVarSizeArray"
                        .to_string(),
                );
            }
        };

        values.len().to_buf(buf, endian)?;
        for value in values {
            match value {
                Some(value) => {
                    let typ = typ.clone();
                    true.to_buf(&PvaType::Boolean, buf, endian)?;
                    value.to_buf(&PvaType::VariantUnion, buf, endian, registry)?;
                }
                None => {
                    false.to_buf(&PvaType::Boolean, buf, endian)?;
                }
            }
        }
        Ok(())
    }
}
