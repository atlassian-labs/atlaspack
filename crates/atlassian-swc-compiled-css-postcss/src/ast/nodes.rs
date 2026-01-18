#![allow(
  clippy::default_constructed_unit_structs,
  clippy::field_reassign_with_default,
  clippy::new_without_default,
  clippy::redundant_closure,
  clippy::result_large_err,
  clippy::should_implement_trait
)]

use std::cell::RefMut;
use std::fmt;

use regex::{Captures, Regex};

use super::{init_node, set_end_position, Node, NodeAccess, NodeData, NodeRef, RawData};
use crate::input::Position;
use crate::processor::{ProcessOptions, Processor, ProcessorError};
use crate::result::Result as PostcssResult;

fn walk_kind<F, M>(node: &NodeRef, matcher: &M, callback: &mut F) -> bool
where
  F: FnMut(NodeRef, usize) -> bool,
  M: Fn(&NodeData) -> bool,
{
  let predicate = |node_ref: &NodeRef| {
    let borrowed = node_ref.borrow();
    matcher(&borrowed.data)
  };
  Node::walk_filtered(node, &predicate, callback)
}

fn walk_kind_filtered<F, M, P>(node: &NodeRef, matcher: &M, predicate: &P, callback: &mut F) -> bool
where
  F: FnMut(NodeRef, usize) -> bool,
  M: Fn(&NodeData) -> Option<&str>,
  P: Fn(&str) -> bool,
{
  let filter = |node_ref: &NodeRef| {
    let borrowed = node_ref.borrow();
    matcher(&borrowed.data)
      .map(|value| predicate(value))
      .unwrap_or(false)
  };
  Node::walk_filtered(node, &filter, callback)
}

pub trait ContainerChildArg {
  fn resolve_index(&self, container: &NodeRef) -> Option<usize>;
}

impl ContainerChildArg for usize {
  fn resolve_index(&self, container: &NodeRef) -> Option<usize> {
    let len = container.borrow().nodes.len();
    if *self < len {
      Some(*self)
    } else {
      None
    }
  }
}

impl ContainerChildArg for NodeRef {
  fn resolve_index(&self, container: &NodeRef) -> Option<usize> {
    Node::index_of(container, self)
  }
}

impl ContainerChildArg for &NodeRef {
  fn resolve_index(&self, container: &NodeRef) -> Option<usize> {
    Node::index_of(container, self)
  }
}

impl<T> ContainerChildArg for &T
where
  T: NodeAccess,
{
  fn resolve_index(&self, container: &NodeRef) -> Option<usize> {
    Node::index_of(container, self.node())
  }
}

#[derive(Clone, Debug)]
pub enum ReplacePattern {
  Exact(String),
  Regex(Regex),
}

impl ReplacePattern {
  fn matches(&self, value: &str) -> bool {
    match self {
      ReplacePattern::Exact(pattern) => !pattern.is_empty() && value.contains(pattern),
      ReplacePattern::Regex(regex) => regex.is_match(value),
    }
  }

  fn replace<R>(&self, value: &str, replacer: &mut R) -> String
  where
    R: ReplaceCallback,
  {
    match self {
      ReplacePattern::Exact(pattern) => replace_exact(value, pattern, replacer),
      ReplacePattern::Regex(regex) => replace_regex(value, regex, replacer),
    }
  }
}

impl From<&str> for ReplacePattern {
  fn from(value: &str) -> Self {
    ReplacePattern::Exact(value.to_string())
  }
}

impl From<String> for ReplacePattern {
  fn from(value: String) -> Self {
    ReplacePattern::Exact(value)
  }
}

impl From<Regex> for ReplacePattern {
  fn from(value: Regex) -> Self {
    ReplacePattern::Regex(value)
  }
}

#[derive(Clone, Debug, Default)]
pub struct ReplaceValuesOptions {
  pub props: Option<Vec<String>>,
  pub fast: Option<String>,
}

impl ReplaceValuesOptions {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn props<I, S>(mut self, props: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    self.props = Some(props.into_iter().map(|value| value.into()).collect());
    self
  }

  pub fn fast<S>(mut self, fast: S) -> Self
  where
    S: Into<String>,
  {
    self.fast = Some(fast.into());
    self
  }
}

pub trait ReplaceCallback {
  fn replace(
    &mut self,
    substring: &str,
    groups: &[Option<String>],
    offset: usize,
    input: &str,
  ) -> String;
}

impl ReplaceCallback for String {
  fn replace(
    &mut self,
    _substring: &str,
    _groups: &[Option<String>],
    _offset: usize,
    _input: &str,
  ) -> String {
    self.clone()
  }
}

impl ReplaceCallback for &str {
  fn replace(
    &mut self,
    _substring: &str,
    _groups: &[Option<String>],
    _offset: usize,
    _input: &str,
  ) -> String {
    (*self).to_string()
  }
}

impl<F> ReplaceCallback for F
where
  F: FnMut(&str) -> String,
{
  fn replace(
    &mut self,
    substring: &str,
    _groups: &[Option<String>],
    _offset: usize,
    _input: &str,
  ) -> String {
    self(substring)
  }
}

fn replace_exact<R>(value: &str, needle: &str, replacer: &mut R) -> String
where
  R: ReplaceCallback,
{
  if needle.is_empty() {
    return value.to_string();
  }

  let mut result = String::with_capacity(value.len());
  let mut start = 0usize;

  while let Some(position) = value[start..].find(needle) {
    let absolute = start + position;
    result.push_str(&value[start..absolute]);
    result.push_str(&replacer.replace(needle, &[], absolute, value));
    start = absolute + needle.len();
  }

  result.push_str(&value[start..]);
  result
}

