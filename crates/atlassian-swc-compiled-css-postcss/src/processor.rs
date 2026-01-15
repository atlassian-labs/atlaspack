use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::ast::NodeRef;
use crate::ast::nodes::{AtRule, Comment, Declaration, Document, NodeKind, Root, RootLike, Rule};
use crate::css_syntax_error::CssSyntaxError;
use crate::parse::{ParseError, ParseOptions, parse_with_options};
use crate::result::{ProcessorMetadata, Result as PostcssResult, ResultOptions, Warning};
use crate::source_map::{MapGenerator, MapOptions, MapSetting, PreviousMapError};

#[derive(Debug)]
pub enum ProcessorError {
  Css(CssSyntaxError),
  Message(String),
  Parse(ParseError),
  Map(PreviousMapError),
}

impl fmt::Display for ProcessorError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ProcessorError::Css(err) => write!(f, "{}", err),
      ProcessorError::Message(msg) => write!(f, "{}", msg),
      ProcessorError::Parse(err) => write!(f, "{}", err),
      ProcessorError::Map(err) => write!(f, "{}", err),
    }
  }
}

impl std::error::Error for ProcessorError {}

impl From<CssSyntaxError> for ProcessorError {
  fn from(value: CssSyntaxError) -> Self {
    ProcessorError::Css(value)
  }
}

impl From<ParseError> for ProcessorError {
  fn from(value: ParseError) -> Self {
    ProcessorError::Parse(value)
  }
}

impl From<PreviousMapError> for ProcessorError {
  fn from(value: PreviousMapError) -> Self {
    ProcessorError::Map(value)
  }
}

pub trait Plugin: Send + Sync {
  fn name(&self) -> &str;

  fn prepare(
    &self,
    _result: &mut PostcssResult,
  ) -> Result<Option<Arc<dyn Plugin>>, ProcessorError> {
    Ok(None)
  }

