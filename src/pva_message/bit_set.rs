use std::{collections::BTreeSet, sync::Arc};

use crate::pva_message::{
    complex::{PvaFieldType, PvaStructType, PvaStructValue},
    header::MsgEndian,
    primitive::{PvaSize, read_n_bytes},
    typ::PvaType,
    type_registry::PvaTypeRegistry,
    value::PvaValue,
    value_validation::validate_pva_value_type,
};

// ---------------------- BitSet --------------------------
pub struct BitSet {
    indices: Vec<u8>,
}

impl BitSet {
    // actually append_to_buf()
    pub fn to_buf(&self, buf: &mut Vec<u8>, endian: MsgEndian) -> Result<(), String> {
        let Some(highest_index) = self.indices.iter().copied().max() else {
            0_usize.to_buf(buf, endian)?;
            return Ok(());
        };

        let mut words = [0_u64; 4];
        for &index in &self.indices {
            let index = usize::from(index);
            words[index / 64] |= 1_u64 << (index % 64);
        }

        let byte_count = usize::from(highest_index) / 8 + 1;
        let mut encoded = Vec::with_capacity(byte_count + 1);
        byte_count.to_buf(&mut encoded, endian)?;

        let complete_words = byte_count / 8;
        for &word in &words[..complete_words] {
            let bytes = match endian {
                MsgEndian::Little => word.to_le_bytes(),
                MsgEndian::Big => word.to_be_bytes(),
            };
            encoded.extend_from_slice(&bytes);
        }

        let remaining = byte_count % 8;
        if remaining != 0 {
            let mut final_word = words[complete_words];
            for _ in 0..remaining {
                encoded.push((final_word & 0xff) as u8);
                final_word >>= 8;
            }
        }

        buf.extend_from_slice(&encoded);
        Ok(())
    }

    /**
     * PVA BitSet limited to flattened field indices 0..=255.
     *
     * EPICS Base supports larger BitSets, but this library intentionally rejects
     * structures requiring more than 256 flattened field indices.
     */
    pub fn from_buf(buf: &[u8], offset: &mut usize, endian: MsgEndian) -> Result<BitSet, String> {
        let mut local_offset = *offset;

        // number of bytes
        let size = usize::from_buf(buf, &mut local_offset, endian)?;
        if size > 32 {
            // 32 * 8 = 256
            return Err("PVA BitSet exceeds the supported maximum field index of 255".to_string());
        }

        let end = local_offset
            .checked_add(size)
            .ok_or_else(|| "PVA BitSet offset overflow".to_string())?;
        if end > buf.len() {
            return Err("Remaining buffer too short for PVA BitSet".to_string());
        }

        let mut indices = Vec::new();

        for word_index in 0..size / 8 {
            let bytes = read_n_bytes::<8>(buf, &mut local_offset, "BitSet word")?;
            let value = match endian {
                MsgEndian::Little => u64::from_le_bytes(bytes),
                MsgEndian::Big => u64::from_be_bytes(bytes),
            };
            append_set_bits(value, word_index * 64, &mut indices)?;
        }

        let remaining = size % 8;
        let final_word_base = (size / 8) * 64;
        for byte_index in 0..remaining {
            let byte = read_n_bytes::<1>(buf, &mut local_offset, "BitSet byte")?[0];
            append_set_bits(
                u64::from(byte),
                final_word_base + byte_index * 8,
                &mut indices,
            )?;
        }

        *offset = local_offset;
        Ok(BitSet { indices })
    }

    pub fn from_field_paths(typ: Arc<PvaType>, field_paths: Vec<String>) -> Result<Self, String> {
        let node_count = typ.num_nodes()?;
        if node_count > 256 {
            return Err(format!(
                "PVA structure has {node_count} flattened field indices, exceeding the supported maximum of 256"
            ));
        }

        let struct_type = match typ.as_ref() {
            PvaType::Struct(struct_type) => struct_type,
            _ => {
                return Err("PVA BitSet field paths require a root structure type".to_string());
            }
        };

        let mut indices = BTreeSet::new();
        for field_path in field_paths {
            let components: Vec<&str> = field_path.split('.').collect();
            if components.iter().any(|component| component.is_empty()) {
                return Err(format!(
                    "Invalid empty component in PVA field path \"{field_path}\""
                ));
            }

            let index = field_index_from_path(struct_type, &components, 0, &field_path)?;
            indices.insert(index);
        }

        Ok(Self {
            indices: indices.into_iter().collect(),
        })
    }