fn replace_regex<R>(value: &str, regex: &Regex, replacer: &mut R) -> String
where
  R: ReplaceCallback,
{
  regex
    .replace_all(value, |caps: &Captures| {
      let matched = caps.get(0).map(|m| m.as_str()).unwrap_or("");
      let offset = caps.get(0).map(|m| m.start()).unwrap_or(0);
      let groups: Vec<Option<String>> = caps
        .iter()
        .skip(1)
        .map(|group| group.map(|m| m.as_str().to_string()))
        .collect();
      replacer.replace(matched, &groups, offset, value)
    })
    .into_owned()
}

fn replace_values_on<R>(
  container: &NodeRef,
  pattern: &ReplacePattern,
  options: &ReplaceValuesOptions,
  replacer: &mut R,
) where
  R: ReplaceCallback,
{
  let props = options.props.as_ref();
  let fast = options.fast.as_deref();

  let mut callback = |decl_ref: NodeRef, _index: usize| {
    let prop_matches = if let Some(props) = props {
      let prop_value = {
        let decl = decl_ref.borrow();
        match &decl.data {
          NodeData::Declaration(data) => data.prop.clone(),
          _ => return true,
        }
      };
      props.iter().any(|allowed| allowed == &prop_value)
    } else {
      true
    };

    if !prop_matches {
      return true;
    }

    let value = {
      let decl = decl_ref.borrow();
      match &decl.data {
        NodeData::Declaration(data) => data.value.clone(),
        _ => return true,
      }
    };

    if let Some(fast) = fast {
      if !value.contains(fast) {
        return true;
      }
    }

    if !pattern.matches(&value) {
      return true;
    }

    let replaced = pattern.replace(&value, replacer);
    if replaced == value {
      return true;
    }

    let mut decl = decl_ref.borrow_mut();
    if let NodeData::Declaration(data) = &mut decl.data {
      data.value = replaced;
      decl.mark_dirty();
    }
    true
  };

  Node::walk_filtered(
    container,
    &|node_ref| matches!(node_ref.borrow().data, NodeData::Declaration(_)),
    &mut callback,
  );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
  Root,
  Document,
  Rule,
  AtRule,
  Declaration,
  Comment,
}

#[derive(Clone, Debug, Default)]
pub struct RootData;

#[derive(Clone, Debug, Default)]
pub struct DocumentData {
  pub mode: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct RuleData {
  pub selector: String,
}

#[derive(Clone, Debug, Default)]
pub struct AtRuleData {
  pub name: String,
  pub params: String,
}

#[derive(Clone, Debug, Default)]
pub struct DeclarationData {
  pub prop: String,
  pub value: String,
  pub important: bool,
}

#[derive(Clone, Debug, Default)]
pub struct CommentData {
  pub text: String,
}

#[derive(Clone, Debug)]
pub struct Root {
  node: NodeRef,
}

impl Root {
  pub fn new() -> Self {
    let node = Node::new(NodeData::Root(RootData::default()));
    Root { node }
  }

  pub(crate) fn from_node(node: NodeRef) -> Self {
    Root { node }
  }

  pub fn clone(&self) -> Self {
    Root::from_node(NodeAccess::clone_node(self))
  }

  pub fn clone_with<F>(&self, callback: F) -> Self
  where
    F: FnOnce(&Self),
  {
    let node = NodeAccess::clone_with(self, |node_ref| {
      let wrapper = Root::from_node(node_ref.clone());
      callback(&wrapper);
    });
    Root::from_node(node)
  }

  pub fn clone_before(&self) -> Option<Self> {
    self.clone_before_with(|_| {})
  }

  pub fn clone_before_with<F>(&self, callback: F) -> Option<Self>
  where
    F: FnOnce(&Self),
  {
    let inserted = <Self as NodeAccess>::clone_before_with(self, |node_ref| {
      let wrapper = Root::from_node(node_ref.clone());
      callback(&wrapper);
    })?;
    Some(Root::from_node(inserted))
  }

  pub fn clone_after(&self) -> Option<Self> {
    self.clone_after_with(|_| {})
  }

  pub fn clone_after_with<F>(&self, callback: F) -> Option<Self>
  where
    F: FnOnce(&Self),
  {
    let inserted = <Self as NodeAccess>::clone_after_with(self, |node_ref| {
      let wrapper = Root::from_node(node_ref.clone());
      callback(&wrapper);
    })?;
    Some(Root::from_node(inserted))
  }

  pub fn raw(&self) -> &NodeRef {
    &self.node
  }

  pub fn to_result(&self) -> std::result::Result<PostcssResult, ProcessorError> {
    self.to_result_with_options(ProcessOptions::new())
  }

  pub fn to_result_with_options(
    &self,
    opts: ProcessOptions,
  ) -> std::result::Result<PostcssResult, ProcessorError> {
    let processor = Processor::new();
    let lazy = processor.process_root_with_options(self.clone(), opts)?;
    lazy.into_result()
  }

  pub fn each<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::each(&self.node, &mut callback)
  }

