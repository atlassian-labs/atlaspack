use std::fmt;

use serde_json::Value;

use crate::ast::nodes::{
  AtRuleData, CommentData, DeclarationData, DocumentData, RootData, RuleData,
};
use crate::ast::{Node, NodeData, NodeRef, RawData};
use crate::input::{Input, InputRef, Position};
use crate::source_map::{PreviousMap, PreviousMapError};

#[derive(Debug)]
pub enum FromJsonError {
  InvalidFormat(String),
  MissingField(&'static str),
  Parse(String),
  PreviousMap(PreviousMapError),
}

impl fmt::Display for FromJsonError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      FromJsonError::InvalidFormat(msg) | FromJsonError::Parse(msg) => write!(f, "{}", msg),
      FromJsonError::MissingField(name) => write!(f, "Missing required field: {}", name),
      FromJsonError::PreviousMap(err) => write!(f, "{}", err),
    }
  }
}

impl std::error::Error for FromJsonError {}

impl From<serde_json::Error> for FromJsonError {
  fn from(value: serde_json::Error) -> Self {
    FromJsonError::Parse(value.to_string())
  }
}

impl From<PreviousMapError> for FromJsonError {
  fn from(value: PreviousMapError) -> Self {
    FromJsonError::PreviousMap(value)
  }
}

#[derive(Debug)]
pub enum FromJsonOutput {
  Node(NodeRef),
  Nodes(Vec<NodeRef>),
}

impl FromJsonOutput {
  pub fn into_node(self) -> Result<NodeRef, FromJsonError> {
    match self {
      FromJsonOutput::Node(node) => Ok(node),
      FromJsonOutput::Nodes(_) => Err(FromJsonError::InvalidFormat(
        "Expected single node but received an array".to_string(),
      )),
    }
  }

  pub fn into_nodes(self) -> Result<Vec<NodeRef>, FromJsonError> {
    match self {
      FromJsonOutput::Node(_) => Err(FromJsonError::InvalidFormat(
        "Expected array of nodes but received a single node".to_string(),
      )),
      FromJsonOutput::Nodes(nodes) => Ok(nodes),
    }
  }
}

pub fn from_json(value: &Value) -> Result<FromJsonOutput, FromJsonError> {
  hydrate_value(value, &[])
}

pub fn from_json_str(input: &str) -> Result<FromJsonOutput, FromJsonError> {
  let value: Value = serde_json::from_str(input)?;
  from_json(&value)
}

fn hydrate_value(value: &Value, inputs: &[InputRef]) -> Result<FromJsonOutput, FromJsonError> {
  match value {
    Value::Array(items) => {
      let mut nodes = Vec::with_capacity(items.len());
      for item in items {
        match hydrate_value(item, inputs)? {
          FromJsonOutput::Node(node) => nodes.push(node),
          FromJsonOutput::Nodes(mut more) => nodes.append(&mut more),
        }
      }
      Ok(FromJsonOutput::Nodes(nodes))
    }
    Value::Object(object) => {
      let mut local_inputs = inputs.to_vec();
      if let Some(own_inputs) = object.get("inputs") {
        local_inputs = hydrate_inputs(own_inputs)?;
      }
      let node = hydrate_node(object, &local_inputs)?;
      Ok(FromJsonOutput::Node(node))
    }
    _ => Err(FromJsonError::InvalidFormat(
      "from_json expects an object or array".to_string(),
    )),
  }
}

fn hydrate_inputs(value: &Value) -> Result<Vec<InputRef>, FromJsonError> {
  let array = value
    .as_array()
    .ok_or_else(|| FromJsonError::InvalidFormat("inputs must be an array".to_string()))?;

  let mut inputs = Vec::with_capacity(array.len());
  for entry in array {
    let object = entry.as_object().ok_or_else(|| {
      FromJsonError::InvalidFormat("Input description must be an object".to_string())
    })?;

    let css = object
      .get("css")
      .and_then(|v| v.as_str())
      .ok_or(FromJsonError::MissingField("inputs[].css"))?
      .to_string();
    let file = object
      .get("file")
      .and_then(|v| v.as_str())
      .map(|s| s.to_string());
    let id = object
      .get("id")
      .and_then(|v| v.as_str())
      .map(|s| s.to_string());
    let has_bom = object
      .get("hasBOM")
      .and_then(|v| v.as_bool())
      .unwrap_or(false);
    let map = if let Some(map_value) = object.get("map") {
      Some(PreviousMap::from_json_value(map_value)?)
    } else {
      None
    };

    let input = Input::from_hydrated(css, file, id, has_bom, map);
    inputs.push(InputRef::new(input));
  }

  Ok(inputs)
}