    pub fn indices(&self) -> &Vec<u8> {
        &self.indices
    }
}

fn field_index_from_path(
    struct_type: &PvaStructType,
    components: &[&str],
    struct_index: usize,
    field_path: &str,
) -> Result<u8, String> {
    let component = components[0];
    let mut field_index = struct_index
        .checked_add(1)
        .ok_or_else(|| "PVA field index overflow".to_string())?;

    for field in &struct_type.fields {
        if field.name == component {
            if components.len() == 1 {
                return u8::try_from(field_index).map_err(|_| {
                    format!(
                        "PVA field path \"{field_path}\" exceeds the supported maximum field index of 255"
                    )
                });
            }

            return match field.typ.as_ref() {
                PvaType::Struct(nested_type) => {
                    field_index_from_path(nested_type, &components[1..], field_index, field_path)
                }
                _ => Err(format!(
                    "Cannot resolve PVA field path \"{field_path}\": \"{component}\" is not a structure"
                )),
            };
        }

        field_index = field_index
            .checked_add(field.typ.num_nodes()?)
            .ok_or_else(|| "PVA field index overflow".to_string())?;
    }

    Err(format!(
        "Cannot resolve PVA field path \"{field_path}\": field \"{component}\" does not exist"
    ))
}

fn append_set_bits(mut value: u64, base_index: usize, indices: &mut Vec<u8>) -> Result<(), String> {
    while value != 0 {
        let bit_index = value.trailing_zeros() as usize;
        let field_index = base_index
            .checked_add(bit_index)
            .ok_or_else(|| "PVA BitSet field index overflow".to_string())?;
        let field_index = u8::try_from(field_index).map_err(|_| {
            "PVA BitSet exceeds the supported maximum field index of 255".to_string()
        })?;

        indices.push(field_index);
        value &= value - 1;
    }

    Ok(())
}

// ------------------ PvaType and PvaValue ------------------

impl PvaType {
    pub fn num_nodes(&self) -> Result<usize, String> {
        match self {
            PvaType::Struct(struct_type) => struct_type.num_nodes(),
            _ => Ok(1),
        }
    }

    pub fn type_at_index(self: Arc<Self>, goal_index: u8) -> Result<Arc<PvaType>, String> {
        // only if this is a struct
        if let PvaType::Struct(struct_typ) = self.as_ref() {
            if goal_index == 0 {
                return Ok(Arc::clone(&self));
            }

            let mut current_index: u8 = 1;

            match struct_typ.type_at_index(goal_index, &mut current_index) {
                // not found
                Ok(None) => {
                    return Err(format!(
                        "PVA type index {goal_index} is not present in the root structure"
                    ));
                }
                // found
                Ok(Some(struct_typ)) => return Ok(struct_typ),
                // error
                Err(error) => return Err(error),
            };
        };

        Err(format!(
            "PVA type lookup at index {goal_index} requires a root structure"
        ))
    }
}