  pub fn walk<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::walk(&self.node, &mut callback)
  }

  pub fn walk_comments<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::Comment(_)),
      &mut callback,
    )
  }

  pub fn walk_decls<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::Declaration(_)),
      &mut callback,
    )
  }

  pub fn walk_decls_if<P, F>(&self, predicate: P, mut callback: F) -> bool
  where
    P: Fn(&str) -> bool,
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind_filtered(
      &self.node,
      &|data| match data {
        NodeData::Declaration(data) => Some(data.prop.as_str()),
        _ => None,
      },
      &predicate,
      &mut callback,
    )
  }

  pub fn walk_rules<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::Rule(_)),
      &mut callback,
    )
  }

  pub fn walk_rules_if<P, F>(&self, predicate: P, mut callback: F) -> bool
  where
    P: Fn(&str) -> bool,
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind_filtered(
      &self.node,
      &|data| match data {
        NodeData::Rule(data) => Some(data.selector.as_str()),
        _ => None,
      },
      &predicate,
      &mut callback,
    )
  }

  pub fn walk_at_rules<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::AtRule(_)),
      &mut callback,
    )
  }

  pub fn walk_at_rules_if<P, F>(&self, predicate: P, mut callback: F) -> bool
  where
    P: Fn(&str) -> bool,
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind_filtered(
      &self.node,
      &|data| match data {
        NodeData::AtRule(data) => Some(data.name.as_str()),
        _ => None,
      },
      &predicate,
      &mut callback,
    )
  }

  pub fn every<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::every(&self.node, &mut callback)
  }

  pub fn some<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::some(&self.node, &mut callback)
  }

  pub fn append(&self, child: NodeRef) {
    Node::append(&self.node, child);
  }

  pub fn prepend(&self, child: NodeRef) {
    Node::insert(&self.node, 0, child);
  }

  pub fn remove_all(&self) {
    Node::remove_all(&self.node);
  }

  pub fn first(&self) -> Option<NodeRef> {
    Node::first_child(&self.node)
  }

  pub fn last(&self) -> Option<NodeRef> {
    Node::last_child(&self.node)
  }

  pub fn child_index<C>(&self, child: C) -> Option<usize>
  where
    C: ContainerChildArg,
  {
    child.resolve_index(&self.node)
  }

  pub fn remove_child<C>(&self, child: C) -> Option<NodeRef>
  where
    C: ContainerChildArg,
  {
    let index = child.resolve_index(&self.node)?;
    let len = {
      let borrowed = self.node.borrow();
      borrowed.nodes.len()
    };
    if index >= len {
      return None;
    }
    Some(Node::remove(&self.node, index))
  }

  pub fn replace_values<P, R>(&self, pattern: P, replacer: R) -> Self
  where
    P: Into<ReplacePattern>,
    R: ReplaceCallback,
  {
    self.replace_values_with_options(pattern, ReplaceValuesOptions::default(), replacer)
  }

  pub fn replace_values_with_options<P, R>(
    &self,
    pattern: P,
    options: ReplaceValuesOptions,
    replacer: R,
  ) -> Self
  where
    P: Into<ReplacePattern>,
    R: ReplaceCallback,
  {
    let pattern = pattern.into();
    let mut replacer = replacer;
    replace_values_on(&self.node, &pattern, &options, &mut replacer);
    Root::from_node(self.node.clone())
  }

  pub fn nodes(&self) -> Vec<NodeRef> {
    self.node.borrow().nodes.clone()
  }

  pub fn is_empty(&self) -> bool {
    self.node.borrow().nodes.is_empty()
  }

  pub fn clean_raws(&self, keep_between: bool) {
    Node::clean_raws(&self.node, keep_between);
  }

  pub fn to_node(&self) -> NodeRef {
    self.node.clone()
  }
}

impl NodeAccess for Root {
  fn node(&self) -> &NodeRef {
    &self.node
  }

  fn node_mut(&mut self) -> &mut NodeRef {
    &mut self.node
  }
}

impl fmt::Display for Root {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let css = crate::stringifier::stringify(self);
    f.write_str(&css)
  }
}

#[derive(Clone, Debug)]
pub struct Document {
  node: NodeRef,
}

impl Document {
  pub fn new() -> Self {
    let node = Node::new(NodeData::Document(DocumentData::default()));
    Document { node }
  }

  pub(crate) fn from_node(node: NodeRef) -> Self {
    Document { node }
  }

  pub fn clone(&self) -> Self {
    Document::from_node(NodeAccess::clone_node(self))
  }

  pub fn clone_with<F>(&self, callback: F) -> Self
  where
    F: FnOnce(&Self),
  {
    let node = NodeAccess::clone_with(self, |node_ref| {
      let wrapper = Document::from_node(node_ref.clone());
      callback(&wrapper);
    });
    Document::from_node(node)
  }

  pub fn clone_before(&self) -> Option<Self> {
    self.clone_before_with(|_| {})
  }

  pub fn clone_before_with<F>(&self, callback: F) -> Option<Self>
  where
    F: FnOnce(&Self),
  {
    let inserted = <Self as NodeAccess>::clone_before_with(self, |node_ref| {
      let wrapper = Document::from_node(node_ref.clone());
      callback(&wrapper);
    })?;
    Some(Document::from_node(inserted))
  }

  pub fn clone_after(&self) -> Option<Self> {
    self.clone_after_with(|_| {})
  }

  pub fn clone_after_with<F>(&self, callback: F) -> Option<Self>
  where
    F: FnOnce(&Self),
  {
    let inserted = <Self as NodeAccess>::clone_after_with(self, |node_ref| {
      let wrapper = Document::from_node(node_ref.clone());
      callback(&wrapper);
    })?;
    Some(Document::from_node(inserted))
  }

