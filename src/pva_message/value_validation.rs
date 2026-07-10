use super::{
    typ::{PvaStructType, PvaType, PvaUnionType},
    value::{PvaStructValue, PvaUnionValue, PvaValue, PvaVariantUnionValue},
};

fn validate_array_len(
    kind: &str,
    actual: usize,
    expected: usize,
    fixed: bool,
) -> Result<(), String> {
    let valid = if fixed {
        actual == expected
    } else {
        actual <= expected
    };

    if valid {
        Ok(())
    } else if fixed {
        Err(format!(
            "PVA {kind} fixed array length {actual} does not match required length {expected}"
        ))
    } else {
        Err(format!(
            "PVA {kind} bounded array length {actual} exceeds bound {expected}"
        ))
    }
}

fn validate_struct_value_type(value: &PvaStructValue, typ: &PvaStructType) -> Result<(), String> {
    if value.fields.len() != typ.fields.len() {
        return Err(format!(
            "PVA struct value field count {} does not match type field count {}",
            value.fields.len(),
            typ.fields.len()
        ));
    }

    for (index, (field_value, field_type)) in value.fields.iter().zip(&typ.fields).enumerate() {
        validate_pva_value_type(field_value, &field_type.typ).map_err(|error| {
            format!(
                "PVA struct field {index} ({}) does not match its type: {error}",
                field_type.name
            )
        })?;
    }

    Ok(())
}

fn validate_union_value_type(value: &PvaUnionValue, typ: &PvaUnionType) -> Result<(), String> {
    match value {
        PvaUnionValue::Null => Ok(()),
        PvaUnionValue::Selected { index, field } => {
            let field_type = typ
                .fields
                .get(*index)
                .ok_or_else(|| format!("PVA union choice {index} is out of range"))?;

            validate_pva_value_type(field, &field_type.typ).map_err(|error| {
                format!(
                    "PVA union choice {index} ({}) does not match its type: {error}",
                    field_type.name
                )
            })
        }
    }
}