  fn run(&self, _result: &mut PostcssResult) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn once(&self, _root: &RootLike, _result: &mut PostcssResult) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn once_exit(&self, _root: &RootLike, _result: &mut PostcssResult) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_root(&self, _root: &Root, _result: &mut PostcssResult) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_root_exit(
    &self,
    _root: &Root,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_document(
    &self,
    _document: &Document,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_document_exit(
    &self,
    _document: &Document,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_rule(&self, _rule: &Rule, _result: &mut PostcssResult) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_rule_filtered(
    &self,
    _selector: &str,
    _normalized_selector: &str,
    _rule: &Rule,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_rule_exit(
    &self,
    _rule: &Rule,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_rule_exit_filtered(
    &self,
    _selector: &str,
    _normalized_selector: &str,
    _rule: &Rule,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_at_rule(
    &self,
    _at_rule: &AtRule,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_at_rule_filtered(
    &self,
    _name: &str,
    _normalized_name: &str,
    _at_rule: &AtRule,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_at_rule_exit(
    &self,
    _at_rule: &AtRule,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_at_rule_exit_filtered(
    &self,
    _name: &str,
    _normalized_name: &str,
    _at_rule: &AtRule,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_declaration(
    &self,
    _decl: &Declaration,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_declaration_filtered(
    &self,
    _prop: &str,
    _normalized_prop: &str,
    _decl: &Declaration,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_declaration_exit(
    &self,
    _decl: &Declaration,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_declaration_exit_filtered(
    &self,
    _prop: &str,
    _normalized_prop: &str,
    _decl: &Declaration,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_comment(
    &self,
    _comment: &Comment,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }

  fn visit_comment_exit(
    &self,
    _comment: &Comment,
    _result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    Ok(())
  }
}

pub trait IntoPlugin {
  fn into_plugin(self) -> Arc<dyn Plugin>;
}

impl<T> IntoPlugin for T
where
  T: Plugin + 'static,
{
  fn into_plugin(self) -> Arc<dyn Plugin> {
    Arc::new(self)
  }
}

impl IntoPlugin for Arc<dyn Plugin> {
  fn into_plugin(self) -> Arc<dyn Plugin> {
    self
  }
}

#[derive(Clone)]
pub struct ProcessOptions {
  pub from: Option<String>,
  pub to: Option<String>,
  pub map: MapSetting,
  pub ignore_errors: bool,
  pub parser: Option<Arc<dyn CustomParser>>,
  pub stringifier: Option<Arc<dyn CustomStringifier>>,
  pub syntax: Option<SyntaxOptions>,
}

impl ProcessOptions {
  pub fn new() -> Self {
    Self {
      from: None,
      to: None,
      map: MapSetting::default(),
      ignore_errors: false,
      parser: None,
      stringifier: None,
      syntax: None,
    }
  }

  pub fn from_path(mut self, from: impl Into<String>) -> Self {
    self.from = Some(from.into());
    self
  }

  pub fn to_path(mut self, to: impl Into<String>) -> Self {
    self.to = Some(to.into());
    self
  }

  pub fn disable_map(mut self) -> Self {
    self.map = MapSetting::disabled();
    self
  }

  pub fn enable_map(mut self) -> Self {
    self.map = MapSetting::enabled(MapOptions::default());
    self
  }

  pub fn enable_map_with(mut self, opts: MapOptions) -> Self {
    self.map = MapSetting::enabled(opts);
    self
  }

  pub fn map(mut self, setting: MapSetting) -> Self {
    self.map = setting;
    self
  }

  pub fn ignore_errors(mut self, value: bool) -> Self {
    self.ignore_errors = value;
    self
  }

  pub fn parser<P>(mut self, parser: P) -> Self
  where
    P: IntoParser,
  {
    self.parser = Some(parser.into_parser());
    self
  }

  pub fn stringifier<S>(mut self, stringifier: S) -> Self
  where
    S: IntoStringifier,
  {
    self.stringifier = Some(stringifier.into_stringifier());
    self
  }

  pub fn syntax(mut self, syntax: SyntaxOptions) -> Self {
    self.syntax = Some(syntax);
    self
  }
}

impl std::fmt::Debug for ProcessOptions {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ProcessOptions")
      .field("from", &self.from)
      .field("to", &self.to)
      .field("map", &self.map)
      .field("ignore_errors", &self.ignore_errors)
      .finish()
  }
}

impl Default for ProcessOptions {
  fn default() -> Self {
    Self::new()
  }
}

pub trait CustomParser: Send + Sync {
  fn parse(&self, css: &str, opts: &ProcessOptions) -> Result<RootLike, ProcessorError>;
}

impl<F> CustomParser for F
where
  F: Fn(&str, &ProcessOptions) -> Result<RootLike, ProcessorError> + Send + Sync,
{
  fn parse(&self, css: &str, opts: &ProcessOptions) -> Result<RootLike, ProcessorError> {
    (self)(css, opts)
  }
}

pub trait IntoParser {
  fn into_parser(self) -> Arc<dyn CustomParser>;
}

impl<T> IntoParser for T
where
  T: CustomParser + 'static,
{
  fn into_parser(self) -> Arc<dyn CustomParser> {
    Arc::new(self)
  }
}

pub trait CustomStringifier: Send + Sync {
  fn stringify(
    &self,
    root: &RootLike,
    builder: &mut dyn FnMut(&str, Option<&NodeRef>, Option<&'static str>),
  );
}

impl<F> CustomStringifier for F
where
  F: Fn(&RootLike, &mut dyn FnMut(&str, Option<&NodeRef>, Option<&'static str>)) + Send + Sync,
{
  fn stringify(
    &self,
    root: &RootLike,
    builder: &mut dyn FnMut(&str, Option<&NodeRef>, Option<&'static str>),
  ) {
    (self)(root, builder)
  }
}

pub trait IntoStringifier {
  fn into_stringifier(self) -> Arc<dyn CustomStringifier>;
}

impl<T> IntoStringifier for T
where
  T: CustomStringifier + 'static,
{
  fn into_stringifier(self) -> Arc<dyn CustomStringifier> {
    Arc::new(self)
  }
}

#[derive(Clone, Default)]
pub struct SyntaxOptions {
  parser: Option<Arc<dyn CustomParser>>,
  stringifier: Option<Arc<dyn CustomStringifier>>,
}

impl SyntaxOptions {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn parser<P>(mut self, parser: P) -> Self
  where
    P: IntoParser,
  {
    self.parser = Some(parser.into_parser());
    self
  }

  pub fn stringifier<S>(mut self, stringifier: S) -> Self
  where
    S: IntoStringifier,
  {
    self.stringifier = Some(stringifier.into_stringifier());
    self
  }

  pub fn parser_ref(&self) -> Option<Arc<dyn CustomParser>> {
    self.parser.clone()
  }

  pub fn stringifier_ref(&self) -> Option<Arc<dyn CustomStringifier>> {
    self.stringifier.clone()
  }
}

#[derive(Default)]
struct DefaultParser;

impl CustomParser for DefaultParser {
  fn parse(&self, css: &str, opts: &ProcessOptions) -> Result<RootLike, ProcessorError> {
    let parse_options = ParseOptions {
      from: opts.from.clone(),
      map: opts.map.clone(),
      ignore_errors: opts.ignore_errors,
    };
    let root = parse_with_options(css, parse_options)?;
    Ok(RootLike::Root(root))
  }
}

#[derive(Default)]
struct DefaultStringifier;

impl CustomStringifier for DefaultStringifier {
  fn stringify(
    &self,
    root: &RootLike,
    builder: &mut dyn FnMut(&str, Option<&NodeRef>, Option<&'static str>),
  ) {
    match root {
      RootLike::Root(root) => crate::stringifier::stringify_with_builder(root.raw(), builder),
      RootLike::Document(document) => {
        let node = document.to_node();
        crate::stringifier::stringify_with_builder(&node, builder);
      }
    }
  }
}

fn default_parser() -> Arc<dyn CustomParser> {
  Arc::new(DefaultParser)
}

fn default_stringifier() -> Arc<dyn CustomStringifier> {
  Arc::new(DefaultStringifier)
}

fn resolve_parser(options: &ProcessOptions) -> Arc<dyn CustomParser> {
  if let Some(parser) = &options.parser {
    parser.clone()
  } else if let Some(syntax) = &options.syntax {
    syntax.parser_ref().unwrap_or_else(default_parser)
  } else {
    default_parser()
  }
}

fn resolve_stringifier(options: &ProcessOptions) -> Arc<dyn CustomStringifier> {
  if let Some(stringifier) = &options.stringifier {
    stringifier.clone()
  } else if let Some(syntax) = &options.syntax {
    syntax.stringifier_ref().unwrap_or_else(default_stringifier)
  } else {
    default_stringifier()
  }
}

pub struct Processor {
  version: &'static str,
  plugins: Vec<Arc<dyn Plugin>>,
}

impl Processor {
  pub fn new() -> Self {
    Self {
      version: "8.4.31",
      plugins: Vec::new(),
    }
  }

  pub fn from_plugins<I, P>(plugins: I) -> Self
  where
    I: IntoIterator<Item = P>,
    P: IntoPlugin,
  {
    let mut processor = Self::new();
    processor.use_plugins(plugins);
    processor
  }

  pub fn version(&self) -> &'static str {
    self.version
  }

  pub fn with_plugin<P: IntoPlugin>(mut self, plugin: P) -> Self {
    self.plugins.push(plugin.into_plugin());
    self
  }

  pub fn use_plugin<P: IntoPlugin>(&mut self, plugin: P) {
    self.plugins.push(plugin.into_plugin());
  }

  pub fn use_plugins<I, P>(&mut self, plugins: I)
  where
    I: IntoIterator<Item = P>,
    P: IntoPlugin,
  {
    for plugin in plugins {
      self.use_plugin(plugin);
    }
  }

  pub fn process(&self, css: &str) -> Result<ProcessResult, ProcessorError> {
    self.process_with_options(css, ProcessOptions::default())
  }

  pub fn process_with_options(
    &self,
    css: &str,
    options: ProcessOptions,
  ) -> Result<ProcessResult, ProcessorError> {
    let parser = resolve_parser(&options);
    let stringifier = resolve_stringifier(&options);
    let result_options = ResultOptions {
      from: options.from.clone(),
      to: options.to.clone(),
      map: options.map.clone(),
    };
    if self.plugins.is_empty()
      && options.parser.is_none()
      && options.stringifier.is_none()
      && options.syntax.is_none()
    {
      return Ok(ProcessResult::NoWork(NoWorkResult::new(
        self.version,
        css.to_string(),
        options,
        parser,
        stringifier,
        result_options,
      )?));
    }
    let plugins: Vec<_> = self.plugins.iter().cloned().collect();
    Ok(ProcessResult::Lazy(LazyResult::from_css(
      self.version,
      plugins,
      css.to_string(),
      options,
      parser,
      stringifier,
      result_options,
    )))
  }

  pub fn process_root(&self, root: Root) -> Result<LazyResult, ProcessorError> {
    self.process_root_with_options(root, ProcessOptions::new())
  }

  pub(crate) fn process_root_with_options(
    &self,
    root: Root,
    options: ProcessOptions,
  ) -> Result<LazyResult, ProcessorError> {
    let parser = resolve_parser(&options);
    let stringifier = resolve_stringifier(&options);
    let result_options = ResultOptions {
      from: options.from.clone(),
      to: options.to.clone(),
      map: options.map.clone(),
    };
    let plugins: Vec<_> = self.plugins.iter().cloned().collect();
    Ok(LazyResult::from_root(
      self.version,
      plugins,
      root,
      options,
      parser,
      stringifier,
      result_options,
    ))
  }

  pub fn process_document(&self, document: Document) -> Result<LazyResult, ProcessorError> {
    self.process_document_with_options(document, ProcessOptions::new())
  }

  pub(crate) fn process_document_with_options(
    &self,
    document: Document,
    options: ProcessOptions,
  ) -> Result<LazyResult, ProcessorError> {
    let parser = resolve_parser(&options);
    let stringifier = resolve_stringifier(&options);
    let result_options = ResultOptions {
      from: options.from.clone(),
      to: options.to.clone(),
      map: options.map.clone(),
    };
    let plugins: Vec<_> = self.plugins.iter().cloned().collect();
    Ok(LazyResult::from_document(
      self.version,
      plugins,
      document,
      options,
      parser,
      stringifier,
      result_options,
    ))
  }
}

fn apply_plugin_visitors(
  plugin: &dyn Plugin,
  result: &mut PostcssResult,
) -> Result<(), ProcessorError> {
  let root_like = result.root_like().clone();
  let root_node = root_like.to_node();
  plugin.once(&root_like, result)?;
  walk_plugin_node(plugin, result, root_node.clone())?;
  plugin.once_exit(&root_like, result)
}

fn walk_plugin_node(
  plugin: &dyn Plugin,
  result: &mut PostcssResult,
  node: NodeRef,
) -> Result<(), ProcessorError> {
  let (kind, children) = {
    let borrowed = node.borrow();
    let kind = borrowed.kind();
    let children = if borrowed.data.is_container() {
      borrowed.nodes.clone()
    } else {
      Vec::new()
    };
    (kind, children)
  };

  match kind {
    NodeKind::Root => {
      let root = Root::from_node(node.clone());
      plugin.visit_root(&root, result)?;
      for child in children {
        walk_plugin_node(plugin, result, child)?;
      }
      plugin.visit_root_exit(&root, result)?;
    }
    NodeKind::Document => {
      let document = Document::from_node(node.clone());
      plugin.visit_document(&document, result)?;
      for child in children {
        walk_plugin_node(plugin, result, child)?;
      }
      plugin.visit_document_exit(&document, result)?;
    }
    NodeKind::Rule => {
      let rule = Rule::from_node(node.clone());
      let selector = rule.selector();
      let normalized_selector = selector.to_lowercase();
      plugin.visit_rule(&rule, result)?;
      plugin.visit_rule_filtered(&selector, &normalized_selector, &rule, result)?;
      for child in children {
        walk_plugin_node(plugin, result, child)?;
      }
      plugin.visit_rule_exit(&rule, result)?;
      plugin.visit_rule_exit_filtered(&selector, &normalized_selector, &rule, result)?;
    }
    NodeKind::AtRule => {
      let at_rule = AtRule::from_node(node.clone());
      let name = at_rule.name();
      let normalized_name = name.to_lowercase();
      plugin.visit_at_rule(&at_rule, result)?;
      plugin.visit_at_rule_filtered(&name, &normalized_name, &at_rule, result)?;
      for child in children {
        walk_plugin_node(plugin, result, child)?;
      }
      plugin.visit_at_rule_exit(&at_rule, result)?;
      plugin.visit_at_rule_exit_filtered(&name, &normalized_name, &at_rule, result)?;
    }
    NodeKind::Declaration => {
      let decl = Declaration::from_node(node.clone());
      // Debug: trace declaration visitation for all plugins when enabled
      if std::env::var("COMPILED_DEBUG_COLORMIN").is_ok() {
        let plugin_name = result
          .last_plugin
          .as_ref()
          .map(|s| s.as_str())
          .unwrap_or("<unknown-plugin>");
        // Try to capture a minimal parent context label
        let parent_label = {
          if let Some(parent) = node.borrow().parent() {
            match parent.borrow().kind() {
              NodeKind::Rule => {
                let r = Rule::from_node(parent.clone());
                format!("rule: {}", r.selector())
              }
              NodeKind::AtRule => {
                let a = AtRule::from_node(parent.clone());
                let name = a.name();
                let params = a.params();
                if params.is_empty() {
                  format!("@{}", name)
                } else {
                  format!("@{} {}", name, params)
                }
              }
              other => format!("{:?}", other),
            }
          } else {
            "<no-parent>".to_string()
          }
        };
        eprintln!(
          "[postcss.decl @{}] {}='{}' [{}]",
          plugin_name,
          decl.prop(),
          decl.value(),
          parent_label
        );
      }
      let prop = decl.prop();
      let normalized_prop = prop.to_lowercase();
      plugin.visit_declaration(&decl, result)?;
      plugin.visit_declaration_filtered(&prop, &normalized_prop, &decl, result)?;
      plugin.visit_declaration_exit(&decl, result)?;
      plugin.visit_declaration_exit_filtered(&prop, &normalized_prop, &decl, result)?;
    }
    NodeKind::Comment => {
      let comment = Comment::from_node(node.clone());
      plugin.visit_comment(&comment, result)?;
      plugin.visit_comment_exit(&comment, result)?;
    }
  }

  Ok(())
}

pub struct NoWorkResult {
  processor_version: &'static str,
  css: String,
  options: ProcessOptions,
  parser: Arc<dyn CustomParser>,
  result: Option<PostcssResult>,
  root: Option<RootLike>,
}

impl NoWorkResult {
  fn new(
    processor_version: &'static str,
    css: String,
    options: ProcessOptions,
    parser: Arc<dyn CustomParser>,
    stringifier: Arc<dyn CustomStringifier>,
    result_options: ResultOptions,
  ) -> Result<Self, ProcessorError> {
    let from = result_options.from.clone();
    let to = result_options.to.clone();
    let map_setting = result_options.map.clone();

    let metadata = ProcessorMetadata::new(processor_version);
    let mut result = PostcssResult::new(
      RootLike::Root(Root::new()),
      metadata,
      result_options,
      stringifier.clone(),
    );
    result.set_css(Some(css.clone()));

    let mut generator = MapGenerator::new(
      None,
      Some(css.clone()),
      &map_setting,
      from.as_deref(),
      to.as_deref(),
      stringifier,
    );
    let generated = generator.generate()?;
    result.set_css(Some(generated.css));
    result.set_map(generated.map);

    Ok(Self {
      processor_version,
      css,
      options,
      parser,
      result: Some(result),
      root: None,
    })
  }

  fn ensure_root(&mut self) -> Result<(), ProcessorError> {
    if self.root.is_some() {
      return Ok(());
    }

    let root = self.parser.parse(&self.css, &self.options)?;
    if let Some(result) = self.result.as_mut() {
      result.replace_root_like(root.clone());
    }
    self.root = Some(root);
    Ok(())
  }

  pub fn processor_version(&self) -> &'static str {
    self.processor_version
  }

  pub fn css(&mut self) -> Result<&str, ProcessorError> {
    let result = self
      .result
      .as_mut()
      .expect("result must be initialized before accessing css");
    Ok(result.css())
  }

  pub fn stringify(&mut self) -> Result<String, ProcessorError> {
    self.css().map(|css| css.to_string())
  }

  pub fn to_css_string(&mut self) -> Result<String, ProcessorError> {
    self.stringify()
  }

  pub fn content(&mut self) -> Result<&str, ProcessorError> {
    let result = self
      .result
      .as_mut()
      .expect("result must be initialized before accessing content");
    Ok(result.content())
  }

  pub fn map(&mut self) -> Result<Option<&str>, ProcessorError> {
    let result = self
      .result
      .as_mut()
      .expect("result must be initialized before accessing map");
    Ok(result.map())
  }

  pub fn messages(&mut self) -> Result<&[crate::result::Message], ProcessorError> {
    let result = self
      .result
      .as_ref()
      .expect("result must be initialized before accessing messages");
    Ok(result.messages())
  }

  pub fn warnings(&mut self) -> Result<&[Warning], ProcessorError> {
    Ok(self.result()?.warnings())
  }

  pub fn processor(&mut self) -> Result<&ProcessorMetadata, ProcessorError> {
    Ok(self.result()?.processor())
  }

  pub fn result(&mut self) -> Result<&PostcssResult, ProcessorError> {
    self.ensure_root()?;
    Ok(
      self
        .result
        .as_ref()
        .expect("result must be initialized before access"),
    )
  }

  pub fn result_mut(&mut self) -> Result<&mut PostcssResult, ProcessorError> {
    self.ensure_root()?;
    Ok(
      self
        .result
        .as_mut()
        .expect("result must be initialized before mutable access"),
    )
  }

  pub fn root_like(&mut self) -> Result<&RootLike, ProcessorError> {
    self.ensure_root()?;
    Ok(self.root.as_ref().expect("root must be initialized"))
  }

  pub fn root(&mut self) -> Result<Option<&Root>, ProcessorError> {
    self.root_like().map(|root| root.as_root())
  }

  pub fn document(&mut self) -> Result<Option<&Document>, ProcessorError> {
    self.root_like().map(|root| root.as_document())
  }

  pub fn into_result(mut self) -> Result<PostcssResult, ProcessorError> {
    self.ensure_root()?;
    Ok(
      self
        .result
        .take()
        .expect("result must be initialized before consuming"),
    )
  }

  pub fn sync(self) -> Result<PostcssResult, ProcessorError> {
    self.into_result()
  }

  pub fn r#async(self) -> impl Future<Output = Result<PostcssResult, ProcessorError>> {
    async move {
      let mut this = self;
      this.ensure_root()?;
      Ok(
        this
          .result
          .take()
          .expect("result must be initialized before completing future"),
      )
    }
  }
}

impl Future for NoWorkResult {
  type Output = Result<PostcssResult, ProcessorError>;

  fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
    match self.ensure_root() {
      Ok(()) => {
        let result = self
          .result
          .as_ref()
          .expect("result must be initialized before polling")
          .clone();
        Poll::Ready(Ok(result))
      }
      Err(err) => Poll::Ready(Err(err)),
    }
  }
}

pub struct LazyResult {
  processor_version: &'static str,
  plugins: Vec<Arc<dyn Plugin>>,
  css_input: Option<String>,
  root_input: Option<RootLike>,
  options: ProcessOptions,
  parser: Arc<dyn CustomParser>,
  stringifier: Arc<dyn CustomStringifier>,
  result_options: ResultOptions,
  processed: bool,
  inner: Option<PostcssResult>,
}

impl LazyResult {
  fn from_css(
    processor_version: &'static str,
    plugins: Vec<Arc<dyn Plugin>>,
    css: String,
    options: ProcessOptions,
    parser: Arc<dyn CustomParser>,
    stringifier: Arc<dyn CustomStringifier>,
    result_options: ResultOptions,
  ) -> Self {
    Self {
      processor_version,
      plugins,
      css_input: Some(css),
      root_input: None,
      options,
      parser,
      stringifier,
      result_options,
      processed: false,
      inner: None,
    }
  }

  fn from_root_like(
    processor_version: &'static str,
    plugins: Vec<Arc<dyn Plugin>>,
    root: RootLike,
    options: ProcessOptions,
    parser: Arc<dyn CustomParser>,
    stringifier: Arc<dyn CustomStringifier>,
    result_options: ResultOptions,
  ) -> Self {
    Self {
      processor_version,
      plugins,
      css_input: None,
      root_input: Some(root),
      options,
      parser,
      stringifier,
      result_options,
      processed: false,
      inner: None,
    }
  }

  fn from_root(
    processor_version: &'static str,
    plugins: Vec<Arc<dyn Plugin>>,
    root: Root,
    options: ProcessOptions,
    parser: Arc<dyn CustomParser>,
    stringifier: Arc<dyn CustomStringifier>,
    result_options: ResultOptions,
  ) -> Self {
    Self::from_root_like(
      processor_version,
      plugins,
      RootLike::Root(root),
      options,
      parser,
      stringifier,
      result_options,
    )
  }

  fn from_document(
    processor_version: &'static str,
    plugins: Vec<Arc<dyn Plugin>>,
    document: Document,
    options: ProcessOptions,
    parser: Arc<dyn CustomParser>,
    stringifier: Arc<dyn CustomStringifier>,
    result_options: ResultOptions,
  ) -> Self {
    Self::from_root_like(
      processor_version,
      plugins,
      RootLike::Document(document),
      options,
      parser,
      stringifier,
      result_options,
    )
  }

  fn ensure_processed(&mut self) -> Result<(), ProcessorError> {
    if self.processed {
      return Ok(());
    }

    let root_like = if let Some(root) = self.root_input.take() {
      root
    } else {
      let css = self
        .css_input
        .as_ref()
        .expect("CSS input should be available before processing");
      self.parser.parse(css, &self.options)?
    };

    let metadata = ProcessorMetadata::new(self.processor_version);
    let mut result = PostcssResult::new(
      root_like.clone(),
      metadata,
      self.result_options.clone(),
      self.stringifier.clone(),
    );

    let mut prepared_plugins = Vec::with_capacity(self.plugins.len());
    for plugin in &self.plugins {
      if let Some(prepared) = plugin.prepare(&mut result)? {
        prepared_plugins.push(prepared);
      } else {
        prepared_plugins.push(Arc::clone(plugin));
      }
    }
    self.plugins = prepared_plugins;

    for plugin in &self.plugins {
      let tracing = std::env::var("COMPILED_CLI_TRACE").is_ok();
      if tracing {
        eprintln!("[postcss] plugin {}: run", plugin.name());
      }
      result.set_last_plugin(Some(plugin.name().to_string()));
      plugin.run(&mut result)?;
      if tracing {
        eprintln!("[postcss] plugin {}: walk", plugin.name());
      }
      apply_plugin_visitors(plugin.as_ref(), &mut result)?;
      if tracing {
        eprintln!("[postcss] plugin {}: done", plugin.name());
      }
    }

    self.processed = true;
    self.inner = Some(result);
    Ok(())
  }

  pub fn css(&mut self) -> Result<&str, ProcessorError> {
    Ok(self.result_mut()?.css())
  }

  pub fn stringify(&mut self) -> Result<String, ProcessorError> {
    self.css().map(|css| css.to_string())
  }

  pub fn to_css_string(&mut self) -> Result<String, ProcessorError> {
    self.stringify()
  }

  pub fn warnings(&mut self) -> Result<&[Warning], ProcessorError> {
    Ok(self.result()?.warnings())
  }

  pub fn root(&mut self) -> Result<Option<&Root>, ProcessorError> {
    Ok(self.result()?.root())
  }

  pub fn document(&mut self) -> Result<Option<&Document>, ProcessorError> {
    Ok(self.result()?.document())
  }

  pub fn into_result(mut self) -> Result<PostcssResult, ProcessorError> {
    self.ensure_processed()?;
    self.inner.as_mut().unwrap().css();
    Ok(self.inner.take().unwrap())
  }

  pub fn sync(mut self) -> Result<PostcssResult, ProcessorError> {
    self.ensure_processed()?;
    let mut result = self.inner.take().expect("result must be initialized");
    result.css();
    Ok(result)
  }

  pub fn r#async(self) -> impl Future<Output = Result<PostcssResult, ProcessorError>> {
    async move {
      let mut this = self;
      this.ensure_processed()?;
      let mut result = this
        .inner
        .take()
        .expect("result must be initialized before completing future");
      result.css();
      Ok(result)
    }
  }

  pub fn processor_version(&self) -> &'static str {
    self.processor_version
  }

  pub fn content(&mut self) -> Result<&str, ProcessorError> {
    Ok(self.result_mut()?.content())
  }

  pub fn map(&mut self) -> Result<Option<&str>, ProcessorError> {
    Ok(self.result_mut()?.map())
  }

  pub fn messages(&mut self) -> Result<&[crate::result::Message], ProcessorError> {
    Ok(self.result()?.messages())
  }

  pub fn processor(&mut self) -> Result<&ProcessorMetadata, ProcessorError> {
    Ok(self.result()?.processor())
  }

  pub fn result(&mut self) -> Result<&PostcssResult, ProcessorError> {
    self.ensure_processed()?;
    Ok(self.inner.as_ref().expect("result must be initialized"))
  }

  pub fn result_mut(&mut self) -> Result<&mut PostcssResult, ProcessorError> {
    self.ensure_processed()?;
    Ok(
      self
        .inner
        .as_mut()
        .expect("result must be initialized for mutable access"),
    )
  }

  pub fn root_like(&mut self) -> Result<&RootLike, ProcessorError> {
    let result = self.result()?;
    Ok(result.root_like())
  }
}

impl Future for LazyResult {
  type Output = Result<PostcssResult, ProcessorError>;

  fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
    match self.ensure_processed() {
      Ok(()) => {
        let result = self
          .inner
          .as_ref()
          .expect("result must be initialized before polling")
          .clone();
        Poll::Ready(Ok(result))
      }
      Err(err) => Poll::Ready(Err(err)),
    }
  }
}

pub enum ProcessResult {
  Lazy(LazyResult),
  NoWork(NoWorkResult),
}

impl ProcessResult {
  pub fn is_no_work(&self) -> bool {
    matches!(self, ProcessResult::NoWork(_))
  }

  pub fn processor_version(&self) -> &'static str {
    match self {
      ProcessResult::Lazy(lazy) => lazy.processor_version(),
      ProcessResult::NoWork(no_work) => no_work.processor_version(),
    }
  }

  pub fn css(&mut self) -> Result<&str, ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.css(),
      ProcessResult::NoWork(no_work) => no_work.css(),
    }
  }

  pub fn stringify(&mut self) -> Result<String, ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.stringify(),
      ProcessResult::NoWork(no_work) => no_work.stringify(),
    }
  }

  pub fn to_css_string(&mut self) -> Result<String, ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.to_css_string(),
      ProcessResult::NoWork(no_work) => no_work.to_css_string(),
    }
  }

  pub fn content(&mut self) -> Result<&str, ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.content(),
      ProcessResult::NoWork(no_work) => no_work.content(),
    }
  }

  pub fn map(&mut self) -> Result<Option<&str>, ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.map(),
      ProcessResult::NoWork(no_work) => no_work.map(),
    }
  }

