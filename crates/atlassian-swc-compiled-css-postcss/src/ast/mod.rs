use std::borrow::Cow;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::BTreeMap;
use std::fmt;
use std::rc::{Rc, Weak};

use crate::css_syntax_error::CssSyntaxError;
use crate::input::{InputRef, Position};
use crate::result::{Result as PostcssResult, Warning, WarningOptions};
use crate::stringifier;

pub mod nodes;

/// Shared pointer to a node in the PostCSS AST.
pub type NodeRef = Rc<RefCell<Node>>;

/// Weak pointer used to avoid reference cycles when pointing to a parent node.
pub type WeakNodeRef = Weak<RefCell<Node>>;

/// Container holding lazily-created iteration state, mirroring PostCSS' `lastEach`
/// and `indexes` bookkeeping so visitor ordering matches JavaScript.
#[derive(Clone, Debug, Default)]
pub struct IterationState {
  last_each: u32,
  indexes: BTreeMap<u32, isize>,
}

impl IterationState {
  pub fn begin(&mut self) -> u32 {
    self.last_each = self.last_each.wrapping_add(1);
    self.indexes.insert(self.last_each, 0);
    self.last_each
  }

  pub fn current(&self, id: u32) -> Option<usize> {
    self
      .indexes
      .get(&id)
      .map(|value| if *value < 0 { 0 } else { *value as usize })
  }

  pub fn current_raw(&self, id: u32) -> Option<isize> {
    self.indexes.get(&id).copied()
  }

  pub fn advance(&mut self, id: u32, next: isize) {
    if let Some(entry) = self.indexes.get_mut(&id) {
      *entry = next;
    }
  }

  pub fn finish(&mut self, id: u32) {
    self.indexes.remove(&id);
  }

  pub fn adjust_insert(&mut self, index: usize, added: usize) {
    let index = index as isize;
    let added = added as isize;
    for value in self.indexes.values_mut() {
      if *value >= index {
        *value += added;
      }
    }
  }

  pub fn adjust_remove(&mut self, index: usize) {
    let index = index as isize;
    for value in self.indexes.values_mut() {
      if *value >= index {
        *value -= 1;
      }
    }
  }

  pub fn clear(&mut self) {
    self.indexes.clear();
  }
}

/// Additional metadata kept on nodes to mirror the mutable JavaScript objects.
#[derive(Clone, Debug, Default)]
pub struct NodeFlags {
  pub is_clean: bool,
  pub proxy_owned: bool,
  pub iteration: RefCell<IterationState>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RawCacheValue {
  Text(String),
  Flag(bool),
}

impl RawCacheValue {
  pub fn as_text(&self) -> Option<&str> {
    match self {
      RawCacheValue::Text(s) => Some(s.as_str()),
      RawCacheValue::Flag(_) => None,
    }
  }

  pub fn as_flag(&self) -> Option<bool> {
    match self {
      RawCacheValue::Flag(flag) => Some(*flag),
      RawCacheValue::Text(text) => Some(!text.is_empty()),
    }
  }
}

pub type RawCache = BTreeMap<String, RawCacheValue>;

/// Common source information available on every AST node.
#[derive(Clone, Debug, Default)]
pub struct Source {
  pub input: Option<InputRef>,
  pub start: Option<Position>,
  pub end: Option<Position>,
}

#[derive(Clone, Debug, Default)]
pub struct PositionByOptions {
  pub index: Option<usize>,
  pub word: Option<String>,
  pub string: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct RangeByOptions {
  pub start: Option<Position>,
  pub end: Option<Position>,
  pub index: Option<usize>,
  pub end_index: Option<usize>,
  pub word: Option<String>,
  pub string: Option<String>,
}

/// Representation of the raw fragments kept on PostCSS nodes. Values can be
/// simple strings or structured `{ value, raw }` pairs as produced by the
/// parser when it preserves the original source lexeme.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawValue {
  Text(String),
  Value { value: String, raw: String },
}

impl RawValue {
  pub fn as_text(&self) -> Option<&str> {
    match self {
      RawValue::Text(s) => Some(s.as_str()),
      RawValue::Value { raw, .. } => Some(raw.as_str()),
    }
  }