pub(crate) fn validate_pva_value_type(value: &PvaValue, typ: &PvaType) -> Result<(), String> {
    match (value, typ) {
        (PvaValue::Null, PvaType::Null)
        | (PvaValue::Boolean(_), PvaType::Boolean)
        | (PvaValue::Byte(_), PvaType::Byte)
        | (PvaValue::Short(_), PvaType::Short)
        | (PvaValue::Int(_), PvaType::Int)
        | (PvaValue::Long(_), PvaType::Long)
        | (PvaValue::UByte(_), PvaType::UByte)
        | (PvaValue::UShort(_), PvaType::UShort)
        | (PvaValue::UInt(_), PvaType::UInt)
        | (PvaValue::ULong(_), PvaType::ULong)
        | (PvaValue::Float(_), PvaType::Float)
        | (PvaValue::Double(_), PvaType::Double)
        | (PvaValue::String(_), PvaType::String)
        | (PvaValue::BooleanVarSizeArray(_), PvaType::BooleanVarSizeArray)
        | (PvaValue::ByteVarSizeArray(_), PvaType::ByteVarSizeArray)
        | (PvaValue::ShortVarSizeArray(_), PvaType::ShortVarSizeArray)
        | (PvaValue::IntVarSizeArray(_), PvaType::IntVarSizeArray)
        | (PvaValue::LongVarSizeArray(_), PvaType::LongVarSizeArray)
        | (PvaValue::UByteVarSizeArray(_), PvaType::UByteVarSizeArray)
        | (PvaValue::UShortVarSizeArray(_), PvaType::UShortVarSizeArray)
        | (PvaValue::UIntVarSizeArray(_), PvaType::UIntVarSizeArray)
        | (PvaValue::ULongVarSizeArray(_), PvaType::ULongVarSizeArray)
        | (PvaValue::FloatVarSizeArray(_), PvaType::FloatVarSizeArray)
        | (PvaValue::DoubleVarSizeArray(_), PvaType::DoubleVarSizeArray)
        | (PvaValue::StringVarSizeArray(_), PvaType::StringVarSizeArray) => Ok(()),

        (PvaValue::BoundString(value), PvaType::BoundString(bound)) => {
            if value.len() <= *bound {
                Ok(())
            } else {
                Err(format!(
                    "PVA bound string length {} exceeds bound {}",
                    value.len(),
                    bound
                ))
            }
        }

        (PvaValue::BooleanBoundArray(values), PvaType::BooleanBoundArray(bound)) => {
            validate_array_len("boolean", values.len(), *bound, false)
        }
        (PvaValue::BooleanFixArray(values), PvaType::BooleanFixArray(len)) => {
            validate_array_len("boolean", values.len(), *len, true)
        }
        (PvaValue::ByteBoundArray(values), PvaType::ByteBoundArray(bound)) => {
            validate_array_len("byte", values.len(), *bound, false)
        }
        (PvaValue::ByteFixArray(values), PvaType::ByteFixArray(len)) => {
            validate_array_len("byte", values.len(), *len, true)
        }
        (PvaValue::ShortBoundArray(values), PvaType::ShortBoundArray(bound)) => {
            validate_array_len("short", values.len(), *bound, false)
        }
        (PvaValue::ShortFixArray(values), PvaType::ShortFixArray(len)) => {
            validate_array_len("short", values.len(), *len, true)
        }
        (PvaValue::IntBoundArray(values), PvaType::IntBoundArray(bound)) => {
            validate_array_len("int", values.len(), *bound, false)
        }
        (PvaValue::IntFixArray(values), PvaType::IntFixArray(len)) => {
            validate_array_len("int", values.len(), *len, true)
        }
        (PvaValue::LongBoundArray(values), PvaType::LongBoundArray(bound)) => {
            validate_array_len("long", values.len(), *bound, false)
        }
        (PvaValue::LongFixArray(values), PvaType::LongFixArray(len)) => {
            validate_array_len("long", values.len(), *len, true)
        }
        (PvaValue::UByteBoundArray(values), PvaType::UByteBoundArray(bound)) => {
            validate_array_len("ubyte", values.len(), *bound, false)
        }
        (PvaValue::UByteFixArray(values), PvaType::UByteFixArray(len)) => {
            validate_array_len("ubyte", values.len(), *len, true)
        }
        (PvaValue::UShortBoundArray(values), PvaType::UShortBoundArray(bound)) => {
            validate_array_len("ushort", values.len(), *bound, false)
        }
        (PvaValue::UShortFixArray(values), PvaType::UShortFixArray(len)) => {
            validate_array_len("ushort", values.len(), *len, true)
        }
        (PvaValue::UIntBoundArray(values), PvaType::UIntBoundArray(bound)) => {
            validate_array_len("uint", values.len(), *bound, false)
        }
        (PvaValue::UIntFixArray(values), PvaType::UIntFixArray(len)) => {
            validate_array_len("uint", values.len(), *len, true)
        }
        (PvaValue::ULongBoundArray(values), PvaType::ULongBoundArray(bound)) => {
            validate_array_len("ulong", values.len(), *bound, false)
        }
        (PvaValue::ULongFixArray(values), PvaType::ULongFixArray(len)) => {
            validate_array_len("ulong", values.len(), *len, true)
        }
        (PvaValue::FloatBoundArray(values), PvaType::FloatBoundArray(bound)) => {
            validate_array_len("float", values.len(), *bound, false)
        }
        (PvaValue::FloatFixArray(values), PvaType::FloatFixArray(len)) => {
            validate_array_len("float", values.len(), *len, true)
        }
        (PvaValue::DoubleBoundArray(values), PvaType::DoubleBoundArray(bound)) => {
            validate_array_len("double", values.len(), *bound, false)
        }
        (PvaValue::DoubleFixArray(values), PvaType::DoubleFixArray(len)) => {
            validate_array_len("double", values.len(), *len, true)
        }
        (PvaValue::StringBoundArray(values), PvaType::StringBoundArray(bound)) => {
            validate_array_len("string", values.len(), *bound, false)
        }
        (PvaValue::StringFixArray(values), PvaType::StringFixArray(len)) => {
            validate_array_len("string", values.len(), *len, true)
        }

        (PvaValue::Struct(value), PvaType::Struct(struct_type)) => {
            validate_struct_value_type(value, struct_type)
        }
        (PvaValue::StructVarSizeArray(values), PvaType::StructVarSizeArray(struct_type)) => {
            for (index, value) in values.iter().enumerate() {
                if let Some(value) = value {
                    validate_struct_value_type(value, struct_type).map_err(|error| {
                        format!("PVA structure array element {index} is invalid: {error}")
                    })?;
                }
            }
            Ok(())
        }
        (PvaValue::Union(value), PvaType::Union(union_type)) => {
            validate_union_value_type(value, union_type)
        }
        (PvaValue::UnionVarSizeArray(values), PvaType::UnionVarSizeArray(union_type)) => {
            for (index, value) in values.iter().enumerate() {
                if let Some(value) = value {
                    validate_union_value_type(value, union_type).map_err(|error| {
                        format!("PVA union array element {index} is invalid: {error}")
                    })?;
                }
            }
            Ok(())
        }
        (PvaValue::VariantUnion(value), PvaType::VariantUnion) => match value {
            PvaVariantUnionValue::Null => Ok(()),
            PvaVariantUnionValue::Selected {
                typ: selected_type,
                value: selected_value,
            } => {
                if matches!(selected_type, PvaType::Null)
                    || matches!(selected_value.as_ref(), PvaValue::Null)
                {
                    return Err(
                        "A selected PVA variant union cannot contain a null type or value"
                            .to_string(),
                    );
                }

                validate_pva_value_type(selected_value, selected_type)
                    .map_err(|error| format!("PVA variant union selection is invalid: {error}"))
            }
        },
        (PvaValue::VariantUnionVarSizeArray, PvaType::VariantUnionVarSizeArray) => {
            Err("PVA variant union array value validation is not implemented".to_string())
        }

        _ => Err(format!("PVA value variant does not match type {typ:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pva_message::{
        header::MsgEndian, primitive::PvaElement, value::PvaVariantUnionValue,
    };

    #[test]
    fn variant_union_selected_type_matches_value() {
        let value = PvaValue::VariantUnion(PvaVariantUnionValue::Selected {
            typ: PvaType::Int,
            value: Box::new(PvaValue::Int(42)),
        });

        assert!(validate_pva_value_type(&value, &PvaType::VariantUnion).is_ok());
    }

    #[test]
    fn variant_union_rejects_mismatched_selected_value() {
        let value = PvaValue::VariantUnion(PvaVariantUnionValue::Selected {
            typ: PvaType::Int,
            value: Box::new(PvaValue::String("not an integer".to_string())),
        });

        let error = validate_pva_value_type(&value, &PvaType::VariantUnion).unwrap_err();
        assert!(error.contains("variant union selection is invalid"));
    }

    #[test]
    fn variant_union_rejects_null_selected_value() {
        let value = PvaValue::VariantUnion(PvaVariantUnionValue::Selected {
            typ: PvaType::Null,
            value: Box::new(PvaValue::Null),
        });

        let error = validate_pva_value_type(&value, &PvaType::VariantUnion).unwrap_err();
        assert!(error.contains("cannot contain a null type or value"));
    }

    #[test]
    fn invalid_variant_union_does_not_modify_output_buffer() {
        let value = PvaVariantUnionValue::Selected {
            typ: PvaType::Int,
            value: Box::new(PvaValue::String("not an integer".to_string())),
        };
        let mut buf = vec![0xaa];

        assert!(
            value
                .to_buf(&PvaType::VariantUnion, &mut buf, MsgEndian::Little)
                .is_err()
        );
        assert_eq!(buf, vec![0xaa]);
    }
}