  pub fn messages(&mut self) -> Result<&[crate::result::Message], ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.messages(),
      ProcessResult::NoWork(no_work) => no_work.messages(),
    }
  }

  pub fn warnings(&mut self) -> Result<&[Warning], ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.warnings(),
      ProcessResult::NoWork(no_work) => no_work.warnings(),
    }
  }

  pub fn processor(&mut self) -> Result<&ProcessorMetadata, ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.processor(),
      ProcessResult::NoWork(no_work) => no_work.processor(),
    }
  }

  pub fn result(&mut self) -> Result<&PostcssResult, ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.result(),
      ProcessResult::NoWork(no_work) => no_work.result(),
    }
  }

  pub fn result_mut(&mut self) -> Result<&mut PostcssResult, ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.result_mut(),
      ProcessResult::NoWork(no_work) => no_work.result_mut(),
    }
  }

  pub fn root_like(&mut self) -> Result<&RootLike, ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.root_like(),
      ProcessResult::NoWork(no_work) => no_work.root_like(),
    }
  }

  pub fn root(&mut self) -> Result<Option<&Root>, ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.root(),
      ProcessResult::NoWork(no_work) => no_work.root(),
    }
  }

  pub fn document(&mut self) -> Result<Option<&Document>, ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.document(),
      ProcessResult::NoWork(no_work) => no_work.document(),
    }
  }

  pub fn into_result(self) -> Result<PostcssResult, ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.into_result(),
      ProcessResult::NoWork(no_work) => no_work.into_result(),
    }
  }

  pub fn sync(self) -> Result<PostcssResult, ProcessorError> {
    match self {
      ProcessResult::Lazy(lazy) => lazy.sync(),
      ProcessResult::NoWork(no_work) => no_work.sync(),
    }
  }

  pub fn r#async(self) -> impl Future<Output = Result<PostcssResult, ProcessorError>> {
    async move {
      match self {
        ProcessResult::Lazy(lazy) => lazy.r#async().await,
        ProcessResult::NoWork(no_work) => no_work.r#async().await,
      }
    }
  }
}

