#![allow(clippy::collapsible_match, clippy::double_ended_iterator_last)]

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::ast::nodes::Root;
use crate::ast::{Node, NodeData, NodeRef, RawCacheValue, RawValue};

fn default_raw(detect: &str) -> RawCacheValue {
  match detect {
    "after" => RawCacheValue::Text("\n".into()),
    "beforeClose" => RawCacheValue::Text("\n".into()),
    "beforeComment" => RawCacheValue::Text("\n".into()),
    "beforeDecl" => RawCacheValue::Text("\n".into()),
    "beforeOpen" => RawCacheValue::Text(" ".into()),
    "beforeRule" => RawCacheValue::Text("\n".into()),
    "colon" => RawCacheValue::Text(": ".into()),
    "commentLeft" => RawCacheValue::Text(" ".into()),
    "commentRight" => RawCacheValue::Text(" ".into()),
    "emptyBody" => RawCacheValue::Text(String::new()),
    "indent" => RawCacheValue::Text("    ".into()),
    "semicolon" => RawCacheValue::Flag(false),
    _ => RawCacheValue::Text(String::new()),
  }
}

fn collapse_non_space(value: &str) -> String {
  value.chars().filter(|c| c.is_whitespace()).collect()
}

fn strip_to_last_newline(value: &str) -> String {
  if let Some(idx) = value.rfind('\n') {
    value[..=idx].to_string()
  } else {
    value.to_string()
  }
}

fn capitalize(detect: &str) -> String {
  let mut chars = detect.chars();
  match chars.next() {
    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    None => String::new(),
  }
}