  pub fn to_node(&self) -> NodeRef {
    self.node.clone()
  }

  pub fn nodes(&self) -> Vec<NodeRef> {
    self.node.borrow().nodes.clone()
  }

  pub fn mode(&self) -> Option<String> {
    let node = self.node.borrow();
    match &node.data {
      NodeData::Document(data) => data.mode.clone(),
      _ => None,
    }
  }

  pub fn set_mode<S>(&self, mode: Option<S>)
  where
    S: Into<String>,
  {
    if let Some(data) = self.node.borrow_mut().as_document_mut() {
      data.mode = mode.map(|value| value.into());
    }
  }

  pub fn each<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::each(&self.node, &mut callback)
  }

  pub fn walk<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::walk(&self.node, &mut callback)
  }

  pub fn walk_comments<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::Comment(_)),
      &mut callback,
    )
  }

  pub fn walk_decls<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::Declaration(_)),
      &mut callback,
    )
  }

  pub fn walk_decls_if<P, F>(&self, predicate: P, mut callback: F) -> bool
  where
    P: Fn(&str) -> bool,
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind_filtered(
      &self.node,
      &|data| match data {
        NodeData::Declaration(data) => Some(data.prop.as_str()),
        _ => None,
      },
      &predicate,
      &mut callback,
    )
  }

  pub fn walk_rules<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::Rule(_)),
      &mut callback,
    )
  }

  pub fn walk_rules_if<P, F>(&self, predicate: P, mut callback: F) -> bool
  where
    P: Fn(&str) -> bool,
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind_filtered(
      &self.node,
      &|data| match data {
        NodeData::Rule(data) => Some(data.selector.as_str()),
        _ => None,
      },
      &predicate,
      &mut callback,
    )
  }

  pub fn walk_at_rules<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::AtRule(_)),
      &mut callback,
    )
  }

  pub fn walk_at_rules_if<P, F>(&self, predicate: P, mut callback: F) -> bool
  where
    P: Fn(&str) -> bool,
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind_filtered(
      &self.node,
      &|data| match data {
        NodeData::AtRule(data) => Some(data.name.as_str()),
        _ => None,
      },
      &predicate,
      &mut callback,
    )
  }

  pub fn append(&self, child: NodeRef) {
    Node::append(&self.node, child);
  }

  pub fn prepend(&self, child: NodeRef) {
    Node::insert(&self.node, 0, child);
  }

  pub fn remove_all(&self) {
    Node::remove_all(&self.node);
  }

  pub fn first(&self) -> Option<NodeRef> {
    Node::first_child(&self.node)
  }

  pub fn last(&self) -> Option<NodeRef> {
    Node::last_child(&self.node)
  }

  pub fn child_index<C>(&self, child: C) -> Option<usize>
  where
    C: ContainerChildArg,
  {
    child.resolve_index(&self.node)
  }

  pub fn remove_child<C>(&self, child: C) -> Option<NodeRef>
  where
    C: ContainerChildArg,
  {
    let index = child.resolve_index(&self.node)?;
    let len = {
      let borrowed = self.node.borrow();
      borrowed.nodes.len()
    };
    if index >= len {
      return None;
    }
    Some(Node::remove(&self.node, index))
  }

  pub fn replace_values<P, R>(&self, pattern: P, replacer: R) -> Self
  where
    P: Into<ReplacePattern>,
    R: ReplaceCallback,
  {
    self.replace_values_with_options(pattern, ReplaceValuesOptions::default(), replacer)
  }

  pub fn replace_values_with_options<P, R>(
    &self,
    pattern: P,
    options: ReplaceValuesOptions,
    replacer: R,
  ) -> Self
  where
    P: Into<ReplacePattern>,
    R: ReplaceCallback,
  {
    let pattern = pattern.into();
    let mut replacer = replacer;
    replace_values_on(&self.node, &pattern, &options, &mut replacer);
    Document::from_node(self.node.clone())
  }

  pub fn to_result(&self) -> std::result::Result<PostcssResult, ProcessorError> {
    self.to_result_with_options(ProcessOptions::new())
  }

  pub fn to_result_with_options(
    &self,
    opts: ProcessOptions,
  ) -> std::result::Result<PostcssResult, ProcessorError> {
    let processor = Processor::new();
    let lazy = processor.process_document_with_options(self.clone(), opts)?;
    lazy.into_result()
  }
}

impl NodeAccess for Document {
  fn node(&self) -> &NodeRef {
    &self.node
  }

  fn node_mut(&mut self) -> &mut NodeRef {
    &mut self.node
  }
}

impl fmt::Display for Document {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let css = crate::stringifier::node_to_string(&self.node);
    f.write_str(&css)
  }
}

#[derive(Clone, Debug)]
pub enum RootLike {
  Root(Root),
  Document(Document),
}

impl RootLike {
  pub fn as_root(&self) -> Option<&Root> {
    match self {
      RootLike::Root(root) => Some(root),
      RootLike::Document(_) => None,
    }
  }

  pub fn as_document(&self) -> Option<&Document> {
    match self {
      RootLike::Root(_) => None,
      RootLike::Document(document) => Some(document),
    }
  }

  pub fn to_node(&self) -> NodeRef {
    match self {
      RootLike::Root(root) => root.to_node(),
      RootLike::Document(document) => document.to_node(),
    }
  }

  pub fn kind(&self) -> NodeKind {
    match self {
      RootLike::Root(_) => NodeKind::Root,
      RootLike::Document(_) => NodeKind::Document,
    }
  }