  pub fn as_value_pair(&self) -> Option<(&str, &str)> {
    match self {
      RawValue::Value { value, raw } => Some((value.as_str(), raw.as_str())),
      _ => None,
    }
  }
}

/// Generic key-value map that stores the raw string fragments preserved during
/// parsing/stringifying. PostCSS keeps these values in nested objects; we model
/// them as a tree of maps keyed by dotted paths.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RawData {
  values: BTreeMap<String, RawValue>,
}

impl RawData {
  pub fn get(&self, key: &str) -> Option<&RawValue> {
    self.values.get(key)
  }

  pub fn get_text(&self, key: &str) -> Option<&str> {
    self.values.get(key).and_then(RawValue::as_text)
  }

  pub fn set_text<S: Into<String>>(&mut self, key: &str, value: S) {
    self
      .values
      .insert(key.to_string(), RawValue::Text(value.into()));
  }

  pub fn set_bool(&mut self, key: &str, value: bool) {
    self.values.insert(
      key.to_string(),
      RawValue::Text(if value { "true".into() } else { "false".into() }),
    );
  }

  pub fn set_value_pair<S: Into<String>, R: Into<String>>(&mut self, key: &str, value: S, raw: R) {
    self.values.insert(
      key.to_string(),
      RawValue::Value {
        value: value.into(),
        raw: raw.into(),
      },
    );
  }

  pub fn remove(&mut self, key: &str) {
    self.values.remove(key);
  }

  pub fn merge(&mut self, other: &RawData) {
    for (k, v) in &other.values {
      self.values.insert(k.clone(), v.clone());
    }
  }

  pub fn is_empty(&self) -> bool {
    self.values.is_empty()
  }

  pub fn iter(&self) -> impl Iterator<Item = (&String, &RawValue)> {
    self.values.iter()
  }
}

/// Strongly-typed payload stored inside each [`Node`].
#[derive(Clone, Debug)]
pub enum NodeData {
  Root(nodes::RootData),
  Document(nodes::DocumentData),
  Rule(nodes::RuleData),
  AtRule(nodes::AtRuleData),
  Declaration(nodes::DeclarationData),
  Comment(nodes::CommentData),
}

impl NodeData {
  pub fn is_container(&self) -> bool {
    matches!(
      self,
      NodeData::Root(_) | NodeData::Document(_) | NodeData::Rule(_) | NodeData::AtRule(_)
    )
  }
}

/// Core mutable node representation mimicking the behaviour of JavaScript PostCSS nodes.
#[derive(Clone, Debug)]
pub struct Node {
  pub data: NodeData,
  pub parent: Option<WeakNodeRef>,
  pub source: Source,
  pub raws: RawData,
  pub flags: NodeFlags,
  pub nodes: Vec<NodeRef>,
  pub raw_cache: Option<RawCache>,
}

impl Node {
  pub fn new(data: NodeData) -> NodeRef {
    Rc::new(RefCell::new(Self {
      data,
      parent: None,
      source: Source::default(),
      raws: RawData::default(),
      flags: NodeFlags {
        is_clean: false,
        proxy_owned: false,
        iteration: RefCell::new(IterationState::default()),
      },
      nodes: Vec::new(),
      raw_cache: None,
    }))
  }

  pub fn downgrade(node: &NodeRef) -> WeakNodeRef {
    Rc::downgrade(node)
  }

  pub fn upgrade(parent: &WeakNodeRef) -> Option<NodeRef> {
    parent.upgrade()
  }

  pub fn parent_ref(node: &NodeRef) -> Option<NodeRef> {
    node.borrow().parent()
  }

  pub fn index(node: &NodeRef) -> Option<usize> {
    Node::parent_and_index(node).map(|(_, index)| index)
  }

  fn parent_and_index(node: &NodeRef) -> Option<(NodeRef, usize)> {
    let parent = node.borrow().parent()?;
    let index = {
      let parent_borrow = parent.borrow();
      parent_borrow
        .nodes
        .iter()
        .position(|child| Rc::ptr_eq(child, node))
    }?;
    Some((parent, index))
  }

  fn detach(node: &NodeRef) {
    if let Some((parent, index)) = Node::parent_and_index(node) {
      Node::remove(&parent, index);
    }
  }

  fn normalize_incoming<I>(nodes: I) -> Vec<NodeRef>
  where
    I: IntoIterator<Item = NodeRef>,
  {
    nodes.into_iter().inspect(Node::detach).collect()
  }

  fn insert_all(parent: &NodeRef, index: usize, nodes: Vec<NodeRef>) {
    for (offset, child) in nodes.into_iter().enumerate() {
      Node::insert(parent, index + offset, child);
    }
  }