impl PvaValue {
    /**
     * Encode only the value fields selected by `bit_set`.
     *
     * This does not encode the BitSet itself. The caller must encode the BitSet
     * immediately before the returned value data in a PVA partial-update payload.
     */
    pub fn to_buf_with_bitset(
        &self,
        typ: Arc<PvaType>,
        bit_set: &BitSet,
        buf: &mut Vec<u8>,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<(), String> {
        if !matches!(typ.as_ref(), PvaType::Struct(_)) {
            return Err("PVA BitSet value encoding requires a root structure type".to_string());
        }

        validate_pva_value_type(self, typ.as_ref()).map_err(|error| {
            format!("Cannot encode PVA BitSet value: value does not match its type: {error}")
        })?;

        let node_count = typ.num_nodes()?;
        if node_count > 256 {
            return Err(format!(
                "PVA structure has {node_count} flattened field indices, exceeding the supported maximum of 256"
            ));
        }

        let mut indices = bit_set.indices.clone();
        indices.sort_unstable();
        indices.dedup();

        let mut encoded = Vec::new();
        let mut bit_coverage = 0_usize;
        for index in indices {
            let index_usize = usize::from(index);
            if index_usize < bit_coverage {
                continue;
            }

            let field_type = typ.clone().type_at_index(index)?;
            bit_coverage = index_usize
                .checked_add(field_type.num_nodes()?)
                .ok_or_else(|| "PVA BitSet coverage overflow".to_string())?;

            let field_value = self.value_at_index(index)?;
            field_value.to_buf(field_type, &mut encoded, endian, registry)?;
        }

        buf.extend_from_slice(&encoded);
        Ok(())
    }

    fn value_at_index(&self, goal_index: u8) -> Result<&PvaValue, String> {
        let PvaValue::Struct(struct_value) = self else {
            return Err("PVA value lookup by BitSet index requires a root structure".to_string());
        };

        if goal_index == 0 {
            return Ok(self);
        }

        let mut current_index = 1_usize;
        struct_value
            .value_at_index(usize::from(goal_index), &mut current_index)?
            .ok_or_else(|| {
                format!("PVA value index {goal_index} is not present in the root structure")
            })
    }

    pub fn set_value_at_index(
        &mut self,
        goal_index: u8,
        new_value: PvaValue,
    ) -> Result<(), PvaValue> {
        if goal_index == 0
            && let PvaValue::Struct(_) = self
            && let PvaValue::Struct(_) = &new_value
        {
            *self = new_value;
            return Ok(());
        }

        if let PvaValue::Struct(struct_value) = self {
            let mut curr_index: u8 = 1;
            return struct_value.set_value_at_index(goal_index, &mut curr_index, new_value);
        }

        Err(new_value)
    }

    /**
     * Decode a buffer that is composed of bit set + value data
     */
    pub fn set_fields_from_bitset_buf(
        &mut self,
        buf: &[u8],
        offset: &mut usize,
        typ: Arc<PvaType>,
        endian: MsgEndian,
        registry: &mut PvaTypeRegistry,
    ) -> Result<(), String> {
        validate_pva_value_type(self, typ.as_ref()).map_err(|error| {
            format!("Cannot apply PVA BitSet update: cached value does not match its type: {error}")
        })?;

        // decode bit set from buffer
        let bit_set = BitSet::from_buf(buf, offset, endian)?;

        // Exclusive end of the subtree covered by the last selected structure.
        let mut bit_coverage = 0_usize;

        let mut indices_values: Vec<(u8, PvaValue)> = vec![];

        for index in bit_set.indices() {
            let index_usize = usize::from(*index);
            if index_usize < bit_coverage {
                continue;
            }

            // find the type for the field
            let field_typ = typ.clone().type_at_index(*index)?;
            bit_coverage = index_usize
                .checked_add(field_typ.num_nodes()?)
                .ok_or_else(|| "PVA BitSet coverage overflow".to_string())?;

            // decode value from buffer
            let value = match PvaValue::from_buf(field_typ, buf, offset, endian, registry) {
                Ok(value) => value,
                Err(error) => return Err(error),
            };

            indices_values.push((*index, value));
        }

        // atomic operation on updating value
        for (index, value) in indices_values {
            match self.set_value_at_index(index, value) {
                Ok(()) => {}
                Err(_) => {
                    return Err(format!(
                        "Cannot replace PVA value at index {index}: the existing value layout does not match the structure type"
                    ));
                }
            };
        }

        Ok(())
    }
}

// ------------- PvaStructType and PvaStructValue ------------

impl PvaStructType {
    /**
     * Number of children and grandchildren nodes, including this struct itself
     */
    fn num_nodes(&self) -> Result<usize, String> {
        let mut count = 1_usize;

        for field in &self.fields {
            count = count
                .checked_add(field.typ.num_nodes()?)
                .ok_or_else(|| "PVA structure node count overflow".to_string())?;
        }
        Ok(count)
    }

    // get the i-th type (recursive)
    fn type_at_index(
        &self,
        goal_index: u8,
        current_index: &mut u8,
    ) -> Result<Option<Arc<PvaType>>, String> {
        for field_type in &self.fields {
            // struct type inside the PvaType
            let typ = field_type.typ.clone();
            if *current_index == goal_index {
                return Ok(Some(typ));
            }
            // do not proceed, reached to the limit
            if *current_index == 255 {
                return Err("bit set size overflow".to_string());
            }
            match typ.as_ref() {
                PvaType::Struct(typ) => {
                    // if current field is a struct
                    *current_index += 1;
                    if let Some(found) = typ.type_at_index(goal_index, current_index)? {
                        return Ok(Some(found));
                    }
                }
                _ => {
                    // if current field is not a struct
                    *current_index += 1;
                }
            };
        }
        // not found
        Ok(None)
    }
}

impl PvaStructValue {
    fn value_at_index(
        &self,
        goal_index: usize,
        current_index: &mut usize,
    ) -> Result<Option<&PvaValue>, String> {
        for field_value in &self.fields {
            if *current_index == goal_index {
                return Ok(Some(field_value));
            }

            *current_index = current_index
                .checked_add(1)
                .ok_or_else(|| "PVA value field index overflow".to_string())?;

            if let PvaValue::Struct(struct_value) = field_value
                && let Some(found) = struct_value.value_at_index(goal_index, current_index)?
            {
                return Ok(Some(found));
            }
        }

        Ok(None)
    }