  pub fn into_root(self) -> Option<Root> {
    match self {
      RootLike::Root(root) => Some(root),
      RootLike::Document(_) => None,
    }
  }

  pub fn into_document(self) -> Option<Document> {
    match self {
      RootLike::Root(_) => None,
      RootLike::Document(document) => Some(document),
    }
  }
}

impl From<Root> for RootLike {
  fn from(root: Root) -> Self {
    RootLike::Root(root)
  }
}

impl From<Document> for RootLike {
  fn from(document: Document) -> Self {
    RootLike::Document(document)
  }
}

#[derive(Clone, Debug)]
pub struct Rule {
  node: NodeRef,
}

impl Rule {
  pub fn new(selector: impl Into<String>) -> Self {
    let mut data = RuleData::default();
    data.selector = selector.into();
    let node = Node::new(NodeData::Rule(data));
    Rule { node }
  }

  pub(crate) fn from_node(node: NodeRef) -> Self {
    Rule { node }
  }

  pub fn clone(&self) -> Self {
    Rule::from_node(NodeAccess::clone_node(self))
  }

  pub fn clone_with<F>(&self, callback: F) -> Self
  where
    F: FnOnce(&Self),
  {
    let node = NodeAccess::clone_with(self, |node_ref| {
      let wrapper = Rule::from_node(node_ref.clone());
      callback(&wrapper);
    });
    Rule::from_node(node)
  }

  pub fn clone_before(&self) -> Option<Self> {
    self.clone_before_with(|_| {})
  }

  pub fn clone_before_with<F>(&self, callback: F) -> Option<Self>
  where
    F: FnOnce(&Self),
  {
    let inserted = <Self as NodeAccess>::clone_before_with(self, |node_ref| {
      let wrapper = Rule::from_node(node_ref.clone());
      callback(&wrapper);
    })?;
    Some(Rule::from_node(inserted))
  }

  pub fn clone_after(&self) -> Option<Self> {
    self.clone_after_with(|_| {})
  }

  pub fn clone_after_with<F>(&self, callback: F) -> Option<Self>
  where
    F: FnOnce(&Self),
  {
    let inserted = <Self as NodeAccess>::clone_after_with(self, |node_ref| {
      let wrapper = Rule::from_node(node_ref.clone());
      callback(&wrapper);
    })?;
    Some(Rule::from_node(inserted))
  }

  pub fn selector(&self) -> String {
    let node = self.node.borrow();
    match &node.data {
      NodeData::Rule(data) => data.selector.clone(),
      _ => String::new(),
    }
  }

  pub fn set_selector(&self, selector: impl Into<String>) {
    if let Some(data) = self.node.borrow_mut().as_rule_mut() {
      data.selector = selector.into();
    }
  }

  pub fn append(&self, child: NodeRef) {
    Node::append(&self.node, child);
  }

  pub fn prepend(&self, child: NodeRef) {
    Node::insert(&self.node, 0, child);
  }

  pub fn remove_all(&self) {
    Node::remove_all(&self.node);
  }

  pub fn first(&self) -> Option<NodeRef> {
    Node::first_child(&self.node)
  }

  pub fn last(&self) -> Option<NodeRef> {
    Node::last_child(&self.node)
  }

  pub fn child_index<C>(&self, child: C) -> Option<usize>
  where
    C: ContainerChildArg,
  {
    child.resolve_index(&self.node)
  }

  pub fn remove_child<C>(&self, child: C) -> Option<NodeRef>
  where
    C: ContainerChildArg,
  {
    let index = child.resolve_index(&self.node)?;
    let len = {
      let borrowed = self.node.borrow();
      borrowed.nodes.len()
    };
    if index >= len {
      return None;
    }
    Some(Node::remove(&self.node, index))
  }

  pub fn nodes(&self) -> Vec<NodeRef> {
    self.node.borrow().nodes.clone()
  }

  pub fn each<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::each(&self.node, &mut callback)
  }

  pub fn to_node(&self) -> NodeRef {
    self.node.clone()
  }

  pub fn walk<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::walk(&self.node, &mut callback)
  }

  pub fn walk_comments<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::Comment(_)),
      &mut callback,
    )
  }

  pub fn walk_decls<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::Declaration(_)),
      &mut callback,
    )
  }

  pub fn walk_decls_if<P, F>(&self, predicate: P, mut callback: F) -> bool
  where
    P: Fn(&str) -> bool,
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind_filtered(
      &self.node,
      &|data| match data {
        NodeData::Declaration(data) => Some(data.prop.as_str()),
        _ => None,
      },
      &predicate,
      &mut callback,
    )
  }

  pub fn every<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::every(&self.node, &mut callback)
  }

  pub fn some<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::some(&self.node, &mut callback)
  }

  pub fn replace_values<P, R>(&self, pattern: P, replacer: R) -> Self
  where
    P: Into<ReplacePattern>,
    R: ReplaceCallback,
  {
    self.replace_values_with_options(pattern, ReplaceValuesOptions::default(), replacer)
  }

  pub fn replace_values_with_options<P, R>(
    &self,
    pattern: P,
    options: ReplaceValuesOptions,
    replacer: R,
  ) -> Self
  where
    P: Into<ReplacePattern>,
    R: ReplaceCallback,
  {
    let pattern = pattern.into();
    let mut replacer = replacer;
    replace_values_on(&self.node, &pattern, &options, &mut replacer);
    Rule::from_node(self.node.clone())
  }

  pub fn walk_rules<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::Rule(_)),
      &mut callback,
    )
  }

  pub fn walk_rules_if<P, F>(&self, predicate: P, mut callback: F) -> bool
  where
    P: Fn(&str) -> bool,
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind_filtered(
      &self.node,
      &|data| match data {
        NodeData::Rule(data) => Some(data.selector.as_str()),
        _ => None,
      },
      &predicate,
      &mut callback,
    )
  }

  pub fn walk_at_rules<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::AtRule(_)),
      &mut callback,
    )
  }

  pub fn walk_at_rules_if<P, F>(&self, predicate: P, mut callback: F) -> bool
  where
    P: Fn(&str) -> bool,
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind_filtered(
      &self.node,
      &|data| match data {
        NodeData::AtRule(data) => Some(data.name.as_str()),
        _ => None,
      },
      &predicate,
      &mut callback,
    )
  }
}