  pub fn index_of(parent: &NodeRef, child: &NodeRef) -> Option<usize> {
    let parent_borrow = parent.borrow();
    parent_borrow
      .nodes
      .iter()
      .position(|node| Rc::ptr_eq(node, child))
  }

  pub fn first_child(parent: &NodeRef) -> Option<NodeRef> {
    parent.borrow().nodes.first().cloned()
  }

  pub fn last_child(parent: &NodeRef) -> Option<NodeRef> {
    parent.borrow().nodes.last().cloned()
  }

  pub fn insert_before<I>(node: &NodeRef, new_nodes: I)
  where
    I: IntoIterator<Item = NodeRef>,
  {
    let (parent, index) = match Node::parent_and_index(node) {
      Some(value) => value,
      None => return,
    };
    let nodes = Node::normalize_incoming(new_nodes);
    Node::insert_all(&parent, index, nodes);
  }

  pub fn insert_after<I>(node: &NodeRef, new_nodes: I)
  where
    I: IntoIterator<Item = NodeRef>,
  {
    let (parent, index) = match Node::parent_and_index(node) {
      Some(value) => value,
      None => return,
    };
    let nodes = Node::normalize_incoming(new_nodes);
    Node::insert_all(&parent, index + 1, nodes);
  }

  pub fn replace_with<I>(node: &NodeRef, new_nodes: I)
  where
    I: IntoIterator<Item = NodeRef>,
  {
    let (parent, index) = match Node::parent_and_index(node) {
      Some(value) => value,
      None => return,
    };
    let nodes = Node::normalize_incoming(new_nodes);
    Node::remove(&parent, index);
    if nodes.is_empty() {
      return;
    }
    Node::insert_all(&parent, index, nodes);
  }

  pub fn remove_self(node: &NodeRef) {
    Node::detach(node);
  }

  fn sibling(node: &NodeRef, offset: isize) -> Option<NodeRef> {
    let (parent, index) = Node::parent_and_index(node)?;
    let target = index as isize + offset;
    if target < 0 {
      return None;
    }
    let next = {
      let parent_borrow = parent.borrow();
      parent_borrow.nodes.get(target as usize).cloned()
    };
    next
  }

  pub fn next(node: &NodeRef) -> Option<NodeRef> {
    Node::sibling(node, 1)
  }

  pub fn prev(node: &NodeRef) -> Option<NodeRef> {
    Node::sibling(node, -1)
  }

  pub fn move_to(node: &NodeRef, container: &NodeRef) {
    Node::detach(node);
    Node::append(container, node.clone());
  }

  pub fn move_before(node: &NodeRef, other: &NodeRef) {
    Node::detach(node);
    Node::insert_before(other, std::iter::once(node.clone()));
  }

  pub fn move_after(node: &NodeRef, other: &NodeRef) {
    Node::detach(node);
    Node::insert_after(other, std::iter::once(node.clone()));
  }

  pub fn kind(&self) -> nodes::NodeKind {
    match &self.data {
      NodeData::Root(_) => nodes::NodeKind::Root,
      NodeData::Document(_) => nodes::NodeKind::Document,
      NodeData::Rule(_) => nodes::NodeKind::Rule,
      NodeData::AtRule(_) => nodes::NodeKind::AtRule,
      NodeData::Declaration(_) => nodes::NodeKind::Declaration,
      NodeData::Comment(_) => nodes::NodeKind::Comment,
    }
  }

