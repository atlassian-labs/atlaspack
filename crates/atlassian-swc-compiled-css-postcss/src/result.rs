use std::fmt;
use std::sync::Arc;

use crate::ast::nodes::{Document, NodeKind, Root, RootLike};
use crate::ast::{NodeRef, error_on};
use crate::processor::CustomStringifier;
use crate::source_map::{MapGenerator, MapSetting};

#[derive(Clone, Debug)]
pub struct ProcessorMetadata {
  version: &'static str,
}

impl ProcessorMetadata {
  pub fn new(version: &'static str) -> Self {
    Self { version }
  }

  pub fn version(&self) -> &'static str {
    self.version
  }
}

#[derive(Clone, Debug, Default)]
pub struct ResultOptions {
  pub from: Option<String>,
  pub to: Option<String>,
  pub map: MapSetting,
}

#[derive(Clone, Debug, Default)]
pub struct WarningOptions {
  pub plugin: Option<String>,
  pub node: Option<NodeRef>,
  pub index: Option<usize>,
  pub word: Option<String>,
}

impl WarningOptions {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn plugin(mut self, plugin: impl Into<String>) -> Self {
    self.plugin = Some(plugin.into());
    self
  }

  pub fn node(mut self, node: NodeRef) -> Self {
    self.node = Some(node);
    self
  }

  pub fn index(mut self, index: usize) -> Self {
    self.index = Some(index);
    self
  }

  pub fn word(mut self, word: impl Into<String>) -> Self {
    self.word = Some(word.into());
    self
  }
}

#[derive(Clone, Debug)]
pub struct Warning {
  pub text: String,
  pub plugin: Option<String>,
  pub node: Option<NodeRef>,
  pub index: Option<usize>,
  pub word: Option<String>,
  pub line: Option<usize>,
  pub column: Option<usize>,
  pub end_line: Option<usize>,
  pub end_column: Option<usize>,
}

impl Warning {
  pub const TYPE: &'static str = "warning";

  pub fn new(text: impl Into<String>, mut opts: WarningOptions) -> Self {
    let node = opts.node.clone();
    let (line, column, end_line, end_column) = if let Some(node_ref) = &node {
      let (start, end) = {
        let borrowed = node_ref.borrow();
        (borrowed.source.start.clone(), borrowed.source.end.clone())
      };
      (
        start.as_ref().map(|pos| pos.line as usize),
        start.as_ref().map(|pos| pos.column as usize),
        end.as_ref().map(|pos| pos.line as usize),
        end.as_ref().map(|pos| pos.column as usize),
      )
    } else {
      (None, None, None, None)
    };

    Self {
      text: text.into(),
      plugin: opts.plugin.take(),
      node,
      index: opts.index.take(),
      word: opts.word.take(),
      line,
      column,
      end_line,
      end_column,
    }
  }

  pub fn message_type(&self) -> &'static str {
    Self::TYPE
  }
}

impl fmt::Display for Warning {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if let Some(node) = &self.node {
      let (start, end) = {
        let borrowed = node.borrow();
        (borrowed.source.start.clone(), borrowed.source.end.clone())
      };
      if let Some(start) = start {
        let err = error_on(node, &self.text, start, end);
        return write!(f, "{}", err);
      }
    }
    if let Some(plugin) = &self.plugin {
      write!(f, "{}: {}", plugin, self.text)
    } else {
      write!(f, "{}", self.text)
    }
  }
}

#[derive(Clone, Debug)]
pub enum Message {
  Warning(Warning),
}

impl Message {
  pub fn message_type(&self) -> &'static str {
    match self {
      Message::Warning(warning) => warning.message_type(),
    }
  }

  pub fn as_warning(&self) -> Option<&Warning> {
    match self {
      Message::Warning(warning) => Some(warning),
    }
  }
}

pub struct Result {
  pub processor: ProcessorMetadata,
  pub root: RootLike,
  pub opts: ResultOptions,
  css: Option<String>,
  map: Option<String>,
  pub warnings: Vec<Warning>,
  pub messages: Vec<Message>,
  pub last_plugin: Option<String>,
  stringifier: Arc<dyn CustomStringifier>,
}