impl NodeAccess for Rule {
  fn node(&self) -> &NodeRef {
    &self.node
  }

  fn node_mut(&mut self) -> &mut NodeRef {
    &mut self.node
  }
}

#[derive(Clone, Debug)]
pub struct AtRule {
  node: NodeRef,
}

impl AtRule {
  pub fn new(name: impl Into<String>) -> Self {
    let mut data = AtRuleData::default();
    data.name = name.into();
    let node = Node::new(NodeData::AtRule(data));
    AtRule { node }
  }

  pub(crate) fn from_node(node: NodeRef) -> Self {
    AtRule { node }
  }

  pub fn clone(&self) -> Self {
    AtRule::from_node(NodeAccess::clone_node(self))
  }

  pub fn clone_with<F>(&self, callback: F) -> Self
  where
    F: FnOnce(&Self),
  {
    let node = NodeAccess::clone_with(self, |node_ref| {
      let wrapper = AtRule::from_node(node_ref.clone());
      callback(&wrapper);
    });
    AtRule::from_node(node)
  }

  pub fn clone_before(&self) -> Option<Self> {
    self.clone_before_with(|_| {})
  }

  pub fn clone_before_with<F>(&self, callback: F) -> Option<Self>
  where
    F: FnOnce(&Self),
  {
    let inserted = <Self as NodeAccess>::clone_before_with(self, |node_ref| {
      let wrapper = AtRule::from_node(node_ref.clone());
      callback(&wrapper);
    })?;
    Some(AtRule::from_node(inserted))
  }

  pub fn clone_after(&self) -> Option<Self> {
    self.clone_after_with(|_| {})
  }

  pub fn clone_after_with<F>(&self, callback: F) -> Option<Self>
  where
    F: FnOnce(&Self),
  {
    let inserted = <Self as NodeAccess>::clone_after_with(self, |node_ref| {
      let wrapper = AtRule::from_node(node_ref.clone());
      callback(&wrapper);
    })?;
    Some(AtRule::from_node(inserted))
  }

  pub fn name(&self) -> String {
    let node = self.node.borrow();
    match &node.data {
      NodeData::AtRule(data) => data.name.clone(),
      _ => String::new(),
    }
  }

  pub fn set_name(&self, name: impl Into<String>) {
    if let Some(data) = self.node.borrow_mut().as_at_rule_mut() {
      data.name = name.into();
    }
  }

  pub fn set_params(&self, params: impl Into<String>) {
    if let Some(data) = self.node.borrow_mut().as_at_rule_mut() {
      data.params = params.into();
    }
  }

  pub fn params(&self) -> String {
    let node = self.node.borrow();
    match &node.data {
      NodeData::AtRule(data) => data.params.clone(),
      _ => String::new(),
    }
  }

  pub fn append(&self, child: NodeRef) {
    Node::append(&self.node, child);
  }

  pub fn prepend(&self, child: NodeRef) {
    Node::insert(&self.node, 0, child);
  }

  pub fn remove_all(&self) {
    Node::remove_all(&self.node);
  }

  pub fn first(&self) -> Option<NodeRef> {
    Node::first_child(&self.node)
  }

  pub fn last(&self) -> Option<NodeRef> {
    Node::last_child(&self.node)
  }

  pub fn child_index<C>(&self, child: C) -> Option<usize>
  where
    C: ContainerChildArg,
  {
    child.resolve_index(&self.node)
  }

  pub fn remove_child<C>(&self, child: C) -> Option<NodeRef>
  where
    C: ContainerChildArg,
  {
    let index = child.resolve_index(&self.node)?;
    let len = {
      let borrowed = self.node.borrow();
      borrowed.nodes.len()
    };
    if index >= len {
      return None;
    }
    Some(Node::remove(&self.node, index))
  }

  pub fn nodes(&self) -> Vec<NodeRef> {
    self.node.borrow().nodes.clone()
  }

  pub fn to_node(&self) -> NodeRef {
    self.node.clone()
  }