  pub fn type_name(&self) -> &'static str {
    match &self.data {
      NodeData::Root(_) => "root",
      NodeData::Document(_) => "document",
      NodeData::Rule(_) => "rule",
      NodeData::AtRule(_) => "atrule",
      NodeData::Declaration(_) => "decl",
      NodeData::Comment(_) => "comment",
    }
  }

  pub fn root(node: &NodeRef) -> NodeRef {
    find_root(node)
  }

  pub fn as_root(&self) -> Option<&nodes::RootData> {
    match &self.data {
      NodeData::Root(data) => Some(data),
      _ => None,
    }
  }

  pub fn as_root_mut(&mut self) -> Option<&mut nodes::RootData> {
    match &mut self.data {
      NodeData::Root(data) => Some(data),
      _ => None,
    }
  }

  pub fn as_document_mut(&mut self) -> Option<&mut nodes::DocumentData> {
    match &mut self.data {
      NodeData::Document(data) => Some(data),
      _ => None,
    }
  }

  pub fn as_rule_mut(&mut self) -> Option<&mut nodes::RuleData> {
    match &mut self.data {
      NodeData::Rule(data) => Some(data),
      _ => None,
    }
  }

  pub fn as_at_rule_mut(&mut self) -> Option<&mut nodes::AtRuleData> {
    match &mut self.data {
      NodeData::AtRule(data) => Some(data),
      _ => None,
    }
  }

  pub fn as_declaration_mut(&mut self) -> Option<&mut nodes::DeclarationData> {
    match &mut self.data {
      NodeData::Declaration(data) => Some(data),
      _ => None,
    }
  }

  pub fn as_comment_mut(&mut self) -> Option<&mut nodes::CommentData> {
    match &mut self.data {
      NodeData::Comment(data) => Some(data),
      _ => None,
    }
  }

  pub fn set_parent(node: &NodeRef, parent: Option<&NodeRef>) {
    if let Some(parent_ref) = parent {
      node.borrow_mut().parent = Some(Node::downgrade(parent_ref));
    } else {
      node.borrow_mut().parent = None;
    }
  }

  pub fn parent(&self) -> Option<NodeRef> {
    self.parent.as_ref().and_then(Node::upgrade)
  }

  pub fn mark_dirty(&mut self) {
    if self.flags.is_clean {
      self.flags.is_clean = false;
      let mut current = self.parent();
      while let Some(parent) = current {
        let mut parent_mut = parent.borrow_mut();
        if !parent_mut.flags.is_clean {
          break;
        }
        parent_mut.flags.is_clean = false;
        parent_mut.raw_cache = None;
        current = parent_mut.parent();
      }
    }
    self.raw_cache = None;
  }

  pub fn clone_node(node: &NodeRef, parent: Option<&NodeRef>) -> NodeRef {
    let cloned_inner = {
      let inner = node.borrow();
      let mut new_node = Node {
        data: inner.data.clone(),
        parent: None,
        source: inner.source.clone(),
        raws: inner.raws.clone(),
        flags: NodeFlags {
          is_clean: inner.flags.is_clean,
          proxy_owned: inner.flags.proxy_owned,
          iteration: RefCell::new(inner.flags.iteration.borrow().clone()),
        },
        nodes: Vec::new(),
        raw_cache: inner.raw_cache.clone(),
      };
      match &inner.data {
        NodeData::Root(_) | NodeData::Document(_) | NodeData::Rule(_) | NodeData::AtRule(_) => {
          new_node.nodes = inner
            .nodes
            .iter()
            .map(|child| Node::clone_node(child, None))
            .collect();
        }
        _ => {}
      }
      new_node
    };

    let result = Rc::new(RefCell::new(cloned_inner));

    if let Some(parent_ref) = parent {
      Node::set_parent(&result, Some(parent_ref));
    }

    if matches!(
      result.borrow().data,
      NodeData::Root(_) | NodeData::Document(_) | NodeData::Rule(_) | NodeData::AtRule(_)
    ) {
      let parent_clone = result.clone();
      for child in &result.borrow().nodes {
        Node::set_parent(child, Some(&parent_clone));
      }
    }

    result
  }

  pub fn append(parent: &NodeRef, child: NodeRef) {
    child.borrow_mut().parent = Some(Node::downgrade(parent));
    let mut parent_mut = parent.borrow_mut();
    parent_mut.mark_dirty();
    parent_mut.nodes.push(child);
  }

  pub fn insert(parent: &NodeRef, index: usize, child: NodeRef) {
    child.borrow_mut().parent = Some(Node::downgrade(parent));
    let mut parent_mut = parent.borrow_mut();
    parent_mut
      .flags
      .iteration
      .borrow_mut()
      .adjust_insert(index, 1);
    parent_mut.mark_dirty();
    parent_mut.nodes.insert(index, child);
  }

  pub fn remove(parent: &NodeRef, index: usize) -> NodeRef {
    let mut parent_mut = parent.borrow_mut();
    let child = parent_mut.nodes.remove(index);
    parent_mut.flags.iteration.borrow_mut().adjust_remove(index);
    parent_mut.mark_dirty();
    child.borrow_mut().parent = None;
    child
  }

  pub fn remove_all(parent: &NodeRef) {
    let nodes = {
      let mut parent_mut = parent.borrow_mut();
      if parent_mut.nodes.is_empty() {
        return;
      }
      parent_mut.flags.iteration.borrow_mut().clear();
      parent_mut.mark_dirty();
      std::mem::take(&mut parent_mut.nodes)
    };

    for child in nodes {
      child.borrow_mut().parent = None;
    }
  }

  pub fn clean_raws(node: &NodeRef, keep_between: bool) {
    let mut inner = node.borrow_mut();
    inner.raws.remove("before");
    inner.raws.remove("after");
    if !keep_between {
      inner.raws.remove("between");
    }
  }

  pub fn walk_children<F>(node: &NodeRef, callback: &mut F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::each(node, callback)
  }

  pub fn each<F>(node: &NodeRef, callback: &mut F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    let iterator = {
      let borrowed = node.borrow();
      let mut state = borrowed.flags.iteration.borrow_mut();
      state.begin()
    };

    let mut completed = true;
    loop {
      let (index, child) = {
        let borrowed = node.borrow();
        let current_index = borrowed
          .flags
          .iteration
          .borrow()
          .current(iterator)
          .unwrap_or(0);
        let child = borrowed.nodes.get(current_index).cloned();
        (current_index, child)
      };

      let Some(child) = child else {
        break;
      };

      if !callback(child.clone(), index) {
        completed = false;
        break;
      }

      let next = {
        let borrowed = node.borrow();
        let state = borrowed.flags.iteration.borrow();
        state
          .current_raw(iterator)
          .unwrap_or(index as isize)
          .saturating_add(1)
      };
      {
        let borrowed = node.borrow();
        let mut state = borrowed.flags.iteration.borrow_mut();
        state.advance(iterator, next);
      }
    }

    {
      let borrowed = node.borrow();
      let mut state = borrowed.flags.iteration.borrow_mut();
      state.finish(iterator);
    }
    completed
  }

  pub fn every<F>(node: &NodeRef, callback: &mut F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    let mut all = true;
    Node::each(node, &mut |child, index| {
      if !callback(child.clone(), index) {
        all = false;
        return false;
      }
      true
    });
    all
  }

  pub fn some<F>(node: &NodeRef, callback: &mut F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    let mut any = false;
    Node::each(node, &mut |child, index| {
      if callback(child.clone(), index) {
        any = true;
        return false;
      }
      true
    });
    any
  }

  pub fn walk<F>(node: &NodeRef, callback: &mut F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    let mut result = true;
    let mut walker = |child: NodeRef, index: usize| {
      if !result {
        return false;
      }
      if !callback(child.clone(), index) {
        result = false;
        return false;
      }
      if child.borrow().data.is_container() && !Node::walk(&child, callback) {
        result = false;
        return false;
      }
      true
    };

    Node::each(node, &mut walker);
    result
  }

  pub fn walk_filtered<F, P>(node: &NodeRef, predicate: &P, callback: &mut F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
    P: Fn(&NodeRef) -> bool,
  {
    let mut result = true;
    let mut walker = |child: NodeRef, index: usize| {
      if !result {
        return false;
      }
      if predicate(&child) && !callback(child.clone(), index) {
        result = false;
        return false;
      }
      true
    };
    Node::walk(node, &mut walker);
    result
  }

  pub fn to_css(node: &NodeRef) -> String {
    stringifier::node_to_string(node)
  }

  pub fn raw_value(node: &NodeRef, own: Option<&str>, detect: Option<&str>) -> RawCacheValue {
    stringifier::raw(node, own, detect)
  }

  pub fn position_inside(node: &NodeRef, index: usize, string: Option<&str>) -> Option<Position> {
    let start = { node.borrow().source.start.clone()? };
    let representation: Cow<'_, str> = match string {
      Some(value) => Cow::Borrowed(value),
      None => Cow::Owned(Node::to_css(node)),
    };

    let mut line = start.line;
    let mut column = start.column;
    let mut offset = start.offset;
    let units: Vec<u16> = representation.encode_utf16().collect();

    for i in 0..index {
      match units.get(i).copied() {
        Some(unit) if unit == '\n' as u16 => {
          line += 1;
          column = 1;
        }
        _ => {
          column += 1;
        }
      }
      offset += 1;
    }

    Some(Position {
      line,
      column,
      offset,
    })
  }

  pub fn position_by(node: &NodeRef, opts: &PositionByOptions) -> Option<Position> {
    let start = { node.borrow().source.start.clone()? };

    if let Some(index) = opts.index {
      return Node::position_inside(node, index, opts.string.as_deref());
    }

    if let Some(word) = &opts.word {
      let representation = opts
        .string
        .as_ref()
        .map(|string| Cow::Borrowed(string.as_str()))
        .unwrap_or_else(|| Cow::Owned(Node::to_css(node)));

      if let Some(index) = representation.as_ref().find(word) {
        return Node::position_inside(node, index, Some(representation.as_ref()));
      }
    }

    Some(start)
  }

  pub fn range_by(node: &NodeRef, opts: &RangeByOptions) -> Option<(Position, Position)> {
    let (mut start, default_end) = {
      let borrowed = node.borrow();
      let source = borrowed.source.start.clone()?;
      let end = borrowed.source.end.clone();
      let default_end = end.map(|pos| Position {
        line: pos.line,
        column: pos.column + 1,
        offset: pos.offset + 1,
      });
      (source, default_end)
    };

    let mut end = default_end.unwrap_or(Position {
      line: start.line,
      column: start.column + 1,
      offset: start.offset + 1,
    });

    if let Some(word) = &opts.word {
      let representation = opts
        .string
        .as_ref()
        .map(|string| Cow::Borrowed(string.as_str()))
        .unwrap_or_else(|| Cow::Owned(Node::to_css(node)));

      if let Some(index) = representation.as_ref().find(word) {
        let word_units = word.encode_utf16().count();
        start = Node::position_inside(node, index, Some(representation.as_ref()))?;
        end = Node::position_inside(node, index + word_units, Some(representation.as_ref()))?;
      }
    } else {
      if let Some(custom_start) = &opts.start {
        start = custom_start.clone();
      } else if let Some(index) = opts.index {
        start = Node::position_inside(node, index, opts.string.as_deref())?;
      }

      if let Some(custom_end) = &opts.end {
        end = custom_end.clone();
      } else if let Some(end_index) = opts.end_index {
        end = Node::position_inside(node, end_index, opts.string.as_deref())?;
      } else if let Some(index) = opts.index {
        end = Node::position_inside(node, index + 1, opts.string.as_deref())?;
      }
    }

    if end.line < start.line || (end.line == start.line && end.column <= start.column) {
      end = Position {
        line: start.line,
        column: start.column + 1,
        offset: start.offset + 1,
      };
    }

    Some((start, end))
  }

  pub fn error(node: &NodeRef, message: &str, opts: RangeByOptions) -> CssSyntaxError {
    if let Some((start, end)) = Node::range_by(node, &opts) {
      return error_on(node, message, start, Some(end));
    }

    CssSyntaxError::new(message, None, None, None, None, None, None, None)
  }
}

