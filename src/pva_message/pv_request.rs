use std::sync::Arc;

use crate::pva_message::{
    complex::{validate_pva_field_name, PvaFieldType, PvaStructType, PvaStructValue},
    typ::PvaType,
    value::PvaValue,
};

// ----------------------- PV Request Node ----------------

/// A parsed pvRequest selection. The synthetic root node has an empty name.
/// For "timeStamp.nanoseconds":
/// PvRequestNode("")
///     └── PvRequestNode("field")
///         └── PvRequestNode("timeStamp")
///             └── PvRequestNode("nanoseconds")
///
/// For "value,alarm,timeStamp":
/// PvRequestNode("")
///     └── PvRequestNode("field")
///         └── PvRequestNode("power")
///             ├── PvRequestNode("value")
///             └── PvRequestNode("alarm")
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PvRequestNode {
    pub name: String,
    pub children: Vec<PvRequestNode>,
}

impl PvRequestNode {
    fn new(name: impl Into<String>, children: Vec<PvRequestNode>) -> Self {
        Self {
            name: name.into(),
            children,
        }
    }
}

/// Parse the field-selection subset of the EPICS pvRequest syntax.
///
/// Plain selections are wrapped in `field`, as EPICS Base does. This parser
/// supports empty requests, explicit `field(...)`, dotted paths, comma-separated
/// siblings, and recursively grouped children with `{...}`.
pub fn parse_pv_request(input: &str) -> Result<PvRequestNode, String> {
    PvRequestParser::new(input).parse()
}

/// Build the `pvRequest` tree used to initialize a channel PUT_GET request.
///
/// Each input uses the same field-selection syntax as [`parse_pv_request`],
/// while the returned root contains the protocol-defined `putField` and
/// `getField` branches.
pub fn parse_put_get_pv_request(
    put_selection: &str,
    get_selection: &str,
) -> Result<PvRequestNode, String> {
    let put_children = parse_selection_children(put_selection)?;
    let get_children = parse_selection_children(get_selection)?;

    Ok(PvRequestNode::new(
        "",
        vec![
            PvRequestNode::new("putField", put_children),
            PvRequestNode::new("getField", get_children),
        ],
    ))
}

fn parse_selection_children(input: &str) -> Result<Vec<PvRequestNode>, String> {
    let root = parse_pv_request(input)?;
    let mut root_children = root.children;

    if root_children.is_empty() {
        return Ok(vec![]);
    }

    if root_children.len() != 1 {
        return Err(String::from(
            "A field-selection pvRequest must contain exactly one field branch",
        ));
    }

    let field = root_children
        .pop()
        .ok_or_else(|| String::from("Missing field-selection pvRequest branch"))?;
    if field.name != "field" {
        return Err(format!(
            "Expected field-selection pvRequest branch, got \"{}\"",
            field.name
        ));
    }

    Ok(field.children)
}

// ----------------------- PV Request Parser ----------------

struct PvRequestParser<'a> {
    input: &'a str,
    offset: usize,
}

