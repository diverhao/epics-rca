use crate::{
    pva_message::{
        header::MsgEndian,
        primitive::{PvaElement, PvaSize}, type_registry::PvaTypeRegistry,
    },
    tcp::tcp::TCP,
};

// ------------------- size -----------------

const NULL_TYPE_CODE: u8 = 0xff;
const ONLY_ID_TYPE_CODE: u8 = 0xfe;
const FULL_WITH_ID_TYPE_CODE: u8 = 0xfd;
const FULL_TAGGED_ID_TYPE_CODE: u8 = 0xfc;

// --------------- PVA type --------------
#[derive(Debug, Clone)]
pub enum PvaType {
    Null,               // special type, with 0xff wrapper code
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

    Struct(PvaStructType),             // 0x80, 0b 100 00 000
    StructVarSizeArray(PvaStructType), // 0x88, 0b 100 01 000

    Union(PvaUnionType),             // 0x81, 0b 100 00 001
    UnionVarSizeArray(PvaUnionType), // 0x89, 0b 100 01 001

    VariantUnion,             // 0x82, 0b 100 00 010
    VariantUnionVarSizeArray, // 0x8A, 0b 100 01 010
}

impl PvaType {
    // real decoder, the part after type code and type id
    pub fn from_buf(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<Self, String> {
        let code = u8::from_buf(&PvaType::UByte, buf, offset, endian)?;

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

            0xff => PvaType::Null,

            0xfe => {
                // 0xfe + type ID, read type from cache
                let id = i16::from_buf(&PvaType::Short, buf, offset, endian)?;
                let typ = match registry.typ(id) {
                    Some(typ) => typ,
                    None => return Err(format!("Cannot find type with ID {id} in registry")),
                };
                return Ok(typ.clone());
            }
            0xfd => {
                // 0xfd + type ID + type
                let id = i16::from_buf(&PvaType::Short, buf, offset, endian)?;
                let typ = PvaType::from_buf(buf, offset, endian, registry)?;
                // register ID
                registry.add(id, typ.clone());
                return Ok(typ);
            }

            0xfc => {
                return Err("0xfc + ID + tag + FieldDesc not supported".to_string());
            }

            0x80 => {
                // retract by 1 for 0x80
                *offset -= 1;
                PvaType::Struct(PvaStructType::from_buf(buf, offset, endian, registry)?)
            }

            0x88 => {
                let pva_type = PvaType::from_buf(buf, offset, endian, registry)?;
                let struct_type = match pva_type {
                    PvaType::Struct(struct_type) => struct_type,
                    other => {
                        return Err(format!(
                            "Error: PVA structure array type code 0x88 requires a structure element type, got {other:?}"
                        ));
                    }
                };
                return Ok(PvaType::StructVarSizeArray(struct_type));
            }

            0x81 => {
                // retract by 1 for 0x81
                *offset -= 1;
                PvaType::Union(PvaUnionType::from_buf(buf, offset, endian, registry)?)
            }

            0x89 => {
                let pva_type = PvaType::from_buf(buf, offset, endian, registry)?;
                let union_type = match pva_type {
                    PvaType::Union(union_type) => union_type,
                    other => {
                        return Err(format!(
                            "Error: PVA union array type code 0x89 requires a union element type, got {other:?}"
                        ));
                    }
                };
                return Ok(PvaType::UnionVarSizeArray(union_type));
            }

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
            Self::Null => buf.push(0xff),
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

            Self::Struct(typ) => {
                typ.to_buf(buf, endian)?;
            }
            Self::StructVarSizeArray(typ) => {
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
    fn to_buf(self: &Self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        // code 0x80
        buf.push(0x80);

        // struct ID string
        self.id.to_buf(&PvaType::String, buf, endian)?;

        // number of fields
        self.fields.len().to_buf(buf, endian)?;

        // field types
        for field_type in &self.fields {
            field_type.to_buf(buf, endian)?;
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

#[derive(Debug, Clone)]
pub struct PvaFieldType {
    pub name: String,
    pub typ: PvaType,
}

impl PvaFieldType {
    pub fn to_buf(self: &Self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        let name = &self.name;
        name.to_buf(&PvaType::String, buf, endian)?;

        let typ = &self.typ;
        typ.to_buf(buf, endian)?;
        Ok(())
    }

    pub fn from_buf(
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

impl PvaUnionType {
    fn to_buf(self: &Self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        // code 0x81
        buf.push(0x81);

        // union ID string
        self.id.to_buf(&PvaType::String, buf, endian)?;

        // number of fields
        self.fields.len().to_buf(buf, endian)?;

        // field types
        for field_type in &self.fields {
            field_type.to_buf(buf, endian)?;
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