impl Result {
  pub fn new(
    root: RootLike,
    processor: ProcessorMetadata,
    opts: ResultOptions,
    stringifier: Arc<dyn CustomStringifier>,
  ) -> Self {
    Self {
      root,
      processor,
      opts,
      css: None,
      map: None,
      warnings: Vec::new(),
      messages: Vec::new(),
      last_plugin: None,
      stringifier,
    }
  }

  pub fn css(&mut self) -> &str {
    if self.css.is_none() {
      let mut generator = MapGenerator::new(
        Some(self.root.clone()),
        None,
        &self.opts.map,
        self.opts.from.as_deref(),
        self.opts.to.as_deref(),
        self.stringifier.clone(),
      );
      let generated = generator.generate().expect("map generation");
      self.map = generated.map;
      self.css = Some(generated.css);
    }
    self.css.as_ref().unwrap()
  }

  pub fn content(&mut self) -> &str {
    self.css()
  }

  pub fn map(&mut self) -> Option<&str> {
    self.css();
    self.map.as_deref()
  }

  pub fn processor(&self) -> &ProcessorMetadata {
    &self.processor
  }

  pub fn opts(&self) -> &ResultOptions {
    &self.opts
  }

  pub fn root(&self) -> Option<&Root> {
    self.root.as_root()
  }

  pub fn document(&self) -> Option<&Document> {
    self.root.as_document()
  }

  pub fn root_like(&self) -> &RootLike {
    &self.root
  }

  pub fn set_last_plugin(&mut self, plugin: Option<String>) {
    self.last_plugin = plugin;
  }

  pub fn warn(&mut self, text: impl Into<String>, mut opts: WarningOptions) -> Warning {
    if opts.plugin.is_none() {
      if let Some(plugin) = &self.last_plugin {
        opts.plugin = Some(plugin.clone());
      }
    }
    let warning = Warning::new(text, opts);
    self.messages.push(Message::Warning(warning.clone()));
    self.warnings.push(warning.clone());
    warning
  }

  pub fn push_warning(&mut self, warning: Warning) {
    self.messages.push(Message::Warning(warning.clone()));
    self.warnings.push(warning);
  }

  pub fn add_warning(&mut self, warning: Warning) {
    self.push_warning(warning);
  }

  pub fn messages(&self) -> &[Message] {
    &self.messages
  }

  pub fn warnings(&self) -> &[Warning] {
    &self.warnings
  }

  pub fn set_css(&mut self, css: Option<String>) {
    self.css = css;
  }

  pub fn set_map(&mut self, map: Option<String>) {
    self.map = map;
  }

  pub fn replace_root(&mut self, root: Root) {
    self.root = RootLike::Root(root);
  }

  pub fn replace_root_like(&mut self, root: RootLike) {
    self.root = root;
  }
}

impl Clone for Result {
  fn clone(&self) -> Self {
    Self {
      processor: self.processor.clone(),
      root: self.root.clone(),
      opts: self.opts.clone(),
      css: self.css.clone(),
      map: self.map.clone(),
      warnings: self.warnings.clone(),
      messages: self.messages.clone(),
      last_plugin: self.last_plugin.clone(),
      stringifier: Arc::clone(&self.stringifier),
    }
  }
}

impl fmt::Debug for Result {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let root_label = match self.root.kind() {
      NodeKind::Root => "<Root>",
      NodeKind::Document => "<Document>",
      NodeKind::Rule => "<Rule>",
      NodeKind::AtRule => "<AtRule>",
      NodeKind::Declaration => "<Declaration>",
      NodeKind::Comment => "<Comment>",
    };
    f.debug_struct("Result")
      .field("processor", &self.processor)
      .field("root", &root_label)
      .field("opts", &self.opts)
      .field("css", &self.css)
      .field("map", &self.map)
      .field("warnings", &self.warnings)
      .field("messages", &self.messages)
      .field("last_plugin", &self.last_plugin)
      .finish()
  }
}