  pub fn each<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::each(&self.node, &mut callback)
  }

  pub fn walk<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::walk(&self.node, &mut callback)
  }

  pub fn every<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::every(&self.node, &mut callback)
  }

  pub fn some<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    Node::some(&self.node, &mut callback)
  }

  pub fn walk_comments<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::Comment(_)),
      &mut callback,
    )
  }

  pub fn walk_decls<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::Declaration(_)),
      &mut callback,
    )
  }

  pub fn walk_decls_if<P, F>(&self, predicate: P, mut callback: F) -> bool
  where
    P: Fn(&str) -> bool,
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind_filtered(
      &self.node,
      &|data| match data {
        NodeData::Declaration(data) => Some(data.prop.as_str()),
        _ => None,
      },
      &predicate,
      &mut callback,
    )
  }

  pub fn walk_rules<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::Rule(_)),
      &mut callback,
    )
  }

  pub fn walk_rules_if<P, F>(&self, predicate: P, mut callback: F) -> bool
  where
    P: Fn(&str) -> bool,
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind_filtered(
      &self.node,
      &|data| match data {
        NodeData::Rule(data) => Some(data.selector.as_str()),
        _ => None,
      },
      &predicate,
      &mut callback,
    )
  }

  pub fn walk_at_rules<F>(&self, mut callback: F) -> bool
  where
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind(
      &self.node,
      &|data| matches!(data, NodeData::AtRule(_)),
      &mut callback,
    )
  }

  pub fn walk_at_rules_if<P, F>(&self, predicate: P, mut callback: F) -> bool
  where
    P: Fn(&str) -> bool,
    F: FnMut(NodeRef, usize) -> bool,
  {
    walk_kind_filtered(
      &self.node,
      &|data| match data {
        NodeData::AtRule(data) => Some(data.name.as_str()),
        _ => None,
      },
      &predicate,
      &mut callback,
    )
  }

  pub fn replace_values<P, R>(&self, pattern: P, replacer: R) -> Self
  where
    P: Into<ReplacePattern>,
    R: ReplaceCallback,
  {
    self.replace_values_with_options(pattern, ReplaceValuesOptions::default(), replacer)
  }

  pub fn replace_values_with_options<P, R>(
    &self,
    pattern: P,
    options: ReplaceValuesOptions,
    replacer: R,
  ) -> Self
  where
    P: Into<ReplacePattern>,
    R: ReplaceCallback,
  {
    let pattern = pattern.into();
    let mut replacer = replacer;
    replace_values_on(&self.node, &pattern, &options, &mut replacer);
    AtRule::from_node(self.node.clone())
  }
}

impl NodeAccess for AtRule {
  fn node(&self) -> &NodeRef {
    &self.node
  }

  fn node_mut(&mut self) -> &mut NodeRef {
    &mut self.node
  }
}

#[derive(Clone, Debug)]
pub struct Declaration {
  node: NodeRef,
}

impl Declaration {
  pub fn new(prop: impl Into<String>, value: impl Into<String>) -> Self {
    let mut data = DeclarationData::default();
    data.prop = prop.into();
    data.value = value.into();
    let node = Node::new(NodeData::Declaration(data));
    Declaration { node }
  }

  pub(crate) fn from_node(node: NodeRef) -> Self {
    Declaration { node }
  }

  pub fn clone(&self) -> Self {
    Declaration::from_node(NodeAccess::clone_node(self))
  }

  pub fn clone_with<F>(&self, callback: F) -> Self
  where
    F: FnOnce(&Self),
  {
    let node = NodeAccess::clone_with(self, |node_ref| {
      let wrapper = Declaration::from_node(node_ref.clone());
      callback(&wrapper);
    });
    Declaration::from_node(node)
  }

  pub fn clone_before(&self) -> Option<Self> {
    self.clone_before_with(|_| {})
  }

  pub fn clone_before_with<F>(&self, callback: F) -> Option<Self>
  where
    F: FnOnce(&Self),
  {
    let inserted = <Self as NodeAccess>::clone_before_with(self, |node_ref| {
      let wrapper = Declaration::from_node(node_ref.clone());
      callback(&wrapper);
    })?;
    Some(Declaration::from_node(inserted))
  }

  pub fn clone_after(&self) -> Option<Self> {
    self.clone_after_with(|_| {})
  }

  pub fn clone_after_with<F>(&self, callback: F) -> Option<Self>
  where
    F: FnOnce(&Self),
  {
    let inserted = <Self as NodeAccess>::clone_after_with(self, |node_ref| {
      let wrapper = Declaration::from_node(node_ref.clone());
      callback(&wrapper);
    })?;
    Some(Declaration::from_node(inserted))
  }

  pub fn prop(&self) -> String {
    let node = self.node.borrow();
    match &node.data {
      NodeData::Declaration(data) => data.prop.clone(),
      _ => String::new(),
    }
  }

  pub fn set_prop(&self, prop: impl Into<String>) {
    if let Some(data) = self.node.borrow_mut().as_declaration_mut() {
      data.prop = prop.into();
    }
  }

  pub fn value(&self) -> String {
    let node = self.node.borrow();
    match &node.data {
      NodeData::Declaration(data) => data.value.clone(),
      _ => String::new(),
    }
  }

  pub fn set_value(&self, value: impl Into<String>) {
    if let Some(data) = self.node.borrow_mut().as_declaration_mut() {
      data.value = value.into();
    }
  }

  pub fn set_important(&self, important: bool) {
    if let Some(data) = self.node.borrow_mut().as_declaration_mut() {
      data.important = important;
    }
  }

  pub fn important(&self) -> bool {
    let node = self.node.borrow();
    matches!(&node.data, NodeData::Declaration(data) if data.important)
  }

  pub fn to_node(&self) -> NodeRef {
    self.node.clone()
  }
}

impl NodeAccess for Declaration {
  fn node(&self) -> &NodeRef {
    &self.node
  }

  fn node_mut(&mut self) -> &mut NodeRef {
    &mut self.node
  }
}

#[derive(Clone, Debug)]
pub struct Comment {
  node: NodeRef,
}

