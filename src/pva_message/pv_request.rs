use std::sync::Arc;

use crate::pva_message::{
    complex::{PvaFieldType, PvaStructType, PvaStructValue, validate_pva_field_name},
    typ::PvaType,
    value::PvaValue,
};

impl PvaType {
    pub fn build_pv_request(str: &str) -> Result<Arc<PvaType>, String> {
        // empty request
        if str == "" {
            return Ok(Arc::new(PvaType::Struct(Arc::new(PvaStructType {
                id: "structure".to_string(),
                fields: vec![],
            }))));
        }

        // obtain each field
        let mut field_names: Vec<&str> = vec![];
        field_names.extend(str.split('.'));

        // the lowest-level type
        let mut typ = Arc::new(PvaStructType {
            id: "structure".to_string(),
            fields: vec![],
        });

        for field_name in field_names.iter().rev() {
            validate_pva_field_name(*field_name)?;
            // the type that contains the field_name type
            typ = Arc::new(PvaStructType {
                id: "structure".to_string(),
                fields: vec![Arc::new(PvaFieldType {
                    name: field_name.to_string(),
                    typ: Arc::new(PvaType::Struct(typ)),
                })],
            });
        }

        // type is the special "field"'s type
        let top_level_typ = PvaType::Struct(Arc::new(PvaStructType {
            id: "structure".to_string(),
            fields: vec![Arc::new(PvaFieldType {
                name: "field".to_string(),
                typ: Arc::new(PvaType::Struct(typ)),
            })],
        }));

        Ok(Arc::new(top_level_typ))
    }
}

impl PvaValue {
    pub fn build_pv_request(str: &str) -> Result<PvaValue, String> {
        // empty request
        if str == "" {
            return Ok(PvaValue::Struct(PvaStructValue { fields: vec![] }));
        }

        // obtain each field
        let mut field_names: Vec<&str> = vec![];
        field_names.extend(str.split('.'));

        // the lowest-level value
        let mut value = PvaValue::Struct(PvaStructValue { fields: vec![] });

        for field_name in field_names.iter().rev() {
            validate_pva_field_name(*field_name)?;
            // the struct value that contains the field_name value
            value = PvaValue::Struct(PvaStructValue {
                fields: vec![value],
            });
        }

        // value is the special "field" struct
        let top_level_value = PvaValue::Struct(PvaStructValue {
            fields: vec![value],
        });

        Ok(top_level_value)
    }
}