    fn set_value_at_index(
        &mut self,
        goal_index: u8,
        curr_index: &mut u8,
        new_value: PvaValue,
    ) -> Result<(), PvaValue> {
        let mut new_value = new_value;
        // find the index
        for field_value in self.fields.iter_mut() {
            if *curr_index == goal_index {
                *field_value = new_value;
                // success, new_value is moved inside Vec
                return Ok(());
            }

            if *curr_index == 255 {
                return Err(new_value);
            }
            *curr_index += 1;
            match field_value {
                PvaValue::Struct(struct_value) => {
                    match struct_value.set_value_at_index(goal_index, curr_index, new_value) {
                        Ok(()) => return Ok(()), // found and replaced the value
                        Err(new_value_back) => new_value = new_value_back, // index not found in this struct, do nothing
                    }
                }
                _ => {}
            }
        }
        Err(new_value)
    }
}

// ---------------- test -------------------------
#[cfg(test)]
mod tests {
    use super::BitSet;
    use crate::pva_message::{
        complex::{PvaFieldType, PvaStructType, PvaStructValue},
        header::MsgEndian,
        typ::PvaType,
        type_registry::PvaTypeRegistry,
        value::PvaValue,
    };
    use std::sync::Arc;

    fn field_path_type() -> Arc<PvaType> {
        Arc::new(PvaType::Struct(Arc::new(PvaStructType {
            id: "root_t".to_string(),
            fields: vec![
                Arc::new(PvaFieldType {
                    name: "value".to_string(),
                    typ: Arc::new(PvaType::Double),
                }),
                Arc::new(PvaFieldType {
                    name: "alarm".to_string(),
                    typ: Arc::new(PvaType::Struct(Arc::new(PvaStructType {
                        id: "alarm_t".to_string(),
                        fields: vec![
                            Arc::new(PvaFieldType {
                                name: "severity".to_string(),
                                typ: Arc::new(PvaType::Int),
                            }),
                            Arc::new(PvaFieldType {
                                name: "message".to_string(),
                                typ: Arc::new(PvaType::String),
                            }),
                        ],
                    }))),
                }),
                Arc::new(PvaFieldType {
                    name: "timeStamp".to_string(),
                    typ: Arc::new(PvaType::Struct(Arc::new(PvaStructType {
                        id: "time_t".to_string(),
                        fields: vec![
                            Arc::new(PvaFieldType {
                                name: "seconds".to_string(),
                                typ: Arc::new(PvaType::Long),
                            }),
                            Arc::new(PvaFieldType {
                                name: "nanoSeconds".to_string(),
                                typ: Arc::new(PvaType::Int),
                            }),
                        ],
                    }))),
                }),
                Arc::new(PvaFieldType {
                    name: "waveform".to_string(),
                    typ: Arc::new(PvaType::DoubleVarSizeArray),
                }),
            ],
        })))
    }

    fn field_path_value() -> PvaValue {
        PvaValue::Struct(PvaStructValue {
            fields: vec![
                PvaValue::Double(1.5),
                PvaValue::Struct(PvaStructValue {
                    fields: vec![PvaValue::Int(2), PvaValue::String("warn".to_string())],
                }),
                PvaValue::Struct(PvaStructValue {
                    fields: vec![PvaValue::Long(100), PvaValue::Int(200)],
                }),
                PvaValue::DoubleVarSizeArray(vec![3.0, 4.0]),
            ],
        })
    }

    #[test]
    fn builds_bitset_from_nested_field_paths() {
        let bit_set = BitSet::from_field_paths(
            field_path_type(),
            vec![
                "waveform".to_string(),
                "alarm.severity".to_string(),
                "value".to_string(),
                "timeStamp.nanoSeconds".to_string(),
                "value".to_string(),
            ],
        )
        .unwrap();

        // root=0, value=1, alarm=2, severity=3, message=4,
        // timeStamp=5, seconds=6, nanoSeconds=7, waveform=8.
        assert_eq!(bit_set.indices, vec![1, 3, 7, 8]);
    }