fn hydrate_node(
  object: &serde_json::Map<String, Value>,
  inputs: &[InputRef],
) -> Result<NodeRef, FromJsonError> {
  let node_type = object
    .get("type")
    .and_then(|v| v.as_str())
    .ok_or(FromJsonError::MissingField("type"))?;

  let node = match node_type {
    "root" => Node::new(NodeData::Root(RootData::default())),
    "document" => {
      let mut data = DocumentData::default();
      if let Some(mode) = object.get("mode").and_then(|v| v.as_str()) {
        data.mode = Some(mode.to_string());
      }
      Node::new(NodeData::Document(data))
    }
    "rule" => {
      let mut data = RuleData::default();
      if let Some(selector) = object.get("selector").and_then(|v| v.as_str()) {
        data.selector = selector.to_string();
      }
      Node::new(NodeData::Rule(data))
    }
    "atrule" => {
      let name = object
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or(FromJsonError::MissingField("name"))?;
      let mut data = AtRuleData::default();
      data.name = name.to_string();
      if let Some(params) = object.get("params").and_then(|v| v.as_str()) {
        data.params = params.to_string();
      }
      Node::new(NodeData::AtRule(data))
    }
    "decl" => {
      let prop = object
        .get("prop")
        .and_then(|v| v.as_str())
        .ok_or(FromJsonError::MissingField("prop"))?;
      let value = object
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or(FromJsonError::MissingField("value"))?;
      let mut data = DeclarationData::default();
      data.prop = prop.to_string();
      data.value = value.to_string();
      if let Some(important) = object.get("important").and_then(|v| v.as_bool()) {
        data.important = important;
      }
      Node::new(NodeData::Declaration(data))
    }
    "comment" => {
      let mut data = CommentData::default();
      if let Some(text) = object.get("text").and_then(|v| v.as_str()) {
        data.text = text.to_string();
      }
      Node::new(NodeData::Comment(data))
    }
    other => {
      return Err(FromJsonError::InvalidFormat(format!(
        "Unknown node type: {}",
        other
      )))
    }
  };

  if let Some(raws_value) = object.get("raws") {
    let raws = parse_raws(raws_value)?;
    node.borrow_mut().raws = raws;
  }

  if let Some(source_value) = object.get("source") {
    let source = parse_source(source_value, inputs)?;
    let mut inner = node.borrow_mut();
    inner.source.start = source.start;
    inner.source.end = source.end;
    inner.source.input = source.input;
  }

  if let Some(nodes_value) = object.get("nodes") {
    let children = nodes_value
      .as_array()
      .ok_or_else(|| FromJsonError::InvalidFormat("nodes must be an array".to_string()))?;
    for child in children {
      match hydrate_value(child, inputs)? {
        FromJsonOutput::Node(node_ref) => Node::append(&node, node_ref),
        FromJsonOutput::Nodes(list) => {
          for child_ref in list {
            Node::append(&node, child_ref);
          }
        }
      }
    }
  }

  Ok(node)
}

fn parse_source(value: &Value, inputs: &[InputRef]) -> Result<SourceFields, FromJsonError> {
  let object = value
    .as_object()
    .ok_or_else(|| FromJsonError::InvalidFormat("source metadata must be an object".to_string()))?;

  let input = if let Some(input_id) = object.get("inputId").and_then(|v| v.as_u64()) {
    let idx = input_id as usize;
    if let Some(found) = inputs.get(idx) {
      Some(found.clone())
    } else {
      return Err(FromJsonError::InvalidFormat(format!(
        "Input reference {} is out of bounds",
        idx
      )));
    }
  } else {
    None
  };

  let start = parse_position(object.get("start"))?;
  let end = parse_position(object.get("end"))?;

  Ok(SourceFields { input, start, end })
}

fn parse_position(value: Option<&Value>) -> Result<Option<Position>, FromJsonError> {
  match value {
    None => Ok(None),
    Some(Value::Object(object)) => {
      let line = object
        .get("line")
        .and_then(|v| v.as_u64())
        .ok_or(FromJsonError::MissingField("source.line"))? as u32;
      let column = object
        .get("column")
        .and_then(|v| v.as_u64())
        .ok_or(FromJsonError::MissingField("source.column"))? as u32;
      let offset = object.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
      Ok(Some(Position::new(line, column, offset)))
    }
    Some(_) => Err(FromJsonError::InvalidFormat(
      "source position must be an object".to_string(),
    )),
  }
}

fn parse_raws(value: &Value) -> Result<RawData, FromJsonError> {
  let object = value
    .as_object()
    .ok_or_else(|| FromJsonError::InvalidFormat("raws metadata must be an object".to_string()))?;

  let mut raws = RawData::default();
  for (key, raw_value) in object {
    assign_raw(&mut raws, key.clone(), raw_value)?;
  }
  Ok(raws)
}

fn assign_raw(raws: &mut RawData, key: String, value: &Value) -> Result<(), FromJsonError> {
  match value {
    Value::Null => Ok(()),
    Value::Bool(flag) => {
      raws.set_bool(&key, *flag);
      Ok(())
    }
    Value::Number(num) => {
      raws.set_text(&key, num.to_string());
      Ok(())
    }
    Value::String(text) => {
      raws.set_text(&key, text.clone());
      Ok(())
    }
    Value::Array(items) => {
      let serialized = serde_json::to_string(items)?;
      raws.set_text(&key, serialized);
      Ok(())
    }
    Value::Object(object) => {
      if let (Some(value_field), Some(raw_field)) = (object.get("value"), object.get("raw")) {
        if let (Some(value_text), Some(raw_text)) = (value_field.as_str(), raw_field.as_str()) {
          raws.set_value_pair(&key, value_text.to_string(), raw_text.to_string());
          return Ok(());
        }
      }
      for (child_key, child_value) in object {
        let nested_key = format!("{}.{}", key, child_key);
        assign_raw(raws, nested_key, child_value)?;
      }
      Ok(())
    }
  }
}

#[derive(Default)]
struct SourceFields {
  input: Option<InputRef>,
  start: Option<Position>,
  end: Option<Position>,
}