impl fmt::Display for Node {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &self.data {
      NodeData::Root(_) => write!(f, "[object Root]"),
      NodeData::Document(_) => write!(f, "[object Document]"),
      NodeData::Rule(data) => write!(f, "[object Rule selector={}]", data.selector),
      NodeData::AtRule(data) => write!(f, "[object AtRule name={}]", data.name),
      NodeData::Declaration(data) => {
        write!(f, "[object Declaration {}: {}]", data.prop, data.value)
      }
      NodeData::Comment(data) => write!(f, "[object Comment text={}]", data.text),
    }
  }
}

/// Utility trait that allows the strongly typed wrappers to expose read/write access
/// to their inner [`Node`].
pub trait NodeAccess {
  fn node(&self) -> &NodeRef;
  fn node_mut(&mut self) -> &mut NodeRef;

  fn borrow(&self) -> Ref<'_, Node> {
    self.node().borrow()
  }

  fn borrow_mut(&self) -> RefMut<'_, Node> {
    self.node().borrow_mut()
  }

  fn parent(&self) -> Option<NodeRef> {
    Node::parent_ref(self.node())
  }

  fn root(&self) -> NodeRef {
    Node::root(self.node())
  }

  fn index(&self) -> Option<usize> {
    Node::index(self.node())
  }

  fn next(&self) -> Option<NodeRef> {
    Node::next(self.node())
  }

  fn prev(&self) -> Option<NodeRef> {
    Node::prev(self.node())
  }

  fn before<I>(&self, nodes: I)
  where
    I: IntoIterator<Item = NodeRef>,
  {
    Node::insert_before(self.node(), nodes);
  }

  fn after<I>(&self, nodes: I)
  where
    I: IntoIterator<Item = NodeRef>,
  {
    Node::insert_after(self.node(), nodes);
  }

  fn replace_with<I>(&self, nodes: I)
  where
    I: IntoIterator<Item = NodeRef>,
  {
    Node::replace_with(self.node(), nodes);
  }

  fn remove(&self) {
    Node::remove_self(self.node());
  }

  fn move_to(&self, container: &NodeRef) {
    Node::move_to(self.node(), container);
  }

  fn move_before(&self, other: &NodeRef) {
    Node::move_before(self.node(), other);
  }

  fn move_after(&self, other: &NodeRef) {
    Node::move_after(self.node(), other);
  }

  fn clean_raws(&self, keep_between: bool) {
    Node::clean_raws(self.node(), keep_between);
  }

  fn clone_node(&self) -> NodeRef {
    Node::clone_node(self.node(), None)
  }

  fn clone_with<F>(&self, callback: F) -> NodeRef
  where
    F: FnOnce(&NodeRef),
  {
    let clone = self.clone_node();
    callback(&clone);
    clone
  }

  fn clone_before(&self) -> Option<NodeRef> {
    self.clone_before_with(|_| {})
  }

  fn clone_after(&self) -> Option<NodeRef> {
    self.clone_after_with(|_| {})
  }

  fn clone_before_with<F>(&self, callback: F) -> Option<NodeRef>
  where
    F: FnOnce(&NodeRef),
  {
    let clone = self.clone_node();
    callback(&clone);
    self.parent()?;
    Node::insert_before(self.node(), std::iter::once(clone.clone()));
    Some(clone)
  }

  fn clone_after_with<F>(&self, callback: F) -> Option<NodeRef>
  where
    F: FnOnce(&NodeRef),
  {
    let clone = self.clone_node();
    callback(&clone);
    self.parent()?;
    Node::insert_after(self.node(), std::iter::once(clone.clone()));
    Some(clone)
  }

  fn to_css(&self) -> String {
    Node::to_css(self.node())
  }

  fn raw_value(&self, prop: &str, default_type: Option<&str>) -> RawCacheValue {
    Node::raw_value(self.node(), Some(prop), default_type)
  }

  fn position_inside(&self, index: usize, string: Option<&str>) -> Option<Position> {
    Node::position_inside(self.node(), index, string)
  }

  fn position_by(&self, opts: PositionByOptions) -> Option<Position> {
    Node::position_by(self.node(), &opts)
  }

  fn range_by(&self, opts: RangeByOptions) -> Option<(Position, Position)> {
    Node::range_by(self.node(), &opts)
  }

  fn error_with_opts(&self, message: &str, opts: RangeByOptions) -> CssSyntaxError {
    Node::error(self.node(), message, opts)
  }

  fn error(&self, message: &str) -> CssSyntaxError {
    self.error_with_opts(message, RangeByOptions::default())
  }

  fn warn(
    &self,
    result: &mut PostcssResult,
    text: impl Into<String>,
    mut opts: WarningOptions,
  ) -> Warning {
    if opts.node.is_none() {
      opts.node = Some(self.node().clone());
    }
    result.warn(text, opts)
  }
}

/// Helper function to compute the ancestor root for a node reference.
pub fn find_root(node: &NodeRef) -> NodeRef {
  let mut current = node.clone();
  loop {
    let parent = {
      let borrowed = current.borrow();
      borrowed.parent()
    };
    match parent {
      Some(next) => current = next,
      None => return current,
    }
  }
}

/// Attach positional metadata to a node, mirroring `Node#init` in JavaScript.
pub fn init_node(node: &NodeRef, start: Option<Position>) {
  node.borrow_mut().source.start = start;
}

/// Update the ending position stored on a node.
pub fn set_end_position(node: &NodeRef, end: Position) {
  node.borrow_mut().source.end = Some(end);
}

/// Raise a `CssSyntaxError` scoped to the provided node.
pub fn error_on(
  node: &NodeRef,
  message: &str,
  start: Position,
  end: Option<Position>,
) -> CssSyntaxError {
  let inner = node.borrow();
  CssSyntaxError::from_input(
    message.to_string(),
    inner.source.input.clone(),
    start,
    end,
    None,
  )
}