    #[test]
    fn rejects_invalid_field_paths() {
        let typ = field_path_type();

        assert!(BitSet::from_field_paths(typ.clone(), vec!["alarm.missing".to_string()]).is_err());
        assert!(BitSet::from_field_paths(typ.clone(), vec!["value.child".to_string()]).is_err());
        assert!(BitSet::from_field_paths(typ, vec!["alarm..severity".to_string()]).is_err());
        assert!(
            BitSet::from_field_paths(Arc::new(PvaType::Int), vec!["value".to_string()]).is_err()
        );
    }

    #[test]
    fn encodes_only_fields_selected_by_bitset() {
        let typ = field_path_type();
        let value = field_path_value();
        let bit_set = BitSet::from_field_paths(
            typ.clone(),
            vec![
                "value".to_string(),
                "alarm.severity".to_string(),
                "waveform".to_string(),
            ],
        )
        .unwrap();
        let mut buf = Vec::new();

        value
            .to_buf_with_bitset(
                typ,
                &bit_set,
                &mut buf,
                MsgEndian::Little,
                &mut PvaTypeRegistry::new(),
            )
            .unwrap();

        let mut expected = Vec::new();
        expected.extend_from_slice(&1.5_f64.to_le_bytes());
        expected.extend_from_slice(&2_i32.to_le_bytes());
        expected.push(2);
        expected.extend_from_slice(&3.0_f64.to_le_bytes());
        expected.extend_from_slice(&4.0_f64.to_le_bytes());
        assert_eq!(buf, expected);
    }

    #[test]
    fn selected_structure_covers_selected_descendants_when_encoding() {
        let typ = field_path_type();
        let value = field_path_value();
        let bit_set = BitSet::from_field_paths(
            typ.clone(),
            vec![
                "alarm".to_string(),
                "alarm.severity".to_string(),
                "timeStamp.nanoSeconds".to_string(),
            ],
        )
        .unwrap();
        let mut buf = Vec::new();

        value
            .to_buf_with_bitset(
                typ,
                &bit_set,
                &mut buf,
                MsgEndian::Little,
                &mut PvaTypeRegistry::new(),
            )
            .unwrap();

        let mut expected = Vec::new();
        expected.extend_from_slice(&2_i32.to_le_bytes());
        expected.push(4);
        expected.extend_from_slice(b"warn");
        expected.extend_from_slice(&200_i32.to_le_bytes());
        assert_eq!(buf, expected);
    }

    #[test]
    fn root_bit_encodes_complete_structure_once() {
        let typ = field_path_type();
        let value = field_path_value();
        let bit_set = BitSet {
            indices: vec![0, 1, 3, 8],
        };
        let mut partial_buf = Vec::new();
        let mut full_buf = Vec::new();

        value
            .to_buf_with_bitset(
                typ.clone(),
                &bit_set,
                &mut partial_buf,
                MsgEndian::Little,
                &mut PvaTypeRegistry::new(),
            )
            .unwrap();
        value
            .to_buf(
                typ,
                &mut full_buf,
                MsgEndian::Little,
                &mut PvaTypeRegistry::new(),
            )
            .unwrap();

        assert_eq!(partial_buf, full_buf);
    }

    #[test]
    fn decodes_partial_bitset_word() {
        let buf = [2, 0x01, 0x0b];
        let mut offset = 0;

        let bit_set = BitSet::from_buf(&buf, &mut offset, MsgEndian::Little).unwrap();

        assert_eq!(bit_set.indices, vec![0, 8, 9, 11]);
        assert_eq!(offset, buf.len());
    }

    #[test]
    fn encodes_partial_bitset_word() {
        for endian in [MsgEndian::Little, MsgEndian::Big] {
            let bit_set = BitSet {
                indices: vec![0, 8, 9, 11],
            };
            let mut buf = Vec::new();

            bit_set.to_buf(&mut buf, endian).unwrap();

            assert_eq!(buf, vec![2, 0x01, 0x0b]);
        }
    }