impl Comment {
  pub fn new(text: impl Into<String>) -> Self {
    let mut data = CommentData::default();
    data.text = text.into();
    let node = Node::new(NodeData::Comment(data));
    Comment { node }
  }

  pub(crate) fn from_node(node: NodeRef) -> Self {
    Comment { node }
  }

  pub fn clone(&self) -> Self {
    Comment::from_node(NodeAccess::clone_node(self))
  }

  pub fn clone_with<F>(&self, callback: F) -> Self
  where
    F: FnOnce(&Self),
  {
    let node = NodeAccess::clone_with(self, |node_ref| {
      let wrapper = Comment::from_node(node_ref.clone());
      callback(&wrapper);
    });
    Comment::from_node(node)
  }

  pub fn clone_before(&self) -> Option<Self> {
    self.clone_before_with(|_| {})
  }

  pub fn clone_before_with<F>(&self, callback: F) -> Option<Self>
  where
    F: FnOnce(&Self),
  {
    let inserted = <Self as NodeAccess>::clone_before_with(self, |node_ref| {
      let wrapper = Comment::from_node(node_ref.clone());
      callback(&wrapper);
    })?;
    Some(Comment::from_node(inserted))
  }

  pub fn clone_after(&self) -> Option<Self> {
    self.clone_after_with(|_| {})
  }

  pub fn clone_after_with<F>(&self, callback: F) -> Option<Self>
  where
    F: FnOnce(&Self),
  {
    let inserted = <Self as NodeAccess>::clone_after_with(self, |node_ref| {
      let wrapper = Comment::from_node(node_ref.clone());
      callback(&wrapper);
    })?;
    Some(Comment::from_node(inserted))
  }

  pub fn text(&self) -> String {
    let node = self.node.borrow();
    match &node.data {
      NodeData::Comment(data) => data.text.clone(),
      _ => String::new(),
    }
  }

  pub fn set_text(&self, text: impl Into<String>) {
    if let Some(data) = self.node.borrow_mut().as_comment_mut() {
      data.text = text.into();
    }
  }

  pub fn to_node(&self) -> NodeRef {
    self.node.clone()
  }
}

impl NodeAccess for Comment {
  fn node(&self) -> &NodeRef {
    &self.node
  }

  fn node_mut(&mut self) -> &mut NodeRef {
    &mut self.node
  }
}

/// Helper to convert an existing node reference into a [`Rule`] wrapper if it
/// stores rule data.
pub fn as_rule(node: &NodeRef) -> Option<Rule> {
  if matches!(node.borrow().data, NodeData::Rule(_)) {
    Some(Rule { node: node.clone() })
  } else {
    None
  }
}

/// Helper to convert an existing node reference into an [`AtRule`] wrapper if it
/// stores at-rule data.
pub fn as_at_rule(node: &NodeRef) -> Option<AtRule> {
  if matches!(node.borrow().data, NodeData::AtRule(_)) {
    Some(AtRule { node: node.clone() })
  } else {
    None
  }
}

/// Helper to convert an existing node reference into a [`Declaration`] wrapper if possible.
pub fn as_declaration(node: &NodeRef) -> Option<Declaration> {
  if matches!(node.borrow().data, NodeData::Declaration(_)) {
    Some(Declaration { node: node.clone() })
  } else {
    None
  }
}

/// Helper to convert an existing node reference into a [`Comment`] wrapper if possible.
pub fn as_comment(node: &NodeRef) -> Option<Comment> {
  if matches!(node.borrow().data, NodeData::Comment(_)) {
    Some(Comment { node: node.clone() })
  } else {
    None
  }
}

/// Populate a node's starting and ending location, mirroring the behaviour of
/// PostCSS' `Node#init` helper.
pub fn init_positions(node: &NodeRef, start: Option<Position>, end: Option<Position>) {
  init_node(node, start);
  if let Some(end_pos) = end {
    set_end_position(node, end_pos);
  }
}

/// Borrow the node mutably and return the guard for callers that need to edit the
/// internal state without exposing `RefCell` in public APIs.
pub fn borrow_mut(node: &NodeRef) -> RefMut<'_, Node> {
  node.borrow_mut()
}

/// Convenience constructor used by the parser to create a declaration with raw metadata.
pub fn declaration_with_raws(
  prop: String,
  value: String,
  important: bool,
  raws: RawData,
) -> NodeRef {
  let mut data = DeclarationData::default();
  data.prop = prop;
  data.value = value;
  data.important = important;
  let node = Node::new(NodeData::Declaration(data));
  node.borrow_mut().raws = raws;
  node
}

/// Create a comment node containing raw metadata.
pub fn comment_with_raws(text: String, raws: RawData) -> NodeRef {
  let node = Node::new(NodeData::Comment(CommentData { text }));
  node.borrow_mut().raws = raws;
  node
}

/// Create a rule node with raw metadata and selector text.
pub fn rule_with_raws(selector: String, raws: RawData) -> NodeRef {
  let mut data = RuleData::default();
  data.selector = selector;
  let node = Node::new(NodeData::Rule(data));
  node.borrow_mut().raws = raws;
  node
}

/// Create an at-rule node with raw metadata.
pub fn at_rule_with_raws(name: String, params: String, raws: RawData) -> NodeRef {
  let mut data = AtRuleData::default();
  data.name = name;
  data.params = params;
  let node = Node::new(NodeData::AtRule(data));
  node.borrow_mut().raws = raws;
  node
}
