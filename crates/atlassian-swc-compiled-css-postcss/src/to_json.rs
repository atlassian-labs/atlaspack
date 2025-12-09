use std::collections::HashMap;

use serde_json::{Map as JsonMap, Value};

use crate::ast::{Node, NodeData, NodeRef, RawData, RawValue};
use crate::input::{InputRef, Position};

#[derive(Default)]
struct InputsRegistry {
  order: Vec<InputRef>,
  lookup: HashMap<usize, usize>,
}

impl InputsRegistry {
  fn register(&mut self, input: &InputRef) -> usize {
    let ptr = input.as_ptr() as usize;
    if let Some(index) = self.lookup.get(&ptr) {
      *index
    } else {
      let index = self.order.len();
      self.order.push(input.clone());
      self.lookup.insert(ptr, index);
      index
    }
  }

  fn emit(&self) -> Option<Value> {
    if self.order.is_empty() {
      return None;
    }
    let inputs = self
      .order
      .iter()
      .map(|input| input.to_json_value())
      .collect();
    Some(Value::Array(inputs))
  }
}

pub fn to_json(node: &NodeRef) -> Value {
  let mut registry = InputsRegistry::default();
  let mut value = serialize_node(node, &mut registry);
  if let Some(inputs) = registry.emit() {
    if let Value::Object(ref mut object) = value {
      object.insert("inputs".to_string(), inputs);
    }
  }
  value
}

pub fn to_json_nodes(nodes: &[NodeRef]) -> Value {
  let values: Vec<Value> = nodes.iter().map(|node| to_json(node)).collect();
  Value::Array(values)
}

fn serialize_node(node: &NodeRef, registry: &mut InputsRegistry) -> Value {
  let borrowed = node.borrow();
  let mut object = JsonMap::new();
  object.insert(
    "type".to_string(),
    Value::String(borrowed.type_name().to_string()),
  );

  match &borrowed.data {
    NodeData::Root(_) => {}
    NodeData::Document(data) => {
      if let Some(mode) = &data.mode {
        object.insert("mode".to_string(), Value::String(mode.clone()));
      }
    }
    NodeData::Rule(data) => {
      object.insert("selector".to_string(), Value::String(data.selector.clone()));
    }
    NodeData::AtRule(data) => {
      object.insert("name".to_string(), Value::String(data.name.clone()));
      if !data.params.is_empty() {
        object.insert("params".to_string(), Value::String(data.params.clone()));
      }
    }
    NodeData::Declaration(data) => {
      object.insert("prop".to_string(), Value::String(data.prop.clone()));
      object.insert("value".to_string(), Value::String(data.value.clone()));
      if data.important {
        object.insert("important".to_string(), Value::Bool(true));
      }
    }
    NodeData::Comment(data) => {
      object.insert("text".to_string(), Value::String(data.text.clone()));
    }
  }

  if let Some(source) = serialize_source(&borrowed, registry) {
    object.insert("source".to_string(), source);
  }

  if let Some(raws) = serialize_raws(&borrowed.raws) {
    object.insert("raws".to_string(), raws);
  }

  if !borrowed.nodes.is_empty() {
    let children = borrowed
      .nodes
      .iter()
      .map(|child| serialize_node(child, registry))
      .collect();
    object.insert("nodes".to_string(), Value::Array(children));
  }

  Value::Object(object)
}

fn serialize_source(node: &Node, registry: &mut InputsRegistry) -> Option<Value> {
  if node.source.input.is_none() && node.source.start.is_none() && node.source.end.is_none() {
    return None;
  }

  let mut object = JsonMap::new();
  if let Some(input) = &node.source.input {
    let id = registry.register(input);
    object.insert(
      "inputId".to_string(),
      Value::Number(serde_json::Number::from(id as u64)),
    );
  }
  if let Some(start) = node.source.start.as_ref() {
    object.insert("start".to_string(), serialize_position(start));
  }
  if let Some(end) = node.source.end.as_ref() {
    object.insert("end".to_string(), serialize_position(end));
  }
  Some(Value::Object(object))
}

fn serialize_position(position: &Position) -> Value {
  let mut object = JsonMap::new();
  object.insert(
    "line".to_string(),
    Value::Number(serde_json::Number::from(position.line)),
  );
  object.insert(
    "column".to_string(),
    Value::Number(serde_json::Number::from(position.column)),
  );
  if position.offset != 0 {
    object.insert(
      "offset".to_string(),
      Value::Number(serde_json::Number::from(position.offset as u64)),
    );
  }
  Value::Object(object)
}

fn serialize_raws(raws: &RawData) -> Option<Value> {
  if raws.is_empty() {
    return None;
  }
  let mut root = JsonMap::new();
  for (key, value) in raws.iter() {
    insert_raw_value(&mut root, key, value);
  }
  Some(Value::Object(root))
}

fn insert_raw_value(target: &mut JsonMap<String, Value>, key: &str, value: &RawValue) {
  let parts: Vec<&str> = key.split('.').collect();
  insert_raw_segments(target, &parts, value);
}

fn insert_raw_segments(target: &mut JsonMap<String, Value>, parts: &[&str], value: &RawValue) {
  if let Some((first, rest)) = parts.split_first() {
    if rest.is_empty() {
      target.insert((*first).to_string(), raw_value_to_json(value));
    } else {
      let entry = target
        .entry((*first).to_string())
        .or_insert_with(|| Value::Object(JsonMap::new()));
      if let Value::Object(child) = entry {
        insert_raw_segments(child, rest, value);
      }
    }
  }
}

fn raw_value_to_json(value: &RawValue) -> Value {
  match value {
    RawValue::Text(text) => match text.as_str() {
      "true" => Value::Bool(true),
      "false" => Value::Bool(false),
      _ => Value::String(text.clone()),
    },
    RawValue::Value { value, raw } => {
      let mut object = JsonMap::new();
      object.insert("value".to_string(), Value::String(value.clone()));
      object.insert("raw".to_string(), Value::String(raw.clone()));
      Value::Object(object)
    }
  }
}