impl Future for ProcessResult {
  type Output = Result<PostcssResult, ProcessorError>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    match self.as_mut().get_mut() {
      ProcessResult::Lazy(lazy) => Pin::new(lazy).poll(cx),
      ProcessResult::NoWork(no_work) => Pin::new(no_work).poll(cx),
    }
  }
}

type RunHook = Box<dyn Fn(&mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync>;

type NodeHook<T> = Box<dyn Fn(&T, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync>;

type PrepareHook =
  Box<dyn Fn(&mut PostcssResult) -> Result<Option<Arc<dyn Plugin>>, ProcessorError> + Send + Sync>;

type FilteredNodeHook<T> =
  Box<dyn Fn(&str, &str, &T, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync>;

fn call_run_hook(hook: &Option<RunHook>, result: &mut PostcssResult) -> Result<(), ProcessorError> {
  if let Some(handler) = hook {
    handler(result)
  } else {
    Ok(())
  }
}

fn call_node_hook<T>(
  hook: &Option<NodeHook<T>>,
  node: &T,
  result: &mut PostcssResult,
) -> Result<(), ProcessorError> {
  if let Some(handler) = hook {
    handler(node, result)
  } else {
    Ok(())
  }
}

fn call_filtered_node_hooks<T>(
  hooks: &HashMap<String, FilteredNodeHook<T>>,
  original: &str,
  normalized: &str,
  node: &T,
  result: &mut PostcssResult,
) -> Result<(), ProcessorError> {
  if let Some(handler) = hooks.get("*") {
    handler(original, normalized, node, result)?;
  }
  if let Some(handler) = hooks.get(normalized) {
    handler(original, normalized, node, result)?;
  }
  Ok(())
}

fn normalize_filter(value: &str) -> String {
  if value == "*" {
    "*".to_string()
  } else {
    value.to_lowercase()
  }
}

#[derive(Default)]
pub struct PluginBuilder {
  name: String,
  run: Option<RunHook>,
  once: Option<NodeHook<RootLike>>,
  once_exit: Option<NodeHook<RootLike>>,
  visit_root: Option<NodeHook<Root>>,
  visit_root_exit: Option<NodeHook<Root>>,
  visit_document: Option<NodeHook<Document>>,
  visit_document_exit: Option<NodeHook<Document>>,
  visit_rule: Option<NodeHook<Rule>>,
  visit_rule_exit: Option<NodeHook<Rule>>,
  visit_at_rule: Option<NodeHook<AtRule>>,
  visit_at_rule_exit: Option<NodeHook<AtRule>>,
  visit_declaration: Option<NodeHook<Declaration>>,
  visit_declaration_exit: Option<NodeHook<Declaration>>,
  visit_comment: Option<NodeHook<Comment>>,
  visit_comment_exit: Option<NodeHook<Comment>>,
  prepare: Option<PrepareHook>,
  visit_rule_filters: HashMap<String, FilteredNodeHook<Rule>>,
  visit_rule_exit_filters: HashMap<String, FilteredNodeHook<Rule>>,
  visit_at_rule_filters: HashMap<String, FilteredNodeHook<AtRule>>,
  visit_at_rule_exit_filters: HashMap<String, FilteredNodeHook<AtRule>>,
  visit_declaration_filters: HashMap<String, FilteredNodeHook<Declaration>>,
  visit_declaration_exit_filters: HashMap<String, FilteredNodeHook<Declaration>>,
}

impl PluginBuilder {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      ..Self::default()
    }
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn run<F>(mut self, f: F) -> Self
  where
    F: Fn(&mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.run = Some(Box::new(f));
    self
  }

  pub fn prepare<F>(mut self, f: F) -> Self
  where
    F: Fn(&mut PostcssResult) -> Result<Option<Arc<dyn Plugin>>, ProcessorError>
      + Send
      + Sync
      + 'static,
  {
    self.prepare = Some(Box::new(f));
    self
  }

  pub fn once<F>(mut self, f: F) -> Self
  where
    F: Fn(&RootLike, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.once = Some(Box::new(f));
    self
  }

  pub fn once_exit<F>(mut self, f: F) -> Self
  where
    F: Fn(&RootLike, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.once_exit = Some(Box::new(f));
    self
  }

  pub fn root<F>(mut self, f: F) -> Self
  where
    F: Fn(&Root, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.visit_root = Some(Box::new(f));
    self
  }

  pub fn root_exit<F>(mut self, f: F) -> Self
  where
    F: Fn(&Root, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.visit_root_exit = Some(Box::new(f));
    self
  }

  pub fn document<F>(mut self, f: F) -> Self
  where
    F: Fn(&Document, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.visit_document = Some(Box::new(f));
    self
  }

  pub fn document_exit<F>(mut self, f: F) -> Self
  where
    F: Fn(&Document, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.visit_document_exit = Some(Box::new(f));
    self
  }

  pub fn rule<F>(mut self, f: F) -> Self
  where
    F: Fn(&Rule, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.visit_rule = Some(Box::new(f));
    self
  }

  pub fn rule_exit<F>(mut self, f: F) -> Self
  where
    F: Fn(&Rule, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.visit_rule_exit = Some(Box::new(f));
    self
  }

  pub fn rule_filter<F>(mut self, filter: impl Into<String>, f: F) -> Self
  where
    F: Fn(&Rule, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    let key = normalize_filter(&filter.into());
    self
      .visit_rule_filters
      .insert(key, Box::new(move |_, _, rule, result| f(rule, result)));
    self
  }

  pub fn rule_filter_exit<F>(mut self, filter: impl Into<String>, f: F) -> Self
  where
    F: Fn(&Rule, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    let key = normalize_filter(&filter.into());
    self
      .visit_rule_exit_filters
      .insert(key, Box::new(move |_, _, rule, result| f(rule, result)));
    self
  }

  pub fn at_rule<F>(mut self, f: F) -> Self
  where
    F: Fn(&AtRule, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.visit_at_rule = Some(Box::new(f));
    self
  }

  pub fn at_rule_exit<F>(mut self, f: F) -> Self
  where
    F: Fn(&AtRule, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.visit_at_rule_exit = Some(Box::new(f));
    self
  }

  pub fn at_rule_filter<F>(mut self, filter: impl Into<String>, f: F) -> Self
  where
    F: Fn(&AtRule, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    let key = normalize_filter(&filter.into());
    self.visit_at_rule_filters.insert(
      key,
      Box::new(move |_, _, at_rule, result| f(at_rule, result)),
    );
    self
  }

  pub fn at_rule_filter_exit<F>(mut self, filter: impl Into<String>, f: F) -> Self
  where
    F: Fn(&AtRule, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    let key = normalize_filter(&filter.into());
    self.visit_at_rule_exit_filters.insert(
      key,
      Box::new(move |_, _, at_rule, result| f(at_rule, result)),
    );
    self
  }

  pub fn decl<F>(mut self, f: F) -> Self
  where
    F: Fn(&Declaration, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.visit_declaration = Some(Box::new(f));
    self
  }

  pub fn decl_exit<F>(mut self, f: F) -> Self
  where
    F: Fn(&Declaration, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.visit_declaration_exit = Some(Box::new(f));
    self
  }

  pub fn decl_filter<F>(mut self, filter: impl Into<String>, f: F) -> Self
  where
    F: Fn(&Declaration, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    let key = normalize_filter(&filter.into());
    self
      .visit_declaration_filters
      .insert(key, Box::new(move |_, _, decl, result| f(decl, result)));
    self
  }

  pub fn decl_filter_exit<F>(mut self, filter: impl Into<String>, f: F) -> Self
  where
    F: Fn(&Declaration, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    let key = normalize_filter(&filter.into());
    self
      .visit_declaration_exit_filters
      .insert(key, Box::new(move |_, _, decl, result| f(decl, result)));
    self
  }

  pub fn comment<F>(mut self, f: F) -> Self
  where
    F: Fn(&Comment, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.visit_comment = Some(Box::new(f));
    self
  }

  pub fn comment_exit<F>(mut self, f: F) -> Self
  where
    F: Fn(&Comment, &mut PostcssResult) -> Result<(), ProcessorError> + Send + Sync + 'static,
  {
    self.visit_comment_exit = Some(Box::new(f));
    self
  }

  pub fn build(self) -> BuiltPlugin {
    BuiltPlugin {
      name: self.name,
      run: self.run,
      once: self.once,
      once_exit: self.once_exit,
      visit_root: self.visit_root,
      visit_root_exit: self.visit_root_exit,
      visit_document: self.visit_document,
      visit_document_exit: self.visit_document_exit,
      visit_rule: self.visit_rule,
      visit_rule_exit: self.visit_rule_exit,
      visit_at_rule: self.visit_at_rule,
      visit_at_rule_exit: self.visit_at_rule_exit,
      visit_declaration: self.visit_declaration,
      visit_declaration_exit: self.visit_declaration_exit,
      visit_comment: self.visit_comment,
      visit_comment_exit: self.visit_comment_exit,
      prepare: self.prepare,
      visit_rule_filters: self.visit_rule_filters,
      visit_rule_exit_filters: self.visit_rule_exit_filters,
      visit_at_rule_filters: self.visit_at_rule_filters,
      visit_at_rule_exit_filters: self.visit_at_rule_exit_filters,
      visit_declaration_filters: self.visit_declaration_filters,
      visit_declaration_exit_filters: self.visit_declaration_exit_filters,
    }
  }
}

impl IntoPlugin for PluginBuilder {
  fn into_plugin(self) -> Arc<dyn Plugin> {
    Arc::new(self.build())
  }
}

pub struct BuiltPlugin {
  name: String,
  run: Option<RunHook>,
  once: Option<NodeHook<RootLike>>,
  once_exit: Option<NodeHook<RootLike>>,
  visit_root: Option<NodeHook<Root>>,
  visit_root_exit: Option<NodeHook<Root>>,
  visit_document: Option<NodeHook<Document>>,
  visit_document_exit: Option<NodeHook<Document>>,
  visit_rule: Option<NodeHook<Rule>>,
  visit_rule_exit: Option<NodeHook<Rule>>,
  visit_at_rule: Option<NodeHook<AtRule>>,
  visit_at_rule_exit: Option<NodeHook<AtRule>>,
  visit_declaration: Option<NodeHook<Declaration>>,
  visit_declaration_exit: Option<NodeHook<Declaration>>,
  visit_comment: Option<NodeHook<Comment>>,
  visit_comment_exit: Option<NodeHook<Comment>>,
  prepare: Option<PrepareHook>,
  visit_rule_filters: HashMap<String, FilteredNodeHook<Rule>>,
  visit_rule_exit_filters: HashMap<String, FilteredNodeHook<Rule>>,
  visit_at_rule_filters: HashMap<String, FilteredNodeHook<AtRule>>,
  visit_at_rule_exit_filters: HashMap<String, FilteredNodeHook<AtRule>>,
  visit_declaration_filters: HashMap<String, FilteredNodeHook<Declaration>>,
  visit_declaration_exit_filters: HashMap<String, FilteredNodeHook<Declaration>>,
}

impl Plugin for BuiltPlugin {
  fn name(&self) -> &str {
    &self.name
  }

  fn prepare(&self, result: &mut PostcssResult) -> Result<Option<Arc<dyn Plugin>>, ProcessorError> {
    if let Some(handler) = &self.prepare {
      handler(result)
    } else {
      Ok(None)
    }
  }

  fn run(&self, result: &mut PostcssResult) -> Result<(), ProcessorError> {
    call_run_hook(&self.run, result)
  }

  fn once(&self, root: &RootLike, result: &mut PostcssResult) -> Result<(), ProcessorError> {
    call_node_hook(&self.once, root, result)
  }

  fn once_exit(&self, root: &RootLike, result: &mut PostcssResult) -> Result<(), ProcessorError> {
    call_node_hook(&self.once_exit, root, result)
  }

  fn visit_root(&self, root: &Root, result: &mut PostcssResult) -> Result<(), ProcessorError> {
    call_node_hook(&self.visit_root, root, result)
  }

  fn visit_root_exit(&self, root: &Root, result: &mut PostcssResult) -> Result<(), ProcessorError> {
    call_node_hook(&self.visit_root_exit, root, result)
  }

  fn visit_document(
    &self,
    document: &Document,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_node_hook(&self.visit_document, document, result)
  }

  fn visit_document_exit(
    &self,
    document: &Document,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_node_hook(&self.visit_document_exit, document, result)
  }

  fn visit_rule(&self, rule: &Rule, result: &mut PostcssResult) -> Result<(), ProcessorError> {
    call_node_hook(&self.visit_rule, rule, result)
  }

  fn visit_rule_filtered(
    &self,
    selector: &str,
    normalized_selector: &str,
    rule: &Rule,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_filtered_node_hooks(
      &self.visit_rule_filters,
      selector,
      normalized_selector,
      rule,
      result,
    )
  }

  fn visit_rule_exit(&self, rule: &Rule, result: &mut PostcssResult) -> Result<(), ProcessorError> {
    call_node_hook(&self.visit_rule_exit, rule, result)
  }

  fn visit_rule_exit_filtered(
    &self,
    selector: &str,
    normalized_selector: &str,
    rule: &Rule,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_filtered_node_hooks(
      &self.visit_rule_exit_filters,
      selector,
      normalized_selector,
      rule,
      result,
    )
  }

  fn visit_at_rule(
    &self,
    at_rule: &AtRule,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_node_hook(&self.visit_at_rule, at_rule, result)
  }

  fn visit_at_rule_filtered(
    &self,
    name: &str,
    normalized_name: &str,
    at_rule: &AtRule,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_filtered_node_hooks(
      &self.visit_at_rule_filters,
      name,
      normalized_name,
      at_rule,
      result,
    )
  }

  fn visit_at_rule_exit(
    &self,
    at_rule: &AtRule,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_node_hook(&self.visit_at_rule_exit, at_rule, result)
  }

  fn visit_at_rule_exit_filtered(
    &self,
    name: &str,
    normalized_name: &str,
    at_rule: &AtRule,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_filtered_node_hooks(
      &self.visit_at_rule_exit_filters,
      name,
      normalized_name,
      at_rule,
      result,
    )
  }

  fn visit_declaration(
    &self,
    decl: &Declaration,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_node_hook(&self.visit_declaration, decl, result)
  }

  fn visit_declaration_filtered(
    &self,
    prop: &str,
    normalized_prop: &str,
    decl: &Declaration,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_filtered_node_hooks(
      &self.visit_declaration_filters,
      prop,
      normalized_prop,
      decl,
      result,
    )
  }

  fn visit_declaration_exit(
    &self,
    decl: &Declaration,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_node_hook(&self.visit_declaration_exit, decl, result)
  }

  fn visit_declaration_exit_filtered(
    &self,
    prop: &str,
    normalized_prop: &str,
    decl: &Declaration,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_filtered_node_hooks(
      &self.visit_declaration_exit_filters,
      prop,
      normalized_prop,
      decl,
      result,
    )
  }

  fn visit_comment(
    &self,
    comment: &Comment,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_node_hook(&self.visit_comment, comment, result)
  }

  fn visit_comment_exit(
    &self,
    comment: &Comment,
    result: &mut PostcssResult,
  ) -> Result<(), ProcessorError> {
    call_node_hook(&self.visit_comment_exit, comment, result)
  }
}

pub fn plugin(name: impl Into<String>) -> PluginBuilder {
  let name_string = name.into();
  PluginBuilder::new(name_string)
}