    #[test]
    fn decodes_complete_bitset_word_in_both_endian_modes() {
        let word = (1_u64 << 63) | (1_u64 << 9) | 1;

        for (endian, bytes) in [
            (MsgEndian::Little, word.to_le_bytes()),
            (MsgEndian::Big, word.to_be_bytes()),
        ] {
            let mut buf = vec![8];
            buf.extend_from_slice(&bytes);
            let mut offset = 0;

            let bit_set = BitSet::from_buf(&buf, &mut offset, endian).unwrap();

            assert_eq!(bit_set.indices, vec![0, 9, 63]);
            assert_eq!(offset, buf.len());
        }
    }

    #[test]
    fn bitset_round_trips_in_both_endian_modes() {
        let expected = vec![0, 9, 63, 64, 129, 255];

        for endian in [MsgEndian::Little, MsgEndian::Big] {
            let bit_set = BitSet {
                indices: expected.clone(),
            };
            let mut buf = Vec::new();
            bit_set.to_buf(&mut buf, endian).unwrap();
            let mut offset = 0;

            let decoded = BitSet::from_buf(&buf, &mut offset, endian).unwrap();

            assert_eq!(decoded.indices, expected);
            assert_eq!(offset, buf.len());
        }
    }

    #[test]
    fn encodes_empty_bitset() {
        let bit_set = BitSet {
            indices: Vec::new(),
        };
        let mut buf = Vec::new();

        bit_set.to_buf(&mut buf, MsgEndian::Little).unwrap();

        assert_eq!(buf, vec![0]);
    }

    #[test]
    fn rejects_indices_above_255() {
        let mut buf = vec![33];
        buf.extend_from_slice(&[0; 32]);
        buf.push(1);
        let mut offset = 0;

        assert!(BitSet::from_buf(&buf, &mut offset, MsgEndian::Little).is_err());
        assert_eq!(offset, 0);
    }

    #[test]
    fn bitset_error_does_not_advance_offset() {
        let buf = [2, 0x01];
        let mut offset = 0;

        assert!(BitSet::from_buf(&buf, &mut offset, MsgEndian::Little).is_err());
        assert_eq!(offset, 0);
    }

    #[test]
    fn selected_structure_covers_selected_children_but_not_siblings() {
        let nested_type = Arc::new(PvaStructType {
            id: "nested_t".to_string(),
            fields: vec![
                Arc::new(PvaFieldType {
                    name: "number".to_string(),
                    typ: Arc::new(PvaType::Int),
                }),
                Arc::new(PvaFieldType {
                    name: "text".to_string(),
                    typ: Arc::new(PvaType::String),
                }),
            ],
        });
        let root_type = Arc::new(PvaType::Struct(Arc::new(PvaStructType {
            id: "root_t".to_string(),
            fields: vec![
                Arc::new(PvaFieldType {
                    name: "nested".to_string(),
                    typ: Arc::new(PvaType::Struct(nested_type)),
                }),
                Arc::new(PvaFieldType {
                    name: "sibling".to_string(),
                    typ: Arc::new(PvaType::Int),
                }),
            ],
        })));
        let mut value = PvaValue::Struct(PvaStructValue {
            fields: vec![
                PvaValue::Struct(PvaStructValue {
                    fields: vec![PvaValue::Int(0), PvaValue::String(String::new())],
                }),
                PvaValue::Int(0),
            ],
        });

        let bit_set = BitSet {
            // 1 is the nested structure, 2 is its child, and 4 is its sibling.
            indices: vec![1, 2, 4],
        };
        let mut buf = Vec::new();
        bit_set.to_buf(&mut buf, MsgEndian::Little).unwrap();
        buf.extend_from_slice(&42_i32.to_le_bytes());
        buf.push(2);
        buf.extend_from_slice(b"ok");
        buf.extend_from_slice(&99_i32.to_le_bytes());

        let mut offset = 0;
        value
            .set_fields_from_bitset_buf(
                &buf,
                &mut offset,
                root_type,
                MsgEndian::Little,
                &mut PvaTypeRegistry::new(),
            )
            .unwrap();

        assert_eq!(offset, buf.len());
        let PvaValue::Struct(root) = value else {
            panic!("root value is not a structure");
        };
        let PvaValue::Struct(nested) = &root.fields[0] else {
            panic!("nested value is not a structure");
        };
        assert!(matches!(nested.fields[0], PvaValue::Int(42)));
        assert!(matches!(&nested.fields[1], PvaValue::String(text) if text == "ok"));
        assert!(matches!(root.fields[1], PvaValue::Int(99)));
    }
}