impl<'a> PvRequestParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, offset: 0 }
    }

    fn parse(mut self) -> Result<PvRequestNode, String> {
        self.skip_whitespace();
        if self.peek().is_none() {
            return Ok(PvRequestNode::new("", vec![]));
        }

        let checkpoint = self.offset;
        let explicit_field = if self.remaining().starts_with("field") {
            self.offset += "field".len();
            self.skip_whitespace();
            if self.consume('(') {
                true
            } else {
                self.offset = checkpoint;
                false
            }
        } else {
            false
        };

        let field_children = if explicit_field {
            self.skip_whitespace();
            if self.consume(')') {
                vec![]
            } else {
                let children = self.parse_node_list(Some(')'))?;
                self.expect(')')?;
                children
            }
        } else {
            self.parse_node_list(None)?
        };

        self.skip_whitespace();
        if let Some(character) = self.peek() {
            return Err(format!(
                "Unexpected character '{character}' at byte {} in pvRequest",
                self.offset
            ));
        }

        Ok(PvRequestNode::new(
            "",
            vec![PvRequestNode::new("field", field_children)],
        ))
    }

    fn parse_node_list(&mut self, terminator: Option<char>) -> Result<Vec<PvRequestNode>, String> {
        let mut nodes: Vec<PvRequestNode> = vec![];

        loop {
            self.skip_whitespace();
            if self.peek().is_none() || self.peek() == terminator {
                return Err(format!(
                    "Expected a field name at byte {} in pvRequest",
                    self.offset
                ));
            }

            let node = self.parse_node()?;
            if nodes.iter().any(|existing| existing.name == node.name) {
                return Err(format!(
                    "Duplicate pvRequest field name \"{}\" at the same level",
                    node.name
                ));
            }
            nodes.push(node);

            self.skip_whitespace();
            if self.consume(',') {
                self.skip_whitespace();
                if self.peek().is_none() || self.peek() == terminator {
                    return Err(format!(
                        "Expected a field name after ',' at byte {} in pvRequest",
                        self.offset
                    ));
                }
                continue;
            }

            break;
        }

        Ok(nodes)
    }

    fn parse_node(&mut self) -> Result<PvRequestNode, String> {
        self.skip_whitespace();
        let name = self.parse_name()?;
        self.skip_whitespace();

        let children = if self.consume('.') {
            vec![self.parse_node()?]
        } else if self.consume('{') {
            self.skip_whitespace();
            if self.peek() == Some('}') {
                return Err(format!(
                    "Field \"{name}\" has an empty child group in pvRequest"
                ));
            }

            let children = self.parse_node_list(Some('}'))?;
            self.expect('}')?;
            children
        } else {
            vec![]
        };

        Ok(PvRequestNode::new(name, children))
    }

    fn parse_name(&mut self) -> Result<String, String> {
        let start = self.offset;
        while let Some(character) = self.peek() {
            if !character.is_ascii_alphanumeric() && character != '_' {
                break;
            }
            self.offset += character.len_utf8();
        }

        if self.offset == start {
            return Err(format!(
                "Expected a field name at byte {} in pvRequest",
                self.offset
            ));
        }

        let name = &self.input[start..self.offset];
        validate_pva_field_name(name)?;
        Ok(name.to_string())
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        self.skip_whitespace();
        if self.consume(expected) {
            Ok(())
        } else {
            Err(format!(
                "Expected '{expected}' at byte {} in pvRequest",
                self.offset
            ))
        }
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.offset += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(character) = self.peek() {
            if !character.is_ascii_whitespace() {
                break;
            }
            self.offset += character.len_utf8();
        }
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.offset..]
    }
}

// ----------------------- PVA Type ----------------

impl PvaType {
    pub fn build_pv_request(node: &PvRequestNode) -> PvaType {
        let mut struct_typ = PvaStructType {
            id: "structure".to_string(),
            fields: vec![],
        };

        for child_node in &node.children {
            let child_pva_type = PvaType::build_pv_request(child_node);
            let node_name = child_node.name.clone();
            let pva_field_type = PvaFieldType {
                name: node_name,
                typ: Arc::new(child_pva_type),
            };
            struct_typ.fields.push(Arc::new(pva_field_type));
        }

        return PvaType::Struct(Arc::new(struct_typ));
    }
}

// ----------------------- PVA Value ----------------

impl PvaValue {
    pub fn build_pv_request(node: &PvRequestNode) -> PvaValue {
        let mut struct_value = PvaStructValue { fields: vec![] };
        for child_node in &node.children {
            let child_value = Self::build_pv_request(child_node);
            struct_value.fields.push(child_value);
        }

        PvaValue::Struct(struct_value)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_put_get_pv_request, PvRequestNode};

    #[test]
    fn composes_put_get_field_selections() {
        let actual = parse_put_get_pv_request("value", "value,alarm,timeStamp").unwrap();
        let expected = PvRequestNode::new(
            "",
            vec![
                PvRequestNode::new("putField", vec![PvRequestNode::new("value", vec![])]),
                PvRequestNode::new(
                    "getField",
                    vec![
                        PvRequestNode::new("value", vec![]),
                        PvRequestNode::new("alarm", vec![]),
                        PvRequestNode::new("timeStamp", vec![]),
                    ],
                ),
            ],
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn composes_explicit_field_wrappers() {
        let actual = parse_put_get_pv_request("field(value)", "field(alarm.severity)").unwrap();
        let expected = PvRequestNode::new(
            "",
            vec![
                PvRequestNode::new("putField", vec![PvRequestNode::new("value", vec![])]),
                PvRequestNode::new(
                    "getField",
                    vec![PvRequestNode::new(
                        "alarm",
                        vec![PvRequestNode::new("severity", vec![])],
                    )],
                ),
            ],
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn composes_empty_put_get_selections() {
        let actual = parse_put_get_pv_request("", "").unwrap();
        let expected = PvRequestNode::new(
            "",
            vec![
                PvRequestNode::new("putField", vec![]),
                PvRequestNode::new("getField", vec![]),
            ],
        );

        assert_eq!(actual, expected);
    }
}
