// ---------------------- ?? ----------------------------------

use crate::pva_message::{
    header::MsgEndian,
    primitive::PvaElement,
    typ::{PvaStructType, PvaUnionType},
};

enum PvaValue {
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
    String(String),      // 0x60, 0b 011 00 000
    BoundString(String), // 0x83, 0b 100 00 011, bound is in type, not in value

    BooleanVarSizeArray(Vec<bool>), // 0x08, 0b 000 01 000
    BooleanBoundArray(Vec<bool>),   // 0x10, 0b 000 10 000, bound is in type, not in value
    BooleanFixArray(Vec<bool>),     // 0x18, 0b 000 11 000, size is in type, not in value

    ByteVarSizeArray(Vec<i8>), // 0x28, 0b 001 01 000
    ByteBoundArray(Vec<i8>),   // 0x30, 0b 001 10 000, bound is in type, not in value
    ByteFixArray(Vec<i8>),     // 0x38, 0b 001 11 000, size is in type, not in value

    ShortVarSizeArray(Vec<i16>), // 0x29, 0b 001 01 001
    ShortBoundArray(Vec<i16>),   // 0x31, 0b 001 10 001 bound is in type, not in value
    ShortFixArray(Vec<i16>),     // 0x39, 0b 001 11 001 size is in type, not in value

    IntVarSizeArray(Vec<i32>), // 0x2A, 0b 001 01 010
    IntBoundArray(Vec<i32>),   // 0x32, 0b 001 10 010 bound is in type, not in value
    IntFixArray(Vec<i32>),     // 0x3A, 0b 001 11 010 size is in type, not in value

    LongVarSizeArray(Vec<i64>), // 0x2B, 0b 001 01 011
    LongBoundArray(Vec<i64>),   // 0x33, 0b 001 10 011 bound is in type, not in value
    LongFixArray(Vec<i64>),     // 0x3B, 0b 001 11 011 size is in type, not in value

    UByteVarSizeArray(Vec<u8>), // 0x2C, 0b 001 01 100
    UByteBoundArray(Vec<u8>),   // 0x34, 0b 001 10 100 bound is in type, not in value
    UByteFixArray(Vec<u8>),     // 0x3C, 0b 001 11 100 size is in type, not in value

    UShortVarSizeArray(Vec<u16>), // 0x2D, 0b 001 01 101
    UShortBoundArray(Vec<u16>),   // 0x35, 0b 001 10 101 bound is in type, not in value
    UShortFixArray(Vec<u16>),     // 0x3D, 0b 001 11 101 size is in type, not in value

    UIntVarSizeArray(Vec<u32>), // 0x2E, 0b 001 01 110
    UIntBoundArray(Vec<u32>),   // 0x36, 0b 001 10 110 bound is in type, not in value
    UIntFixArray(Vec<u32>),     // 0x3E, 0b 001 11 110 size is in type, not in value

    ULongVarSizeArray(Vec<u64>), // 0x2F, 0b 001 01 111
    ULongBoundArray(Vec<u64>),   // 0x37, 0b 001 10 111 bound is in type, not in value
    ULongFixArray(Vec<u64>),     // 0x3F, 0b 001 11 111 size is in type, not in value

    FloatVarSizeArray(Vec<f32>), // 0x4A, 0b 010 01 010
    FloatBoundArray(Vec<f32>),   // 0x52, 0b 010 10 010 bound is in type, not in value
    FloatFixArray(Vec<f32>),     // 0x5A, 0b 010 11 010 size is in type, not in value

    DoubleVarSizeArray(Vec<f64>), // 0x4B, 0b 010 01 011
    DoubleBoundArray(Vec<f64>),   // 0x53, 0b 010 10 011 bound is in type, not in value
    DoubleFixArray(Vec<f64>),     // 0x5B, 0b 010 11 011 size is in type, not in value

    StringVarSizeArray(Vec<String>), // 0x68, 0b 011 01 000
    StringBoundArray(Vec<String>),   // 0x70, 0b 011 10 000 bound is in type, not in value
    StringFixArray(Vec<String>),     // 0x78, 0b 011 11 000 size is in type, not in value

    Structure(PvaStructType),             // 0x80, 0b 100 00 000
    StructureVarSizeArray(PvaStructType), // 0x88, 0b 100 01 000

    Union(PvaUnionType),             // 0x81, 0b 100 00 001
    UnionVarSizeArray(PvaUnionType), // 0x89, 0b 100 01 001

    VariantUnion,             // 0x82, 0b 100 00 010
    VariantUnionVarSizeArray, // 0x8A, 0b 100 01 010
}