struct Stringifier<'a, B>
where
  B: FnMut(&str, Option<&NodeRef>, Option<&'static str>),
{
  builder: &'a mut B,
}

impl<'a, B> Stringifier<'a, B>
where
  B: FnMut(&str, Option<&NodeRef>, Option<&'static str>),
{
  fn builder(&mut self, text: &str, node: Option<&NodeRef>, string_type: Option<&'static str>) {
    (self.builder)(text, node, string_type);
  }

  fn stringify_node(&mut self, node: &NodeRef, semicolon: bool) {
    let node_type = node.borrow().type_name();
    match node_type {
      "atrule" => self.atrule(node, semicolon),
      "rule" => self.rule(node, semicolon),
      "decl" => self.decl(node, semicolon),
      "comment" => self.comment(node),
      "document" => self.document(node),
      "root" => self.root(node),
      _ => panic!(
        "Unknown AST node type {}. Maybe you need to change PostCSS stringifier.",
        node_type
      ),
    }
  }

  fn atrule(&mut self, node: &NodeRef, semicolon: bool) {
    let mut name = String::new();
    let mut has_children = false;
    let mut has_params = false;
    {
      let borrowed = node.borrow();
      if let NodeData::AtRule(data) = &borrowed.data {
        name = format!("@{}", data.name);
        has_children = !borrowed.nodes.is_empty();
        has_params = !data.params.is_empty();
        if let Some(after_name) = borrowed.raws.get_text("afterName") {
          name.push_str(after_name);
        } else if has_params {
          name.push(' ');
        }
      }
    }

    let params = if has_params {
      self.raw_value(node, "params")
    } else {
      String::new()
    };

    if has_children {
      let start = format!("{}{}", name, params);
      self.block(node, &start);
    } else {
      let mut end = self.raw_string(node, Some("between"), None);
      if semicolon {
        end.push(';');
      }
      self.builder(&(name + &params + &end), Some(node), None);
    }
  }

  fn before_after(&mut self, node: &NodeRef, detect: &str) -> String {
    let mut value = match node.borrow().type_name() {
      "decl" => self.raw_string(node, None, Some("beforeDecl")),
      "comment" => self.raw_string(node, None, Some("beforeComment")),
      _ => {
        if detect == "before" {
          self.raw_string(node, None, Some("beforeRule"))
        } else {
          self.raw_string(node, None, Some("beforeClose"))
        }
      }
    };

    let mut depth = 0;
    let mut current = node.borrow().parent();
    while let Some(parent) = current {
      if parent.borrow().type_name() == "root" {
        break;
      }
      depth += 1;
      current = parent.borrow().parent();
    }

    if value.contains('\n') {
      let indent = self.raw_string(node, None, Some("indent"));
      if !indent.is_empty() {
        for _ in 0..depth {
          value.push_str(&indent);
        }
      }
    }

    value
  }

  fn block(&mut self, node: &NodeRef, start: &str) {
    let between = self.raw_string(node, Some("between"), Some("beforeOpen"));
    self.builder(
      &(start.to_string() + &between + "{"),
      Some(node),
      Some("start"),
    );

    let after = {
      let borrowed = node.borrow();
      if !borrowed.nodes.is_empty() {
        drop(borrowed);
        self.body(node);
        self.raw_string(node, Some("after"), None)
      } else {
        drop(borrowed);
        self.raw_string(node, Some("after"), Some("emptyBody"))
      }
    };

    if !after.is_empty() {
      self.builder(&after, None, None);
    }
    self.builder("}", Some(node), Some("end"));
  }

  fn body(&mut self, node: &NodeRef) {
    let children = node.borrow().nodes.clone();
    let mut last = if children.is_empty() {
      0
    } else {
      children.len() - 1
    };
    while last > 0 {
      let is_comment = {
        let borrowed = children[last].borrow();
        matches!(borrowed.data, NodeData::Comment(_))
      };
      if !is_comment {
        break;
      }
      last -= 1;
    }

    let semicolon = self.raw_bool(node, Some("semicolon"), None);
    for (index, child) in children.into_iter().enumerate() {
      let before = self.raw_string(&child, Some("before"), None);
      if !before.is_empty() {
        self.builder(&before, None, None);
      }
      let child_semicolon = index != last || semicolon;
      self.stringify_node(&child, child_semicolon);
    }
  }

  fn comment(&mut self, node: &NodeRef) {
    let left = self.raw_string(node, Some("left"), Some("commentLeft"));
    let right = self.raw_string(node, Some("right"), Some("commentRight"));
    let text = {
      let borrowed = node.borrow();
      if let NodeData::Comment(data) = &borrowed.data {
        data.text.clone()
      } else {
        String::new()
      }
    };
    self.builder(&format!("/*{}{}{}*/", left, text, right), Some(node), None);
  }

  fn decl(&mut self, node: &NodeRef, semicolon: bool) {
    let between = self.raw_string(node, Some("between"), Some("colon"));
    let (prop, _value, important_text) = {
      let borrowed = node.borrow();
      if let NodeData::Declaration(data) = &borrowed.data {
        let important = if data.important {
          borrowed
            .raws
            .get_text("important")
            .map(|s| s.to_string())
            .unwrap_or_else(|| " !important".into())
        } else {
          String::new()
        };
        (data.prop.clone(), data.value.clone(), important)
      } else {
        (String::new(), String::new(), String::new())
      }
    };

    let mut string = format!("{}{}{}", prop, between, self.raw_value(node, "value"));
    if !important_text.is_empty() {
      string.push_str(&important_text);
    }
    if semicolon {
      string.push(';');
    }
    self.builder(&string, Some(node), None);
  }

  fn document(&mut self, node: &NodeRef) {
    self.body(node);
  }

  fn raw(&mut self, node: &NodeRef, own: Option<&str>, detect: Option<&str>) -> RawCacheValue {
    let detect_key = detect.or(own).expect("detect or own key");
    if let Some(own_key) = own {
      if let Some(raw_value) = node.borrow().raws.get(own_key) {
        return match raw_value {
          RawValue::Text(text) => RawCacheValue::Text(text.clone()),
          RawValue::Value { raw, .. } => RawCacheValue::Text(raw.clone()),
        };
      }
    }

    let parent = node.borrow().parent();
    if detect_key == "before" {
      if parent.is_none() {
        return RawCacheValue::Text(String::new());
      }
      if let Some(parent_ref) = &parent {
        let parent_type = parent_ref.borrow().type_name().to_string();
        if parent_type == "root" {
          let is_first = parent_ref
            .borrow()
            .nodes
            .first()
            .map(|item| Rc::ptr_eq(item, node))
            .unwrap_or(false);
          if is_first {
            return RawCacheValue::Text(String::new());
          }
        }
        if parent_type == "document" {
          return RawCacheValue::Text(String::new());
        }
      }
    }

    if parent.is_none() {
      return default_raw(detect_key);
    }

    let root = Node::root(node);
    {
      let mut root_borrow = root.borrow_mut();
      let cache = root_borrow.raw_cache.get_or_insert_with(BTreeMap::new);
      if let Some(cached) = cache.get(detect_key) {
        return cached.clone();
      }
    }

    let resolved = if detect_key == "before" || detect_key == "after" {
      RawCacheValue::Text(self.before_after(node, detect_key))
    } else {
      let value = match detect_key {
        "beforeClose" => RawCacheValue::Text(self.raw_before_close(&root)),
        "beforeComment" => RawCacheValue::Text(self.raw_before_comment(&root, node)),
        "beforeDecl" => RawCacheValue::Text(self.raw_before_decl(&root, node)),
        "beforeOpen" => {
          if let Some(v) = self.raw_before_open(&root) {
            RawCacheValue::Text(v)
          } else {
            RawCacheValue::Text(String::new())
          }
        }
        "beforeRule" => RawCacheValue::Text(self.raw_before_rule(&root)),
        "colon" => RawCacheValue::Text(self.raw_colon(&root)),
        "emptyBody" => RawCacheValue::Text(self.raw_empty_body(&root)),
        "indent" => RawCacheValue::Text(self.raw_indent(&root)),
        "semicolon" => RawCacheValue::Flag(self.raw_semicolon(&root)),
        other => {
          let method_name = format!("raw{}", capitalize(other));
          if method_name == "rawAfter" {
            RawCacheValue::Text(self.raw_after(&root, own))
          } else {
            self
              .raw_from_children(&root, own)
              .unwrap_or_else(|| default_raw(detect_key))
          }
        }
      };
      value
    };

    {
      let mut root_borrow = root.borrow_mut();
      let cache = root_borrow.raw_cache.get_or_insert_with(BTreeMap::new);
      cache.insert(detect_key.to_string(), resolved.clone());
    }

    resolved
  }

  fn raw_after(&self, root: &NodeRef, own: Option<&str>) -> String {
    if let Some(own_key) = own {
      if let Some(value) = self.raw_from_children(root, Some(own_key)) {
        if let RawCacheValue::Text(text) = value {
          return text;
        }
      }
    }
    String::new()
  }

  fn raw_before_close(&self, root: &NodeRef) -> String {
    let mut value: Option<String> = None;
    let root_wrapper = Root::from_node(root.clone());
    root_wrapper.walk(|node, _| {
      let result = {
        let borrowed = node.borrow();
        if !borrowed.nodes.is_empty() {
          if let Some(after) = borrowed.raws.get_text("after") {
            let mut text = after.to_string();
            if text.contains('\n') {
              text = strip_to_last_newline(&text);
            }
            Some(text)
          } else {
            None
          }
        } else {
          None
        }
      };
      if let Some(text) = result {
        value = Some(text);
        return false;
      }
      true
    });

    if let Some(mut result) = value {
      if !result.is_empty() {
        result = collapse_non_space(&result);
      }
      result
    } else {
      String::new()
    }
  }

  fn raw_before_comment(&mut self, root: &NodeRef, node: &NodeRef) -> String {
    let mut value: Option<String> = None;
    let root_wrapper = Root::from_node(root.clone());
    root_wrapper.walk_comments(|comment, _| {
      if let Some(before) = comment.borrow().raws.get_text("before") {
        let mut text = before.to_string();
        if text.contains('\n') {
          text = strip_to_last_newline(&text);
        }
        value = Some(text);
        return false;
      }
      true
    });

    if let Some(mut result) = value {
      if !result.is_empty() {
        result = collapse_non_space(&result);
      }
      result
    } else {
      self.raw_string(node, None, Some("beforeDecl"))
    }
  }

  fn raw_before_decl(&mut self, root: &NodeRef, node: &NodeRef) -> String {
    let mut value: Option<String> = None;
    let root_wrapper = Root::from_node(root.clone());
    root_wrapper.walk_decls(|decl, _| {
      if let Some(before) = decl.borrow().raws.get_text("before") {
        let mut text = before.to_string();
        if text.contains('\n') {
          text = strip_to_last_newline(&text);
        }
        value = Some(text);
        return false;
      }
      true
    });

    if let Some(mut result) = value {
      if !result.is_empty() {
        result = collapse_non_space(&result);
      }
      result
    } else {
      self.raw_string(node, None, Some("beforeRule"))
    }
  }

  fn raw_before_open(&self, root: &NodeRef) -> Option<String> {
    let mut value: Option<String> = None;
    let root_wrapper = Root::from_node(root.clone());
    root_wrapper.walk(|child, _| {
      let borrowed = child.borrow();
      if !matches!(borrowed.data, NodeData::Declaration(_)) {
        if let Some(between) = borrowed.raws.get_text("between") {
          value = Some(between.to_string());
          return false;
        }
      }
      true
    });
    value
  }

  fn raw_before_rule(&self, root: &NodeRef) -> String {
    let mut value: Option<String> = None;
    let root_wrapper = Root::from_node(root.clone());
    root_wrapper.walk(|child, _| {
      let candidate = {
        let borrowed = child.borrow();
        if borrowed.nodes.is_empty() {
          None
        } else if let Some(before) = borrowed.raws.get_text("before") {
          let mut text = before.to_string();
          if text.contains('\n') {
            text = strip_to_last_newline(&text);
          }
          Some((borrowed.parent(), text))
        } else {
          None
        }
      };

      if let Some((parent_opt, text)) = candidate {
        let should_use = if let Some(parent) = parent_opt {
          if Rc::ptr_eq(&parent, root) {
            let is_first = parent
              .borrow()
              .nodes
              .first()
              .map(|first| Rc::ptr_eq(first, &child))
              .unwrap_or(false);
            !is_first
          } else {
            true
          }
        } else {
          false
        };

        if should_use {
          value = Some(text);
          return false;
        }
      }

      true
    });

    if let Some(result) = value {
      collapse_non_space(&result)
    } else {
      String::new()
    }
  }

  fn raw_colon(&self, root: &NodeRef) -> String {
    let mut value: Option<String> = None;
    let root_wrapper = Root::from_node(root.clone());
    root_wrapper.walk_decls(|decl, _| {
      if let Some(between) = decl.borrow().raws.get_text("between") {
        let filtered: String = between
          .chars()
          .filter(|c| c.is_whitespace() || *c == ':')
          .collect();
        value = Some(filtered);
        return false;
      }
      true
    });
    value.unwrap_or_else(|| match default_raw("colon") {
      RawCacheValue::Text(text) => text,
      RawCacheValue::Flag(flag) => {
        if flag {
          "true".into()
        } else {
          String::new()
        }
      }
    })
  }

  fn raw_empty_body(&self, root: &NodeRef) -> String {
    let mut value: Option<String> = None;
    let root_wrapper = Root::from_node(root.clone());
    root_wrapper.walk(|child, _| {
      let borrowed = child.borrow();
      if borrowed.nodes.is_empty() {
        if let Some(after) = borrowed.raws.get_text("after") {
          value = Some(after.to_string());
          return false;
        }
      }
      true
    });
    value.unwrap_or_default()
  }

  fn raw_indent(&self, root: &NodeRef) -> String {
    if let Some(indent) = root.borrow().raws.get_text("indent") {
      return indent.to_string();
    }
    let mut value: Option<String> = None;
    let root_wrapper = Root::from_node(root.clone());
    root_wrapper.walk(|child, _| {
      let mut before_value = None;
      {
        let borrowed = child.borrow();
        if let Some(parent) = borrowed.parent() {
          if !Rc::ptr_eq(&parent, root) {
            if let Some(grand) = parent.borrow().parent() {
              if Rc::ptr_eq(&grand, root) {
                if let Some(before) = borrowed.raws.get_text("before") {
                  before_value = Some(before.to_string());
                }
              }
            }
          }
        }
      }

      if let Some(before) = before_value {
        if let Some(last) = before.split('\n').last() {
          value = Some(collapse_non_space(last));
          return false;
        }
      }

      true
    });
    value.unwrap_or_default()
  }

  fn raw_semicolon(&self, root: &NodeRef) -> bool {
    let mut flag = None;
    let root_wrapper = Root::from_node(root.clone());
    root_wrapper.walk(|child, _| {
      let mut last_child = None;
      let mut semicolon_raw = None;
      let has_children = {
        let borrowed = child.borrow();
        if borrowed.nodes.is_empty() {
          false
        } else {
          last_child = borrowed.nodes.last().cloned();
          semicolon_raw = borrowed.raws.get_text("semicolon").map(|s| s.to_string());
          true
        }
      };

      if !has_children {
        return true;
      }

      if let Some(last) = last_child {
        if matches!(last.borrow().data, NodeData::Declaration(_)) {
          let value = semicolon_raw.map(|s| s != "false").unwrap_or(false);
          flag = Some(value);
          return false;
        }
      }

      true
    });
    flag.unwrap_or(false)
  }

  fn raw_string(&mut self, node: &NodeRef, own: Option<&str>, detect: Option<&str>) -> String {
    match self.raw(node, own, detect) {
      RawCacheValue::Text(text) => text,
      RawCacheValue::Flag(flag) => {
        if flag {
          String::from("true")
        } else {
          String::new()
        }
      }
    }
  }

  fn raw_bool(&mut self, node: &NodeRef, own: Option<&str>, detect: Option<&str>) -> bool {
    match self.raw(node, own, detect) {
      RawCacheValue::Flag(flag) => flag,
      RawCacheValue::Text(text) => !text.is_empty() && text != "false",
    }
  }

  fn raw_value(&self, node: &NodeRef, prop: &str) -> String {
    let (value, raw_pair) = {
      let borrowed = node.borrow();
      let (value, raw) = match (&borrowed.data, prop) {
        (NodeData::Rule(data), "selector") => (Some(&data.selector), borrowed.raws.get("selector")),
        (NodeData::AtRule(data), "params") => (Some(&data.params), borrowed.raws.get("params")),
        (NodeData::Declaration(data), "value") => (Some(&data.value), borrowed.raws.get("value")),
        _ => (None, None),
      };
      (value.cloned(), raw.cloned())
    };

    if let (Some(value), Some(RawValue::Value { value: stored, raw })) =
      (value.clone(), raw_pair.clone())
    {
      if stored == value {
        return raw;
      }
    }

    value.unwrap_or_default()
  }

  fn raw_from_children(&self, root: &NodeRef, own: Option<&str>) -> Option<RawCacheValue> {
    let key = own?;
    let mut found: Option<RawCacheValue> = None;
    let root_wrapper = Root::from_node(root.clone());
    root_wrapper.walk(|child, _| {
      if let Some(raw_value) = child.borrow().raws.get(key) {
        found = Some(match raw_value {
          RawValue::Text(text) => RawCacheValue::Text(text.clone()),
          RawValue::Value { raw, .. } => RawCacheValue::Text(raw.clone()),
        });
        return false;
      }
      true
    });
    found
  }

  fn root(&mut self, node: &NodeRef) {
    self.body(node);
    if let Some(after) = node.borrow().raws.get_text("after") {
      if !after.is_empty() {
        self.builder(after, None, None);
      }
    }
  }

  fn rule(&mut self, node: &NodeRef, _semicolon: bool) {
    let selector = self.raw_value(node, "selector");
    self.block(node, &selector);
    if let Some(own_semicolon) = node.borrow().raws.get_text("ownSemicolon") {
      if !own_semicolon.is_empty() {
        self.builder(own_semicolon, Some(node), Some("end"));
      }
    }
  }
}

pub fn stringify_with_builder<F>(node: &NodeRef, mut builder: F)
where
  F: FnMut(&str, Option<&NodeRef>, Option<&'static str>),
{
  let mut stringifier = Stringifier {
    builder: &mut builder,
  };
  stringifier.stringify_node(node, false);
}

pub fn stringify(root: &Root) -> String {
  let mut output = String::new();
  stringify_with_builder(root.raw(), |text, _, _| output.push_str(text));
  output
}

pub fn raw(node: &NodeRef, own: Option<&str>, detect: Option<&str>) -> RawCacheValue {
  let mut noop = |_: &str, _: Option<&NodeRef>, _: Option<&'static str>| {};
  let mut stringifier = Stringifier { builder: &mut noop };
  stringifier.raw(node, own, detect)
}

pub fn node_to_string(node: &NodeRef) -> String {
  let mut output = String::new();
  stringify_with_builder(node, |text, _, _| output.push_str(text));
  output
}
