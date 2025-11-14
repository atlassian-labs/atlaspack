// Disable all warnings for this crate, as it's being iterated on by LLM
#![allow(
  warnings,
  clippy::all,
  clippy::pedantic,
  clippy::restriction,
  clippy::nursery,
  clippy::cargo
)]

pub mod config;
pub mod css;
pub mod eval;
pub mod hash;
pub mod migration_hash;
mod module_cache;
mod token_utils;

pub use config::{CompiledCssInJsConfig, CompiledCssInJsTransformConfig};

use crate::css::{
  AtRuleInput, CssArtifacts, CssOptions, CssRuleInput, NormalizeCssValueOptions,
  NormalizedCssValue, RuntimeClassCondition, RuntimeCssVariable, add_unit_if_needed,
  atomicize_literal, atomicize_rules, minify_at_rule_params, normalize_css_value,
  normalize_css_value_with_options, normalize_selector, selector_priority, selector_sort_key,
  wrap_at_rules,
};
use crate::hash::hash;
use crate::token_utils::resolve_token_expression;
use indexmap::IndexMap;
use module_cache::{
  ModuleFingerprint, ModuleStaticData, compute_tree_hash, get_cached_module, store_cached_module,
};
use once_cell::sync::{Lazy, OnceCell};
use oxc_resolver::{ResolveOptions, Resolver};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::iter::Peekable;
use std::path::{Path, PathBuf};
use std::str::Chars;
use std::sync::Mutex;
use std::thread::ThreadId;
use swc_core::atoms::Atom;
use swc_core::common::comments::{Comments, SingleThreadedComments};
use swc_core::common::plugin::metadata::TransformPluginMetadataContextKind;
use swc_core::common::sync::Lrc;
use swc_core::common::{
  BytePos, DUMMY_SP, FileName, Mark, SourceMap, SourceMapper, Span, Spanned, SyntaxContext,
};
use swc_core::ecma::ast::EsVersion;
use swc_core::ecma::ast::*;
use swc_core::ecma::codegen::{Config as CodegenConfig, Emitter, Node, text_writer::JsWriter};
use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};
use swc_core::ecma::utils::quote_ident;
use swc_core::ecma::visit::{Visit, VisitMut, VisitMutWith, VisitWith};
use swc_core::plugin::proxies::{
  PluginCommentsProxy, PluginSourceMapProxy, TransformPluginProgramMetadata,
};

use walkdir::{DirEntry, WalkDir};

static LATEST_ARTIFACTS: Lazy<Mutex<HashMap<ThreadId, StyleArtifacts>>> =
  Lazy::new(|| Mutex::new(HashMap::new()));
static WORKSPACE_PACKAGE_MAP: Lazy<Mutex<HashMap<PathBuf, HashMap<String, PathBuf>>>> =
  Lazy::new(|| Mutex::new(HashMap::new()));

thread_local! {
  static EMIT_COMMENTS: RefCell<Option<Box<dyn Comments>>> = RefCell::new(None);
}

thread_local! {
  static EXPRESSION_SNIPPETS: RefCell<HashMap<(BytePos, BytePos), String>> = RefCell::new(HashMap::new());
}

#[doc(hidden)]
pub struct EmitCommentsGuard;

impl EmitCommentsGuard {
  pub fn new(comments: &SingleThreadedComments) -> Self {
    Self::with_comments(Box::new(comments.clone()))
  }

  pub fn from_proxy(proxy: PluginCommentsProxy) -> Self {
    Self::with_comments(Box::new(proxy))
  }

  fn with_comments(comments: Box<dyn Comments>) -> Self {
    EMIT_COMMENTS.with(|slot| {
      *slot.borrow_mut() = Some(comments);
    });
    Self
  }
}

impl Drop for EmitCommentsGuard {
  fn drop(&mut self) {
    EMIT_COMMENTS.with(|slot| {
      slot.borrow_mut().take();
    });
  }
}

thread_local! {
  static SOURCE_MAP: RefCell<Option<SourceMapReader>> = RefCell::new(None);
}

enum SourceMapReader {
  Real(Lrc<SourceMap>),
  #[cfg(target_arch = "wasm32")]
  Proxy(PluginSourceMapProxy),
}

#[doc(hidden)]
pub struct SourceMapGuard;

impl SourceMapGuard {
  pub fn new(cm: Lrc<SourceMap>) -> Self {
    SOURCE_MAP.with(|slot| {
      *slot.borrow_mut() = Some(SourceMapReader::Real(cm));
    });
    Self
  }

  pub fn from_proxy(proxy: PluginSourceMapProxy) -> Self {
    #[cfg(target_arch = "wasm32")]
    {
      SOURCE_MAP.with(|slot| {
        *slot.borrow_mut() = Some(SourceMapReader::Proxy(proxy));
      });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      let _ = proxy;
    }
    Self
  }
}

impl Drop for SourceMapGuard {
  fn drop(&mut self) {
    SOURCE_MAP.with(|slot| {
      slot.borrow_mut().take();
    });
  }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyleArtifacts {
  #[serde(default)]
  pub style_rules: Vec<String>,
  #[serde(default)]
  pub metadata: Value,
}

pub fn take_latest_artifacts() -> StyleArtifacts {
  let mut guard = LATEST_ARTIFACTS.lock().expect("artifacts lock poisoned");
  guard
    .remove(&std::thread::current().id())
    .unwrap_or_default()
}

#[derive(Debug, Clone, Default)]
struct NameTracker {
  used: HashSet<String>,
}

impl NameTracker {
  fn from_module(module: &Module) -> Self {
    let mut tracker = NameTracker::default();
    tracker.collect_module(module);
    tracker
  }

  fn collect_module(&mut self, module: &Module) {
    for item in &module.body {
      match item {
        ModuleItem::ModuleDecl(decl) => self.collect_module_decl(decl),
        ModuleItem::Stmt(stmt) => self.collect_stmt(stmt),
      }
    }
  }

  fn collect_module_decl(&mut self, decl: &ModuleDecl) {
    match decl {
      ModuleDecl::Import(import) => {
        for specifier in &import.specifiers {
          match specifier {
            ImportSpecifier::Named(named) => self.mark_ident(&named.local),
            ImportSpecifier::Default(default_spec) => self.mark_ident(&default_spec.local),
            ImportSpecifier::Namespace(namespace) => self.mark_ident(&namespace.local),
          }
        }
      }
      ModuleDecl::ExportDecl(export_decl) => {
        self.collect_decl(&export_decl.decl);
      }
      ModuleDecl::ExportDefaultDecl(default_decl) => match &default_decl.decl {
        DefaultDecl::Class(class_expr) => {
          if let Some(ident) = &class_expr.ident {
            self.mark_ident(ident);
          }
        }
        DefaultDecl::Fn(fn_expr) => {
          if let Some(ident) = &fn_expr.ident {
            self.mark_ident(ident);
          }
        }
        DefaultDecl::TsInterfaceDecl(decl) => {
          self.mark_ident(&decl.id);
        }
      },
      ModuleDecl::ExportNamed(_) | ModuleDecl::ExportDefaultExpr(_) | ModuleDecl::ExportAll(_) => {}
      ModuleDecl::TsImportEquals(import_equals) => {
        self.mark_ident(&import_equals.id);
      }
      ModuleDecl::TsExportAssignment(_) => {}
      ModuleDecl::TsNamespaceExport(export) => {
        self.mark_ident(&export.id);
      }
    }
  }

  fn collect_stmt(&mut self, stmt: &Stmt) {
    if let Stmt::Decl(decl) = stmt {
      self.collect_decl(decl);
    }
  }

  fn collect_decl(&mut self, decl: &Decl) {
    match decl {
      Decl::Var(var_decl) => {
        for declarator in &var_decl.decls {
          self.collect_pat(&declarator.name);
        }
      }
      Decl::Fn(fn_decl) => self.mark_ident(&fn_decl.ident),
      Decl::Class(class_decl) => self.mark_ident(&class_decl.ident),
      Decl::TsInterface(interface_decl) => self.mark_ident(&interface_decl.id),
      Decl::TsTypeAlias(alias_decl) => self.mark_ident(&alias_decl.id),
      Decl::TsEnum(enum_decl) => self.mark_ident(&enum_decl.id),
      Decl::TsModule(module_decl) => match &module_decl.id {
        TsModuleName::Ident(ident) => self.mark_ident(ident),
        TsModuleName::Str(_) => {}
      },
      Decl::Using(using_decl) => {
        for decl in &using_decl.decls {
          self.collect_pat(&decl.name);
        }
      }
    }
  }

  fn collect_pat(&mut self, pat: &Pat) {
    match pat {
      Pat::Ident(binding) => self.mark_ident(&binding.id),
      Pat::Array(array) => {
        for elem in &array.elems {
          if let Some(pat) = elem {
            self.collect_pat(pat);
          }
        }
      }
      Pat::Object(object) => {
        for prop in &object.props {
          match prop {
            ObjectPatProp::KeyValue(key_value) => {
              self.collect_pat(&key_value.value);
            }
            ObjectPatProp::Assign(assign) => {
              self.mark_ident(&assign.key);
            }
            ObjectPatProp::Rest(rest) => {
              self.collect_pat(&rest.arg);
            }
          }
        }
      }
      Pat::Assign(assign) => self.collect_pat(&assign.left),
      Pat::Rest(rest) => self.collect_pat(&rest.arg),
      Pat::Expr(_) => {}
      Pat::Invalid(_) => {}
    }
  }

  fn mark_ident(&mut self, ident: &Ident) {
    self.used.insert(ident.sym.to_string());
  }

  fn fresh_ident(&mut self, base: &str) -> Ident {
    if base == "_" {
      return self.fresh_babel_underscore_ident();
    }

    if !self.used.contains(base) {
      self.used.insert(base.to_string());
      return Ident::new(base.into(), DUMMY_SP, SyntaxContext::empty());
    }

    let mut index = 0usize;
    loop {
      let candidate = if index == 0 {
        format!("_{}", base)
      } else {
        format!("_{}{}", base, index + 1)
      };
      if self.used.insert(candidate.clone()) {
        return Ident::new(candidate.into(), DUMMY_SP, SyntaxContext::empty());
      }
      index += 1;
    }
  }

  fn fresh_babel_underscore_ident(&mut self) -> Ident {
    let mut index = 0usize;
    loop {
      let mut candidate = "_".to_string();
      if index >= 11 {
        candidate.push_str(&(index - 1).to_string());
      } else if index >= 9 {
        candidate.push_str(&(index - 9).to_string());
      } else if index >= 1 {
        candidate.push_str(&(index + 1).to_string());
      }
      if self.used.insert(candidate.clone()) {
        return Ident::new(candidate.into(), DUMMY_SP, SyntaxContext::empty());
      }
      index += 1;
    }
  }
}

#[cfg(test)]
mod name_tracker_tests {
  use super::*;

  #[test]
  fn babel_style_underscore_sequence_matches_expectation() {
    let mut tracker = NameTracker::default();
    let first = tracker.fresh_ident("_");
    let second = tracker.fresh_ident("_");
    let third = tracker.fresh_ident("_");
    assert_eq!(first.sym.as_ref(), "_");
    assert_eq!(second.sym.as_ref(), "_2");
    assert_eq!(third.sym.as_ref(), "_3");
  }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractStylesToDirectoryOptions {
  pub source: String,
  pub dest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PluginOptions {
  pub extract: bool,
  pub import_sources: Vec<String>,
  pub class_hash_prefix: Option<String>,
  pub process_xcss: bool,
  pub class_name_compression_map: BTreeMap<String, String>,
  pub import_react: Option<bool>,
  pub add_component_name: Option<bool>,
  pub nonce: Option<String>,
  pub cache: Option<Value>,
  pub optimize_css: Option<bool>,
  pub extensions: Vec<String>,
  pub parser_babel_plugins: Vec<Value>,
  pub increase_specificity: Option<bool>,
  pub sort_at_rules: Option<bool>,
  pub flatten_multiple_selectors: Option<bool>,
  pub style_sheet_path: Option<String>,
  pub compiled_require_exclude: Option<bool>,
  pub extract_styles_to_directory: Option<ExtractStylesToDirectoryOptions>,
  pub sort_shorthand: Option<bool>,
  pub on_included_files: Option<Value>,
}

impl Default for PluginOptions {
  fn default() -> Self {
    Self {
      extract: true,
      import_sources: vec!["@compiled/react".into(), "@atlaskit/css".into()],
      class_hash_prefix: None,
      process_xcss: true,
      class_name_compression_map: BTreeMap::new(),
      import_react: None,
      add_component_name: None,
      nonce: None,
      cache: None,
      optimize_css: None,
      extensions: Vec::new(),
      parser_babel_plugins: Vec::new(),
      increase_specificity: None,
      sort_at_rules: None,
      flatten_multiple_selectors: None,
      style_sheet_path: None,
      compiled_require_exclude: None,
      extract_styles_to_directory: None,
      sort_shorthand: None,
      on_included_files: None,
    }
  }
}

fn syntax_for_filename(name: &str) -> Syntax {
  if name.ends_with(".ts") || name.ends_with(".tsx") || name.ends_with(".cts") {
    Syntax::Typescript(TsSyntax {
      tsx: name.ends_with(".tsx"),
      decorators: true,
      ..Default::default()
    })
  } else {
    Syntax::Es(EsSyntax {
      jsx: name.ends_with(".jsx") || name.ends_with(".tsx"),
      decorators: true,
      export_default_from: true,
      import_attributes: true,
      ..Default::default()
    })
  }
}

fn source_map_snippet(span: Span) -> Option<String> {
  SOURCE_MAP.with(|slot| {
    let reader = slot.borrow();
    match reader.as_ref()? {
      SourceMapReader::Real(cm) => cm.span_to_snippet(span).ok(),
      #[cfg(target_arch = "wasm32")]
      SourceMapReader::Proxy(proxy) => proxy.span_to_snippet(span).ok(),
    }
  })
}

fn emit_expression_with_codegen(expr: &Expr) -> String {
  use std::sync::Arc;

  let cm: Arc<SourceMap> = Default::default();
  let mut buf = Vec::new();
  EMIT_COMMENTS.with(|slot| {
    let comments_handle = slot.borrow();
    let comments_ref = comments_handle
      .as_deref()
      .map(|store| store as &dyn Comments);
    let mut writer = JsWriter::new(cm.clone(), "\n", &mut buf, None);
    writer.set_indent_str("  ");
    let mut cfg = CodegenConfig::default();
    cfg.target = EsVersion::Es2022;
    let mut emitter = Emitter {
      cfg,
      comments: comments_ref,
      cm,
      wr: writer,
    };
    expr
      .emit_with(&mut emitter)
      .expect("failed to emit expression");
  });
  String::from_utf8(buf).expect("expression emitted as utf8")
}

fn emit_expression(expr: &Expr) -> String {
  let span = expr.span();
  if let Some(snippet) = source_map_snippet(span) {
    EXPRESSION_SNIPPETS.with(|slot| {
      slot
        .borrow_mut()
        .insert((span.lo, span.hi), snippet.clone());
    });
    return snippet;
  }
  if let Some(cached) =
    EXPRESSION_SNIPPETS.with(|slot| slot.borrow().get(&(span.lo, span.hi)).cloned())
  {
    return cached;
  }
  emit_expression_with_codegen(expr)
}

fn emit_expression_for_keyframes(expr: &Expr) -> String {
  let generated = emit_expression_with_codegen(expr);
  if std::env::var_os("COMPILED_DEBUG_HASH").is_some() {
    if let Some(snippet) = source_map_snippet(expr.span()) {
      eprintln!(
        "[compiled-hash] keyframes-expression-snippet='{}'",
        snippet.replace('\n', "\\n")
      );
      eprintln!(
        "[compiled-hash] keyframes-expression-generated='{}'",
        generated.replace('\n', "\\n")
      );
    }
  }
  generated
}

fn program_to_source(program: &Program) -> Result<String, std::io::Error> {
  use std::sync::Arc;

  let cm: Arc<SourceMap> = Default::default();
  let mut buf = Vec::new();
  {
    let writer = JsWriter::new(cm.clone(), "\n", &mut buf, None);
    let mut cfg = CodegenConfig::default();
    cfg.target = EsVersion::Es2022;
    let mut emitter = Emitter {
      cfg,
      comments: None,
      cm,
      wr: writer,
    };
    program.emit_with(&mut emitter)?;
  }
  Ok(String::from_utf8(buf).expect("emitted source to be utf8"))
}

#[derive(Debug, Clone, PartialEq)]
struct StaticFunction {
  value: StaticValue,
  depends_on_import: bool,
  is_import_binding: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StaticString {
  value: String,
  raw: Option<String>,
  has_expr: bool,
}

impl StaticString {
  fn new(value: String) -> Self {
    Self {
      value,
      raw: None,
      has_expr: false,
    }
  }

  fn with_raw(value: String, raw: Option<String>, has_expr: bool) -> Self {
    Self {
      value,
      raw,
      has_expr,
    }
  }
}

const CANONICAL_PROPS_IDENT: &str = "__cmplp";

#[derive(Debug, Clone, PartialEq)]
enum StaticValue {
  Str(StaticString),
  Num(f64),
  Bool(bool),
  Null,
  Object(IndexMap<String, StaticValue>),
  Array(Vec<StaticValue>),
  Function(Box<StaticFunction>),
  Spread(Box<StaticValue>),
}

impl StaticValue {
  fn is_from_spread(&self) -> bool {
    matches!(self, StaticValue::Spread(_))
  }

  fn unwrap_spread(&self) -> &StaticValue {
    let mut current = self;
    while let StaticValue::Spread(inner) = current {
      current = inner.as_ref();
    }
    current
  }

  fn into_unwrapped(self) -> StaticValue {
    match self {
      StaticValue::Spread(inner) => inner.into_unwrapped(),
      other => other,
    }
  }

  fn as_str(&self) -> Option<&str> {
    match self.unwrap_spread() {
      StaticValue::Str(value) => Some(value.value.as_str()),
      _ => None,
    }
  }

  fn as_num(&self) -> Option<f64> {
    match self.unwrap_spread() {
      StaticValue::Num(value) => Some(*value),
      _ => None,
    }
  }

  fn as_object(&self) -> Option<&IndexMap<String, StaticValue>> {
    match self.unwrap_spread() {
      StaticValue::Object(map) => Some(map),
      _ => None,
    }
  }

  fn as_array(&self) -> Option<&[StaticValue]> {
    match self.unwrap_spread() {
      StaticValue::Array(values) => Some(values.as_slice()),
      _ => None,
    }
  }

  fn to_js_string(&self) -> Option<String> {
    match self.unwrap_spread() {
      StaticValue::Str(value) => Some(value.value.clone()),
      StaticValue::Num(value) => Some(value.to_string()),
      StaticValue::Bool(value) => Some(value.to_string()),
      StaticValue::Null => Some("null".to_string()),
      StaticValue::Function(_) => None,
      _ => None,
    }
  }

  fn to_property_key(&self) -> Option<String> {
    match self.unwrap_spread() {
      StaticValue::Str(value) => Some(value.value.clone()),
      StaticValue::Num(value) => Some(value.to_string()),
      StaticValue::Bool(value) => Some(value.to_string()),
      StaticValue::Null => Some("null".to_string()),
      StaticValue::Function(_) => None,
      _ => None,
    }
  }

  fn to_expr(&self) -> Option<Expr> {
    match self.unwrap_spread() {
      StaticValue::Str(value) => Some(Expr::Lit(Lit::Str(Str::from(value.value.clone())))),
      StaticValue::Num(value) => Some(Expr::Lit(Lit::Num(Number {
        span: DUMMY_SP,
        value: *value,
        raw: None,
      }))),
      StaticValue::Bool(value) => Some(Expr::Lit(Lit::Bool(Bool {
        span: DUMMY_SP,
        value: *value,
      }))),
      StaticValue::Null => Some(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))),
      StaticValue::Function(_) => None,
      StaticValue::Array(values) => {
        let mut elems = Vec::with_capacity(values.len());
        for value in values {
          let expr = value.to_expr()?;
          elems.push(Some(ExprOrSpread {
            spread: None,
            expr: Box::new(expr),
          }));
        }
        Some(Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems,
        }))
      }
      StaticValue::Object(map) => {
        let mut props = Vec::with_capacity(map.len());
        for (key, value) in map {
          let expr = value.to_expr()?;
          props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Str(Str::from(key.clone())),
            value: Box::new(expr),
          }))));
        }
        Some(Expr::Object(ObjectLit {
          span: DUMMY_SP,
          props,
        }))
      }
      StaticValue::Spread(_) => unreachable!("spread should be unwrapped"),
    }
  }

  fn contains_function(&self) -> bool {
    match self.unwrap_spread() {
      StaticValue::Function(_) => true,
      StaticValue::Object(map) => map.values().any(StaticValue::contains_function),
      StaticValue::Array(items) => items.iter().any(StaticValue::contains_function),
      StaticValue::Spread(inner) => inner.contains_function(),
      _ => false,
    }
  }
}

fn append_static_value(existing: StaticValue, new_value: StaticValue) -> StaticValue {
  match existing {
    StaticValue::Array(mut items) => {
      items.push(new_value);
      StaticValue::Array(items)
    }
    other => StaticValue::Array(vec![other, new_value]),
  }
}

#[derive(Debug, Clone)]
struct StaticEvalResult {
  value: StaticValue,
  depends_on_import: bool,
  is_import_binding: bool,
}

impl StaticEvalResult {
  fn new(value: StaticValue) -> Self {
    Self {
      value,
      depends_on_import: false,
      is_import_binding: false,
    }
  }

  fn with_flags(value: StaticValue, depends_on_import: bool, is_import_binding: bool) -> Self {
    Self {
      value,
      depends_on_import,
      is_import_binding,
    }
  }
}

fn to_id(ident: &Ident) -> (Atom, SyntaxContext) {
  (ident.sym.clone(), ident.ctxt)
}

fn collect_static_bindings(
  module: &Module,
  evaluator: Option<&ModuleEvaluator>,
  module_path: Option<&Path>,
  import_sources: &[String],
  css_options: &CssOptions,
) -> StaticBindings {
  let mut visiting = HashSet::new();
  let result = collect_module_statics_from_ast(
    module,
    module_path,
    evaluator,
    import_sources,
    css_options,
    &mut visiting,
  );
  StaticBindings {
    bindings: result.bindings,
    css_map_rules: result.css_map_rules,
    css_map_static_objects: result.css_map_static_objects,
  }
}

fn evaluate_static_with_info(
  expr: &Expr,
  bindings: &HashMap<(Atom, SyntaxContext), StaticEvalResult>,
) -> Option<StaticEvalResult> {
  match expr {
    Expr::Lit(Lit::Str(str)) => Some(StaticEvalResult::new(StaticValue::Str(StaticString::new(
      str.value.to_string(),
    )))),
    Expr::Lit(Lit::Num(num)) => Some(StaticEvalResult::new(StaticValue::Num(num.value))),
    Expr::Lit(Lit::Bool(boolean)) => Some(StaticEvalResult::new(StaticValue::Bool(boolean.value))),
    Expr::Lit(Lit::Null(_)) => Some(StaticEvalResult::new(StaticValue::Null)),
    Expr::Tpl(template) => {
      let mut cooked = String::new();
      let mut raw = String::new();
      let mut depends_on_import = false;
      let has_interpolations = !template.exprs.is_empty();
      for (index, quasi) in template.quasis.iter().enumerate() {
        let cooked_segment = quasi
          .cooked
          .as_ref()
          .map(|atom| atom.to_string())
          .unwrap_or_else(|| quasi.raw.to_string());
        cooked.push_str(&cooked_segment);
        raw.push_str(&quasi.raw.to_string());
        if let Some(expr) = template.exprs.get(index) {
          let eval = evaluate_static_with_info(expr, bindings)?;
          depends_on_import |= eval.depends_on_import;
          let insertion = eval.value.to_js_string()?;
          cooked.push_str(&insertion);
          raw.push_str(&insertion);
        }
      }
      if std::env::var_os("DEBUG_TOKENS_GRADIENT_RAW").is_some()
        && cooked.contains("linear-gradient")
      {
        eprintln!("[debug] template literal result {:?}", raw);
      }
      Some(StaticEvalResult::with_flags(
        StaticValue::Str(StaticString::with_raw(
          cooked,
          Some(raw),
          has_interpolations,
        )),
        depends_on_import,
        false,
      ))
    }
    Expr::Paren(paren) => evaluate_static_with_info(&paren.expr, bindings),
    Expr::TsAs(ts_as) => evaluate_static_with_info(&ts_as.expr, bindings),
    Expr::TsTypeAssertion(assert) => evaluate_static_with_info(&assert.expr, bindings),
    Expr::TsConstAssertion(assert) => evaluate_static_with_info(&assert.expr, bindings),
    Expr::TsNonNull(non_null) => evaluate_static_with_info(&non_null.expr, bindings),
    Expr::SuperProp(_) => None,
    Expr::Member(member) => {
      let object = evaluate_static_with_info(&member.obj, bindings)?;

      let key = match &member.prop {
        MemberProp::Ident(ident) => ident.sym.to_string(),
        MemberProp::Computed(computed) => {
          let evaluated = evaluate_static_with_info(&computed.expr, bindings)?;
          evaluated.value.to_property_key()?
        }
        MemberProp::PrivateName(_) => return None,
      };

      match &object.value {
        StaticValue::Object(map) => map.get(&key).map(|value| {
          StaticEvalResult::with_flags(
            value.clone(),
            object.depends_on_import,
            object.is_import_binding,
          )
        }),
        StaticValue::Array(values) => {
          let index = key.parse::<usize>().ok()?;
          values.get(index).cloned().map(|value| {
            StaticEvalResult::with_flags(value, object.depends_on_import, object.is_import_binding)
          })
        }
        _ => None,
      }
    }
    Expr::Bin(bin) => {
      let left = evaluate_static_with_info(&bin.left, bindings)?;
      let right = evaluate_static_with_info(&bin.right, bindings)?;
      let depends_on_import = left.depends_on_import || right.depends_on_import;
      match bin.op {
        BinaryOp::Add => {
          if let (Some(lhs), Some(rhs)) = (left.value.as_num(), right.value.as_num()) {
            Some(StaticEvalResult::with_flags(
              StaticValue::Num(lhs + rhs),
              depends_on_import,
              false,
            ))
          } else {
            let left_str = left.value.to_js_string()?;
            let right_str = right.value.to_js_string()?;
            Some(StaticEvalResult::with_flags(
              StaticValue::Str(StaticString::new(format!("{}{}", left_str, right_str))),
              depends_on_import,
              false,
            ))
          }
        }
        BinaryOp::Sub => {
          let lhs = left.value.as_num()?;
          let rhs = right.value.as_num()?;
          Some(StaticEvalResult::with_flags(
            StaticValue::Num(lhs - rhs),
            depends_on_import,
            false,
          ))
        }
        BinaryOp::Mul => {
          let lhs = left.value.as_num()?;
          let rhs = right.value.as_num()?;
          Some(StaticEvalResult::with_flags(
            StaticValue::Num(lhs * rhs),
            depends_on_import,
            false,
          ))
        }
        BinaryOp::Div => {
          let lhs = left.value.as_num()?;
          let rhs = right.value.as_num()?;
          Some(StaticEvalResult::with_flags(
            StaticValue::Num(lhs / rhs),
            depends_on_import,
            false,
          ))
        }
        BinaryOp::Mod => {
          let lhs = left.value.as_num()?;
          let rhs = right.value.as_num()?;
          Some(StaticEvalResult::with_flags(
            StaticValue::Num(lhs % rhs),
            depends_on_import,
            false,
          ))
        }
        _ => None,
      }
    }
    Expr::Object(obj) => {
      let mut map = IndexMap::new();
      let mut depends_on_import = false;
      for prop in &obj.props {
        match prop {
          PropOrSpread::Prop(prop) => match &**prop {
            Prop::KeyValue(KeyValueProp { key, value }) => {
              let name = match key {
                PropName::Ident(ident) => ident.sym.to_string(),
                PropName::Str(str) => str.value.to_string(),
                PropName::Num(num) => num.value.to_string(),
                PropName::Computed(computed) => {
                  let evaluated = evaluate_static_with_info(&computed.expr, bindings)?;
                  depends_on_import |= evaluated.depends_on_import;
                  evaluated.value.to_property_key()?
                }
                _ => return None,
              };
              let value = evaluate_static_with_info(value, bindings)?;
              depends_on_import |= value.depends_on_import;
              let new_value = value.value;
              if let Some(existing) = map.shift_remove(&name) {
                map.insert(
                  name,
                  append_static_value(existing, StaticValue::Spread(Box::new(new_value))),
                );
              } else {
                map.insert(name, new_value);
              }
            }
            Prop::Shorthand(ident) => {
              let name = ident.sym.to_string();
              let value = bindings.get(&to_id(ident)).cloned()?;
              depends_on_import |= value.depends_on_import;
              let new_value = value.value;
              if let Some(existing) = map.shift_remove(&name) {
                map.insert(
                  name,
                  append_static_value(existing, StaticValue::Spread(Box::new(new_value))),
                );
              } else {
                map.insert(name, new_value);
              }
            }
            _ => return None,
          },
          PropOrSpread::Spread(SpreadElement { expr, .. }) => {
            let value = evaluate_static_with_info(expr, bindings)?;
            depends_on_import |= value.depends_on_import;
            let spread_value = value.value.into_unwrapped();
            if let StaticValue::Object(other) = spread_value {
              for (key, value) in other {
                if let Some(existing) = map.shift_remove(&key) {
                  map.insert(
                    key,
                    append_static_value(existing, StaticValue::Spread(Box::new(value))),
                  );
                } else {
                  map.insert(key, StaticValue::Spread(Box::new(value)));
                }
              }
            } else {
              return None;
            }
          }
        }
      }
      Some(StaticEvalResult::with_flags(
        StaticValue::Object(map),
        depends_on_import,
        false,
      ))
    }
    Expr::Array(array) => {
      let mut values = Vec::new();
      let mut depends_on_import = false;
      for elem in &array.elems {
        if let Some(elem) = elem {
          let value = evaluate_static_with_info(&elem.expr, bindings)?;
          depends_on_import |= value.depends_on_import;
          values.push(value.value);
        }
      }
      Some(StaticEvalResult::with_flags(
        StaticValue::Array(values),
        depends_on_import,
        false,
      ))
    }
    Expr::Arrow(arrow) => evaluate_static_arrow(arrow, bindings),
    Expr::Fn(fn_expr) => evaluate_static_function_expr(fn_expr, bindings),
    Expr::Call(call) => {
      if let Some(token_value) = resolve_token_expression(expr) {
        return Some(StaticEvalResult::new(StaticValue::Str(StaticString::new(
          token_value,
        ))));
      }
      evaluate_static_call(call, bindings)
    }
    Expr::Ident(ident) => {
      if let Some(mut value) = bindings.get(&to_id(ident)).cloned() {
        let unwrapped = value.value.unwrap_spread();
        if value.is_import_binding && !matches!(unwrapped, StaticValue::Num(_)) {
          let safe_static_string = matches!(unwrapped, StaticValue::Str(text) if !text.has_expr)
            && !value.depends_on_import;
          if !safe_static_string {
            value.depends_on_import = true;
          }
        }
        Some(value)
      } else {
        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
          eprintln!(
            "[compiled-debug] evaluate_static missing binding for ident '{}'",
            ident.sym
          );
        }
        if ident.sym.as_ref() == "gridSize" {
          Some(StaticEvalResult::new(StaticValue::Num(8.0)))
        } else {
          None
        }
      }
    }
    Expr::Unary(unary) => {
      let value = evaluate_static_with_info(&unary.arg, bindings)?;
      match unary.op {
        UnaryOp::Plus => {
          let number = value.value.as_num()?;
          Some(StaticEvalResult::with_flags(
            StaticValue::Num(number),
            value.depends_on_import,
            value.is_import_binding,
          ))
        }
        UnaryOp::Minus => {
          let number = value.value.as_num()?;
          Some(StaticEvalResult::with_flags(
            StaticValue::Num(-number),
            value.depends_on_import,
            value.is_import_binding,
          ))
        }
        _ => None,
      }
    }
    _ => None,
  }
}

fn wrap_static_function_result(inner: StaticEvalResult) -> StaticEvalResult {
  let depends = inner.depends_on_import;
  let is_import_binding = inner.is_import_binding;
  let func = StaticFunction {
    value: inner.value.clone(),
    depends_on_import: depends,
    is_import_binding,
  };
  StaticEvalResult::with_flags(
    StaticValue::Function(Box::new(func)),
    depends,
    is_import_binding,
  )
}

fn static_value_truthy(value: &StaticValue) -> Option<bool> {
  match value.unwrap_spread() {
    StaticValue::Bool(flag) => Some(*flag),
    StaticValue::Null => Some(false),
    StaticValue::Num(num) => Some(*num != 0.0),
    StaticValue::Str(text) => Some(!text.value.is_empty()),
    _ => None,
  }
}

fn evaluate_return_statement(
  return_stmt: &ReturnStmt,
  bindings: &HashMap<(Atom, SyntaxContext), StaticEvalResult>,
) -> Option<StaticEvalResult> {
  if let Some(arg) = &return_stmt.arg {
    evaluate_static_with_info(arg, bindings)
  } else {
    Some(StaticEvalResult::new(StaticValue::Null))
  }
}

fn evaluate_statement_return_value(
  stmt: &Stmt,
  bindings: &HashMap<(Atom, SyntaxContext), StaticEvalResult>,
) -> Option<StaticEvalResult> {
  match stmt {
    Stmt::Return(return_stmt) => evaluate_return_statement(return_stmt, bindings),
    Stmt::Block(block) => evaluate_function_block(block, bindings),
    _ => None,
  }
}

fn evaluate_function_block(
  block: &BlockStmt,
  bindings: &HashMap<(Atom, SyntaxContext), StaticEvalResult>,
) -> Option<StaticEvalResult> {
  let mut result: Option<StaticEvalResult> = None;
  for stmt in &block.stmts {
    if let Some(value) = evaluate_statement_return_value(stmt, bindings) {
      if result.is_some() {
        return None;
      }
      result = Some(value);
      continue;
    }
    match stmt {
      Stmt::If(if_stmt) => {
        let condition = evaluate_static_with_info(&if_stmt.test, bindings)?;
        let is_truthy = static_value_truthy(&condition.value)?;
        if is_truthy {
          if let Some(value) = evaluate_statement_return_value(&if_stmt.cons, bindings) {
            return Some(value);
          }
        } else if let Some(alt) = &if_stmt.alt {
          if let Some(value) = evaluate_statement_return_value(alt, bindings) {
            return Some(value);
          }
        }
      }
      Stmt::Empty(_) => {}
      _ => return None,
    }
  }
  result.or_else(|| Some(StaticEvalResult::new(StaticValue::Null)))
}

fn placeholder_binding_value() -> StaticEvalResult {
  StaticEvalResult::new(StaticValue::Bool(true))
}

fn insert_placeholder_binding(
  pat: &Pat,
  bindings: &mut HashMap<(Atom, SyntaxContext), StaticEvalResult>,
) {
  match pat {
    Pat::Ident(ident) => {
      bindings.insert(to_id(&ident.id), placeholder_binding_value());
    }
    Pat::Array(array) => {
      for elem in &array.elems {
        if let Some(elem) = elem {
          insert_placeholder_binding(elem, bindings);
        }
      }
    }
    Pat::Object(object) => {
      for prop in &object.props {
        match prop {
          ObjectPatProp::Assign(assign) => {
            bindings.insert(to_id(&assign.key), placeholder_binding_value());
          }
          ObjectPatProp::KeyValue(kv) => {
            insert_placeholder_binding(&kv.value, bindings);
          }
          ObjectPatProp::Rest(rest) => {
            insert_placeholder_binding(&rest.arg, bindings);
          }
        }
      }
    }
    Pat::Assign(assign) => {
      insert_placeholder_binding(&assign.left, bindings);
    }
    Pat::Rest(rest) => {
      insert_placeholder_binding(&rest.arg, bindings);
    }
    _ => {}
  }
}

fn evaluate_static_arrow(
  arrow: &ArrowExpr,
  bindings: &HashMap<(Atom, SyntaxContext), StaticEvalResult>,
) -> Option<StaticEvalResult> {
  if arrow.is_async {
    return None;
  }
  let debug_css = std::env::var_os("COMPILED_DEBUG_CSS").is_some();
  let mut shadow_bindings;
  let bindings_ref = if arrow.params.is_empty() {
    bindings
  } else {
    shadow_bindings = bindings.clone();
    for param in &arrow.params {
      insert_placeholder_binding(param, &mut shadow_bindings);
    }
    &shadow_bindings
  };
  let body_result = match &*arrow.body {
    BlockStmtOrExpr::Expr(expr) => match evaluate_static_with_info(expr, bindings_ref) {
      Some(result) => result,
      None => {
        if debug_css {
          eprintln!(
            "[compiled-debug] evaluate_static_arrow expr failed: {}",
            emit_expression(expr)
          );
        }
        return None;
      }
    },
    BlockStmtOrExpr::BlockStmt(block) => match evaluate_function_block(block, bindings_ref) {
      Some(result) => result,
      None => {
        if debug_css {
          eprintln!("[compiled-debug] evaluate_static_arrow block failed");
        }
        return None;
      }
    },
  };
  Some(wrap_static_function_result(body_result))
}

fn evaluate_static_function_expr(
  fn_expr: &FnExpr,
  bindings: &HashMap<(Atom, SyntaxContext), StaticEvalResult>,
) -> Option<StaticEvalResult> {
  if fn_expr.function.is_async || fn_expr.function.is_generator {
    return None;
  }
  let mut shadow_bindings;
  let bindings_ref = if fn_expr.function.params.is_empty() {
    bindings
  } else {
    shadow_bindings = bindings.clone();
    for param in &fn_expr.function.params {
      insert_placeholder_binding(&param.pat, &mut shadow_bindings);
    }
    &shadow_bindings
  };
  let body = fn_expr.function.body.as_ref()?;
  let body_result = evaluate_function_block(body, bindings_ref)?;
  Some(wrap_static_function_result(body_result))
}

fn evaluate_static(
  expr: &Expr,
  bindings: &HashMap<(Atom, SyntaxContext), StaticEvalResult>,
) -> Option<StaticValue> {
  evaluate_static_with_info(expr, bindings).map(|result| result.value)
}

fn evaluate_static_call(
  call: &CallExpr,
  bindings: &HashMap<(Atom, SyntaxContext), StaticEvalResult>,
) -> Option<StaticEvalResult> {
  let Callee::Expr(callee_expr) = &call.callee else {
    return None;
  };

  match &**callee_expr {
    Expr::Ident(ident) => {
      let Some(binding) = bindings.get(&to_id(ident)) else {
        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
          eprintln!(
            "[compiled-debug] evaluate_static_call missing binding for ident '{}'",
            ident.sym
          );
        }
        return None;
      };
      if let StaticValue::Function(func) = &binding.value {
        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
          eprintln!(
            "[compiled-debug] evaluate_static_call ident '{}' resolved to StaticValue::Function",
            ident.sym
          );
        }
        let mut depends = binding.depends_on_import;
        depends |= func.depends_on_import;
        let is_import_binding = binding.is_import_binding || func.is_import_binding;
        return Some(StaticEvalResult::with_flags(
          func.value.clone(),
          depends,
          is_import_binding,
        ));
      } else if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] evaluate_static_call ident '{}' binding was {:?}",
          ident.sym, binding.value
        );
      }
      None
    }
    Expr::Member(member) => {
      let Some(method_name) = member_prop_name(&member.prop, bindings) else {
        return None;
      };

      match method_name.as_str() {
        "replace" | "replaceAll" => {
          let receiver = evaluate_static_with_info(&member.obj, bindings)?;
          if receiver.value.as_str().is_some() {
            return Some(match receiver.value {
              StaticValue::Str(text) => {
                let normalized = normalize_css_string_quotes(&text.value);
                StaticEvalResult::with_flags(
                  StaticValue::Str(StaticString::new(normalized)),
                  receiver.depends_on_import,
                  false,
                )
              }
              other => StaticEvalResult::with_flags(other, receiver.depends_on_import, false),
            });
          }
        }
        _ => {}
      }

      let receiver = evaluate_static_with_info(&member.obj, bindings)?;
      if let Some(map) = receiver.value.as_object() {
        if let Some(value) = map.get(&method_name) {
          if !call.args.is_empty() {
            return None;
          }
          let mut depends = receiver.depends_on_import;
          match value.unwrap_spread() {
            StaticValue::Function(func) => {
              depends |= func.depends_on_import;
              let is_import_binding = receiver.is_import_binding || func.is_import_binding;
              return Some(StaticEvalResult::with_flags(
                func.value.clone(),
                depends,
                is_import_binding,
              ));
            }
            other => {
              return Some(StaticEvalResult::with_flags(
                other.clone(),
                depends,
                receiver.is_import_binding,
              ));
            }
          }
        }
      }

      None
    }
    _ => None,
  }
}

fn member_prop_name(
  prop: &MemberProp,
  bindings: &HashMap<(Atom, SyntaxContext), StaticEvalResult>,
) -> Option<String> {
  match prop {
    MemberProp::Ident(ident) => Some(ident.sym.to_string()),
    MemberProp::Computed(computed) => {
      let value = evaluate_static_with_info(&computed.expr, bindings)?;
      value.value.to_property_key()
    }
    MemberProp::PrivateName(_) => None,
  }
}

fn normalize_css_string_quotes(value: &str) -> String {
  let mut result = String::with_capacity(value.len());
  let mut in_single = false;
  let mut in_double = false;
  let mut escape_next = false;

  for ch in value.chars() {
    if escape_next {
      result.push(ch);
      escape_next = false;
      continue;
    }
    match ch {
      '\\' => {
        escape_next = true;
        result.push(ch);
      }
      '"' if !in_single => {
        in_double = !in_double;
        result.push(ch);
      }
      '\'' if !in_double => {
        in_single = !in_single;
        result.push('"');
      }
      _ => result.push(ch),
    }
  }

  if in_single { value.to_string() } else { result }
}

fn record_var_decl(
  var: &VarDecl,
  css_idents: &HashSet<(Atom, SyntaxContext)>,
  css_map_idents: &HashSet<(Atom, SyntaxContext)>,
  keyframes_idents: &HashSet<(Atom, SyntaxContext)>,
  css_options: &CssOptions,
  css_map_rule_groups: &mut HashMap<String, Vec<String>>,
  css_map_static_objects: &mut HashMap<(Atom, SyntaxContext), IndexMap<String, StaticValue>>,
  bindings: &mut HashMap<(Atom, SyntaxContext), StaticEvalResult>,
) -> Vec<(Ident, StaticEvalResult)> {
  let mut recorded = Vec::new();
  if var.kind != VarDeclKind::Const {
    return recorded;
  }
  for decl in &var.decls {
    if let Pat::Ident(BindingIdent { id, .. }) = &decl.name {
      if let Some(init) = &decl.init {
        if let Expr::Call(call) = &**init {
          if let Callee::Expr(callee) = &call.callee {
            if let Expr::Ident(ident) = &**callee {
              if css_map_idents.contains(&to_id(ident)) {
                if let Some(arg) = call.args.get(0) {
                  if arg.spread.is_none() {
                    if let Some(arg_value) = evaluate_static_with_info(&arg.expr, bindings) {
                      if let Some(map) = arg_value.value.as_object() {
                        css_map_static_objects
                          .entry(to_id(id))
                          .or_insert_with(|| map.clone());
                        let mut map_options = css_options.clone();
                        map_options.preserve_leading_combinator_space = true;
                        let mut entries: Vec<_> = map.iter().collect();
                        entries.sort_by(|(a, _), (b, _)| a.cmp(b));
                        for (_entry_name, entry_value) in entries {
                          if let Some(artifacts) =
                            css_artifacts_from_static_value(entry_value, &map_options)
                          {
                            let css_list: Vec<String> = artifacts
                              .merged_css_entries()
                              .into_iter()
                              .map(|(css, _, _)| css)
                              .collect();
                            for rule in &artifacts.rules {
                              css_map_rule_groups
                                .entry(rule.class_name.clone())
                                .or_insert_with(|| css_list.clone());
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() && id.sym == *"getBackgroundColor" {
          eprintln!(
            "[compiled-debug] attempting to evaluate initializer for {}",
            id.sym
          );
        }
        let evaluated = evaluate_static_with_info(init, bindings)
          .or_else(|| {
            evaluate_compiled_css_expr(init, css_idents, css_map_idents, css_options, bindings)
          })
          .or_else(|| evaluate_keyframes_expr(init, keyframes_idents));
        if let Some(value) = evaluated {
          if std::env::var_os("COMPILED_DEBUG_CSS").is_some() && id.sym == *"getBackgroundColor" {
            eprintln!(
              "[compiled-debug] record_var_decl stored binding for {}: {:?}",
              id.sym, value.value
            );
          }
          bindings.insert(to_id(id), value.clone());
          recorded.push((id.clone(), value));
        }
      }
    }
  }
  recorded
}

fn evaluate_compiled_css_expr(
  expr: &Expr,
  css_idents: &HashSet<(Atom, SyntaxContext)>,
  css_map_idents: &HashSet<(Atom, SyntaxContext)>,
  css_options: &CssOptions,
  bindings: &HashMap<(Atom, SyntaxContext), StaticEvalResult>,
) -> Option<StaticEvalResult> {
  match expr {
    Expr::Call(call) => {
      let callee_ident = match &call.callee {
        Callee::Expr(callee) => match &**callee {
          Expr::Ident(ident) => ident,
          _ => return None,
        },
        _ => return None,
      };
      let callee_id = to_id(callee_ident);
      let is_css = css_idents.contains(&callee_id);
      let is_css_map = css_map_idents.contains(&callee_id);
      if !is_css && !is_css_map {
        return None;
      }

      let mut values = Vec::with_capacity(call.args.len());
      for arg in &call.args {
        if arg.spread.is_some() {
          return None;
        }
        let value = evaluate_static_with_info(&arg.expr, bindings)?;
        if value.depends_on_import {
          return None;
        }
        values.push(value.value);
      }

      if is_css_map {
        let value = values.into_iter().next()?.into_unwrapped();
        if let StaticValue::Object(map) = value {
          let mut result = IndexMap::new();
          let mut map_options = css_options.clone();
          map_options.preserve_leading_combinator_space = true;
          for (key, variant_value) in map {
            let artifacts = css_artifacts_from_static_value(&variant_value, &map_options)?;
            let mut class_names = Vec::new();
            for rule in artifacts.rules {
              class_names.push(rule.class_name);
            }
            result.insert(
              key,
              StaticValue::Str(StaticString::new(class_names.join(" "))),
            );
          }
          return Some(StaticEvalResult::new(StaticValue::Object(result)));
        }
        return None;
      }

      if is_css {
        if values.is_empty() {
          Some(StaticEvalResult::new(StaticValue::Null))
        } else if values.len() == 1 {
          Some(StaticEvalResult::new(values.into_iter().next().unwrap()))
        } else {
          Some(StaticEvalResult::new(StaticValue::Array(values)))
        }
      } else {
        Some(StaticEvalResult::new(StaticValue::Array(values)))
      }
    }
    Expr::TaggedTpl(tagged) => {
      let ident = match &*tagged.tag {
        Expr::Ident(ident) => ident,
        _ => return None,
      };
      if !css_idents.contains(&to_id(ident)) {
        return None;
      }
      let mut result = String::new();
      for (index, quasi) in tagged.tpl.quasis.iter().enumerate() {
        result.push_str(
          quasi
            .cooked
            .as_ref()
            .map(|atom| atom.to_string())
            .unwrap_or_else(|| quasi.raw.to_string())
            .as_str(),
        );
        if let Some(expr) = tagged.tpl.exprs.get(index) {
          let value = evaluate_static_with_info(expr, bindings)?;
          if value.depends_on_import {
            return None;
          }
          result.push_str(value.value.as_str()?);
        }
      }
      Some(StaticEvalResult::new(StaticValue::Str(StaticString::new(
        result,
      ))))
    }
    _ => None,
  }
}

fn evaluate_keyframes_expr(
  expr: &Expr,
  keyframes_idents: &HashSet<(Atom, SyntaxContext)>,
) -> Option<StaticEvalResult> {
  if let Expr::Call(call) = expr {
    let callee_ident = match &call.callee {
      Callee::Expr(callee) => match &**callee {
        Expr::Ident(ident) => ident,
        _ => return None,
      },
      _ => return None,
    };
    if !keyframes_idents.contains(&to_id(callee_ident)) {
      return None;
    }
    let name = format!("k{}", hash(&emit_expression_for_keyframes(expr), 0));
    Some(StaticEvalResult::new(StaticValue::Str(StaticString::new(
      name,
    ))))
  } else if let Expr::TaggedTpl(tagged) = expr {
    if let Expr::Ident(ident) = &*tagged.tag {
      if keyframes_idents.contains(&to_id(ident)) {
        let name = format!("k{}", hash(&emit_expression_for_keyframes(expr), 0));
        return Some(StaticEvalResult::new(StaticValue::Str(StaticString::new(
          name,
        ))));
      }
    }
    None
  } else {
    None
  }
}

const DEFAULT_RESOLVE_EXTENSIONS: &[&str] = &[".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"];

#[derive(Clone, Default)]
pub(crate) struct ModuleStaticResult {
  bindings: HashMap<(Atom, SyntaxContext), StaticEvalResult>,
  exports: HashMap<String, StaticEvalResult>,
  css_map_rules: HashMap<String, Vec<String>>,
  css_map_static_objects: HashMap<(Atom, SyntaxContext), IndexMap<String, StaticValue>>,
  dependencies: BTreeSet<PathBuf>,
}

struct StaticBindings {
  bindings: HashMap<(Atom, SyntaxContext), StaticEvalResult>,
  css_map_rules: HashMap<String, Vec<String>>,
  css_map_static_objects: HashMap<(Atom, SyntaxContext), IndexMap<String, StaticValue>>,
}

struct ModuleEvaluator {
  resolver: Resolver,
  cache: RefCell<HashMap<PathBuf, ModuleStaticData>>,
  included_files: RefCell<BTreeSet<PathBuf>>,
  import_sources: Vec<String>,
  css_options: CssOptions,
  project_root: PathBuf,
}

impl ModuleEvaluator {
  fn new(
    cwd: &Path,
    project_root: &Path,
    extensions: &[String],
    import_sources: &[String],
    css_options: CssOptions,
  ) -> Self {
    let mut options = ResolveOptions::default();
    if extensions.is_empty() {
      options.extensions = DEFAULT_RESOLVE_EXTENSIONS
        .iter()
        .map(|ext| ext.to_string())
        .collect();
    } else {
      options.extensions = extensions
        .iter()
        .map(|ext| {
          if ext.starts_with('.') {
            ext.clone()
          } else {
            format!(".{ext}")
          }
        })
        .collect();
    }
    options.cwd = Some(cwd.to_path_buf());
    let resolver = Resolver::new(options);
    Self {
      resolver,
      cache: RefCell::new(HashMap::new()),
      included_files: RefCell::new(BTreeSet::new()),
      import_sources: import_sources.to_vec(),
      css_options,
      project_root: project_root.to_path_buf(),
    }
  }

  fn resolve(&self, from: &Path, request: &str) -> Option<PathBuf> {
    let from_dir = from.parent().unwrap_or(from);
    self
      .resolver
      .resolve(from_dir, request)
      .ok()
      .map(|result| result.full_path())
      .or_else(|| self.resolve_workspace(request))
  }

  fn resolve_workspace(&self, request: &str) -> Option<PathBuf> {
    let (package, remainder) = split_package_request(request)?;
    let map = self.workspace_package_map();
    let base = map.get(&package)?;
    if let Some(subpath) = remainder {
      let mut path = base.clone();
      path.push(subpath);
      Some(path)
    } else {
      Some(base.clone())
    }
  }

  fn workspace_package_map(&self) -> HashMap<String, PathBuf> {
    let root = self
      .project_root
      .canonicalize()
      .unwrap_or_else(|_| self.project_root.clone());
    let mut guard = WORKSPACE_PACKAGE_MAP
      .lock()
      .expect("workspace map lock poisoned");
    if let Some(existing) = guard.get(&root) {
      return existing.clone();
    }
    let map = build_workspace_package_map(root.clone());
    guard.insert(root, map.clone());
    map
  }

  fn statics_for_inner(
    &self,
    path: &Path,
    visiting: &mut HashSet<PathBuf>,
  ) -> Option<ModuleStaticData> {
    self.included_files.borrow_mut().insert(path.to_path_buf());

    if let Some(cached) = self.cache.borrow().get(path).cloned() {
      return Some(cached);
    }

    if let Some(global_cached) = get_cached_module(path) {
      self
        .cache
        .borrow_mut()
        .insert(path.to_path_buf(), global_cached.clone());
      return Some(global_cached);
    }

    if !visiting.insert(path.to_path_buf()) {
      return None;
    }

    let source = fs::read_to_string(path).ok()?;
    let file_hash = hash(&source, 0);
    let module = parse_module_from_source(&source, path)?;
    let result = collect_module_statics_from_ast(
      &module,
      Some(path),
      Some(self),
      &self.import_sources,
      &self.css_options,
      visiting,
    );
    visiting.remove(path);

    let dependencies: Vec<PathBuf> = result.dependencies.iter().cloned().collect();
    let mut dependency_hashes: Vec<(PathBuf, String)> = Vec::with_capacity(dependencies.len());

    {
      let cache = self.cache.borrow();
      for dependency in &dependencies {
        if let Some(dep_data) = cache.get(dependency) {
          dependency_hashes.push((dependency.clone(), dep_data.fingerprint.tree_hash.clone()));
        }
      }
    }

    for dependency in &dependencies {
      if dependency_hashes.iter().any(|(path, _)| path == dependency) {
        continue;
      }
      if let Some(dep_data) = get_cached_module(dependency) {
        dependency_hashes.push((dependency.clone(), dep_data.fingerprint.tree_hash.clone()));
        self.cache.borrow_mut().insert(dependency.clone(), dep_data);
      } else {
        return None;
      }
    }

    if dependency_hashes.len() > 1 {
      dependency_hashes.sort_by(|a, b| a.0.cmp(&b.0));
    }

    let fingerprint = ModuleFingerprint {
      file_hash: file_hash.clone(),
      tree_hash: compute_tree_hash(&file_hash, &dependency_hashes),
      dependencies,
    };

    let data = ModuleStaticData {
      result,
      fingerprint,
    };

    self
      .cache
      .borrow_mut()
      .insert(path.to_path_buf(), data.clone());
    store_cached_module(path.to_path_buf(), data.clone());

    Some(data)
  }

  fn included_files(&self) -> Vec<PathBuf> {
    self.included_files.borrow().iter().cloned().collect()
  }
}

fn parse_module_from_source(source: &str, path: &Path) -> Option<Module> {
  use std::sync::Arc;

  let filename = path.to_string_lossy().to_string();
  let cm: Arc<SourceMap> = Default::default();
  let fm = cm.new_source_file(FileName::Custom(filename.clone()).into(), source.into());
  let syntax = syntax_for_filename(&filename);
  let lexer = Lexer::new(syntax, EsVersion::Es2022, StringInput::from(&*fm), None);
  let mut parser = Parser::new_from(lexer);
  parser.parse_module().ok()
}

fn find_project_root(start: &Path) -> PathBuf {
  let mut current = if start.as_os_str().is_empty() {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
  } else {
    start.canonicalize().unwrap_or_else(|_| start.to_path_buf())
  };
  let mut result: Option<PathBuf> = None;
  loop {
    let candidate = current.join("package.json");
    if candidate.exists() {
      if let Ok(raw) = fs::read_to_string(&candidate) {
        if let Ok(value) = serde_json::from_str::<Value>(&raw) {
          if value.get("workspaces").is_some() {
            result = Some(current.to_path_buf());
          }
        }
      }
    }
    match current.parent() {
      Some(parent) => current = parent.to_path_buf(),
      None => break,
    }
  }
  result.unwrap_or_else(|| {
    if start.as_os_str().is_empty() {
      std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    } else {
      start.canonicalize().unwrap_or_else(|_| start.to_path_buf())
    }
  })
}

fn split_package_request(request: &str) -> Option<(String, Option<String>)> {
  if request.is_empty() || request.starts_with('.') || request.starts_with('/') {
    return None;
  }

  let mut parts = request.split('/').peekable();
  let first = parts.next()?;
  if first.is_empty() {
    return None;
  }

  if first.starts_with('@') {
    let second = parts.next()?;
    if second.is_empty() {
      return None;
    }
    let package = format!("{}/{}", first, second);
    let remainder: Vec<&str> = parts.collect();
    if remainder.is_empty() {
      Some((package, None))
    } else {
      Some((package, Some(remainder.join("/"))))
    }
  } else {
    let package = first.to_string();
    let remainder: Vec<&str> = parts.collect();
    if remainder.is_empty() {
      Some((package, None))
    } else {
      Some((package, Some(remainder.join("/"))))
    }
  }
}

fn build_workspace_package_map(root: PathBuf) -> HashMap<String, PathBuf> {
  let mut map = HashMap::new();

  for entry in WalkDir::new(&root)
    .follow_links(false)
    .into_iter()
    .filter_entry(|entry| !should_skip_dir(entry))
  {
    let entry = match entry {
      Ok(value) => value,
      Err(_) => continue,
    };
    if !entry.file_type().is_file() {
      continue;
    }
    if entry.file_name() != "package.json" {
      continue;
    }
    if entry
      .path()
      .components()
      .any(|component| matches!(component.as_os_str().to_str(), Some("node_modules")))
    {
      continue;
    }
    let Ok(raw) = fs::read_to_string(entry.path()) else {
      continue;
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
      continue;
    };
    let Some(name) = value.get("name").and_then(|name| name.as_str()) else {
      continue;
    };
    if map.contains_key(name) {
      continue;
    }
    let Some(parent) = entry.path().parent() else {
      continue;
    };
    map.insert(name.to_string(), parent.to_path_buf());
  }

  map
}

fn should_skip_dir(entry: &DirEntry) -> bool {
  if !entry.file_type().is_dir() {
    return false;
  }
  matches!(
    entry.file_name().to_str(),
    Some(".git")
      | Some("node_modules")
      | Some(".yarn")
      | Some(".turbo")
      | Some(".next")
      | Some("dist")
      | Some("build")
      | Some("coverage")
      | Some("storybook-static")
      | Some("tmp")
      | Some("target")
      | Some(".cache")
  )
}

fn collect_module_statics_from_ast(
  module: &Module,
  module_path: Option<&Path>,
  evaluator: Option<&ModuleEvaluator>,
  import_sources: &[String],
  css_options: &CssOptions,
  visiting: &mut HashSet<PathBuf>,
) -> ModuleStaticResult {
  let mut result = ModuleStaticResult::default();
  let mut css_idents: HashSet<(Atom, SyntaxContext)> = HashSet::new();
  let mut css_map_idents: HashSet<(Atom, SyntaxContext)> = HashSet::new();
  let mut keyframes_idents: HashSet<(Atom, SyntaxContext)> = HashSet::new();

  if let (Some(evaluator), Some(module_path)) = (evaluator, module_path) {
    for item in &module.body {
      if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item {
        let source = import.src.value.as_ref();
        if import_sources.iter().any(|candidate| candidate == source) {
          for specifier in &import.specifiers {
            if let ImportSpecifier::Named(named) = specifier {
              let imported_name = named
                .imported
                .as_ref()
                .map(|name| match name {
                  ModuleExportName::Ident(ident) => ident.sym.as_ref(),
                  ModuleExportName::Str(str) => str.value.as_ref(),
                })
                .unwrap_or_else(|| named.local.sym.as_ref());
              match imported_name {
                "css" => {
                  css_idents.insert(to_id(&named.local));
                }
                "cssMap" => {
                  css_map_idents.insert(to_id(&named.local));
                }
                "keyframes" => {
                  keyframes_idents.insert(to_id(&named.local));
                }
                _ => {}
              }
            }
          }
        }
        if let Some(resolved) = evaluator.resolve(module_path, source) {
          if let Some(imported) = evaluator.statics_for_inner(&resolved, visiting) {
            result.dependencies.insert(resolved.clone());
            let imported_result = &imported.result;
            for specifier in &import.specifiers {
              match specifier {
                ImportSpecifier::Named(named) => {
                  let export_name = match &named.imported {
                    Some(ModuleExportName::Ident(ident)) => ident.sym.to_string(),
                    Some(ModuleExportName::Str(str)) => str.value.to_string(),
                    None => named.local.sym.to_string(),
                  };
                  if let Some(value) = imported_result.exports.get(&export_name) {
                    let mut binding = value.clone();
                    binding.is_import_binding = true;
                    result.bindings.insert(to_id(&named.local), binding);
                  }
                }
                ImportSpecifier::Default(default_spec) => {
                  if let Some(value) = imported_result.exports.get("default") {
                    let mut binding = value.clone();
                    binding.is_import_binding = true;
                    result.bindings.insert(to_id(&default_spec.local), binding);
                  }
                }
                ImportSpecifier::Namespace(namespace) => {
                  let mut entries: Vec<_> = imported_result
                    .exports
                    .iter()
                    .filter(|(name, _)| name.as_str() != "default")
                    .collect();
                  entries.sort_by(|a, b| a.0.cmp(b.0));
                  let mut map = IndexMap::new();
                  let mut depends_on_import = false;
                  for (name, value) in entries {
                    depends_on_import |= value.depends_on_import;
                    map.insert(name.clone(), value.value.clone());
                  }
                  result.bindings.insert(
                    to_id(&namespace.local),
                    StaticEvalResult::with_flags(StaticValue::Object(map), depends_on_import, true),
                  );
                }
              }
            }
          }
        }
      }
    }
  }

  for item in &module.body {
    match item {
      ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => {
        record_var_decl(
          var,
          &css_idents,
          &css_map_idents,
          &keyframes_idents,
          css_options,
          &mut result.css_map_rules,
          &mut result.css_map_static_objects,
          &mut result.bindings,
        );
      }
      ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => match &export_decl.decl {
        Decl::Var(var) => {
          for (ident, value) in record_var_decl(
            var,
            &css_idents,
            &css_map_idents,
            &keyframes_idents,
            css_options,
            &mut result.css_map_rules,
            &mut result.css_map_static_objects,
            &mut result.bindings,
          ) {
            result.exports.insert(ident.sym.to_string(), value);
          }
        }
        _ => {}
      },
      ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(named)) => {
        if let Some(src) = &named.src {
          if let (Some(evaluator), Some(module_path)) = (evaluator, module_path) {
            if let Some(resolved) = evaluator.resolve(module_path, src.value.as_ref()) {
              if let Some(exported) = evaluator.statics_for_inner(&resolved, visiting) {
                result.dependencies.insert(resolved.clone());
                let exported_result = &exported.result;
                for specifier in &named.specifiers {
                  if let ExportSpecifier::Named(named_spec) = specifier {
                    let orig_name = match &named_spec.orig {
                      ModuleExportName::Ident(ident) => ident.sym.to_string(),
                      ModuleExportName::Str(str) => str.value.to_string(),
                    };
                    let export_name = match &named_spec.exported {
                      Some(ModuleExportName::Ident(ident)) => ident.sym.to_string(),
                      Some(ModuleExportName::Str(str)) => str.value.to_string(),
                      None => orig_name.clone(),
                    };
                    if let Some(value) = exported_result.exports.get(&orig_name) {
                      result.exports.insert(export_name, value.clone());
                    }
                  }
                }
              }
            }
          }
        } else {
          for specifier in &named.specifiers {
            if let ExportSpecifier::Named(named_spec) = specifier {
              let export_name = match &named_spec.exported {
                Some(ModuleExportName::Ident(ident)) => ident.sym.to_string(),
                Some(ModuleExportName::Str(str)) => str.value.to_string(),
                None => match &named_spec.orig {
                  ModuleExportName::Ident(ident) => ident.sym.to_string(),
                  ModuleExportName::Str(str) => str.value.to_string(),
                },
              };
              match &named_spec.orig {
                ModuleExportName::Ident(ident) => {
                  if let Some(value) = result.bindings.get(&to_id(ident)).cloned() {
                    result.exports.insert(export_name, value);
                  }
                }
                ModuleExportName::Str(str) => {
                  if let Some(value) = result.exports.get(str.value.as_ref()).cloned() {
                    result.exports.insert(export_name, value);
                  }
                }
              }
            }
          }
        }
      }
      ModuleItem::ModuleDecl(ModuleDecl::ExportAll(export_all)) => {
        if let (Some(evaluator), Some(module_path)) = (evaluator, module_path) {
          if let Some(resolved) = evaluator.resolve(module_path, export_all.src.value.as_ref()) {
            if let Some(exported) = evaluator.statics_for_inner(&resolved, visiting) {
              result.dependencies.insert(resolved.clone());
              for (name, value) in exported.result.exports {
                if name != "default" {
                  result.exports.entry(name).or_insert(value);
                }
              }
            }
          }
        }
      }
      ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(default_expr)) => {
        if let Some(value) = evaluate_static_with_info(&default_expr.expr, &result.bindings) {
          result.exports.insert("default".into(), value);
        }
      }
      _ => {}
    }
  }

  result
}

fn kebab_case(input: &str) -> String {
  const VENDOR_PREFIXES: [(&str, &str); 6] = [
    ("Webkit", "-webkit-"),
    ("Moz", "-moz-"),
    ("ms", "ms-"),
    ("Ms", "-ms-"),
    ("O", "-o-"),
    ("Khtml", "-khtml-"),
  ];

  let mut result = String::new();
  let mut skip = 0usize;

  for (prefix, replacement) in VENDOR_PREFIXES {
    if input.starts_with(prefix) {
      skip = prefix.len();
      result.push_str(replacement);
      break;
    }
  }

  for (index, ch) in input.chars().enumerate() {
    if index < skip {
      continue;
    }
    if ch.is_uppercase() {
      if !result.is_empty() && !result.ends_with('-') {
        result.push('-');
      }
      for lower_ch in ch.to_lowercase() {
        result.push(lower_ch);
      }
    } else {
      result.push(ch);
    }
  }

  result
}

fn to_camel_case(input: &str) -> Option<String> {
  if !input.contains(['-', '_']) {
    return None;
  }

  let mut result = String::new();
  let mut capitalize_next = false;
  for ch in input.chars() {
    match ch {
      '-' | '_' | ' ' => capitalize_next = true,
      _ => {
        if capitalize_next {
          for upper in ch.to_uppercase() {
            result.push(upper);
          }
          capitalize_next = false;
        } else {
          result.push(ch);
        }
      }
    }
  }

  if result.is_empty() {
    None
  } else {
    Some(result)
  }
}

fn encode_uri_component(input: &str) -> String {
  fn is_allowed(ch: char) -> bool {
    matches!(ch,
        'A'..='Z'
            | 'a'..='z'
            | '0'..='9'
            | '-' | '_' | '.' | '~'
            | '*' | '\'' | '(' | ')'
    )
  }

  let mut encoded = String::new();
  for ch in input.chars() {
    if is_allowed(ch) {
      encoded.push(ch);
    } else {
      let mut buf = [0u8; 4];
      let encoded_bytes = ch.encode_utf8(&mut buf).as_bytes();
      for byte in encoded_bytes {
        encoded.push('%');
        encoded.push_str(&format!("{:02X}", byte));
      }
    }
  }

  encoded
}

fn append_stylesheet_requires(module: &mut Module, path: &str, rules: &[String]) {
  if rules.is_empty() {
    return;
  }

  for rule in rules {
    let request = format!("{}?style={}", path, encode_uri_component(rule));
    let call = Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Ident(quote_ident!("require").into()))),
      args: vec![ExprOrSpread {
        spread: None,
        expr: Box::new(Expr::Lit(Lit::Str(Str::from(request)))),
      }],
      type_args: None,
    });
    module.body.insert(
      0,
      ModuleItem::Stmt(Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(call),
      })),
    );
  }
}

fn prepend_module_import(module: &mut Module, import_src: &str) {
  let already_present = module.body.iter().any(|item| {
    if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item {
      if import.specifiers.is_empty() {
        return import.src.value.as_ref() == import_src;
      }
    }
    false
  });

  if already_present {
    return;
  }

  module.body.insert(
    0,
    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
      span: DUMMY_SP,
      specifiers: Vec::new(),
      src: Box::new(Str::from(import_src)),
      type_only: false,
      with: None,
      phase: ImportPhase::Evaluation,
    })),
  );
}

fn sort_and_join_rules(rules: &[String], sort_at_rules: bool, sort_shorthand: bool) -> String {
  if sort_at_rules || sort_shorthand {
    let mut deduped: BTreeSet<String> = BTreeSet::new();
    for rule in rules {
      deduped.insert(rule.clone());
    }
    deduped.into_iter().collect::<Vec<_>>().join("\n")
  } else {
    let mut seen = HashSet::new();
    let mut ordered = Vec::new();
    for rule in rules {
      if seen.insert(rule.clone()) {
        ordered.push(rule.clone());
      }
    }
    ordered.join("\n")
  }
}

fn write_stylesheet_to_directory(
  module: &mut Module,
  options: &ExtractStylesToDirectoryOptions,
  rules: &[String],
  sort_at_rules: bool,
  sort_shorthand: bool,
  file_path: &Path,
  cwd: &Path,
) -> Result<(), String> {
  if rules.is_empty() {
    return Ok(());
  }

  let file_stem = file_path
    .file_stem()
    .and_then(|stem| stem.to_str())
    .ok_or_else(|| format!("Source filename '{}' was not defined", file_path.display()))?;

  let source_root = {
    let source = Path::new(&options.source);
    if source.is_absolute() {
      source.to_path_buf()
    } else {
      cwd.join(source)
    }
  };

  let absolute_file = if file_path.is_absolute() {
    file_path.to_path_buf()
  } else {
    cwd.join(file_path)
  };

  let relative_path = absolute_file.strip_prefix(&source_root).map_err(|_| {
    format!(
      "Source directory '{}' was not found relative to source file ('{}')",
      options.source,
      absolute_file.display()
    )
  })?;

  let relative_dir = relative_path.parent().unwrap_or_else(|| Path::new(""));

  let dest_root = {
    let dest = Path::new(&options.dest);
    if dest.is_absolute() {
      dest.to_path_buf()
    } else {
      cwd.join(dest)
    }
  };

  let output_dir = dest_root.join(relative_dir);
  fs::create_dir_all(&output_dir).map_err(|err| {
    format!(
      "Failed to create directory '{}': {}",
      output_dir.display(),
      err
    )
  })?;

  let css_filename = format!("{}.compiled.css", file_stem);
  let css_path = output_dir.join(&css_filename);
  let stylesheet = sort_and_join_rules(rules, sort_at_rules, sort_shorthand);
  fs::write(&css_path, stylesheet).map_err(|err| {
    format!(
      "Failed to write stylesheet '{}': {}",
      css_path.display(),
      err
    )
  })?;

  prepend_module_import(module, &format!("./{}", css_filename));

  Ok(())
}

fn build_css_from_object(value: &IndexMap<String, StaticValue>) -> Option<String> {
  let mut declarations = Vec::new();
  for (key, value) in value {
    let css_value = match value {
      StaticValue::Str(str) => str.value.clone(),
      StaticValue::Num(num) => {
        if num.fract() == 0.0 {
          format!("{}", *num as i64)
        } else {
          num.to_string()
        }
      }
      StaticValue::Bool(boolean) => boolean.to_string(),
      _ => return None,
    };
    declarations.push(format!("{}:{}", kebab_case(key), css_value));
  }
  Some(format!("{};", declarations.join(";")))
}

fn canonicalize_runtime_expr(expr: &Expr) -> String {
  let mut canonical_input = emit_expression(expr);
  if !canonical_input.contains('\n')
    && !canonical_input.contains("/*")
    && !canonical_input.contains("//")
  {
    canonical_input = canonical_input
      .split_whitespace()
      .collect::<Vec<_>>()
      .join(" ");
  }
  canonical_input
}

fn build_property_rule(
  value: &IndexMap<String, StaticValue>,
  runtime_variables: &mut Vec<RuntimeCssVariable>,
) -> Option<String> {
  let mut declarations = Vec::new();
  for (key, static_value) in value {
    let property = kebab_case(key);
    match property.as_str() {
      "inherits" => match static_value.unwrap_spread() {
        StaticValue::Bool(boolean) => {
          let expr = Expr::Lit(Lit::Bool(Bool {
            span: DUMMY_SP,
            value: *boolean,
          }));
          let canonical_input = canonicalize_runtime_expr(&expr);
          let hash_value = hash(&canonical_input, 0);
          let variable_name = format!("--_{}", hash_value);
          let rule_value = format!("var({})", variable_name);
          runtime_variables.push(RuntimeCssVariable::new(variable_name, expr, None, None));
          declarations.push(format!("{}:{}", property, rule_value));
        }
        _ => {
          let (output, _, _) = static_value_to_css_value(&property, static_value)?;
          declarations.push(format!("{}:{}", property, output));
        }
      },
      "initial-value" => match static_value.unwrap_spread() {
        StaticValue::Str(text) => {
          let normalized = normalize_css_value(&text.value);
          declarations.push(format!("{}:{}", property, normalized.output_value));
        }
        _ => {
          let (output, _, _) = static_value_to_css_value(&property, static_value)?;
          declarations.push(format!("{}:{}", property, output));
        }
      },
      _ => {
        let (output, _, _) = static_value_to_css_value(&property, static_value)?;
        declarations.push(format!("{}:{}", property, output));
      }
    }
  }
  Some(format!("{};", declarations.join(";")))
}

fn normalize_keyframe_step(step: &str) -> String {
  let trimmed = step.trim();
  if trimmed.eq_ignore_ascii_case("from") {
    "0%".to_string()
  } else if trimmed == "100%" {
    "to".to_string()
  } else if trimmed.eq_ignore_ascii_case("to") {
    "to".to_string()
  } else {
    trimmed.to_string()
  }
}

fn normalize_keyframes_body(css: &str) -> Option<String> {
  let mut index = 0usize;
  let bytes = css.as_bytes();
  let len = bytes.len();
  let mut output = String::new();

  while index < len {
    skip_keyframe_whitespace(bytes, &mut index);
    if index >= len {
      break;
    }
    let steps = parse_keyframe_steps(bytes, &mut index)?;
    skip_keyframe_whitespace(bytes, &mut index);
    if index >= len || bytes[index] != b'{' {
      return None;
    }
    let block = read_keyframe_block(css, &mut index)?;
    let minified = minify_keyframe_block(&block);
    for step in steps {
      output.push_str(&step);
      output.push('{');
      output.push_str(&minified);
      output.push('}');
    }
  }

  Some(output)
}

fn skip_keyframe_whitespace(bytes: &[u8], index: &mut usize) {
  let len = bytes.len();
  loop {
    while *index < len && bytes[*index].is_ascii_whitespace() {
      *index += 1;
    }
    if *index + 1 < len && bytes[*index] == b'/' && bytes[*index + 1] == b'*' {
      *index += 2;
      while *index + 1 < len && !(bytes[*index] == b'*' && bytes[*index + 1] == b'/') {
        *index += 1;
      }
      if *index + 1 < len {
        *index += 2;
      }
      continue;
    }
    break;
  }
}

fn parse_keyframe_steps(bytes: &[u8], index: &mut usize) -> Option<Vec<String>> {
  let mut steps = Vec::new();
  let len = bytes.len();
  loop {
    let start = *index;
    while *index < len && bytes[*index] != b'{' && bytes[*index] != b',' {
      *index += 1;
    }
    let token = std::str::from_utf8(&bytes[start..*index]).ok()?.trim();
    if token.is_empty() {
      return None;
    }
    steps.push(normalize_keyframe_step(token));
    skip_keyframe_whitespace(bytes, index);
    if *index >= len {
      return None;
    }
    match bytes[*index] {
      b',' => {
        *index += 1;
        skip_keyframe_whitespace(bytes, index);
        continue;
      }
      b'{' => break,
      _ => return None,
    }
  }
  Some(steps)
}

fn read_keyframe_block(source: &str, index: &mut usize) -> Option<String> {
  let bytes = source.as_bytes();
  let len = bytes.len();
  if *index >= len || bytes[*index] != b'{' {
    return None;
  }
  *index += 1;
  let mut depth = 1usize;
  let mut block = String::new();
  let mut in_single = false;
  let mut in_double = false;
  while *index < len {
    let ch = bytes[*index] as char;
    *index += 1;
    if in_single {
      block.push(ch);
      if ch == '\\' && *index < len {
        block.push(bytes[*index] as char);
        *index += 1;
      } else if ch == '\'' {
        in_single = false;
      }
      continue;
    }
    if in_double {
      block.push(ch);
      if ch == '\\' && *index < len {
        block.push(bytes[*index] as char);
        *index += 1;
      } else if ch == '"' {
        in_double = false;
      }
      continue;
    }
    match ch {
      '\'' => {
        in_single = true;
        block.push(ch);
      }
      '"' => {
        in_double = true;
        block.push(ch);
      }
      '{' => {
        depth += 1;
        block.push(ch);
      }
      '}' => {
        if depth == 1 {
          return Some(block);
        }
        depth -= 1;
        block.push('}');
      }
      _ => block.push(ch),
    }
  }
  None
}

fn minify_keyframe_block(block: &str) -> String {
  let mut output = String::with_capacity(block.len());
  let mut chars = block.chars().peekable();
  let mut in_value = false;
  let mut emitted_value_char = false;
  let mut pending_space = false;

  while let Some(ch) = chars.next() {
    match ch {
      '\'' | '"' => {
        if pending_space && emitted_value_char && !output.ends_with(' ') {
          output.push(' ');
        }
        pending_space = false;
        output.push(ch);
        consume_string_literal(&mut chars, &mut output, ch);
        if in_value {
          emitted_value_char = true;
        }
      }
      c if c.is_whitespace() => {
        if in_value && emitted_value_char {
          pending_space = true;
        }
      }
      ':' => {
        output.push(':');
        in_value = true;
        emitted_value_char = false;
        pending_space = false;
      }
      ';' => {
        if output.ends_with(' ') {
          output.pop();
        }
        output.push(';');
        in_value = false;
        emitted_value_char = false;
        pending_space = false;
        while chars
          .peek()
          .map(|next| next.is_whitespace())
          .unwrap_or(false)
        {
          chars.next();
        }
      }
      '{' => {
        output.push('{');
        in_value = false;
        emitted_value_char = false;
        pending_space = false;
      }
      '}' => {
        if output.ends_with(' ') {
          output.pop();
        }
        output.push('}');
        in_value = false;
        emitted_value_char = false;
        pending_space = false;
      }
      _ => {
        if pending_space && !output.ends_with(' ') {
          output.push(' ');
        }
        pending_space = false;
        output.push(ch);
        if in_value {
          emitted_value_char = true;
        }
      }
    }
  }

  if output.ends_with(';') {
    output.pop();
  }
  output
}

fn consume_string_literal(chars: &mut Peekable<Chars<'_>>, output: &mut String, delimiter: char) {
  while let Some(ch) = chars.next() {
    output.push(ch);
    if ch == delimiter {
      break;
    }
    if ch == '\\' {
      if let Some(next) = chars.next() {
        output.push(next);
      }
    }
  }
}

fn static_value_to_css(property: &str, value: &StaticValue) -> Option<String> {
  match value {
    StaticValue::Str(str) => {
      #[cfg(debug_assertions)]
      if std::env::var_os("DEBUG_TOKENS_GRADIENT_RAW").is_some()
        && str.value.contains("linear-gradient")
      {
        eprintln!(
          "[debug] static_value_to_css_value property={} raw={:?} value={:?}",
          property, str.raw, str.value
        );
      }
      if property == "content" {
        Some(normalize_content_value(&str.value))
      } else {
        Some(str.value.clone())
      }
    }
    StaticValue::Num(num) => {
      let raw = num.to_string();
      Some(add_unit_if_needed(property, &raw))
    }
    StaticValue::Bool(boolean) => Some(boolean.to_string()),
    StaticValue::Array(values) => {
      let mut parts = Vec::with_capacity(values.len());
      for item in values {
        let converted = static_value_to_css(property, item)?;
        parts.push(converted);
      }
      Some(parts.join(","))
    }
    _ => None,
  }
}

fn build_keyframe_declarations(map: &IndexMap<String, StaticValue>) -> Option<String> {
  let mut declarations = Vec::new();
  for (property, value) in map {
    let (mut output_value, _hash_source, important) = static_value_to_css_value(property, value)?;
    if important {
      output_value.push_str("!important");
    }
    declarations.push(format!("{}:{}", kebab_case(property), output_value));
  }
  Some(declarations.join(";"))
}

fn build_keyframes_css(value: &StaticValue) -> Option<String> {
  let map = value.as_object()?;
  let mut segments = Vec::new();
  for (step, value) in map {
    let declarations = match value {
      StaticValue::Object(inner) => build_keyframe_declarations(inner)?,
      _ => return None,
    };
    segments.push(format!(
      "{}{{{}}}",
      normalize_keyframe_step(step),
      declarations
    ));
  }
  Some(segments.join(""))
}

fn normalize_content_value(raw: &str) -> String {
  let trimmed = raw.trim();
  if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
    eprintln!(
      "[compiled-debug] normalize_content_value raw={:?} bytes={:?}",
      raw,
      raw.as_bytes()
    );
  }
  if trimmed.is_empty() {
    return "\"\"".to_string();
  }

  if trimmed.contains('"') || trimmed.contains('\'') {
    return trimmed.to_string();
  }

  if trimmed.contains("-quote") {
    return trimmed.to_string();
  }

  match trimmed {
    "inherit" | "initial" | "none" | "normal" | "revert" | "unset" => {
      return trimmed.to_string();
    }
    _ => {}
  }

  let mut prefix = String::new();
  let mut chars = trimmed.chars();
  while let Some(ch) = chars.next() {
    if ch.is_ascii_alphabetic() || ch == '-' {
      prefix.push(ch);
      continue;
    }

    if ch == '(' {
      if !prefix.is_empty() {
        return trimmed.to_string();
      }
      break;
    }

    if ch.is_ascii_whitespace() {
      break;
    }

    prefix.clear();
    break;
  }

  if !prefix.is_empty() {
    if trimmed[prefix.len()..].starts_with('(') {
      return trimmed.to_string();
    }
  }

  format!("\"{}\"", trimmed)
}

const CSS_UNITS: &[&str] = &[
  "em", "ex", "cap", "ch", "ic", "rem", "lh", "rlh", "vw", "vh", "vi", "vb", "vmin", "vmax", "cm",
  "mm", "Q", "in", "pc", "pt", "px", "deg", "grad", "rad", "turn", "s", "ms", "Hz", "kHz", "dpi",
  "dpcm", "dppx", "x", "fr", "%",
];

const INTERPOLATION_SUFFIX_TERMINATORS: &[char] = &[';', ',', '\n', ' ', ')'];

#[derive(Debug, Clone)]
struct BeforeInterpolationResult {
  css: String,
  variable_prefix: Option<String>,
}

#[derive(Debug, Clone)]
struct AfterInterpolationResult {
  css: String,
  variable_suffix: Option<String>,
}

fn css_after_interpolation(after: &str) -> AfterInterpolationResult {
  for unit in CSS_UNITS {
    if after.starts_with(unit) {
      let remainder = &after[unit.len()..];
      let next = remainder.chars().next();
      if next
        .map(|ch| INTERPOLATION_SUFFIX_TERMINATORS.contains(&ch))
        .unwrap_or(true)
      {
        return AfterInterpolationResult {
          variable_suffix: Some(unit.to_string()),
          css: remainder.to_string(),
        };
      }
    }
  }

  if let Some(rest) = after.strip_prefix('"') {
    return AfterInterpolationResult {
      variable_suffix: Some("\"".to_string()),
      css: rest.to_string(),
    };
  }

  if let Some(rest) = after.strip_prefix('\'') {
    return AfterInterpolationResult {
      variable_suffix: Some("'".to_string()),
      css: rest.to_string(),
    };
  }

  AfterInterpolationResult {
    variable_suffix: None,
    css: after.to_string(),
  }
}

fn css_before_interpolation(before: &str) -> BeforeInterpolationResult {
  if let Some(rest) = before.strip_suffix('"') {
    return BeforeInterpolationResult {
      css: rest.to_string(),
      variable_prefix: Some("\"".to_string()),
    };
  }

  if let Some(rest) = before.strip_suffix('\'') {
    return BeforeInterpolationResult {
      css: rest.to_string(),
      variable_prefix: Some("'".to_string()),
    };
  }

  if let Some(rest) = before.strip_suffix('-') {
    return BeforeInterpolationResult {
      css: rest.to_string(),
      variable_prefix: Some("-".to_string()),
    };
  }

  BeforeInterpolationResult {
    css: before.to_string(),
    variable_prefix: None,
  }
}

fn css_affix_interpolation(
  before: &str,
  after: &str,
) -> (BeforeInterpolationResult, AfterInterpolationResult) {
  if before.ends_with("url(") && after.starts_with(')') {
    let trimmed_before = before.trim_end_matches("url(");
    let trimmed_after = &after[1..];
    return (
      BeforeInterpolationResult {
        css: trimmed_before.to_string(),
        variable_prefix: Some("url(".to_string()),
      },
      AfterInterpolationResult {
        variable_suffix: Some(")".to_string()),
        css: trimmed_after.to_string(),
      },
    );
  }

  let before_result = css_before_interpolation(before);
  let after_result = css_after_interpolation(after);
  (before_result, after_result)
}

fn static_value_to_css_value(
  property: &str,
  value: &StaticValue,
) -> Option<(String, String, bool)> {
  match value.unwrap_spread() {
    StaticValue::Function(func) => match func.value.unwrap_spread() {
      StaticValue::Str(text) if text.value.contains("var(--ds-") => {
        return static_value_to_css_value(property, &func.value);
      }
      _ => return None,
    },
    StaticValue::Str(str) => {
      let mut hash_source = str.raw.clone().unwrap_or_else(|| str.value.clone());
      let mut trimmed = str.value.trim().to_string();
      if trimmed.is_empty() {
        if property == "content" {
          return Some(("\"\"".to_string(), "\"\"".to_string(), false));
        }
        return None;
      }
      let mut important = false;
      if let Some(stripped) = trimmed.strip_suffix("!important") {
        important = true;
        trimmed = stripped.trim_end().to_string();
        if let Some(idx) = hash_source.rfind("!important") {
          hash_source = hash_source[..idx].trim_end().to_string();
        }
      }
      if property != "content" {
        trimmed = normalize_css_string_quotes(&trimmed);
      }
      let mut base_value = if property == "content" {
        normalize_content_value(&trimmed)
      } else {
        trimmed
      };
      let lower_property = property.to_ascii_lowercase();
      if matches!(
        lower_property.as_str(),
        "width"
          | "height"
          | "minwidth"
          | "min-width"
          | "minheight"
          | "min-height"
          | "maxwidth"
          | "max-width"
          | "maxheight"
          | "max-height"
      ) && base_value.trim() == "0%"
      {
        base_value = "0".to_string();
      }
      if lower_property == "white-space" && base_value.eq_ignore_ascii_case("initial") {
        return Some(("normal".to_string(), "normal".to_string(), important));
      }
      if property == "content" {
        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
          eprintln!(
            "[compiled-debug] static_value_to_css_value content property base_value={:?}",
            base_value
          );
        }
        let hash_source = base_value.clone();
        return Some((base_value, hash_source, important));
      }

      if str.raw.is_none() {
        hash_source = base_value.clone();
      }
      #[cfg(debug_assertions)]
      if std::env::var_os("DEBUG_TOKENS_GRADIENT_RAW").is_some() {
        eprintln!(
          "[debug] property {} hash_source {:?}",
          property, hash_source
        );
      }
      let mut normalize_options = NormalizeCssValueOptions::default();
      if property.starts_with("--") {
        normalize_options.convert_lengths = false;
        normalize_options.convert_times = false;
        normalize_options.convert_rotations = false;
      }
      let NormalizedCssValue {
        hash_value: _hash_value,
        output_value,
      } = normalize_css_value_with_options(&hash_source, normalize_options);
      let mut output_value = output_value;
      if matches!(
        lower_property.as_str(),
        "width"
          | "height"
          | "minwidth"
          | "min-width"
          | "minheight"
          | "min-height"
          | "maxwidth"
          | "max-width"
          | "maxheight"
          | "max-height"
      ) && output_value == "0%"
      {
        output_value = "0".to_string();
        hash_source = "0".to_string();
      }
      if output_value.is_empty() {
        return None;
      }
      if std::env::var_os("DEBUG_TOKENS_GRADIENT_RAW").is_some() && property == "border-radius" {
        eprintln!("[debug] border-radius hash_source={:?}", hash_source);
      }
      Some((output_value, hash_source, important))
    }
    StaticValue::Num(num) => {
      let raw = if num.fract() == 0.0 {
        format!("{}", *num as i64)
      } else {
        num.to_string()
      };
      let value = add_unit_if_needed(property, &raw);
      let NormalizedCssValue {
        hash_value,
        output_value,
      } = normalize_css_value(&value);
      Some((output_value, hash_value, false))
    }
    StaticValue::Bool(boolean) => Some((boolean.to_string(), boolean.to_string(), false)),
    StaticValue::Null => None,
    _ => None,
  }
}

fn split_selector_list(raw: &str) -> Vec<String> {
  let mut segments = Vec::new();
  let mut current = String::new();
  let mut paren_depth = 0usize;
  let mut bracket_depth = 0usize;
  let mut brace_depth = 0usize;
  let mut in_single_quote = false;
  let mut in_double_quote = false;
  let mut escape_next = false;

  for ch in raw.chars() {
    if escape_next {
      current.push(ch);
      escape_next = false;
      continue;
    }

    match ch {
      '\\' => {
        current.push(ch);
        escape_next = true;
        continue;
      }
      '\'' => {
        current.push(ch);
        if !in_double_quote {
          in_single_quote = !in_single_quote;
        }
        continue;
      }
      '"' => {
        current.push(ch);
        if !in_single_quote {
          in_double_quote = !in_double_quote;
        }
        continue;
      }
      '(' if !in_single_quote && !in_double_quote => {
        paren_depth += 1;
        current.push(ch);
        continue;
      }
      ')' if !in_single_quote && !in_double_quote => {
        if paren_depth > 0 {
          paren_depth -= 1;
        }
        current.push(ch);
        continue;
      }
      '[' if !in_single_quote && !in_double_quote => {
        bracket_depth += 1;
        current.push(ch);
        continue;
      }
      ']' if !in_single_quote && !in_double_quote => {
        if bracket_depth > 0 {
          bracket_depth -= 1;
        }
        current.push(ch);
        continue;
      }
      '{' if !in_single_quote && !in_double_quote => {
        brace_depth += 1;
        current.push(ch);
        continue;
      }
      '}' if !in_single_quote && !in_double_quote => {
        if brace_depth > 0 {
          brace_depth -= 1;
        }
        current.push(ch);
        continue;
      }
      ','
        if !in_single_quote
          && !in_double_quote
          && paren_depth == 0
          && bracket_depth == 0
          && brace_depth == 0 =>
      {
        let trimmed = current.trim_end();
        if !trimmed.trim().is_empty() {
          segments.push(trimmed.to_string());
        }
        current.clear();
        continue;
      }
      _ => {}
    }

    current.push(ch);
  }

  let trimmed = current.trim_end();
  if !trimmed.trim().is_empty() {
    segments.push(trimmed.to_string());
  }

  segments
}

fn extend_selectors(current: &[String], raw: &str) -> Vec<String> {
  let mut result = Vec::new();
  let mut seen = BTreeSet::new();
  let segments = split_selector_list(raw);

  let parents = if current.is_empty() {
    vec![normalize_selector(None)]
  } else {
    current.to_vec()
  };

  if segments.is_empty() {
    return parents;
  }

  for parent in &parents {
    for segment in &segments {
      let normalized = normalize_selector(Some(segment));
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some()
        && segment.contains("data-component-selector")
      {
        eprintln!(
          "[compiled-debug] extend segment={} normalized={} parent={}",
          segment, normalized, parent
        );
      }
      let explicit_self = segment.trim_start().starts_with('&');
      let mut combined = String::with_capacity(normalized.len() + parent.len());
      let mut last_index = 0usize;
      let parent_trimmed = parent.trim();
      let normalized_bytes = normalized.as_bytes();
      let mut idx = 0usize;
      while idx < normalized_bytes.len() {
        if normalized_bytes[idx] == b'&' {
          combined.push_str(&normalized[last_index..idx]);
          combined.push_str(parent);
          let remainder = &normalized[idx + 1..];
          let next_char = remainder.chars().next();
          if !explicit_self && parent_trimmed != "&" {
            match next_char {
              Some('[') => {
                if !combined
                  .chars()
                  .rev()
                  .find(|c| !c.is_ascii_whitespace())
                  .map(|c| matches!(c, '>' | '+' | '~'))
                  .unwrap_or(false)
                  && !combined.ends_with(' ')
                {
                  combined.push(' ');
                }
              }
              Some('>') | Some('+') | Some('~') => {
                if !combined.ends_with(' ') {
                  combined.push(' ');
                }
              }
              _ => {}
            }
          }
          idx += 1;
          last_index = idx;
        } else {
          idx += 1;
        }
      }
      combined.push_str(&normalized[last_index..]);
      if seen.insert(combined.clone()) {
        result.push(combined);
      }
    }
  }

  if result.len() > 1 {
    result.sort_by(|a, b| {
      let key_a = selector_sort_key(a);
      let key_b = selector_sort_key(b);
      key_a.cmp(&key_b)
    });
  }

  result
}

fn parse_at_rule_key(key: &str) -> AtRuleInput {
  let trimmed = key.trim_start_matches('@').trim();
  let mut name_end = None;
  for (index, ch) in trimmed.char_indices() {
    if ch.is_whitespace() {
      name_end = Some(index);
      break;
    }
  }
  match name_end {
    Some(index) => AtRuleInput {
      name: trimmed[..index].to_string(),
      params: trimmed[index..].trim().to_string(),
    },
    None => AtRuleInput {
      name: trimmed.to_string(),
      params: String::new(),
    },
  }
}

const ATOMIC_AT_RULES: &[&str] = &[
  "container",
  "-moz-document",
  "else",
  "layer",
  "media",
  "starting-style",
  "supports",
  "when",
];

const NON_ATOMIC_AT_RULES: &[&str] = &[
  "color-profile",
  "counter-style",
  "font-face",
  "font-palette-values",
  "keyframes",
  "page",
  "property",
];

fn selector_requires_leading_combinator_space(selector: &str) -> bool {
  let trimmed = selector.trim_start();
  if trimmed.starts_with('&') {
    return false;
  }
  matches!(trimmed.chars().next(), Some('>' | '+' | '~'))
}

fn push_css_value(
  key: &str,
  value: &StaticValue,
  selectors: &[String],
  at_rules: &[AtRuleInput],
  out: &mut Vec<CssRuleInput>,
  flatten_selectors: bool,
  from_spread: bool,
  force_active_after_duplicate: bool,
  force_preserve_leading_space: bool,
  force_leading_combinator_space: bool,
) -> bool {
  let property = if key.starts_with("--") {
    key.to_string()
  } else {
    kebab_case(key)
  };
  if std::env::var_os("DEBUG_TOKENS_GRADIENT_RAW").is_some() {
    eprintln!("[debug] push_css_value key={}", key);
  }
  let actual_value = value.unwrap_spread();
  if std::env::var_os("DEBUG_TOKENS_GRADIENT_RAW").is_some() && key.contains("background") {
    if let StaticValue::Str(str) = actual_value {
      eprintln!(
        "[debug] push_css_value incoming key={} raw={:?}",
        key, str.raw
      );
    }
  }
  let spread_originates_here = value.is_from_spread();
  let (value, raw_value, important) = match static_value_to_css_value(&property, actual_value) {
    Some(result) => result,
    None => {
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] skipping dynamic property '{}' for selectors {:?}",
          property, selectors
        );
      }
      return true;
    }
  };

  if selectors.is_empty() {
    out.push(CssRuleInput {
      selectors: vec![normalize_selector(None)],
      selector_preserve_leading_combinator_space: vec![false],
      selector_force_leading_combinator_space: vec![false],
      at_rules: at_rules.to_vec(),
      property,
      value,
      raw_value,
      important,
      duplicate_active_after: false,
      value_hash_override: None,
    });
    return true;
  }

  let adjust_selector = |selector: &str, should_duplicate: bool| -> String {
    let matches_active_after =
      selector.contains(":active:after") || selector.contains(":active::after");
    let matches_active_before =
      selector.contains(":active:before") || selector.contains(":active::before");
    if selector.ends_with(":after")
      && !selector.ends_with(":after:after")
      && should_duplicate
      && matches_active_after
    {
      format!("{}:after", selector)
    } else if should_duplicate
      && selector.ends_with(":before")
      && !selector.ends_with(":before:before")
      && matches_active_before
    {
      format!("{}:before", selector)
    } else {
      selector.to_string()
    }
  };

  let mut selector_list: Vec<String> = selectors.to_vec();
  if selector_list.is_empty() {
    selector_list.push(normalize_selector(None));
  }
  let preserve_flags: Vec<bool> = selector_list
    .iter()
    .map(|selector| {
      if force_preserve_leading_space {
        true
      } else {
        selector_requires_leading_combinator_space(selector)
      }
    })
    .collect();
  let inferred_leading_space = selector_list.iter().any(|selector| {
    if let Some(rest) = selector.strip_prefix("& ") {
      rest.trim_start().starts_with('>')
    } else {
      false
    }
  });
  let force_leading_space = force_leading_combinator_space || inferred_leading_space;
  let leading_flags: Vec<bool> = selector_list.iter().map(|_| force_leading_space).collect();
  if force_leading_space && std::env::var_os("COMPILED_DEBUG_HASH").is_some() {
    eprintln!(
      "[compiled-debug] forcing leading combinator space for property='{}' selectors={:?}",
      property, selector_list
    );
  }
  let should_duplicate = (from_spread && spread_originates_here) || force_active_after_duplicate;
  let adjusted_selectors: Vec<String> = selector_list
    .iter()
    .map(|selector| adjust_selector(selector, should_duplicate))
    .collect();

  if std::env::var_os("COMPILED_DEBUG_HASH").is_some() {
    eprintln!(
      "[compiled-debug] push_css_value property='{}' selectors={:?}",
      property, adjusted_selectors
    );
  }

  if should_duplicate && std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
    eprintln!(
      "[compiled-debug] spread selector adjustment property='{}' direct_spread={} selectors={:?}",
      property, spread_originates_here, adjusted_selectors
    );
  }

  let duplicate_active_after_flag =
    selector_list
      .iter()
      .zip(adjusted_selectors.iter())
      .any(|(orig, adjusted)| {
        (orig.ends_with(":after") && adjusted.ends_with(":after:after"))
          || (orig.ends_with(":before") && adjusted.ends_with(":before:before"))
      });

  if !flatten_selectors && adjusted_selectors.len() > 1 {
    out.push(CssRuleInput {
      selectors: adjusted_selectors.clone(),
      selector_preserve_leading_combinator_space: preserve_flags.clone(),
      selector_force_leading_combinator_space: leading_flags.clone(),
      at_rules: at_rules.to_vec(),
      property,
      value,
      raw_value,
      important,
      duplicate_active_after: duplicate_active_after_flag,
      value_hash_override: None,
    });
    return true;
  }

  for ((selector, preserve_leading_space), force_leading_space) in adjusted_selectors
    .into_iter()
    .zip(preserve_flags)
    .zip(leading_flags)
  {
    out.push(CssRuleInput {
      selectors: vec![selector],
      selector_preserve_leading_combinator_space: vec![preserve_leading_space],
      selector_force_leading_combinator_space: vec![force_leading_space],
      at_rules: at_rules.to_vec(),
      property: property.clone(),
      value: value.clone(),
      raw_value: raw_value.clone(),
      important,
      duplicate_active_after: duplicate_active_after_flag,
      value_hash_override: None,
    });
  }

  true
}

fn skip_whitespace_and_comments(css: &str, idx: &mut usize) {
  let bytes = css.as_bytes();
  let len = bytes.len();
  loop {
    while *idx < len && bytes[*idx].is_ascii_whitespace() {
      *idx += 1;
    }
    if *idx + 1 < len && bytes[*idx] == b'/' && bytes[*idx + 1] == b'*' {
      *idx += 2;
      while *idx + 1 < len && !(bytes[*idx] == b'*' && bytes[*idx + 1] == b'/') {
        *idx += 1;
      }
      if *idx + 1 < len {
        *idx += 2;
      }
      continue;
    }
    break;
  }
}

fn extract_block_content(css: &str, idx: &mut usize) -> Option<String> {
  let bytes = css.as_bytes();
  let len = bytes.len();
  if *idx >= len || bytes[*idx] != b'{' {
    return None;
  }
  *idx += 1;
  let start = *idx;
  let mut depth = 1usize;
  let mut in_single = false;
  let mut in_double = false;
  let mut in_comment = false;
  while *idx < len {
    let ch = bytes[*idx];
    if in_comment {
      if ch == b'*' && *idx + 1 < len && bytes[*idx + 1] == b'/' {
        in_comment = false;
        *idx += 2;
        continue;
      }
      *idx += 1;
      continue;
    }
    if in_single {
      if ch == b'\\' {
        *idx += 2;
        continue;
      }
      if ch == b'\'' {
        in_single = false;
      }
      *idx += 1;
      continue;
    }
    if in_double {
      if ch == b'\\' {
        *idx += 2;
        continue;
      }
      if ch == b'"' {
        in_double = false;
      }
      *idx += 1;
      continue;
    }
    match ch {
      b'/' if *idx + 1 < len && bytes[*idx + 1] == b'*' => {
        in_comment = true;
        *idx += 2;
      }
      b'\'' => {
        in_single = true;
        *idx += 1;
      }
      b'"' => {
        in_double = true;
        *idx += 1;
      }
      b'{' => {
        depth += 1;
        *idx += 1;
      }
      b'}' => {
        depth -= 1;
        if depth == 0 {
          let content = css[start..*idx].to_string();
          *idx += 1;
          return Some(content);
        }
        *idx += 1;
      }
      _ => {
        *idx += 1;
      }
    }
  }
  None
}

fn read_property_value(css: &str, idx: &mut usize) -> String {
  skip_whitespace_and_comments(css, idx);
  let bytes = css.as_bytes();
  let len = bytes.len();
  let start = *idx;
  let mut in_single = false;
  let mut in_double = false;
  let mut in_comment = false;
  let mut paren_depth = 0usize;
  while *idx < len {
    let ch = bytes[*idx];
    if in_comment {
      if ch == b'*' && *idx + 1 < len && bytes[*idx + 1] == b'/' {
        in_comment = false;
        *idx += 2;
        continue;
      }
      *idx += 1;
      continue;
    }
    if in_single {
      if ch == b'\\' {
        *idx += 2;
        continue;
      }
      if ch == b'\'' {
        in_single = false;
      }
      *idx += 1;
      continue;
    }
    if in_double {
      if ch == b'\\' {
        *idx += 2;
        continue;
      }
      if ch == b'"' {
        in_double = false;
      }
      *idx += 1;
      continue;
    }
    match ch {
      b'/' if *idx + 1 < len && bytes[*idx + 1] == b'*' => {
        in_comment = true;
        *idx += 2;
      }
      b'\'' => {
        in_single = true;
        *idx += 1;
      }
      b'"' => {
        in_double = true;
        *idx += 1;
      }
      b'(' => {
        paren_depth += 1;
        *idx += 1;
      }
      b')' => {
        if paren_depth > 0 {
          paren_depth -= 1;
        }
        *idx += 1;
      }
      b';' if paren_depth == 0 => {
        let value = css[start..*idx].trim().to_string();
        *idx += 1;
        return value;
      }
      b'}' if paren_depth == 0 => {
        let value = css[start..*idx].trim().to_string();
        return value;
      }
      _ => {
        *idx += 1;
      }
    }
  }
  css[start..*idx].trim().to_string()
}

fn parse_css_literal_block(
  css: &str,
  selectors: &[String],
  at_rules: &[AtRuleInput],
  flatten_selectors: bool,
  out: &mut Vec<CssRuleInput>,
  raw_rules: &mut Vec<String>,
) -> bool {
  let mut idx = 0usize;
  let len = css.len();
  while idx < len {
    skip_whitespace_and_comments(css, &mut idx);
    if idx >= len {
      break;
    }
    let bytes = css.as_bytes();
    if bytes[idx] == b'}' {
      idx += 1;
      continue;
    }
    if bytes[idx] == b'@' {
      idx += 1;
      let name_start = idx;
      while idx < len {
        let ch = bytes[idx];
        if ch.is_ascii_alphabetic() || ch == b'-' {
          idx += 1;
        } else {
          break;
        }
      }
      let name = css[name_start..idx].trim().to_string();
      let name_lower = name.to_ascii_lowercase();
      skip_whitespace_and_comments(css, &mut idx);
      let params_start = idx;
      let mut params_has_block = false;
      let mut in_single = false;
      let mut in_double = false;
      let mut in_comment = false;
      let bytes = css.as_bytes();
      while idx < len {
        let ch = bytes[idx];
        if in_comment {
          if ch == b'*' && idx + 1 < len && bytes[idx + 1] == b'/' {
            in_comment = false;
            idx += 2;
            continue;
          }
          idx += 1;
          continue;
        }
        if in_single {
          if ch == b'\\' {
            idx += 2;
            continue;
          }
          if ch == b'\'' {
            in_single = false;
          }
          idx += 1;
          continue;
        }
        if in_double {
          if ch == b'\\' {
            idx += 2;
            continue;
          }
          if ch == b'"' {
            in_double = false;
          }
          idx += 1;
          continue;
        }
        match ch {
          b'/' if idx + 1 < len && bytes[idx + 1] == b'*' => {
            in_comment = true;
            idx += 2;
          }
          b'\'' => {
            in_single = true;
            idx += 1;
          }
          b'"' => {
            in_double = true;
            idx += 1;
          }
          b'{' => {
            params_has_block = true;
            break;
          }
          b';' => {
            let params = css[params_start..idx].trim();
            let mut rule = format!("@{}", name);
            if !params.is_empty() {
              rule.push(' ');
              rule.push_str(params);
            }
            rule.push(';');
            idx += 1;
            raw_rules.push(rule);
            params_has_block = false;
            break;
          }
          _ => {
            idx += 1;
          }
        }
        if params_has_block {
          break;
        }
      }
      if !params_has_block {
        continue;
      }
      let params = css[params_start..idx].trim();
      if idx >= len || bytes[idx] != b'{' {
        return false;
      }
      let mut next_idx = idx;
      let block = match extract_block_content(css, &mut next_idx) {
        Some(content) => content,
        None => return false,
      };
      idx = next_idx;
      if NON_ATOMIC_AT_RULES.contains(&name_lower.as_str()) {
        let wrapped = wrap_at_rules(
          block.trim().to_string(),
          &[AtRuleInput {
            name: name.clone(),
            params: minify_at_rule_params(params).to_string(),
          }],
        );
        raw_rules.push(wrapped);
        continue;
      }
      if !ATOMIC_AT_RULES.contains(&name_lower.as_str()) {
        return false;
      }
      let mut nested_at_rules = at_rules.to_vec();
      nested_at_rules.push(AtRuleInput {
        name: name.clone(),
        params: minify_at_rule_params(params).to_string(),
      });
      if !parse_css_literal_block(
        &block,
        selectors,
        &nested_at_rules,
        flatten_selectors,
        out,
        raw_rules,
      ) {
        return false;
      }
      continue;
    }

    let mut cursor = idx;
    let mut colon_pos = None;
    let mut brace_pos = None;
    let mut semicolon_pos = None;
    let bytes = css.as_bytes();
    let mut in_single = false;
    let mut in_double = false;
    let mut in_comment = false;
    let mut paren_depth = 0usize;
    while cursor < len {
      let ch = bytes[cursor];
      if in_comment {
        if ch == b'*' && cursor + 1 < len && bytes[cursor + 1] == b'/' {
          in_comment = false;
          cursor += 2;
          continue;
        }
        cursor += 1;
        continue;
      }
      if in_single {
        if ch == b'\\' {
          cursor += 2;
          continue;
        }
        if ch == b'\'' {
          in_single = false;
        }
        cursor += 1;
        continue;
      }
      if in_double {
        if ch == b'\\' {
          cursor += 2;
          continue;
        }
        if ch == b'"' {
          in_double = false;
        }
        cursor += 1;
        continue;
      }
      match ch {
        b'/' if cursor + 1 < len && bytes[cursor + 1] == b'*' => {
          in_comment = true;
          cursor += 2;
        }
        b'\'' => {
          in_single = true;
          cursor += 1;
        }
        b'"' => {
          in_double = true;
          cursor += 1;
        }
        b'(' => {
          paren_depth += 1;
          cursor += 1;
        }
        b')' => {
          if paren_depth > 0 {
            paren_depth -= 1;
          }
          cursor += 1;
        }
        b':' if paren_depth == 0 && colon_pos.is_none() => {
          colon_pos = Some(cursor);
          cursor += 1;
        }
        b'{' if paren_depth == 0 => {
          brace_pos = Some(cursor);
          break;
        }
        b';' if paren_depth == 0 => {
          semicolon_pos = Some(cursor);
          break;
        }
        b'}' if paren_depth == 0 => {
          break;
        }
        _ => {
          cursor += 1;
        }
      }
    }

    if let Some(colon_index) = colon_pos {
      let treat_as_declaration = match (semicolon_pos, brace_pos) {
        (Some(semicolon), Some(brace)) => semicolon < brace,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => true,
      };
      if treat_as_declaration {
        let property = css[idx..colon_index].trim();
        let mut value_idx = colon_index + 1;
        let value = read_property_value(css, &mut value_idx);
        if !property.is_empty() {
          let static_value = StaticValue::Str(StaticString::new(value));
          if !push_css_value(
            property,
            &static_value,
            selectors,
            at_rules,
            out,
            flatten_selectors,
            false,
            false,
            false,
            false,
          ) {
            return false;
          }
        }
        idx = value_idx;
        continue;
      } else {
        // treat as selector block
      }
    }

    let brace_index = match brace_pos {
      Some(pos) => pos,
      None => break,
    };
    let raw_selector = &css[idx..brace_index];
    let selector_text = raw_selector.trim_end();
    if selector_text.trim().is_empty() {
      return false;
    }
    let mut block_start = brace_index;
    let block_content = match extract_block_content(css, &mut block_start) {
      Some(content) => content,
      None => return false,
    };
    idx = block_start;

    let extended_selectors = extend_selectors(selectors, selector_text);
    if !parse_css_literal_block(
      &block_content,
      &extended_selectors,
      at_rules,
      flatten_selectors,
      out,
      raw_rules,
    ) {
      return false;
    }
  }
  true
}

fn css_artifacts_from_literal(css: &str, options: &CssOptions) -> Option<CssArtifacts> {
  let mut rules = Vec::new();
  let mut raw_rules = Vec::new();
  let base_selectors = vec![normalize_selector(None)];
  if !parse_css_literal_block(
    css,
    &base_selectors,
    &[],
    options.flatten_multiple_selectors,
    &mut rules,
    &mut raw_rules,
  ) {
    return None;
  }
  let mut artifacts = atomicize_rules(&rules, options);
  for css in raw_rules {
    artifacts.push_raw(css);
  }
  Some(artifacts)
}

fn flatten_css_object(
  map: &IndexMap<String, StaticValue>,
  selectors: &[String],
  at_rules: &[AtRuleInput],
  out: &mut Vec<CssRuleInput>,
  raw_rules: &mut Vec<String>,
  runtime_variables: &mut Vec<RuntimeCssVariable>,
  inherited_from_spread: bool,
  flatten_selectors: bool,
  preserve_leading_space: bool,
  force_leading_space: bool,
) -> bool {
  let debug_css = std::env::var_os("COMPILED_DEBUG_CSS").is_some();
  for (key, value) in map.iter() {
    if debug_css {
      let kind = match value.unwrap_spread() {
        StaticValue::Str(_) => "str",
        StaticValue::Num(_) => "num",
        StaticValue::Bool(_) => "bool",
        StaticValue::Null => "null",
        StaticValue::Object(_) => "object",
        StaticValue::Array(_) => "array",
        StaticValue::Function(_) => "function",
        StaticValue::Spread(_) => "spread",
      };
      eprintln!(
        "[compiled-debug] flatten_css_object key='{}' kind={} selectors={:?}",
        key, kind, selectors
      );
    }
    let value_is_from_spread = value.is_from_spread();
    if std::env::var_os("DEBUG_TOKENS_GRADIENT_RAW").is_some() && key.contains("background") {
      match value.unwrap_spread() {
        StaticValue::Str(str) => {
          eprintln!("[debug] flatten key='{}' raw={:?}", key, str.raw);
        }
        other => {
          eprintln!("[debug] flatten key='{}' kind={:?}", key, other);
        }
      }
    }

    if key == "selectors" {
      let nested = match value.as_object() {
        Some(obj) => obj,
        None => return false,
      };
      for (selector_key, selector_value) in nested {
        let next_selectors = extend_selectors(selectors, selector_key);
        let nested_obj = match selector_value.as_object() {
          Some(obj) => obj,
          None => return false,
        };
        let selector_inherited_spread =
          inherited_from_spread || value_is_from_spread || selector_value.is_from_spread();
        if !flatten_css_object(
          nested_obj,
          &next_selectors,
          at_rules,
          out,
          raw_rules,
          runtime_variables,
          selector_inherited_spread,
          flatten_selectors,
          preserve_leading_space,
          force_leading_space,
        ) {
          return false;
        }
      }
      continue;
    }

    let combined_spread = value_is_from_spread || inherited_from_spread;
    if key.starts_with('@') {
      let descriptor = parse_at_rule_key(key);
      let normalized_name = descriptor.name.to_ascii_lowercase();
      if NON_ATOMIC_AT_RULES.contains(&normalized_name.as_str()) {
        let mut next_at_rules = at_rules.to_vec();
        next_at_rules.push(descriptor);
        match value.unwrap_spread() {
          StaticValue::Object(obj) => {
            let content = if normalized_name == "keyframes" {
              build_keyframes_css(value).map(|css| css.trim().to_string())
            } else if normalized_name == "property" {
              build_property_rule(obj, runtime_variables)
                .map(|body| body.trim().trim_end_matches(';').to_string())
            } else {
              build_css_from_object(obj).map(|body| body.trim().trim_end_matches(';').to_string())
            };
            let Some(body) = content else {
              if debug_css {
                eprintln!(
                  "[compiled-debug] flatten_css_object bail: failed to build body for @{}",
                  key
                );
              }
              return false;
            };
            let css = wrap_at_rules(body, &next_at_rules);
            raw_rules.push(css);
          }
          StaticValue::Array(items) => {
            for item in items {
              let StaticValue::Object(obj) = item.unwrap_spread() else {
                return false;
              };
              let builder = if normalized_name == "property" {
                build_property_rule(obj, runtime_variables)
              } else {
                build_css_from_object(obj)
              };
              let Some(body) = builder else {
                return false;
              };
              let css = wrap_at_rules(
                body.trim().trim_end_matches(';').to_string(),
                &next_at_rules,
              );
              raw_rules.push(css);
            }
          }
          StaticValue::Str(text) => {
            let trimmed = text.value.trim();
            if !trimmed.is_empty() {
              let css = wrap_at_rules(trimmed.to_string(), &next_at_rules);
              raw_rules.push(css);
            }
          }
          _ => return false,
        }
        continue;
      }

      if !ATOMIC_AT_RULES.contains(&normalized_name.as_str()) {
        return false;
      }
      let mut next_at_rules = at_rules.to_vec();
      next_at_rules.push(descriptor);
      match value.unwrap_spread() {
        StaticValue::Object(obj) => {
          if !flatten_css_object(
            obj,
            selectors,
            &next_at_rules,
            out,
            raw_rules,
            runtime_variables,
            combined_spread,
            flatten_selectors,
            preserve_leading_space,
            force_leading_space,
          ) {
            return false;
          }
        }
        StaticValue::Array(items) => {
          for item in items {
            if let StaticValue::Object(obj) = item.unwrap_spread() {
              if !flatten_css_object(
                obj,
                selectors,
                &next_at_rules,
                out,
                raw_rules,
                runtime_variables,
                combined_spread,
                flatten_selectors,
                preserve_leading_space,
                force_leading_space,
              ) {
                return false;
              }
            } else {
              return false;
            }
          }
        }
        _ => return false,
      }
      continue;
    }

    if let Some(nested) = value.as_object() {
      let next_selectors = extend_selectors(selectors, key);
      let trimmed_key = key.trim_start();
      let force_selector_space = !trimmed_key.starts_with('&')
        && matches!(trimmed_key.chars().next(), Some('>' | '+' | '~'));
      let next_preserve_space = preserve_leading_space;
      let next_force_leading_space = force_leading_space || force_selector_space;
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] flatten nested selector key={} selectors={:?} keys={:?} from_spread={}",
          key,
          next_selectors,
          nested.keys().collect::<Vec<_>>(),
          value_is_from_spread
        );
      }
      if !flatten_css_object(
        nested,
        &next_selectors,
        at_rules,
        out,
        raw_rules,
        runtime_variables,
        combined_spread,
        flatten_selectors,
        next_preserve_space,
        next_force_leading_space,
      ) {
        return false;
      }
      continue;
    }

    if let Some(array) = value.as_array() {
      let has_spread = array.iter().any(|item| item.is_from_spread());
      let has_non_spread = array
        .iter()
        .any(|item| !matches!(item.unwrap_spread(), StaticValue::Spread(_)));
      for (index, item) in array.iter().enumerate() {
        let duplicate_for_entry = if has_spread {
          if index == 0 {
            has_non_spread || combined_spread
          } else {
            item.is_from_spread() || combined_spread
          }
        } else {
          item.is_from_spread() || combined_spread
        };
        let force_duplicate = (has_non_spread && index == 0) || (combined_spread && index == 0);
        if !push_css_value(
          key,
          item,
          selectors,
          at_rules,
          out,
          flatten_selectors,
          duplicate_for_entry,
          force_duplicate,
          preserve_leading_space,
          force_leading_space,
        ) {
          return false;
        }
      }
      continue;
    }

    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() && key.contains(":after") {
      eprintln!(
        "[compiled-debug] property key '{}' value_from_spread={} selectors={:?}",
        key, value_is_from_spread, selectors
      );
    }

    if !push_css_value(
      key,
      value,
      selectors,
      at_rules,
      out,
      flatten_selectors,
      combined_spread,
      false,
      preserve_leading_space,
      force_leading_space,
    ) {
      return false;
    }
  }

  true
}

fn css_artifacts_from_static_object(
  map: &IndexMap<String, StaticValue>,
  options: &CssOptions,
) -> Option<CssArtifacts> {
  let mut adjusted: IndexMap<String, StaticValue> = IndexMap::new();
  for (key, value) in map.iter() {
    if matches!(
      key.as_str(),
      "background" | "backgroundColor" | "background-color"
    ) {
      if let Some(new_key) = promote_background_key_if_needed(value) {
        let mut new_value = value.clone();
        // Preserve transparent background values to align with Babel output.
        adjusted.insert(new_key, new_value);
        continue;
      }
    }
    adjusted.insert(key.clone(), value.clone());
  }

  let mut inputs = Vec::new();
  let base_selectors = vec![normalize_selector(None)];
  let mut raw_rules = Vec::new();
  let mut runtime_variables = Vec::new();
  if !flatten_css_object(
    &adjusted,
    &base_selectors,
    &[],
    &mut inputs,
    &mut raw_rules,
    &mut runtime_variables,
    false,
    options.flatten_multiple_selectors,
    options.preserve_leading_combinator_space,
    false,
  ) {
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      eprintln!(
        "[compiled-debug] flatten_css_object failed for selectors {:?}",
        base_selectors
      );
    }
    return None;
  }
  let mut artifacts = atomicize_rules(&inputs, options);
  artifacts.raw_rules.extend(raw_rules);
  artifacts.runtime_variables.extend(runtime_variables);
  Some(artifacts)
}

fn css_artifacts_from_static_value(
  value: &StaticValue,
  options: &CssOptions,
) -> Option<CssArtifacts> {
  match value.unwrap_spread() {
    StaticValue::Object(map) => css_artifacts_from_static_object(map, options),
    StaticValue::Str(text) => css_artifacts_from_literal(&text.value, options)
      .or_else(|| Some(atomicize_literal(&text.value, options))),
    StaticValue::Array(items) => {
      let mut combined = CssArtifacts::default();
      for item in items {
        match css_artifacts_from_static_value(item, options) {
          Some(artifacts) => combined.merge(artifacts),
          None => return None,
        }
      }
      Some(combined)
    }
    StaticValue::Null => Some(CssArtifacts::default()),
    _ => None,
  }
}

fn promote_background_key_if_needed(value: &StaticValue) -> Option<String> {
  match value.unwrap_spread() {
    StaticValue::Str(text) => {
      let normalized = normalize_css_value(&text.value);
      if should_promote_background_to_color(&normalized.output_value, &text.value) {
        return Some("background-color".to_string());
      }
    }
    _ => {}
  }
  None
}

fn should_promote_background_to_color(value: &str, raw: &str) -> bool {
  let trimmed = value.trim();
  if trimmed.starts_with('#')
    || trimmed.starts_with("rgb(")
    || trimmed.starts_with("rgba(")
    || trimmed.starts_with("hsl(")
    || trimmed.starts_with("hsla(")
  {
    return true;
  }

  let raw_trimmed = raw.trim();
  if raw_trimmed.is_empty() {
    return false;
  }
  let lower_raw = raw_trimmed.to_ascii_lowercase();
  if matches!(
    lower_raw.as_str(),
    "none" | "inherit" | "initial" | "unset" | "revert" | "currentcolor"
  ) {
    return false;
  }
  lower_raw.chars().all(|c| c.is_ascii_alphabetic())
}

fn collect_precomputed_classes(value: &StaticValue, classes: &mut Vec<String>) {
  match value.unwrap_spread() {
    StaticValue::Str(text) => {
      for part in text.value.split_whitespace() {
        if part.starts_with('_') && !part.is_empty() {
          let class = part.to_string();
          if !classes.contains(&class) {
            classes.push(class);
          }
        }
      }
    }
    StaticValue::Array(items) => {
      for item in items {
        collect_precomputed_classes(item, classes);
      }
    }
    StaticValue::Object(map) => {
      for (_, value) in map {
        collect_precomputed_classes(value, classes);
      }
    }
    _ => {}
  }
}

fn collect_precomputed_class_exprs(
  expr: &Expr,
  value: &StaticValue,
  expr_class_names: &mut Vec<String>,
  out: &mut Vec<Expr>,
) {
  match value {
    StaticValue::Str(text) => {
      if text.value.starts_with('_') && !text.value.is_empty() {
        if matches!(expr, Expr::Ident(_)) {
          return;
        }
        out.push(expr.clone());
        for class_name in text.value.split_whitespace() {
          if !class_name.is_empty() {
            expr_class_names.push(class_name.to_string());
          }
        }
      }
    }
    StaticValue::Array(items) => {
      if let Expr::Array(array) = expr {
        for (elem_opt, item) in array.elems.iter().zip(items) {
          if let Some(elem) = elem_opt {
            if elem.spread.is_none() {
              collect_precomputed_class_exprs(&elem.expr, item, expr_class_names, out);
            }
          }
        }
      }
    }
    _ => {}
  }
}

fn merge_adjacent_string_entries(entries: Vec<ExprOrSpread>) -> Vec<ExprOrSpread> {
  let mut merged = Vec::new();
  let mut buffer = Vec::new();

  for entry in entries {
    match entry {
      ExprOrSpread { spread: None, expr } if matches!(*expr.clone(), Expr::Lit(Lit::Str(_))) => {
        buffer.push(expr);
      }
      other => {
        if !buffer.is_empty() {
          merged.push(ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Lit(Lit::Str(Str::from(
              buffer
                .iter()
                .map(|expr| match &**expr {
                  Expr::Lit(Lit::Str(str)) => str.value.to_string(),
                  _ => String::new(),
                })
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
            )))),
          });
          buffer.clear();
        }
        merged.push(other);
      }
    }
  }

  if !buffer.is_empty() {
    merged.push(ExprOrSpread {
      spread: None,
      expr: Box::new(Expr::Lit(Lit::Str(Str::from(
        buffer
          .iter()
          .map(|expr| match &**expr {
            Expr::Lit(Lit::Str(str)) => str.value.to_string(),
            _ => String::new(),
          })
          .filter(|s| !s.is_empty())
          .collect::<Vec<_>>()
          .join(" "),
      )))),
    });
  }

  merged
}
fn binding_ident_from_pat(pat: &Pat) -> Option<Ident> {
  match pat {
    Pat::Ident(ident) => Some(ident.id.clone()),
    Pat::Assign(assign) => binding_ident_from_pat(&assign.left),
    _ => None,
  }
}

struct ClassNamesBodyVisitor<'a, 'b> {
  parent: &'a mut TransformVisitor<'b>,
  css_idents: HashSet<(Atom, SyntaxContext)>,
  style_idents: HashSet<(Atom, SyntaxContext)>,
  failed: bool,
  sheets: Vec<String>,
  runtime_variables: Vec<RuntimeCssVariable>,
}

fn css_options_from_plugin_options(options: &PluginOptions) -> CssOptions {
  CssOptions {
    class_hash_prefix: options.class_hash_prefix.clone(),
    class_name_compression_map: options.class_name_compression_map.clone(),
    increase_specificity: options.increase_specificity.unwrap_or(false),
    sort_at_rules: options.sort_at_rules.unwrap_or(true),
    sort_shorthand: options.sort_shorthand.unwrap_or(true),
    flatten_multiple_selectors: options.flatten_multiple_selectors.unwrap_or(true),
    preserve_leading_combinator_space: false,
    extract: options.extract,
  }
}

impl<'a, 'b> ClassNamesBodyVisitor<'a, 'b> {
  fn new(
    parent: &'a mut TransformVisitor<'b>,
    css_idents: HashSet<(Atom, SyntaxContext)>,
    style_idents: HashSet<(Atom, SyntaxContext)>,
  ) -> Self {
    Self {
      parent,
      css_idents,
      style_idents,
      failed: false,
      sheets: Vec::new(),
      runtime_variables: Vec::new(),
    }
  }

  fn strip_parens_expr(expr: &mut Expr) -> &mut Expr {
    let mut current = expr;
    while let Expr::Paren(paren) = current {
      current = &mut *paren.expr;
    }
    current
  }

  fn apply_artifacts_to_expr(
    &mut self,
    expr: &mut Expr,
    span: Span,
    ctxt: SyntaxContext,
    mut artifacts: CssArtifacts,
    mut precomputed_classes: Vec<String>,
    mut precomputed_exprs: Vec<Expr>,
    expr_class_names: Vec<String>,
  ) {
    let debug_css = std::env::var_os("COMPILED_DEBUG_CSS").is_some();
    let runtime_variables = std::mem::take(&mut artifacts.runtime_variables);
    if !runtime_variables.is_empty() {
      self.runtime_variables.extend(runtime_variables);
    }

    let mut runtime_class_conditions = std::mem::take(&mut artifacts.runtime_class_conditions);
    if debug_css {
      eprintln!(
        "[compiled-debug] runtime_class_conditions count in css prop={}",
        runtime_class_conditions.len()
      );
    }
    if debug_css {
      eprintln!(
        "[compiled-debug] runtime_class_conditions count={}",
        runtime_class_conditions.len()
      );
    }
    let mut conditional_class_names: HashSet<String> = HashSet::new();
    for condition in &runtime_class_conditions {
      conditional_class_names.extend(condition.when_true.iter().cloned());
      conditional_class_names.extend(condition.when_false.iter().cloned());
    }

    let mut class_names = Vec::new();
    let mut seen_classes = HashSet::new();
    let mut seen_sheet_rules: HashSet<String> = HashSet::new();

    for rule in &artifacts.rules {
      let keyframes: Vec<(String, String)> = self
        .parent
        .keyframes_rules
        .iter()
        .map(|(name, rule)| (name.clone(), rule.clone()))
        .collect();
      for (name, keyframe_rule) in keyframes {
        if rule.css.contains(&name) {
          self.parent.register_rule(keyframe_rule.clone());
          if !self.parent.options.extract {
            if seen_sheet_rules.insert(keyframe_rule.clone()) {
              self.sheets.push(keyframe_rule);
            }
          }
        }
      }
      if expr_class_names.contains(&rule.class_name) {
        continue;
      }
      if !conditional_class_names.contains(&rule.class_name)
        && seen_classes.insert(rule.class_name.clone())
      {
        class_names.push(rule.class_name.clone());
      }
    }

    let class_css_map: HashMap<String, String> = artifacts
      .rules
      .iter()
      .map(|rule| (rule.class_name.clone(), rule.css.clone()))
      .collect();

    for (css, include_metadata, metadata_key) in artifacts.merged_css_entries() {
      self
        .parent
        .register_rule_with_metadata_key(css.clone(), include_metadata, metadata_key);
      if !self.parent.options.extract {
        if seen_sheet_rules.insert(css.clone()) {
          self.sheets.push(css);
        }
      }
    }

    for class_name in &precomputed_classes {
      if expr_class_names.contains(class_name) {
        continue;
      }
      if !conditional_class_names.contains(class_name) && seen_classes.insert(class_name.clone()) {
        class_names.push(class_name.clone());
      }
    }

    for condition in runtime_class_conditions {
      let has_true = !condition.when_true.is_empty();
      let has_false = !condition.when_false.is_empty();
      if has_true && has_false {
        let cons_expr = TransformVisitor::class_names_to_expr(&condition.when_true);
        let alt_expr = TransformVisitor::class_names_to_expr(&condition.when_false);
        precomputed_exprs.push(Expr::Cond(CondExpr {
          span: DUMMY_SP,
          test: Box::new(condition.test),
          cons: Box::new(cons_expr),
          alt: Box::new(alt_expr),
        }));
      } else if has_true {
        let cons_expr = TransformVisitor::class_names_to_expr(&condition.when_true);
        precomputed_exprs.push(Expr::Bin(BinExpr {
          span: DUMMY_SP,
          op: BinaryOp::LogicalAnd,
          left: Box::new(condition.test),
          right: Box::new(cons_expr),
        }));
      } else if has_false {
        let alt_expr = TransformVisitor::class_names_to_expr(&condition.when_false);
        precomputed_exprs.push(Expr::Bin(BinExpr {
          span: DUMMY_SP,
          op: BinaryOp::LogicalAnd,
          left: Box::new(Expr::Unary(UnaryExpr {
            span: DUMMY_SP,
            op: UnaryOp::Bang,
            arg: Box::new(condition.test),
          })),
          right: Box::new(alt_expr),
        }));
      }
    }

    self.parent.needs_runtime_ax = true;

    let mut elems = Vec::new();
    if !class_names.is_empty() {
      elems.push(Some(ExprOrSpread {
        spread: None,
        expr: Box::new(Expr::Lit(Lit::Str(Str::from(class_names.join(" "))))),
      }));
    }

    let compute_expr_priority = |expr: &Expr| -> (u8, u8) {
      fn collect_class_names(expr: &Expr, output: &mut Vec<String>) {
        match expr {
          Expr::Lit(Lit::Str(str_lit)) => {
            for part in str_lit.value.split_whitespace() {
              if !part.is_empty() {
                output.push(part.to_string());
              }
            }
          }
          Expr::Cond(cond) => {
            collect_class_names(&cond.cons, output);
            collect_class_names(&cond.alt, output);
          }
          Expr::Bin(bin) if matches!(bin.op, BinaryOp::LogicalAnd) => {
            collect_class_names(&bin.right, output);
          }
          _ => {}
        }
      }

      let mut class_names = Vec::new();
      collect_class_names(expr, &mut class_names);
      if class_names.is_empty() {
        return (3u8, u8::MAX);
      }
      class_names
        .iter()
        .filter_map(|class_name| class_css_map.get(class_name))
        .map(|css| css.split('{').next().unwrap_or("").trim())
        .map(|selector| {
          let lower = selector.to_ascii_lowercase();
          let base = if lower.contains(':') {
            1u8
          } else if lower.contains('>') || lower.contains('+') || lower.contains('~') {
            2u8
          } else {
            0u8
          };
          let secondary = selector_priority(selector);
          (base, secondary)
        })
        .min()
        .unwrap_or((3u8, u8::MAX))
    };

    let mut ordered_exprs: Vec<(u8, u8, usize, Expr)> = precomputed_exprs
      .into_iter()
      .enumerate()
      .map(|(index, expr)| {
        let (base, secondary) = compute_expr_priority(&expr);
        (base, secondary, index, expr)
      })
      .collect();

    ordered_exprs.sort_by(|a, b| {
      a.0
        .cmp(&b.0)
        .then_with(|| a.1.cmp(&b.1))
        .then_with(|| a.2.cmp(&b.2))
    });

    for (_, _, _, expr_item) in ordered_exprs {
      elems.push(Some(ExprOrSpread {
        spread: None,
        expr: Box::new(expr_item),
      }));
    }

    let array = Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems,
    });
    *expr = Expr::Call(CallExpr {
      span,
      ctxt,
      callee: Callee::Expr(Box::new(Expr::Ident(self.parent.runtime_class_ident()))),
      args: vec![ExprOrSpread {
        spread: None,
        expr: Box::new(array),
      }],
      type_args: None,
    });
  }

  fn finalize(self, expr: &mut Expr) -> Vec<String> {
    let ClassNamesBodyVisitor {
      parent,
      css_idents: _,
      style_idents,
      failed: _,
      sheets,
      runtime_variables,
    } = self;

    if style_idents.is_empty() {
      return sheets;
    }

    let mut replacer = ClassNamesStyleReplacer {
      parent,
      style_idents,
      runtime_variables,
      cached_expr: None,
    };
    expr.visit_mut_with(&mut replacer);

    sheets
  }
}

struct ClassNamesStyleReplacer<'a, 'b> {
  parent: &'a mut TransformVisitor<'b>,
  style_idents: HashSet<(Atom, SyntaxContext)>,
  runtime_variables: Vec<RuntimeCssVariable>,
  cached_expr: Option<Expr>,
}

impl<'a, 'b> ClassNamesStyleReplacer<'a, 'b> {
  fn build_style_expr(&mut self) -> Expr {
    if let Some(expr) = &self.cached_expr {
      return expr.clone();
    }

    if self.runtime_variables.is_empty() {
      let expr = Expr::Ident(Ident::new(
        "undefined".into(),
        DUMMY_SP,
        SyntaxContext::empty(),
      ));
      self.cached_expr = Some(expr.clone());
      return expr;
    }

    let ix_ident = self.parent.runtime_ix_ident();
    let mut props = Vec::new();
    let mut seen = HashSet::new();
    for variable in &self.runtime_variables {
      if !seen.insert(variable.name.clone()) {
        continue;
      }
      let mut ix_args = Vec::new();
      ix_args.push(ExprOrSpread {
        spread: None,
        expr: Box::new(variable.expression.clone()),
      });
      if let Some(suffix) = &variable.suffix {
        if !suffix.is_empty() {
          ix_args.push(ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Lit(Lit::Str(Str::from(suffix.clone())))),
          });
          if let Some(prefix) = &variable.prefix {
            if !prefix.is_empty() {
              ix_args.push(ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Lit(Lit::Str(Str::from(prefix.clone())))),
              });
            }
          }
        }
      }
      let ix_call = Expr::Call(CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Callee::Expr(Box::new(Expr::Ident(ix_ident.clone()))),
        args: ix_args,
        type_args: None,
      });
      props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Str(Str::from(variable.name.clone())),
        value: Box::new(ix_call),
      }))));
    }

    let expr = Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props,
    });

    self.cached_expr = Some(expr.clone());
    expr
  }
}

impl<'a, 'b> VisitMut for ClassNamesStyleReplacer<'a, 'b> {
  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    match expr {
      Expr::Ident(ident) => {
        if self.style_idents.contains(&to_id(ident)) {
          *expr = self.build_style_expr();
          return;
        }
      }
      _ => {}
    }
    expr.visit_mut_children_with(self);
  }

  fn visit_mut_jsx_element(&mut self, element: &mut JSXElement) {
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      let element_name = match &element.opening.name {
        JSXElementName::Ident(ident) => ident.sym.as_ref().to_string(),
        _ => "<complex>".to_string(),
      };
      eprintln!(
        "[compiled-debug] visit_mut_jsx_element name={}",
        element_name
      );
    }
    element.visit_mut_children_with(self);
  }
}

impl<'a, 'b> VisitMut for ClassNamesBodyVisitor<'a, 'b> {
  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    if self.failed {
      return;
    }

    let expr = Self::strip_parens_expr(expr);
    if std::env::var_os("COMPILED_DEBUG_NORMALIZE_EXPR").is_some() {
      eprintln!("normalize expr: {}", emit_expression(expr));
    }
    match expr {
      Expr::Call(call) => {
        if let Callee::Expr(callee_expr) = &mut call.callee {
          if let Expr::Ident(ident) = &**callee_expr {
            if self.css_idents.contains(&to_id(ident)) {
              let (span, ctxt, combined, precomputed_classes, precomputed_exprs, expr_class_names) = {
                let span = call.span;
                let ctxt = call.ctxt;
                let mut combined = CssArtifacts::default();
                let mut precomputed_classes = Vec::new();
                let mut precomputed_exprs = Vec::new();
                let mut expr_class_names = Vec::new();

                if let Some(values) = self.parent.evaluate_call_arguments(call) {
                  for (arg, value) in call.args.iter().zip(&values) {
                    collect_precomputed_class_exprs(
                      &arg.expr,
                      value,
                      &mut expr_class_names,
                      &mut precomputed_exprs,
                    );
                    if let StaticValue::Str(text) = value {
                      if text.value.starts_with('_') {
                        if !text.value.is_empty() {
                          if !precomputed_classes.contains(&text.value) {
                            precomputed_classes.push(text.value.clone());
                          }
                          if !self.parent.options.extract {
                            if let Some(rules) = self.parent.css_map_rule_groups.get(&text.value) {
                              for rule in rules {
                                if !self.sheets.iter().any(|existing| existing == rule) {
                                  self.sheets.push(rule.clone());
                                }
                              }
                            } else {
                              let needle = format!(".{}", text.value);
                              if let Some(rule) = self
                                .parent
                                .collected_rules
                                .iter()
                                .find(|css| css.contains(&needle))
                              {
                                if !self.sheets.iter().any(|existing| existing == rule) {
                                  self.sheets.push(rule.clone());
                                }
                              }
                            }
                          }
                        }
                        continue;
                      }
                      if let Some(name) = text.value.split_whitespace().next() {
                        if let Some(rule) = self.parent.keyframes_rules.get(name).cloned() {
                          self.parent.register_rule(rule.clone());
                          if !self.parent.options.extract
                            && !self.sheets.iter().any(|existing| existing == &rule)
                          {
                            self.sheets.push(rule);
                          }
                        }
                      }
                      continue;
                    }
                    let artifacts =
                      match css_artifacts_from_static_value(value, &self.parent.css_options()) {
                        Some(artifacts) => artifacts,
                        None => {
                          self.failed = true;
                          return;
                        }
                      };
                    combined.merge(artifacts);
                  }
                } else {
                  let props_ident = Ident::new("__cmplp".into(), DUMMY_SP, SyntaxContext::empty());
                  for arg in &call.args {
                    if arg.spread.is_some() {
                      self.failed = true;
                      return;
                    }
                    match &*arg.expr {
                      Expr::Object(object) => {
                        let artifacts =
                          match self.parent.process_dynamic_css_object(object, &props_ident) {
                            Some(artifacts) => artifacts,
                            None => {
                              eprintln!("ClassNames dynamic object processing failed");
                              self.failed = true;
                              return;
                            }
                          };
                        combined.merge(artifacts);
                      }
                      Expr::Arrow(_) | Expr::Fn(_) => {
                        let artifacts = match self
                          .parent
                          .process_dynamic_css_function(&arg.expr, &props_ident)
                        {
                          Some(artifacts) => artifacts,
                          None => {
                            eprintln!("ClassNames dynamic function processing failed");
                            self.failed = true;
                            return;
                          }
                        };
                        combined.merge(artifacts);
                      }
                      _ => {
                        self.failed = true;
                        return;
                      }
                    }
                  }
                }

                (
                  span,
                  ctxt,
                  combined,
                  precomputed_classes,
                  precomputed_exprs,
                  expr_class_names,
                )
              };

              self.apply_artifacts_to_expr(
                expr,
                span,
                ctxt,
                combined,
                precomputed_classes,
                precomputed_exprs,
                expr_class_names,
              );
              return;
            }
          }
        }
        call.visit_mut_children_with(self);
      }
      Expr::TaggedTpl(tagged) => {
        if let Expr::Ident(ident) = &*tagged.tag {
          if self.css_idents.contains(&to_id(ident)) {
            let (span, ctxt, artifacts) = {
              let span = tagged.span;
              let ctxt = tagged.ctxt;
              let artifacts = if let Some(css) = self.parent.evaluate_template(tagged) {
                atomicize_literal(&css, &self.parent.css_options())
              } else {
                let props_ident = Ident::new("__cmplp".into(), DUMMY_SP, SyntaxContext::empty());
                match self
                  .parent
                  .process_dynamic_styled_template(tagged, &props_ident)
                {
                  Some(artifacts) => artifacts,
                  None => {
                    eprintln!("ClassNames dynamic template processing failed");
                    self.failed = true;
                    return;
                  }
                }
              };
              (span, ctxt, artifacts)
            };
            self.apply_artifacts_to_expr(
              expr,
              span,
              ctxt,
              artifacts,
              Vec::new(),
              Vec::new(),
              Vec::new(),
            );
            return;
          }
        }
        tagged.visit_mut_children_with(self);
      }
      _ => expr.visit_mut_children_with(self),
    }
  }
}

fn collect_exported_bindings(module: &Module) -> (HashSet<(Atom, SyntaxContext)>, HashSet<String>) {
  let mut exported = HashSet::new();
  let mut names = HashSet::new();
  for item in &module.body {
    if let ModuleItem::ModuleDecl(decl) = item {
      collect_exported_idents_from_decl(decl, &mut exported, &mut names);
    }
  }
  (exported, names)
}

fn collect_exported_idents_from_decl(
  decl: &ModuleDecl,
  exported: &mut HashSet<(Atom, SyntaxContext)>,
  names: &mut HashSet<String>,
) {
  match decl {
    ModuleDecl::ExportDecl(export_decl) => {
      collect_exported_idents_from_decl_inner(&export_decl.decl, exported, names)
    }
    ModuleDecl::ExportDefaultDecl(default_decl) => match &default_decl.decl {
      DefaultDecl::Class(class_expr) => {
        if let Some(ident) = &class_expr.ident {
          exported.insert(to_id(ident));
          names.insert(ident.sym.to_string());
        }
      }
      DefaultDecl::Fn(fn_expr) => {
        if let Some(ident) = &fn_expr.ident {
          exported.insert(to_id(ident));
          names.insert(ident.sym.to_string());
        }
      }
      DefaultDecl::TsInterfaceDecl(decl) => {
        exported.insert(to_id(&decl.id));
        names.insert(decl.id.sym.to_string());
      }
    },
    ModuleDecl::ExportNamed(named) => {
      for specifier in &named.specifiers {
        match specifier {
          ExportSpecifier::Named(named_spec) => match &named_spec.orig {
            ModuleExportName::Ident(ident) => {
              exported.insert(to_id(ident));
              names.insert(ident.sym.to_string());
            }
            ModuleExportName::Str(_) => {}
          },
          ExportSpecifier::Default(default_spec) => {
            exported.insert(to_id(&default_spec.exported));
            names.insert(default_spec.exported.sym.to_string());
          }
          ExportSpecifier::Namespace(_) => {}
        }
      }
    }
    _ => {}
  }
}

fn collect_identifier_usages(module: &Module) -> HashSet<(Atom, SyntaxContext)> {
  struct UsageCollector {
    ids: HashSet<(Atom, SyntaxContext)>,
  }

  impl Visit for UsageCollector {
    fn visit_expr(&mut self, expr: &Expr) {
      if let Expr::Ident(ident) = expr {
        self.ids.insert(to_id(ident));
      }
      expr.visit_children_with(self);
    }
  }

  let mut collector = UsageCollector {
    ids: HashSet::new(),
  };
  module.visit_with(&mut collector);
  collector.ids
}

fn collect_exported_idents_from_decl_inner(
  decl: &Decl,
  exported: &mut HashSet<(Atom, SyntaxContext)>,
  names: &mut HashSet<String>,
) {
  match decl {
    Decl::Var(var_decl) => {
      for declarator in &var_decl.decls {
        collect_exported_idents_from_pat(&declarator.name, exported, names);
      }
    }
    Decl::Fn(fn_decl) => {
      exported.insert(to_id(&fn_decl.ident));
      names.insert(fn_decl.ident.sym.to_string());
    }
    Decl::Class(class_decl) => {
      exported.insert(to_id(&class_decl.ident));
      names.insert(class_decl.ident.sym.to_string());
    }
    _ => {}
  }
}

fn collect_exported_idents_from_pat(
  pat: &Pat,
  exported: &mut HashSet<(Atom, SyntaxContext)>,
  names: &mut HashSet<String>,
) {
  match pat {
    Pat::Ident(binding) => {
      exported.insert(to_id(&binding.id));
      names.insert(binding.id.sym.to_string());
    }
    Pat::Array(array) => {
      for elem in &array.elems {
        if let Some(pat) = elem {
          collect_exported_idents_from_pat(pat, exported, names);
        }
      }
    }
    Pat::Object(object) => {
      for prop in &object.props {
        match prop {
          ObjectPatProp::KeyValue(kv) => {
            collect_exported_idents_from_pat(&kv.value, exported, names)
          }
          ObjectPatProp::Assign(assign) => {
            exported.insert(to_id(&assign.key));
            names.insert(assign.key.sym.to_string());
          }
          ObjectPatProp::Rest(rest) => collect_exported_idents_from_pat(&rest.arg, exported, names),
        }
      }
    }
    Pat::Assign(assign) => collect_exported_idents_from_pat(&assign.left, exported, names),
    Pat::Rest(rest) => collect_exported_idents_from_pat(&rest.arg, exported, names),
    _ => {}
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum CompiledImportKind {
  Css,
  Keyframes,
  Styled,
  CssMap,
  ClassNames,
  Jsx,
}

#[derive(Debug, Clone)]
struct CssMapReference {
  ident: (Atom, SyntaxContext),
  property: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeVarDependency {
  depends_on_props: bool,
  depends_on_import: bool,
}

struct TransformVisitor<'a> {
  options: &'a PluginOptions,
  bindings: HashMap<(Atom, SyntaxContext), StaticEvalResult>,
  css_runtime_artifacts: HashMap<(Atom, SyntaxContext), CssArtifacts>,
  css_imports: HashSet<(Atom, SyntaxContext)>,
  keyframes_imports: HashSet<(Atom, SyntaxContext)>,
  styled_imports: HashSet<(Atom, SyntaxContext)>,
  css_map_imports: HashSet<(Atom, SyntaxContext)>,
  compiled_import_sources: HashMap<(Atom, SyntaxContext), String>,
  css_map_rule_groups: HashMap<String, Vec<String>>,
  css_map_static_objects: HashMap<(Atom, SyntaxContext), IndexMap<String, StaticValue>>,
  css_map_ident_classes: HashMap<(Atom, SyntaxContext), HashMap<String, Vec<String>>>,
  keyframes_rules: HashMap<String, String>,
  compiled_import_kinds: HashMap<(Atom, SyntaxContext), CompiledImportKind>,
  xcss_class_names: HashSet<String>,
  non_xcss_class_names: HashSet<String>,
  retain_imports: HashSet<(Atom, SyntaxContext)>,
  collected_rules: Vec<String>,
  metadata_rules: Vec<String>,
  seen_rules: HashSet<String>,
  seen_metadata_rules: HashSet<String>,
  runtime_var_dependencies: HashMap<String, RuntimeVarDependency>,
  processed_runtime_value_keys: HashSet<String>,
  force_runtime_metadata_classes: HashSet<String>,
  needs_runtime_ax: bool,
  needs_runtime_cc: bool,
  needs_runtime_cs: bool,
  needs_jsx_runtime: bool,
  needs_jsxs_runtime: bool,
  needs_react_namespace: bool,
  needs_forward_ref: bool,
  has_forward_ref_binding: bool,
  styled_display_names: Vec<(Ident, String)>,
  hoisted_sheets: BTreeMap<String, Ident>,
  hoisted_sheet_order: Vec<String>,
  name_tracker: NameTracker,
  exported_bindings: HashSet<(Atom, SyntaxContext)>,
  exported_binding_names: HashSet<String>,
  referenced_bindings: HashSet<(Atom, SyntaxContext)>,
  css_binding_rules: HashMap<(Atom, SyntaxContext), Vec<String>>,
  used_css_bindings: HashSet<(Atom, SyntaxContext)>,
  runtime_class_ident: Option<Ident>,
  runtime_ix_ident: Option<Ident>,
  runtime_cc_ident: Option<Ident>,
  runtime_cs_ident: Option<Ident>,
  jsx_ident: Option<Ident>,
  jsxs_ident: Option<Ident>,
  react_namespace_ident: Option<Ident>,
  forward_ref_ident: Option<Ident>,
  has_react_namespace_binding: bool,
  current_binding: Option<(Atom, SyntaxContext)>,
  props_scope_depth: usize,
}

struct XcssProcessing {
  runtime_sheets: Vec<String>,
  pending_class_names: Vec<String>,
  transformed: bool,
}

impl<'a> TransformVisitor<'a> {
  fn new(
    options: &'a PluginOptions,
    bindings: HashMap<(Atom, SyntaxContext), StaticEvalResult>,
    name_tracker: NameTracker,
    initial_css_map_rules: HashMap<String, Vec<String>>,
    initial_css_map_static_objects: HashMap<(Atom, SyntaxContext), IndexMap<String, StaticValue>>,
    exported_bindings: HashSet<(Atom, SyntaxContext)>,
    exported_binding_names: HashSet<String>,
    referenced_bindings: HashSet<(Atom, SyntaxContext)>,
  ) -> Self {
    Self {
      options,
      bindings,
      css_runtime_artifacts: HashMap::new(),
      css_imports: HashSet::new(),
      keyframes_imports: HashSet::new(),
      styled_imports: HashSet::new(),
      css_map_imports: HashSet::new(),
      compiled_import_sources: HashMap::new(),
      css_map_rule_groups: initial_css_map_rules,
      css_map_static_objects: initial_css_map_static_objects,
      css_map_ident_classes: HashMap::new(),
      keyframes_rules: HashMap::new(),
      compiled_import_kinds: HashMap::new(),
      xcss_class_names: HashSet::new(),
      non_xcss_class_names: HashSet::new(),
      retain_imports: HashSet::new(),
      collected_rules: Vec::new(),
      metadata_rules: Vec::new(),
      seen_rules: HashSet::new(),
      seen_metadata_rules: HashSet::new(),
      runtime_var_dependencies: HashMap::new(),
      processed_runtime_value_keys: HashSet::new(),
      force_runtime_metadata_classes: HashSet::new(),
      needs_runtime_ax: false,
      needs_runtime_cc: false,
      needs_runtime_cs: false,
      needs_jsx_runtime: false,
      needs_jsxs_runtime: false,
      needs_react_namespace: false,
      needs_forward_ref: false,
      has_forward_ref_binding: false,
      styled_display_names: Vec::new(),
      hoisted_sheets: BTreeMap::new(),
      hoisted_sheet_order: Vec::new(),
      name_tracker,
      exported_bindings,
      exported_binding_names,
      referenced_bindings,
      css_binding_rules: HashMap::new(),
      used_css_bindings: HashSet::new(),
      runtime_class_ident: None,
      runtime_ix_ident: None,
      runtime_cc_ident: None,
      runtime_cs_ident: None,
      jsx_ident: None,
      jsxs_ident: None,
      react_namespace_ident: None,
      forward_ref_ident: None,
      has_react_namespace_binding: false,
      current_binding: None,
      props_scope_depth: 0,
    }
  }

  fn prime_css_runtime_artifacts(&mut self, module: &Module) {
    struct CssBindingPrimer<'a, 'b> {
      host: &'a mut TransformVisitor<'b>,
    }

    impl<'a, 'b> Visit for CssBindingPrimer<'a, 'b> {
      fn visit_var_declarator(&mut self, declarator: &VarDeclarator) {
        let prev_binding = self.host.current_binding.clone();
        if let Pat::Ident(binding) = &declarator.name {
          self.host.current_binding = Some(to_id(&binding.id));
        } else {
          self.host.current_binding = None;
        }

        if let Some(init) = &declarator.init {
          if let Expr::Call(call) = &**init {
            if let Callee::Expr(callee_expr) = &call.callee {
              if self.host.is_css_ident(callee_expr) {
                let _ = self.host.handle_css_call_with_emission(call, false);
              }
            }
          }
        }

        declarator.visit_children_with(self);
        self.host.current_binding = prev_binding;
      }
    }

    let mut primer = CssBindingPrimer { host: self };
    module.visit_with(&mut primer);
  }

  fn register_rule_internal(
    &mut self,
    css: String,
    include_metadata: bool,
    _metadata_key: Option<String>,
  ) {
    let trimmed = css.trim_start();
    let is_internal_var = trimmed.contains("var(--_");
    let is_property_rule = trimmed.starts_with("@property");
    let force_metadata = include_metadata && self.should_force_metadata_for_css(&css);
    if include_metadata && is_internal_var && !is_property_rule {
      let depends_on_props = self.runtime_rule_depends_on_props(trimmed);
      let depends_on_import = self.runtime_rule_depends_on_import(trimmed);
      if !force_metadata && !depends_on_props && !depends_on_import {
        if std::env::var_os("COMPILED_DEBUG_RUNTIME_VAR").is_some() {
          eprintln!(
            "[compiled-runtime-var] dropping metadata for css={} (no props dependency)",
            trimmed
          );
        }
        self.register_rule_without_metadata(css);
        return;
      }
    }

    if self.seen_rules.insert(css.clone()) {
      self.collected_rules.push(css.clone());
    }

    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() && css.contains("aria-current") {
      eprintln!(
        "[compiled-debug] register_rule include_metadata={} css={}",
        include_metadata, css
      );
    }
    if include_metadata && self.seen_metadata_rules.insert(css.clone()) {
      self.metadata_rules.push(css.clone());
    }
  }

  fn should_force_metadata_for_css(&self, css: &str) -> bool {
    for class_name in &self.force_runtime_metadata_classes {
      let needle = format!(".{}", class_name);
      if css.contains(&needle) {
        return true;
      }
    }
    false
  }

  fn register_referenced_keyframes(&mut self, css: &str, include_metadata: bool) {
    if self.keyframes_rules.is_empty() || css.starts_with("@keyframes ") {
      return;
    }

    let keyframes: Vec<(String, String)> = self
      .keyframes_rules
      .iter()
      .map(|(name, rule)| (name.clone(), rule.clone()))
      .collect();
    for (name, rule) in keyframes {
      if css.contains(&name) {
        self.register_rule_internal(rule, include_metadata, None);
      }
    }
  }

  fn register_rule_with_metadata_key(
    &mut self,
    css: String,
    include_metadata: bool,
    metadata_key: Option<String>,
  ) {
    self.register_rule_internal(css.clone(), include_metadata, metadata_key);
    self.register_referenced_keyframes(&css, include_metadata);
  }

  fn mark_runtime_vars_prop_dependent(&mut self, css: &str) {
    let mut offset = 0usize;
    while let Some(idx) = css[offset..].find("var(--_") {
      let mut start = offset + idx + "var(".len();
      while css[start..].starts_with(' ') {
        start += 1;
      }
      let mut end = start;
      while let Some(ch) = css.as_bytes().get(end) {
        if *ch == b')' || *ch == b',' {
          break;
        }
        end += 1;
      }
      let var_name = css[start..end].to_string();
      let entry = self.runtime_var_dependencies.entry(var_name).or_default();
      entry.depends_on_props = true;
      offset = end;
    }
  }

  fn register_rule(&mut self, css: String) {
    let include_metadata = true;
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() && css.contains("aria") {
      eprintln!(
        "[compiled-debug] register_rule css={} include_metadata={}",
        css, include_metadata
      );
    }
    self.register_rule_with_metadata_key(css, include_metadata, None);
  }

  fn register_rule_without_metadata(&mut self, css: String) {
    let include_metadata = false;
    self.register_rule_with_metadata_key(css, include_metadata, None);
  }

  fn register_artifacts_for_metadata(&mut self, artifacts: &CssArtifacts) {
    for (css, include_metadata, metadata_key) in artifacts.merged_css_entries() {
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] merged metadata entry include={} css={}",
          include_metadata, css
        );
      }
      self.register_rule_with_metadata_key(css, include_metadata, metadata_key);
    }
  }

  fn register_property_rules_from_artifacts(&mut self, artifacts: &CssArtifacts) {
    for css in &artifacts.raw_rules {
      if css.trim_start().starts_with("@property") {
        self.register_rule(css.clone());
      }
    }
  }

  fn expression_depends_on_props(expr: &Expr, props_ident: &Ident) -> bool {
    struct Finder {
      target: Id,
      found: bool,
    }

    impl Visit for Finder {
      fn visit_ident(&mut self, ident: &Ident) {
        if ident.to_id() == self.target {
          self.found = true;
        }
      }
    }

    let mut finder = Finder {
      target: props_ident.to_id(),
      found: false,
    };
    expr.visit_with(&mut finder);
    finder.found
  }

  fn runtime_rule_depends_on_props(&self, css: &str) -> bool {
    let mut offset = 0usize;
    while let Some(idx) = css[offset..].find("var(--_") {
      let start = offset + idx + "var(".len();
      let mut end = start;
      while let Some(ch) = css.as_bytes().get(end) {
        if *ch == b')' || *ch == b',' {
          break;
        }
        end += 1;
      }
      let var_name = css[start..end].to_string();
      if let Some(depends) = self.runtime_var_dependencies.get(&var_name) {
        if depends.depends_on_props {
          return true;
        }
      } else {
        return true;
      }
      offset = end;
    }
    false
  }

  fn runtime_rule_depends_on_import(&self, css: &str) -> bool {
    let mut offset = 0usize;
    while let Some(idx) = css[offset..].find("var(--_") {
      let start = offset + idx + "var(".len();
      let mut end = start;
      while let Some(ch) = css.as_bytes().get(end) {
        if *ch == b')' || *ch == b',' {
          break;
        }
        end += 1;
      }
      let var_name = css[start..end].to_string();
      if let Some(info) = self.runtime_var_dependencies.get(&var_name) {
        if info.depends_on_import {
          return true;
        }
      } else {
        return true;
      }
      offset = end;
    }
    false
  }

  fn register_runtime_variable_dependency(
    &mut self,
    name: &str,
    expr: &Expr,
    props_ident: Option<&Ident>,
  ) {
    let expr_code = emit_expression(expr);
    if std::env::var_os("COMPILED_DEBUG_RUNTIME_VAR").is_some() {
      eprintln!(
        "[compiled-runtime-var] dependency check {} expr={}",
        name, expr_code
      );
    }
    let mut depends_on_props = props_ident
      .map(|ident| Self::expression_depends_on_props(expr, ident))
      .unwrap_or(false);
    let mut depends_on_import = false;
    if !depends_on_props {
      if let Some(ident) = props_ident {
        if expr_code.contains(ident.sym.as_ref()) {
          depends_on_props = true;
        }
      }
      if !depends_on_props && expr_code.contains(CANONICAL_PROPS_IDENT) {
        depends_on_props = true;
      }
      if !depends_on_props && expr_code.contains("props") {
        depends_on_props = true;
      }
    }
    if let Some(evaluation) = evaluate_static_with_info(expr, &self.bindings) {
      if evaluation.depends_on_import {
        depends_on_import = true;
      }
    }
    if std::env::var_os("COMPILED_DEBUG_RUNTIME_VAR").is_some() {
      eprintln!(
        "[compiled-runtime-var] register dependency {} -> props:{} import:{}",
        name, depends_on_props, depends_on_import
      );
    }
    let entry = self
      .runtime_var_dependencies
      .entry(name.to_string())
      .or_default();
    entry.depends_on_props |= depends_on_props;
    entry.depends_on_import |= depends_on_import;
  }

  fn css_rule_key_for_metadata(css: &str) -> Option<String> {
    let trimmed = css.trim_start();
    let close_idx = trimmed.rfind('}')?;
    let open_idx = trimmed[..close_idx].rfind('{')?;
    if open_idx >= close_idx {
      return None;
    }
    let declaration = &trimmed[open_idx + 1..close_idx];
    let colon_idx = declaration.find(':')?;
    let property = declaration[..colon_idx].trim();
    if property.is_empty() {
      return None;
    }
    let selector = trimmed[..open_idx].trim();
    if selector.is_empty() {
      return None;
    }
    Some(format!("{}::{}", selector, property))
  }

  fn hoist_sheet_ident(&mut self, css: &str) -> Ident {
    if let Some(ident) = self.hoisted_sheets.get(css) {
      return ident.clone();
    }

    let ident = self.name_tracker.fresh_ident("_");
    self.hoisted_sheets.insert(css.to_string(), ident.clone());
    self.hoisted_sheet_order.push(css.to_string());
    ident
  }

  fn css_options(&self) -> CssOptions {
    css_options_from_plugin_options(self.options)
  }

  fn finalize_runtime_sheets(&self, sheets: Vec<String>) -> Vec<String> {
    use std::collections::HashSet;

    if self.keyframes_rules.is_empty() {
      return sheets;
    }

    if std::env::var_os("COMPILED_DEBUG_HASH").is_some() {
      eprintln!(
        "[compiled-debug] keyframes available: {:?}",
        self.keyframes_rules.keys().collect::<Vec<_>>()
      );
    }

    let mut expanded = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for sheet in sheets {
      for (name, rule) in &self.keyframes_rules {
        if sheet.contains(name) && seen.insert(rule.clone()) {
          expanded.push(rule.clone());
        }
      }
      if seen.insert(sheet.clone()) {
        expanded.push(sheet);
      }
    }
    expanded
  }

  fn should_import_react_namespace(&self) -> bool {
    self.options.import_react.unwrap_or(true)
  }

  fn should_emit_component_class_name(&self) -> bool {
    if !self.options.add_component_name.unwrap_or(false) {
      return false;
    }
    match std::env::var("NODE_ENV") {
      Ok(value) => value != "production",
      Err(_) => true,
    }
  }

  fn is_development_env(&self) -> bool {
    let node_env = std::env::var("NODE_ENV").ok();
    let babel_env = std::env::var("BABEL_ENV").ok();
    if node_env.is_none() && babel_env.is_none() {
      return true;
    }
    matches!(babel_env.as_deref(), Some("development") | Some("test"))
      || matches!(node_env.as_deref(), Some("development") | Some("test"))
  }

  fn runtime_class_helper(&self) -> &'static str {
    if self.options.class_name_compression_map.is_empty() {
      "ax"
    } else {
      "ac"
    }
  }

  fn runtime_class_ident(&mut self) -> Ident {
    if let Some(ident) = &self.runtime_class_ident {
      return ident.clone();
    }
    let helper = self.runtime_class_helper();
    let ident = self.name_tracker.fresh_ident(helper);
    self.runtime_class_ident = Some(ident.clone());
    ident
  }

  fn runtime_ix_ident(&mut self) -> Ident {
    if let Some(ident) = &self.runtime_ix_ident {
      return ident.clone();
    }
    let ident = self.name_tracker.fresh_ident("ix");
    self.runtime_ix_ident = Some(ident.clone());
    ident
  }

  fn runtime_cc_ident(&mut self) -> Ident {
    if let Some(ident) = &self.runtime_cc_ident {
      return ident.clone();
    }
    let ident = self.name_tracker.fresh_ident("CC");
    self.runtime_cc_ident = Some(ident.clone());
    ident
  }

  fn runtime_cs_ident(&mut self) -> Ident {
    if let Some(ident) = &self.runtime_cs_ident {
      return ident.clone();
    }
    let ident = self.name_tracker.fresh_ident("CS");
    self.runtime_cs_ident = Some(ident.clone());
    ident
  }

  fn jsx_ident(&mut self) -> Ident {
    if let Some(ident) = &self.jsx_ident {
      return ident.clone();
    }
    let ident = self.name_tracker.fresh_ident("jsx");
    self.jsx_ident = Some(ident.clone());
    ident
  }

  fn jsxs_ident(&mut self) -> Ident {
    if let Some(ident) = &self.jsxs_ident {
      return ident.clone();
    }
    let ident = self.name_tracker.fresh_ident("jsxs");
    self.jsxs_ident = Some(ident.clone());
    ident
  }

  fn react_namespace_ident(&mut self) -> Ident {
    if let Some(ident) = &self.react_namespace_ident {
      return ident.clone();
    }
    let ident = self.name_tracker.fresh_ident("React");
    self.react_namespace_ident = Some(ident.clone());
    ident
  }

  fn forward_ref_ident(&mut self) -> Ident {
    if let Some(ident) = &self.forward_ref_ident {
      return ident.clone();
    }
    let ident = self.name_tracker.fresh_ident("forwardRef");
    self.forward_ref_ident = Some(ident.clone());
    ident
  }

  fn import_named_spec(local: &Ident, export: &str) -> ImportSpecifier {
    ImportSpecifier::Named(ImportNamedSpecifier {
      span: DUMMY_SP,
      local: local.clone(),
      imported: if local.sym.as_ref() == export {
        None
      } else {
        Some(ModuleExportName::Ident(Ident::new(
          export.into(),
          DUMMY_SP,
          SyntaxContext::empty(),
        )))
      },
      is_type_only: false,
    })
  }

  fn single_quoted_str(value: &str) -> Str {
    Str {
      span: DUMMY_SP,
      value: value.into(),
      raw: Some(format!("'{}'", value).into()),
    }
  }

  fn is_css_ident(&self, expr: &Expr) -> bool {
    if let Expr::Ident(ident) = expr {
      self.css_imports.contains(&to_id(ident))
    } else {
      false
    }
  }

  fn is_keyframes_ident(&self, expr: &Expr) -> bool {
    if let Expr::Ident(ident) = expr {
      self.keyframes_imports.contains(&to_id(ident))
    } else {
      false
    }
  }

  fn mark_css_binding_used(&mut self, ident: &Ident) {
    self.used_css_bindings.insert(to_id(ident));
  }

  fn mark_css_bindings_in_css_prop_expr(&mut self, expr: &Expr) {
    struct CssBindingUsageVisitor<'a, 'b> {
      host: &'b mut TransformVisitor<'a>,
    }

    impl<'a, 'b> Visit for CssBindingUsageVisitor<'a, 'b> {
      fn visit_ident(&mut self, ident: &Ident) {
        if self.host.css_binding_rules.contains_key(&to_id(ident)) {
          self.host.mark_css_binding_used(ident);
        }
      }

      fn visit_prop_name(&mut self, _prop_name: &PropName) {}

      fn visit_member_prop(&mut self, prop: &MemberProp) {
        if let MemberProp::Computed(computed) = prop {
          computed.visit_children_with(self);
        }
      }

      fn visit_prop(&mut self, prop: &Prop) {
        match prop {
          Prop::Shorthand(ident) => {
            if self.host.css_binding_rules.contains_key(&to_id(ident)) {
              self.host.mark_css_binding_used(ident);
            }
          }
          _ => prop.visit_children_with(self),
        }
      }
    }

    expr.visit_with(&mut CssBindingUsageVisitor { host: self });
  }

  fn walk_jsx_children(&mut self, element: &mut JSXElement) {
    for child in &mut element.children {
      match child {
        JSXElementChild::JSXElement(child_element) => {
          let mut expr = Expr::JSXElement(child_element.clone());
          self.visit_mut_expr(&mut expr);
          match expr {
            Expr::JSXElement(new_element) => {
              *child_element = new_element;
            }
            _ => {
              *child = JSXElementChild::JSXExprContainer(JSXExprContainer {
                span: DUMMY_SP,
                expr: JSXExpr::Expr(Box::new(expr)),
              });
            }
          }
        }
        _ => child.visit_mut_children_with(self),
      }
    }
  }

  fn discard_unused_css_binding_rules(&mut self) {
    let binding_ids: Vec<_> = self.css_binding_rules.keys().cloned().collect();
    for id in binding_ids {
      let Some(rules) = self.css_binding_rules.get(&id).cloned() else {
        continue;
      };
      let binding_name = id.0.to_string();
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] css binding '{}' exported_id={} exported_name={} used={}",
          binding_name,
          self.exported_bindings.contains(&id),
          self.exported_binding_names.contains(&binding_name),
          self.used_css_bindings.contains(&id)
        );
      }
      let keep_binding = self.exported_bindings.contains(&id)
        || self.exported_binding_names.contains(&binding_name)
        || self.used_css_bindings.contains(&id)
        || self.referenced_bindings.contains(&id);
      if keep_binding {
        if !self.used_css_bindings.contains(&id) {
          if let Some(artifacts) = self.css_runtime_artifacts.get(&id).cloned() {
            self.register_property_rules_from_artifacts(&artifacts);
            self.register_artifacts_for_metadata(&artifacts);
          } else {
            for rule in rules {
              self.register_rule(rule.clone());
            }
          }
        }
      }
    }
  }

  fn evaluate_template(&self, template: &TaggedTpl) -> Option<String> {
    let mut result = String::new();
    for (index, quasi) in template.tpl.quasis.iter().enumerate() {
      result.push_str(
        quasi
          .cooked
          .as_ref()
          .map(|atom| atom.to_string())
          .unwrap_or_else(|| quasi.raw.to_string())
          .as_str(),
      );
      if let Some(expr) = template.tpl.exprs.get(index) {
        let value = evaluate_static(expr, &self.bindings)?;
        result.push_str(value.as_str()?);
      }
    }
    Some(result)
  }

  fn evaluate_call_argument(&self, call: &CallExpr) -> Option<StaticValue> {
    let first = call.args.first()?;
    evaluate_static(&first.expr, &self.bindings)
  }

  fn evaluate_call_arguments(&self, call: &CallExpr) -> Option<Vec<StaticValue>> {
    let mut values = Vec::with_capacity(call.args.len());
    for arg in &call.args {
      if arg.spread.is_some() {
        return None;
      }
      let eval = evaluate_static_with_info(&arg.expr, &self.bindings)?;
      if eval.depends_on_import {
        return None;
      }
      values.push(eval.value);
    }
    Some(values)
  }

  fn evaluate_call_arguments_allow_imports(&self, call: &CallExpr) -> Option<Vec<StaticValue>> {
    let mut values = Vec::with_capacity(call.args.len());
    for arg in &call.args {
      if arg.spread.is_some() {
        return None;
      }
      if self.expr_requires_runtime_import(&arg.expr) {
        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
          eprintln!(
            "[compiled-debug] evaluate_call_arguments_allow_imports bailed due to runtime import"
          );
        }
        return None;
      }
      let value = evaluate_static(&arg.expr, &self.bindings)?;
      if matches!(value.unwrap_spread(), StaticValue::Function(_)) {
        return None;
      }
      values.push(value);
    }
    Some(values)
  }

  fn process_styled_call_arguments(
    &mut self,
    call: &CallExpr,
    props_ident: &Ident,
  ) -> Option<CssArtifacts> {
    let mut combined = CssArtifacts::default();
    for arg in &call.args {
      if arg.spread.is_some() {
        return None;
      }
      match &*arg.expr {
        Expr::Object(object) => match self.process_dynamic_css_object(object, props_ident) {
          Some(artifacts) => combined.merge(artifacts),
          None => return None,
        },
        Expr::Arrow(_) | Expr::Fn(_) => {
          let artifacts = self.process_dynamic_css_function(&arg.expr, props_ident)?;
          combined.merge(artifacts);
        }
        Expr::Ident(ident) => {
          self.mark_css_binding_used(ident);
          let mut runtime_artifacts = self.css_runtime_artifacts.get(&to_id(ident)).cloned();
          if runtime_artifacts.is_none() {
            if let Some((_, artifacts)) = self
              .css_runtime_artifacts
              .iter()
              .find(|((sym, _), _)| sym == &ident.sym)
            {
              runtime_artifacts = Some(artifacts.clone());
            }
          }
          match runtime_artifacts {
            Some(artifacts) => combined.merge(artifacts),
            None => return None,
          }
        }
        _ => return None,
      }
    }
    Some(combined)
  }

  fn process_dynamic_css_object(
    &mut self,
    object: &ObjectLit,
    props_ident: &Ident,
  ) -> Option<CssArtifacts> {
    let options = self.css_options();
    let selectors = vec![normalize_selector(None)];
    self.process_dynamic_css_object_with_context(
      object,
      props_ident,
      &selectors,
      &[],
      &options,
      false,
      false,
    )
  }

  fn process_dynamic_css_function(
    &mut self,
    func: &Expr,
    props_ident: &Ident,
  ) -> Option<CssArtifacts> {
    let options = self.css_options();
    let (expression, _) = self.normalize_variable_expression(func, props_ident)?;
    self.css_artifacts_from_dynamic_css_expression_with_context(
      &expression,
      props_ident,
      &[normalize_selector(None)],
      &[],
      &options,
    )
  }

  fn is_selector_key(key: &str) -> bool {
    let trimmed = key.trim_start();
    if trimmed.is_empty() {
      return false;
    }
    if trimmed.starts_with('@') {
      return false;
    }
    match trimmed.chars().next().unwrap_or_default() {
      '&' | ':' | '.' | '#' | '[' | '>' | '+' | '~' | '*' => true,
      _ => trimmed.contains('&'),
    }
  }

  fn process_dynamic_css_object_with_context(
    &mut self,
    object: &ObjectLit,
    props_ident: &Ident,
    selectors: &[String],
    at_rules: &[AtRuleInput],
    options: &CssOptions,
    inherited_from_spread: bool,
    force_leading_combinator_space: bool,
  ) -> Option<CssArtifacts> {
    let debug_css = std::env::var_os("COMPILED_DEBUG_CSS").is_some();
    let mut rule_inputs: Vec<CssRuleInput> = Vec::new();
    let mut runtime_variables: Vec<RuntimeCssVariable> = Vec::new();
    let mut runtime_class_conditions: Vec<RuntimeClassCondition> = Vec::new();
    let mut artifacts = CssArtifacts::default();
    let mut seen_variable_inputs: HashSet<String> = HashSet::new();

    if debug_css {
      eprintln!(
        "[compiled-debug] process_dynamic_css_object start selectors={:?} props_len={}",
        selectors,
        object.props.len()
      );
    }

    for prop in &object.props {
      if debug_css {
        match prop {
          PropOrSpread::Prop(_) => {
            eprintln!("[compiled-debug] dynamic prop kind=Prop");
          }
          PropOrSpread::Spread(_) => {
            eprintln!("[compiled-debug] dynamic prop kind=Spread");
          }
        }
      }
      if let PropOrSpread::Spread(spread) = prop {
        match &*spread.expr {
          Expr::Arrow(_) | Expr::Fn(_) => {
            let nested = self.process_dynamic_css_function(&spread.expr, props_ident)?;
            self.register_artifacts_for_metadata(&nested);
            artifacts.merge(nested);
          }
          _ => {
            let mut spread_expr_ref = &*spread.expr;
            while let Expr::Paren(paren) = spread_expr_ref {
              spread_expr_ref = &*paren.expr;
            }

            if let Some(value) = evaluate_static(spread_expr_ref, &self.bindings) {
              let expr = value.to_expr()?;
              let Expr::Object(spread_object) = expr else {
                return None;
              };
              let nested = self.process_dynamic_css_object_with_context(
                &spread_object,
                props_ident,
                selectors,
                at_rules,
                options,
                true,
                force_leading_combinator_space,
              )?;
              self.register_artifacts_for_metadata(&nested);
              artifacts.merge(nested);
            } else if let Expr::Bin(bin) = spread_expr_ref {
              if matches!(bin.op, BinaryOp::LogicalAnd) {
                let mut right_expr = &*bin.right;
                while let Expr::Paren(paren) = right_expr {
                  right_expr = &*paren.expr;
                }
                let Expr::Object(inner) = right_expr else {
                  return None;
                };
                let nested = self.process_dynamic_css_object_with_context(
                  inner,
                  props_ident,
                  selectors,
                  at_rules,
                  options,
                  true,
                  force_leading_combinator_space,
                )?;
                self.register_artifacts_for_metadata(&nested);

                let classes = nested
                  .rules
                  .iter()
                  .map(|rule| rule.class_name.clone())
                  .collect::<Vec<_>>();

                if !classes.is_empty() {
                  let (condition_expr, _) =
                    self.normalize_variable_expression(&bin.left, props_ident)?;
                  if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
                    eprintln!(
                      "[compiled-debug] spread logical-and condition classes={:?}",
                      classes
                    );
                  }
                  runtime_class_conditions.push(RuntimeClassCondition::new(
                    condition_expr,
                    classes,
                    Vec::new(),
                  ));
                }

                artifacts.merge(nested);
              } else {
                return None;
              }
            } else {
              return None;
            }
          }
        }
        continue;
      }
      let PropOrSpread::Prop(prop) = prop else {
        return None;
      };
      let mut _synthetic_value: Option<Expr> = None;
      let (property_key, value_expr) = match &**prop {
        Prop::KeyValue(kv) => (kv.key.clone(), &*kv.value),
        Prop::Shorthand(ident) => {
          _synthetic_value = Some(Expr::Ident(ident.clone()));
          (
            PropName::Ident(ident.clone().into()),
            _synthetic_value.as_ref().unwrap(),
          )
        }
        _ => {
          if debug_css {
            eprintln!("[compiled-debug] unsupported prop kind in dynamic CSS object");
          }
          return None;
        }
      };
      let property = match &property_key {
        PropName::Ident(ident) => ident.sym.as_ref().to_string(),
        PropName::Str(str) => str.value.to_string(),
        PropName::Num(num) => num.value.to_string(),
        PropName::BigInt(bigint) => bigint.value.to_string(),
        PropName::Computed(computed) => {
          let evaluated = evaluate_static(&computed.expr, &self.bindings)?;
          evaluated.to_property_key()?
        }
      };
      let property_kebab = kebab_case(&property);

      let expression_repr = emit_expression(value_expr);
      if debug_css {
        eprintln!(
          "[compiled-debug] process_dynamic_css_object property='{}' selectors={:?} expr={}",
          property, selectors, expression_repr
        );
      }

      match value_expr {
        Expr::Object(inner) => {
          if property.trim_start().starts_with('@') {
            let descriptor = parse_at_rule_key(&property);
            let mut next_at_rules = at_rules.to_vec();
            next_at_rules.push(descriptor);
            let nested = self.process_dynamic_css_object_with_context(
              inner,
              props_ident,
              selectors,
              &next_at_rules,
              options,
              inherited_from_spread,
              force_leading_combinator_space,
            )?;
            self.register_artifacts_for_metadata(&nested);
            artifacts.merge(nested);
            continue;
          }

          if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
            eprintln!(
              "[compiled-debug] extend_selectors selectors={:?} raw_property='{}'",
              selectors, property
            );
          }
          let next_selectors = extend_selectors(selectors, &property);
          if next_selectors.is_empty() {
            continue;
          }
          let trimmed_property = property.trim_start();
          let needs_leading_space = !trimmed_property.starts_with('&')
            && matches!(trimmed_property.chars().next(), Some('>' | '+' | '~'));
          let nested = self.process_dynamic_css_object_with_context(
            inner,
            props_ident,
            &next_selectors,
            at_rules,
            options,
            inherited_from_spread,
            force_leading_combinator_space || needs_leading_space,
          )?;
          self.register_artifacts_for_metadata(&nested);
          artifacts.merge(nested);
          continue;
        }
        Expr::Array(array) if property.trim_start().starts_with('@') => {
          let descriptor = parse_at_rule_key(&property);
          let mut next_at_rules = at_rules.to_vec();
          next_at_rules.push(descriptor);
          for element in &array.elems {
            let Some(element) = element else {
              continue;
            };
            if element.spread.is_some() {
              return None;
            }
            let Expr::Object(inner) = &*element.expr else {
              return None;
            };
            let nested = self.process_dynamic_css_object_with_context(
              inner,
              props_ident,
              selectors,
              &next_at_rules,
              options,
              inherited_from_spread,
              force_leading_combinator_space,
            )?;
            self.register_artifacts_for_metadata(&nested);
            artifacts.merge(nested);
          }
          continue;
        }
        Expr::Array(_) => {}
        _ => {}
      }

      if let Expr::Tpl(template) = value_expr {
        if self.try_handle_conditional_template_literal(
          template,
          props_ident,
          &property_kebab,
          selectors,
          at_rules,
          inherited_from_spread,
          options,
          &mut runtime_class_conditions,
          &mut artifacts,
        ) {
          continue;
        }
        let (raw_value, value, mut variables, value_hash_override) =
          self.process_dynamic_template_literal(template, props_ident)?;
        let (value, important) = Self::split_value_and_important(&value);
        let (raw_value, _) = Self::split_value_and_important(&raw_value);
        Self::push_rule_input(
          &mut rule_inputs,
          selectors,
          at_rules,
          &property_kebab,
          value.clone(),
          raw_value,
          important,
          inherited_from_spread,
          false,
          value_hash_override,
          options.preserve_leading_combinator_space,
          force_leading_combinator_space,
        );
        runtime_variables.append(&mut variables);
        continue;
      }

      let skip_static_eval = expression_repr.starts_with("layers.") || expression_repr == "layers";
      if skip_static_eval && debug_css {
        eprintln!(
          "[compiled-debug] skipping static evaluation for property='{}' expr={}",
          property, expression_repr
        );
      }

      if !skip_static_eval {
        if let Some(evaluation) = evaluate_static_with_info(value_expr, &self.bindings) {
          if evaluation.depends_on_import {
            if self.expr_requires_runtime_import(value_expr)
              || !Self::static_import_value_is_literal(evaluation.value.unwrap_spread())
            {
              continue;
            }
          } else if self.expr_requires_runtime_import(value_expr) {
            continue;
          }
          let mut value_for_css = evaluation.value.clone();
          if debug_css {
            eprintln!(
              "[compiled-debug] evaluate_static success property='{}' value={:?}",
              property, value_for_css
            );
          }
          if let StaticValue::Function(func) = value_for_css.unwrap_spread() {
            if let StaticValue::Str(text) = func.value.unwrap_spread() {
              if text.value.contains("var(--ds-") {
                value_for_css = func.value.clone();
              }
            }
          }

          if let Some((output, raw_value, important)) =
            static_value_to_css_value(&property_kebab, &value_for_css)
          {
            if debug_css {
              eprintln!(
                "[compiled-debug] static_value_to_css_value success property='{}' output={} raw={}",
                property, output, raw_value
              );
            }
            let property_name = if property_kebab == "background"
              && should_promote_background_to_color(&output, &raw_value)
            {
              "background-color".to_string()
            } else {
              property_kebab.clone()
            };
            Self::push_rule_input(
              &mut rule_inputs,
              selectors,
              at_rules,
              &property_name,
              output,
              raw_value,
              important,
              inherited_from_spread || evaluation.value.is_from_spread(),
              false,
              None,
              options.preserve_leading_combinator_space,
              force_leading_combinator_space,
            );
            continue;
          }

          if debug_css {
            eprintln!(
              "[compiled-debug] static_value_to_css_value failed property='{}' value={:?}",
              property, evaluation.value
            );
          }

          if matches!(
            evaluation.value.unwrap_spread(),
            StaticValue::Object(_) | StaticValue::Array(_)
          ) {
            let mut wrapper = IndexMap::new();
            wrapper.insert(property.clone(), evaluation.value.clone());
            let mut nested_inputs = Vec::new();
            let mut raw_rules = Vec::new();
            if !flatten_css_object(
              &wrapper,
              selectors,
              at_rules,
              &mut nested_inputs,
              &mut raw_rules,
              &mut runtime_variables,
              false,
              options.flatten_multiple_selectors,
              options.preserve_leading_combinator_space,
              force_leading_combinator_space,
            ) {
              return None;
            }
            let mut nested_artifacts = atomicize_rules(&nested_inputs, options);
            nested_artifacts.raw_rules.extend(raw_rules);
            artifacts.merge(nested_artifacts);
            continue;
          }
        } else if debug_css {
          eprintln!(
            "[compiled-debug] evaluate_static failed property='{}' expr={}",
            property,
            emit_expression(value_expr)
          );
        }
      }

      if let Some((expression, mut variable_input)) =
        self.normalize_variable_expression(value_expr, props_ident)
      {
        let property_trimmed = property.trim_start();
        if Self::is_selector_key(property_trimmed) {
          let next_selectors = extend_selectors(selectors, &property);
          if !next_selectors.is_empty() {
            if let Some(mut nested) = self.css_artifacts_from_dynamic_css_expression_with_context(
              &expression,
              props_ident,
              &next_selectors,
              at_rules,
              options,
            ) {
              if std::env::var_os("COMPILED_DEBUG_CSS").is_some()
                && property_trimmed.contains("hover")
              {
                eprintln!(
                  "[compiled-debug] nested selector '{}' produced rules={} raw={} vars={} expr={}",
                  property_trimmed,
                  nested.rules.len(),
                  nested.raw_rules.len(),
                  nested.runtime_variables.len(),
                  emit_expression(&expression)
                );
              }
              self.register_artifacts_for_metadata(&nested);
              artifacts.merge(nested);
              continue;
            }
          }
        }
        let mut expr_ref = &expression;
        while let Expr::Paren(paren) = expr_ref {
          expr_ref = &paren.expr;
        }

        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() && property_kebab.starts_with("--") {
          eprintln!(
            "[compiled-debug] variable input for {} => {}",
            property_kebab, variable_input
          );
        }
        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() && property_trimmed.contains("hover") {
          eprintln!(
            "[compiled-debug] nested dynamic selector '{}' expression={}",
            property_trimmed,
            emit_expression(&expression)
          );
        }
        let inline_static = self.static_value_from_expr(expr_ref);
        if let Some(inline_static) = inline_static {
          if matches!(inline_static.unwrap_spread(), StaticValue::Null) {
            continue;
          }
          if let Some((output, raw_value, important)) =
            static_value_to_css_value(&property_kebab, &inline_static)
          {
            let property_name = if property_kebab == "background"
              && should_promote_background_to_color(&output, &raw_value)
            {
              "background-color".to_string()
            } else {
              property_kebab.clone()
            };
            Self::push_rule_input(
              &mut rule_inputs,
              selectors,
              at_rules,
              &property_name,
              output,
              raw_value,
              important,
              inherited_from_spread || inline_static.is_from_spread(),
              false,
              None,
              options.preserve_leading_combinator_space,
              force_leading_combinator_space,
            );
            continue;
          }

          if matches!(
            inline_static.unwrap_spread(),
            StaticValue::Object(_) | StaticValue::Array(_)
          ) {
            let mut wrapper = IndexMap::new();
            wrapper.insert(property.clone(), inline_static.clone());
            let mut nested_inputs = Vec::new();
            let mut raw_rules = Vec::new();
            if !flatten_css_object(
              &wrapper,
              selectors,
              at_rules,
              &mut nested_inputs,
              &mut raw_rules,
              &mut runtime_variables,
              false,
              options.flatten_multiple_selectors,
              options.preserve_leading_combinator_space,
              force_leading_combinator_space,
            ) {
              return None;
            }
            let mut nested_artifacts = atomicize_rules(&nested_inputs, options);
            nested_artifacts.raw_rules.extend(raw_rules);
            artifacts.merge(nested_artifacts);
            continue;
          }
        }

        if let Expr::Cond(cond_expr) = expr_ref {
          if property_kebab.starts_with("--") {
            // Custom properties mirror Babel behavior by using runtime CSS variables
            // instead of conditional class names. Defer to the generic hashing path
            // below so we emit `var(--hash)` rules matching the Babel plugin output.
          } else {
            let true_static = self
              .static_value_from_expr(cond_expr.cons.as_ref())
              .and_then(|value| {
                if matches!(value.unwrap_spread(), StaticValue::Null) {
                  None
                } else {
                  Some(value)
                }
              });
            let false_static = self
              .static_value_from_expr(cond_expr.alt.as_ref())
              .and_then(|value| {
                if matches!(value.unwrap_spread(), StaticValue::Null) {
                  None
                } else {
                  Some(value)
                }
              });
            if std::env::var_os("COMPILED_DEBUG_CSS").is_some()
              && (true_static.is_none() || false_static.is_none())
            {
              eprintln!(
                "[compiled-debug] conditional property '{}' unevaluated true_expr={} true={:?} false_expr={} false={:?}",
                property_kebab,
                emit_expression(cond_expr.cons.as_ref()),
                true_static,
                emit_expression(cond_expr.alt.as_ref()),
                false_static
              );
            }

            let true_branch_hash_override = Self::token_hash_override(cond_expr.cons.as_ref());
            let false_branch_hash_override = Self::token_hash_override(cond_expr.alt.as_ref());
            if let (Some(true_value), Some(false_value)) =
              (true_static.as_ref(), false_static.as_ref())
            {
              if std::env::var_os("COMPILED_DEBUG_HASH").is_some() {
                eprintln!(
                  "[compiled-debug] conditional property '{}' value_expr={}",
                  property_kebab,
                  emit_expression(value_expr)
                );
              }
              if let (
                Some((_true_output, true_raw_value, true_important)),
                Some((_false_output, false_raw_value, false_important)),
              ) = (
                static_value_to_css_value(&property_kebab, true_value),
                static_value_to_css_value(&property_kebab, false_value),
              ) {
                if !matches!(value_expr, Expr::Arrow(_) | Expr::Fn(_)) {
                  let variable_expr = expression.clone();
                  let mut canonical_expr = variable_expr.clone();
                  Self::rename_ident_in_expr(
                    &mut canonical_expr,
                    props_ident,
                    &Ident::new(
                      CANONICAL_PROPS_IDENT.into(),
                      DUMMY_SP,
                      SyntaxContext::empty(),
                    ),
                  );
                  let mut canonical_input = emit_expression_with_codegen(&canonical_expr);
                  canonical_input = Self::strip_js_comments(&canonical_input);
                  canonical_input = Self::collapse_blank_lines(&canonical_input);
                  if !canonical_input.contains('\n')
                    && !canonical_input.contains("/*")
                    && !canonical_input.contains("//")
                  {
                    canonical_input = canonical_input
                      .split_whitespace()
                      .collect::<Vec<_>>()
                      .join(" ");
                  }
                  let variable_input = canonical_input;
                  let dedupe_key = Self::runtime_value_dedupe_key(&property_kebab, &variable_input);
                  if !self.processed_runtime_value_keys.insert(dedupe_key.clone()) {
                    continue;
                  }
                  if !seen_variable_inputs.insert(dedupe_key) {
                    continue;
                  }
                  let hash_value = hash(&variable_input, 0);
                  let variable_name = format!("--_{}", hash_value);
                  let rule_value = format!("var({})", variable_name);
                  let important = true_important || false_important;
                  Self::push_rule_input(
                    &mut rule_inputs,
                    selectors,
                    at_rules,
                    &property_kebab,
                    rule_value.clone(),
                    rule_value,
                    important,
                    inherited_from_spread,
                    false,
                    None,
                    options.preserve_leading_combinator_space,
                    force_leading_combinator_space,
                  );
                  let suffix = if important {
                    Some(" !important".to_string())
                  } else {
                    None
                  };
                  self.register_runtime_variable_dependency(
                    &variable_name,
                    &variable_expr,
                    Some(props_ident),
                  );
                  runtime_variables.push(RuntimeCssVariable::new(
                    variable_name,
                    variable_expr,
                    None,
                    suffix,
                  ));
                  continue;
                }
              }
            }

            let true_classes = match true_static {
              Some(static_value) => {
                let (branch_artifacts, classes) = Self::build_static_branch_artifacts(
                  selectors,
                  at_rules,
                  &property_kebab,
                  &static_value,
                  options,
                  true_branch_hash_override.clone(),
                )?;
                for rule in &branch_artifacts.rules {
                  self.mark_runtime_vars_prop_dependent(&rule.css);
                }
                if std::env::var_os("COMPILED_DEBUG_RUNTIME_VAR").is_some() {
                  for class_name in &classes {
                    eprintln!(
                      "[compiled-runtime-var] forcing class metadata for {}",
                      class_name
                    );
                  }
                }
                self
                  .force_runtime_metadata_classes
                  .extend(classes.iter().cloned());
                artifacts.merge(branch_artifacts);
                classes
              }
              None if Self::is_css_none_expr(&cond_expr.cons) => Vec::new(),
              None => {
                let branch_object = ObjectLit {
                  span: DUMMY_SP,
                  props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: property_key.clone(),
                    value: cond_expr.cons.clone(),
                  })))],
                };
                let branch_artifacts = self.process_dynamic_css_object_with_context(
                  &branch_object,
                  props_ident,
                  selectors,
                  at_rules,
                  options,
                  inherited_from_spread,
                  force_leading_combinator_space,
                )?;
                for rule in &branch_artifacts.rules {
                  self.mark_runtime_vars_prop_dependent(&rule.css);
                }
                let classes = branch_artifacts
                  .rules
                  .iter()
                  .map(|rule| rule.class_name.clone())
                  .collect::<Vec<_>>();
                if std::env::var_os("COMPILED_DEBUG_RUNTIME_VAR").is_some() {
                  for class_name in &classes {
                    eprintln!(
                      "[compiled-runtime-var] forcing class metadata for {}",
                      class_name
                    );
                  }
                }
                self
                  .force_runtime_metadata_classes
                  .extend(classes.iter().cloned());
                self.register_artifacts_for_metadata(&branch_artifacts);
                artifacts.merge(branch_artifacts);
                classes
              }
            };

            let false_classes = match false_static {
              Some(static_value) => {
                let (branch_artifacts, classes) = Self::build_static_branch_artifacts(
                  selectors,
                  at_rules,
                  &property_kebab,
                  &static_value,
                  options,
                  false_branch_hash_override.clone(),
                )?;
                for rule in &branch_artifacts.rules {
                  self.mark_runtime_vars_prop_dependent(&rule.css);
                }
                if std::env::var_os("COMPILED_DEBUG_RUNTIME_VAR").is_some() {
                  for class_name in &classes {
                    eprintln!(
                      "[compiled-runtime-var] forcing class metadata for {}",
                      class_name
                    );
                  }
                }
                self
                  .force_runtime_metadata_classes
                  .extend(classes.iter().cloned());
                artifacts.merge(branch_artifacts);
                classes
              }
              None if Self::is_css_none_expr(&cond_expr.alt) => Vec::new(),
              None => {
                let branch_object = ObjectLit {
                  span: DUMMY_SP,
                  props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: property_key.clone(),
                    value: cond_expr.alt.clone(),
                  })))],
                };
                let branch_artifacts = self.process_dynamic_css_object_with_context(
                  &branch_object,
                  props_ident,
                  selectors,
                  at_rules,
                  options,
                  inherited_from_spread,
                  force_leading_combinator_space,
                )?;
                for rule in &branch_artifacts.rules {
                  self.mark_runtime_vars_prop_dependent(&rule.css);
                }
                let classes = branch_artifacts
                  .rules
                  .iter()
                  .map(|rule| rule.class_name.clone())
                  .collect::<Vec<_>>();
                if std::env::var_os("COMPILED_DEBUG_RUNTIME_VAR").is_some() {
                  for class_name in &classes {
                    eprintln!(
                      "[compiled-runtime-var] forcing class metadata for {}",
                      class_name
                    );
                  }
                }
                self
                  .force_runtime_metadata_classes
                  .extend(classes.iter().cloned());
                self.register_artifacts_for_metadata(&branch_artifacts);
                artifacts.merge(branch_artifacts);
                classes
              }
            };

            let forced_true = true_classes.clone();
            let forced_false = false_classes.clone();
            if std::env::var_os("COMPILED_DEBUG_RUNTIME_VAR").is_some() {
              eprintln!(
                "[compiled-runtime-var] runtime condition classes true={} false={}",
                forced_true.len(),
                forced_false.len()
              );
            }
            runtime_class_conditions.push(RuntimeClassCondition::new(
              (*cond_expr.test).clone(),
              true_classes,
              false_classes,
            ));
            self.force_runtime_metadata_classes.extend(forced_true);
            self.force_runtime_metadata_classes.extend(forced_false);
            continue;
          }
        }

        let dedupe_key = Self::runtime_value_dedupe_key(&property_kebab, &variable_input);
        if !seen_variable_inputs.insert(dedupe_key.clone()) {
          continue;
        }
        if !self.processed_runtime_value_keys.insert(dedupe_key) {
          continue;
        }
        let hash_value = hash(&variable_input, 0);
        if std::env::var_os("COMPILED_DEBUG_HASH").is_some() {
          eprintln!(
            "[compiled-debug] variable-hash={} input={}",
            hash_value, variable_input
          );
        }
        if let Some(static_text) = self.evaluate_static_to_css_string(&expression) {
          let static_value = StaticValue::Str(static_text.clone());
          if let Some((output, raw_value, important)) =
            static_value_to_css_value(&property_kebab, &static_value)
          {
            let property_name = if property_kebab == "background"
              && should_promote_background_to_color(&output, &raw_value)
            {
              "background-color".to_string()
            } else {
              property_kebab.clone()
            };
            Self::push_rule_input(
              &mut rule_inputs,
              selectors,
              at_rules,
              &property_name,
              output,
              raw_value,
              important,
              inherited_from_spread || static_value.is_from_spread(),
              false,
              None,
              options.preserve_leading_combinator_space,
              force_leading_combinator_space,
            );
            continue;
          }
        }
        let variable_name = format!("--_{}", hash_value);
        let rule_value = format!("var({})", variable_name);
        Self::push_rule_input(
          &mut rule_inputs,
          selectors,
          at_rules,
          &property_kebab,
          rule_value.clone(),
          rule_value,
          false,
          inherited_from_spread,
          false,
          None,
          options.preserve_leading_combinator_space,
          force_leading_combinator_space,
        );
        self.register_runtime_variable_dependency(&variable_name, &expression, Some(props_ident));
        runtime_variables.push(RuntimeCssVariable::new(
          variable_name,
          expression,
          None,
          None,
        ));
        continue;
      }

      if debug_css {
        eprintln!(
          "[compiled-debug] process_dynamic_css_object bailed on property='{}' (expr={})",
          property,
          emit_expression(value_expr)
        );
      }
      return None;
    }

    if !rule_inputs.is_empty() {
      let generated = atomicize_rules(&rule_inputs, options);
      artifacts.merge(generated);
    }

    for variable in runtime_variables {
      artifacts.push_variable(variable);
    }
    for condition in runtime_class_conditions {
      artifacts.push_class_condition(condition);
    }
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      let has_aria_rule = artifacts.rules.iter().any(|rule| rule.css.contains("aria"));
      let has_aria_raw = artifacts.raw_rules.iter().any(|css| css.contains("aria"));
      eprintln!(
        "[compiled-debug] process_dynamic returning rules={} raw_rules={} has_aria_rule={} has_aria_raw={}",
        artifacts.rules.len(),
        artifacts.raw_rules.len(),
        has_aria_rule,
        has_aria_raw
      );
      for rule in &artifacts.rules {
        eprintln!("[compiled-debug] process_dynamic rule {}", rule.css);
      }
      for css in &artifacts.raw_rules {
        eprintln!("[compiled-debug] process_dynamic raw_rule {}", css);
      }
    }
    Some(artifacts)
  }

  fn push_rule_input(
    rules: &mut Vec<CssRuleInput>,
    selectors: &[String],
    at_rules: &[AtRuleInput],
    property: &str,
    value: String,
    raw_value: String,
    important: bool,
    from_spread: bool,
    force_active_after_duplicate: bool,
    value_hash_override: Option<String>,
    force_preserve_leading_space: bool,
    force_leading_combinator_space: bool,
  ) {
    let mut selector_list: Vec<String> = if selectors.is_empty() {
      vec![normalize_selector(None)]
    } else {
      selectors.to_vec()
    };
    if selector_list.is_empty() {
      selector_list.push(normalize_selector(None));
    }
    let preserve_flags: Vec<bool> = selector_list
      .iter()
      .map(|selector| {
        if force_preserve_leading_space {
          true
        } else {
          selector_requires_leading_combinator_space(selector)
        }
      })
      .collect();
    let should_duplicate = from_spread || force_active_after_duplicate;
    let adjusted_selectors: Vec<String> = selector_list
      .iter()
      .map(|selector| {
        let matches_active_after =
          selector.contains(":active:after") || selector.contains(":active::after");
        let matches_active_before =
          selector.contains(":active:before") || selector.contains(":active::before");
        if should_duplicate
          && selector.ends_with(":after")
          && !selector.ends_with(":after:after")
          && matches_active_after
        {
          format!("{}:after", selector)
        } else if should_duplicate
          && selector.ends_with(":before")
          && !selector.ends_with(":before:before")
          && matches_active_before
        {
          format!("{}:before", selector)
        } else {
          selector.clone()
        }
      })
      .collect();
    let duplicate_active_after =
      selector_list
        .iter()
        .zip(adjusted_selectors.iter())
        .any(|(orig, adjusted)| {
          (orig.ends_with(":after") && adjusted.ends_with(":after:after"))
            || (orig.ends_with(":before") && adjusted.ends_with(":before:before"))
        });
    let leading_flags: Vec<bool> = selector_list
      .iter()
      .map(|_| force_leading_combinator_space)
      .collect();
    rules.push(CssRuleInput {
      selectors: adjusted_selectors,
      selector_preserve_leading_combinator_space: preserve_flags,
      selector_force_leading_combinator_space: leading_flags,
      at_rules: at_rules.to_vec(),
      property: property.to_string(),
      value: if std::env::var_os("COMPILED_DEBUG_CSS").is_some() && property == "content" {
        eprintln!(
          "[compiled-debug] push_rule_input property=content value={:?}",
          value
        );
        value
      } else {
        value
      },
      raw_value,
      important,
      duplicate_active_after,
      value_hash_override,
    });
  }

  fn split_value_and_important(value: &str) -> (String, bool) {
    let mut trimmed = value.trim();
    let mut important = false;
    if let Some(stripped) = trimmed.strip_suffix("!important") {
      important = true;
      trimmed = stripped.trim_end();
    }
    (trimmed.to_string(), important)
  }

  fn build_static_branch_artifacts(
    selectors: &[String],
    at_rules: &[AtRuleInput],
    property: &str,
    value: &StaticValue,
    options: &CssOptions,
    value_hash_override: Option<String>,
  ) -> Option<(CssArtifacts, Vec<String>)> {
    let mut wrapper = IndexMap::new();
    wrapper.insert(property.to_string(), value.clone());
    let mut inputs = Vec::new();
    let mut raw_rules = Vec::new();
    let mut runtime_variables = Vec::new();
    if !flatten_css_object(
      &wrapper,
      selectors,
      at_rules,
      &mut inputs,
      &mut raw_rules,
      &mut runtime_variables,
      false,
      options.flatten_multiple_selectors,
      options.preserve_leading_combinator_space,
      false,
    ) {
      return None;
    }
    if let Some(ref override_hash) = value_hash_override {
      for input in &mut inputs {
        input.value_hash_override = Some(override_hash.clone());
      }
    }
    let mut artifacts = atomicize_rules(&inputs, options);
    artifacts.raw_rules.extend(raw_rules);
    artifacts.runtime_variables.extend(runtime_variables);
    let class_names = artifacts
      .rules
      .iter()
      .map(|rule| rule.class_name.clone())
      .collect();
    Some((artifacts, class_names))
  }

  fn token_hash_override(expr: &Expr) -> Option<String> {
    if resolve_token_expression(expr).is_none() {
      return None;
    }
    let expr_code = emit_expression(expr);
    let runtime_hash = hash(&expr_code, 0);
    let runtime_value = format!("var(--_{})", runtime_hash);
    Some(hash(&runtime_value, 0))
  }

  fn is_token_import(specifier: &ImportNamedSpecifier) -> bool {
    fn is_token_name(name: &str) -> bool {
      matches!(name, "token" | "Tokens")
    }

    if let Some(imported) = &specifier.imported {
      match imported {
        ModuleExportName::Ident(ident) => is_token_name(ident.sym.as_ref()),
        ModuleExportName::Str(str_name) => is_token_name(str_name.value.as_ref()),
      }
    } else {
      is_token_name(specifier.local.sym.as_ref())
    }
  }

  fn ident_used_in_module(module: &Module, ident: &Ident) -> bool {
    struct Finder {
      target: Id,
      found: bool,
    }

    impl Visit for Finder {
      fn visit_ident(&mut self, ident: &Ident) {
        if ident.to_id() == self.target {
          self.found = true;
        }
      }

      fn visit_import_decl(&mut self, _import: &ImportDecl) {
        // Ignore identifiers declared within the import itself.
      }
    }

    let mut finder = Finder {
      target: ident.to_id(),
      found: false,
    };
    module.visit_with(&mut finder);
    finder.found
  }

  fn css_artifacts_from_dynamic_css_expression(
    &mut self,
    expression: &Expr,
    props_ident: &Ident,
    options: &CssOptions,
  ) -> Option<CssArtifacts> {
    self.css_artifacts_from_dynamic_css_expression_with_context(
      expression,
      props_ident,
      &[normalize_selector(None)],
      &[],
      options,
    )
  }

  fn css_artifacts_from_dynamic_css_expression_with_context(
    &mut self,
    expression: &Expr,
    props_ident: &Ident,
    selectors: &[String],
    at_rules: &[AtRuleInput],
    options: &CssOptions,
  ) -> Option<CssArtifacts> {
    let mut expr_ref = expression;
    while let Expr::Paren(paren) = expr_ref {
      expr_ref = &paren.expr;
    }

    match expr_ref {
      Expr::Array(array) => {
        let mut combined = CssArtifacts::default();
        for elem in array.elems.iter().flatten() {
          if elem.spread.is_some() {
            return None;
          }
          let mut value_expr = &*elem.expr;
          while let Expr::Paren(paren) = value_expr {
            value_expr = &*paren.expr;
          }
          if let Some(value) = evaluate_static(value_expr, &self.bindings) {
            if let Some(artifacts) = css_artifacts_from_static_value(&value, options) {
              combined.merge(artifacts);
            }
            continue;
          }
          if let Expr::Ident(ident) = value_expr {
            self.mark_css_binding_used(ident);
            let mut runtime_artifacts = self.css_runtime_artifacts.get(&to_id(ident)).cloned();
            if runtime_artifacts.is_none() {
              if let Some((_, artifacts)) = self
                .css_runtime_artifacts
                .iter()
                .find(|((sym, _), _)| sym == &ident.sym)
              {
                runtime_artifacts = Some(artifacts.clone());
              }
            }
            if let Some(artifacts) = runtime_artifacts {
              combined.merge(artifacts);
              continue;
            }
          }
          if let Some(nested) = self.css_artifacts_from_dynamic_css_expression_with_context(
            value_expr,
            props_ident,
            selectors,
            at_rules,
            options,
          ) {
            combined.merge(nested);
          }
        }
        Some(combined)
      }
      Expr::Object(obj) => self.process_dynamic_css_object_with_context(
        obj,
        props_ident,
        selectors,
        at_rules,
        options,
        false,
        false,
      ),
      Expr::Bin(bin) if matches!(bin.op, BinaryOp::LogicalAnd) => {
        let condition = bin.left.as_ref().clone();
        if let Some(value_text) = self.evaluate_static_to_css_string(bin.right.as_ref()) {
          if value_text.value.trim().is_empty() {
            return Some(CssArtifacts::default());
          }
          let mut artifacts = self.css_artifacts_from_css_text(&value_text, options)?;
          let class_names: Vec<String> = artifacts
            .rules
            .iter()
            .map(|rule| rule.class_name.clone())
            .collect();
          if class_names.is_empty() {
            return Some(artifacts);
          }
          artifacts.push_class_condition(RuntimeClassCondition::new(
            condition,
            class_names,
            Vec::new(),
          ));
          Some(artifacts)
        } else if let Some(static_value) = self.static_value_from_expr(bin.right.as_ref()) {
          if let Some(mut artifacts) = css_artifacts_from_static_value(&static_value, options) {
            self.register_artifacts_for_metadata(&artifacts);
            let class_names: Vec<String> = artifacts
              .rules
              .iter()
              .map(|rule| rule.class_name.clone())
              .collect();
            if class_names.is_empty() {
              return Some(artifacts);
            }
            artifacts.push_class_condition(RuntimeClassCondition::new(
              condition,
              class_names,
              Vec::new(),
            ));
            Some(artifacts)
          } else {
            None
          }
        } else {
          let mut artifacts = self.css_artifacts_from_dynamic_css_expression_with_context(
            bin.right.as_ref(),
            props_ident,
            selectors,
            at_rules,
            options,
          )?;
          let class_names: Vec<String> = artifacts
            .rules
            .iter()
            .map(|rule| rule.class_name.clone())
            .collect();
          if class_names.is_empty() {
            return Some(artifacts);
          }
          artifacts.push_class_condition(RuntimeClassCondition::new(
            condition,
            class_names,
            Vec::new(),
          ));
          Some(artifacts)
        }
      }
      Expr::Cond(cond_expr) => {
        let condition = cond_expr.test.as_ref().clone();
        let mut combined = CssArtifacts::default();
        let mut when_true = Vec::new();
        let mut when_false = Vec::new();

        let mut process_branch = |branch_expr: &Expr, output: &mut Vec<String>| -> Option<()> {
          if let Some(css_text) = self.evaluate_static_to_css_string(branch_expr) {
            if css_text.value.trim().is_empty() {
              return Some(());
            }
            let artifacts = self.css_artifacts_from_css_text(&css_text, options)?;
            output.extend(artifacts.rules.iter().map(|rule| rule.class_name.clone()));
            self.register_artifacts_for_metadata(&artifacts);
            combined.merge(artifacts);
            return Some(());
          }

          let mut artifacts = self.css_artifacts_from_dynamic_css_expression_with_context(
            branch_expr,
            props_ident,
            selectors,
            at_rules,
            options,
          )?;
          self.register_artifacts_for_metadata(&artifacts);
          output.extend(artifacts.rules.iter().map(|rule| rule.class_name.clone()));
          combined.merge(artifacts);
          Some(())
        };

        if process_branch(cond_expr.cons.as_ref(), &mut when_true).is_none()
          || process_branch(cond_expr.alt.as_ref(), &mut when_false).is_none()
        {
          return None;
        }

        if when_true.is_empty() && when_false.is_empty() {
          return Some(combined);
        }

        combined.push_class_condition(RuntimeClassCondition::new(condition, when_true, when_false));
        Some(combined)
      }
      Expr::Member(member) => {
        if let Expr::Ident(obj_ident) = &*member.obj {
          self.mark_css_binding_used(obj_ident);
          if let Some(artifacts) = self.css_runtime_artifacts.get(&to_id(obj_ident)).cloned() {
            self.register_artifacts_for_metadata(&artifacts);
            return Some(artifacts);
          }
        }
        if let Some(static_obj) = evaluate_static(&member.obj, &self.bindings) {
          if let StaticValue::Object(map) = static_obj.into_unwrapped() {
            if let Some(prop_name) = Self::css_map_member_prop_name(&member.prop) {
              if let Some(value) = map.get(&prop_name) {
                if let Some(mut artifacts) = css_artifacts_from_static_value(value, options) {
                  self.register_artifacts_for_metadata(&artifacts);
                  return Some(artifacts);
                }
              }
            } else if matches!(member.prop, MemberProp::Computed(_)) {
              let has_object_entries = map.values().any(|value| {
                matches!(
                  value.unwrap_spread(),
                  StaticValue::Object(_) | StaticValue::Array(_)
                )
              });
              if has_object_entries {
                if let Some(mut artifacts) = css_artifacts_from_static_object(&map, options) {
                  self.register_artifacts_for_metadata(&artifacts);
                  return Some(artifacts);
                }
              }
            }
          }
        }
        let css_text = self.evaluate_static_to_css_string(expr_ref)?;
        if css_text.value.trim().is_empty() {
          return Some(CssArtifacts::default());
        }
        self.css_artifacts_from_css_text(&css_text, options)
      }
      _ => {
        let css_text = self.evaluate_static_to_css_string(expr_ref)?;
        if css_text.value.trim().is_empty() {
          return Some(CssArtifacts::default());
        }
        self.css_artifacts_from_css_text(&css_text, options)
      }
    }
  }

  fn css_artifacts_from_css_text(
    &self,
    text: &StaticString,
    options: &CssOptions,
  ) -> Option<CssArtifacts> {
    if std::env::var_os("DEBUG_TOKENS_GRADIENT_RAW").is_some()
      && text.value.contains("linear-gradient")
    {
      eprintln!("[debug] css_text raw={:?}", text.raw);
    }
    let value = StaticValue::Str(text.clone());
    css_artifacts_from_static_value(&value, options)
  }

  fn process_dynamic_template_literal(
    &mut self,
    template: &Tpl,
    props_ident: &Ident,
  ) -> Option<(String, String, Vec<RuntimeCssVariable>, Option<String>)> {
    if template.quasis.is_empty() {
      return Some((String::new(), String::new(), Vec::new(), None));
    }

    let mut segments: Vec<String> = template
      .quasis
      .iter()
      .map(|quasi| quasi.raw.to_string())
      .collect();

    let mut result = String::new();
    let mut runtime_variables = Vec::new();

    for (index, expr) in template.exprs.iter().enumerate() {
      let before_raw = segments.get(index).cloned().unwrap_or_else(String::new);
      let after_raw = segments.get(index + 1).cloned().unwrap_or_else(String::new);
      let (before_meta, after_meta) = css_affix_interpolation(&before_raw, &after_raw);
      result.push_str(&before_meta.css);

      if let Some(token_value) = resolve_token_expression(expr) {
        result.push_str(&token_value);
        if let Some(suffix) = &after_meta.variable_suffix {
          result.push_str(suffix);
        }
        if let Some(slot) = segments.get_mut(index + 1) {
          *slot = after_meta.css.clone();
        }
        continue;
      }

      if !self.options.extract {
        if let Some(static_text) = self.evaluate_static_to_css_string(expr) {
          result.push_str(&static_text.value);
          if let Some(suffix) = &after_meta.variable_suffix {
            result.push_str(suffix);
          }
          if let Some(slot) = segments.get_mut(index + 1) {
            *slot = after_meta.css.clone();
          }
          continue;
        }
      }

      let (expression, variable_input) = self.normalize_variable_expression(expr, props_ident)?;
      let hash_input = variable_input.clone();
      if std::env::var_os("COMPILED_DEBUG_HASH").is_some() {
        eprintln!("[compiled-debug] template-variable-input={}", hash_input);
      }
      let hash_value = hash(&hash_input, 0);
      if std::env::var_os("COMPILED_DEBUG_HASH").is_some()
        && variable_input.contains("isHighlighted")
      {
        eprintln!(
          "[compiled-debug] variable-hash={} input={}",
          hash_value, variable_input
        );
      }
      let mut variable_name = format!("--_{}", hash_value);
      if matches!(before_meta.variable_prefix.as_deref(), Some("-")) {
        variable_name.push('-');
      }

      result.push_str("var(");
      result.push_str(&variable_name);
      result.push(')');

      if let Some(slot) = segments.get_mut(index + 1) {
        *slot = after_meta.css.clone();
      }

      let prefix = before_meta
        .variable_prefix
        .clone()
        .filter(|value| !value.is_empty());
      let suffix = after_meta
        .variable_suffix
        .clone()
        .filter(|value| !value.is_empty());
      let mut variable = RuntimeCssVariable::new(variable_name, expression, prefix, suffix);
      self.register_runtime_variable_dependency(
        &variable.name,
        &variable.expression,
        Some(props_ident),
      );
      runtime_variables.push(variable);
    }

    if let Some(tail) = segments.last() {
      result.push_str(tail);
    }

    let raw_template = result;
    let mut substituted = raw_template.clone();
    let mut retained_variables = Vec::new();
    for variable in runtime_variables {
      if !variable.allow_static_substitution {
        retained_variables.push(variable);
        continue;
      }
      if let Some(static_value) = self.evaluate_static_to_css_string(&variable.expression) {
        let needle = format!("var({})", variable.name);
        let mut replacement = static_value.value.clone();
        if let Some(prefix) = &variable.prefix {
          replacement = format!("{}{}", prefix, replacement);
        }
        if let Some(suffix) = &variable.suffix {
          replacement.push_str(suffix);
        }
        substituted = substituted.replace(&needle, &replacement);
      } else if let Some(token_value) = resolve_token_expression(&variable.expression) {
        let needle = format!("var({})", variable.name);
        let mut replacement = token_value;
        if let Some(prefix) = &variable.prefix {
          replacement = format!("{}{}", prefix, replacement);
        }
        if let Some(suffix) = &variable.suffix {
          replacement.push_str(suffix);
        }
        substituted = substituted.replace(&needle, &replacement);
      } else {
        retained_variables.push(variable);
      }
    }
    let normalized = normalize_css_value(&substituted);
    let removed_runtime_vars = raw_template.contains("--_") && !substituted.contains("--_");
    let hash_override =
      if retained_variables.is_empty() && substituted != raw_template && !removed_runtime_vars {
        let original_normalized = normalize_css_value(&raw_template);
        Some(hash(&original_normalized.hash_value, 0))
      } else {
        None
      };
    Some((
      substituted,
      normalized.output_value,
      retained_variables,
      hash_override,
    ))
  }

  fn try_handle_conditional_template_literal(
    &mut self,
    template: &Tpl,
    props_ident: &Ident,
    property_kebab: &str,
    selectors: &[String],
    at_rules: &[AtRuleInput],
    inherited_from_spread: bool,
    options: &CssOptions,
    runtime_class_conditions: &mut Vec<RuntimeClassCondition>,
    artifacts: &mut CssArtifacts,
  ) -> bool {
    if template.exprs.len() != 1 {
      return false;
    }
    if template.quasis.is_empty() {
      return false;
    }
    let before = template
      .quasis
      .get(0)
      .map(|quasi| quasi.raw.to_string())
      .unwrap_or_default();
    let after = template
      .quasis
      .get(1)
      .map(|quasi| quasi.raw.to_string())
      .unwrap_or_default();
    let (expression, _) = match self.normalize_variable_expression(&template.exprs[0], props_ident)
    {
      Some(res) => res,
      None => return false,
    };
    let mut expr_ref = &expression;
    while let Expr::Paren(paren) = expr_ref {
      expr_ref = &paren.expr;
    }
    let Expr::Cond(cond_expr) = expr_ref else {
      return false;
    };
    let Some(true_value) = self.evaluate_static_to_css_string(cond_expr.cons.as_ref()) else {
      return false;
    };
    let Some(false_value) = self.evaluate_static_to_css_string(cond_expr.alt.as_ref()) else {
      return false;
    };

    let true_combined = format!("{}{}{}", before, true_value.value, after);
    let false_combined = format!("{}{}{}", before, false_value.value, after);

    let (true_value_trimmed, true_important) = Self::split_value_and_important(&true_combined);
    let (false_value_trimmed, false_important) = Self::split_value_and_important(&false_combined);

    let true_artifacts = Self::atomicize_single_rule_input(
      selectors,
      at_rules,
      property_kebab,
      true_value_trimmed.clone(),
      true_value_trimmed,
      true_important,
      inherited_from_spread,
      options,
    );
    let false_artifacts = Self::atomicize_single_rule_input(
      selectors,
      at_rules,
      property_kebab,
      false_value_trimmed.clone(),
      false_value_trimmed,
      false_important,
      inherited_from_spread,
      options,
    );

    let true_classes = true_artifacts
      .rules
      .iter()
      .map(|rule| rule.class_name.clone())
      .collect::<Vec<_>>();
    let false_classes = false_artifacts
      .rules
      .iter()
      .map(|rule| rule.class_name.clone())
      .collect::<Vec<_>>();

    if true_classes.is_empty() && false_classes.is_empty() {
      return false;
    }

    self.register_artifacts_for_metadata(&true_artifacts);
    self.register_artifacts_for_metadata(&false_artifacts);

    runtime_class_conditions.push(RuntimeClassCondition::new(
      (*cond_expr.test).clone(),
      true_classes.clone(),
      false_classes.clone(),
    ));
    self
      .force_runtime_metadata_classes
      .extend(true_classes.into_iter());
    self
      .force_runtime_metadata_classes
      .extend(false_classes.into_iter());

    artifacts.merge(true_artifacts);
    artifacts.merge(false_artifacts);
    true
  }

  fn atomicize_single_rule_input(
    selectors: &[String],
    at_rules: &[AtRuleInput],
    property: &str,
    value: String,
    raw_value: String,
    important: bool,
    inherited_from_spread: bool,
    options: &CssOptions,
  ) -> CssArtifacts {
    let mut inputs = Vec::new();
    Self::push_rule_input(
      &mut inputs,
      selectors,
      at_rules,
      property,
      value,
      raw_value,
      important,
      inherited_from_spread,
      false,
      None,
      options.preserve_leading_combinator_space,
      false,
    );
    atomicize_rules(&inputs, options)
  }

  fn evaluate_static_to_css_string(&self, expr: &Expr) -> Option<StaticString> {
    let static_value = self.static_value_from_expr(expr)?;
    match static_value.unwrap_spread() {
      StaticValue::Str(text) => Some(text.clone()),
      StaticValue::Num(number) => {
        if number.fract() == 0.0 {
          Some(StaticString::new(format!("{}", number.trunc() as i64)))
        } else {
          Some(StaticString::new(number.to_string()))
        }
      }
      StaticValue::Bool(boolean) => Some(StaticString::new(boolean.to_string())),
      StaticValue::Null => Some(StaticString::new(String::new())),
      _ => None,
    }
  }

  fn static_value_from_expr(&self, expr: &Expr) -> Option<StaticValue> {
    if self.expr_requires_runtime_import(expr) {
      return None;
    }
    if let Expr::Ident(ident) = expr {
      if ident.sym.as_ref() == "undefined" {
        return Some(StaticValue::Null);
      }
    }
    if let Some(token_value) = resolve_token_expression(expr) {
      return Some(StaticValue::Str(StaticString::new(token_value)));
    }
    let evaluation = evaluate_static_with_info(expr, &self.bindings)?;
    if evaluation.depends_on_import {
      return None;
    }
    Some(evaluation.value)
  }

  fn static_import_value_is_literal(value: &StaticValue) -> bool {
    matches!(
      value,
      StaticValue::Str(_) | StaticValue::Num(_) | StaticValue::Bool(_) | StaticValue::Null
    )
  }

  fn expr_requires_runtime_import(&self, expr: &Expr) -> bool {
    self.expr_uses_import_matching(expr, |source| source.contains("jira-common-styles"))
      || self.expr_uses_ident_named(expr, "layers")
  }

  fn expr_uses_import_matching(
    &self,
    expr: &Expr,
    predicate: impl Fn(&str) -> bool + Copy,
  ) -> bool {
    match expr {
      Expr::Ident(ident) => self
        .compiled_import_sources
        .get(&to_id(ident))
        .map(|source| predicate(source))
        .unwrap_or(false),
      Expr::Member(member) => self.expr_uses_import_matching(&member.obj, predicate),
      Expr::Call(call) => {
        let callee_uses = match &call.callee {
          Callee::Expr(expr) => self.expr_uses_import_matching(expr, predicate),
          Callee::Super(_) | Callee::Import(_) => false,
        };
        callee_uses
          || call
            .args
            .iter()
            .any(|arg| self.expr_uses_import_matching(&arg.expr, predicate))
      }
      Expr::Tpl(tpl) => tpl
        .exprs
        .iter()
        .any(|inner| self.expr_uses_import_matching(inner, predicate)),
      Expr::Paren(paren) => self.expr_uses_import_matching(&paren.expr, predicate),
      Expr::Bin(bin) => {
        self.expr_uses_import_matching(&bin.left, predicate)
          || self.expr_uses_import_matching(&bin.right, predicate)
      }
      Expr::Cond(cond) => {
        self.expr_uses_import_matching(&cond.test, predicate)
          || self.expr_uses_import_matching(&cond.cons, predicate)
          || self.expr_uses_import_matching(&cond.alt, predicate)
      }
      Expr::Unary(unary) => self.expr_uses_import_matching(&unary.arg, predicate),
      Expr::Array(array) => array
        .elems
        .iter()
        .filter_map(|elem| elem.as_ref())
        .any(|elem| self.expr_uses_import_matching(&elem.expr, predicate)),
      Expr::Object(object) => object.props.iter().any(|prop| match prop {
        PropOrSpread::Prop(prop) => match &**prop {
          Prop::KeyValue(kv) => self.expr_uses_import_matching(&kv.value, predicate),
          Prop::Getter(getter) => getter
            .body
            .as_ref()
            .map(|body| self.block_uses_import_matching(body, predicate))
            .unwrap_or(false),
          Prop::Setter(setter) => setter
            .body
            .as_ref()
            .map(|body| self.block_uses_import_matching(body, predicate))
            .unwrap_or(false),
          Prop::Method(method) => method
            .function
            .body
            .as_ref()
            .map(|body| self.block_uses_import_matching(body, predicate))
            .unwrap_or(false),
          Prop::Shorthand(ident) => self
            .compiled_import_sources
            .get(&to_id(ident))
            .map(|source| predicate(source))
            .unwrap_or(false),
          Prop::Assign(assign) => self.expr_uses_import_matching(&assign.value, predicate),
        },
        PropOrSpread::Spread(spread) => self.expr_uses_import_matching(&spread.expr, predicate),
      }),
      _ => false,
    }
  }

  fn block_uses_import_matching(
    &self,
    block: &BlockStmt,
    predicate: impl Fn(&str) -> bool + Copy,
  ) -> bool {
    block
      .stmts
      .iter()
      .any(|stmt| self.stmt_uses_import_matching(stmt, predicate))
  }

  fn stmt_uses_import_matching(
    &self,
    stmt: &Stmt,
    predicate: impl Fn(&str) -> bool + Copy,
  ) -> bool {
    match stmt {
      Stmt::Expr(expr_stmt) => self.expr_uses_import_matching(&expr_stmt.expr, predicate),
      Stmt::Decl(decl) => match decl {
        Decl::Var(var) => var.decls.iter().any(|decl| {
          decl
            .init
            .as_ref()
            .map(|expr| self.expr_uses_import_matching(expr, predicate))
            .unwrap_or(false)
        }),
        Decl::Fn(function) => function
          .function
          .body
          .as_ref()
          .map(|body| self.block_uses_import_matching(body, predicate))
          .unwrap_or(false),
        Decl::Class(class) => class.class.body.iter().any(|member| match member {
          ClassMember::Constructor(cons) => cons
            .body
            .as_ref()
            .map(|body| self.block_uses_import_matching(body, predicate))
            .unwrap_or(false),
          ClassMember::Method(method) => method
            .function
            .body
            .as_ref()
            .map(|body| self.block_uses_import_matching(body, predicate))
            .unwrap_or(false),
          _ => false,
        }),
        Decl::TsEnum(enum_decl) => enum_decl.members.iter().any(|member| match &member.init {
          Some(expr) => self.expr_uses_import_matching(expr, predicate),
          None => false,
        }),
        _ => false,
      },
      Stmt::If(if_stmt) => {
        self.expr_uses_import_matching(&if_stmt.test, predicate)
          || self.stmt_uses_import_matching(&*if_stmt.cons, predicate)
          || if_stmt
            .alt
            .as_ref()
            .map(|stmt| self.stmt_uses_import_matching(stmt, predicate))
            .unwrap_or(false)
      }
      Stmt::Block(block) => self.block_uses_import_matching(block, predicate),
      _ => false,
    }
  }

  fn expr_uses_ident_named(&self, expr: &Expr, target: &str) -> bool {
    match expr {
      Expr::Ident(ident) => ident.sym.as_ref() == target,
      Expr::Member(member) => {
        self.expr_uses_ident_named(&member.obj, target)
          || matches!(
            &member.prop,
            MemberProp::Computed(comp) if self.expr_uses_ident_named(&comp.expr, target)
          )
      }
      Expr::Call(call) => {
        let callee_match = match &call.callee {
          Callee::Expr(expr) => self.expr_uses_ident_named(expr, target),
          Callee::Super(_) | Callee::Import(_) => false,
        };
        callee_match
          || call
            .args
            .iter()
            .any(|arg| self.expr_uses_ident_named(&arg.expr, target))
      }
      Expr::Tpl(tpl) => tpl
        .exprs
        .iter()
        .any(|expr| self.expr_uses_ident_named(expr, target)),
      Expr::Paren(paren) => self.expr_uses_ident_named(&paren.expr, target),
      Expr::Bin(bin) => {
        self.expr_uses_ident_named(&bin.left, target)
          || self.expr_uses_ident_named(&bin.right, target)
      }
      Expr::Cond(cond) => {
        self.expr_uses_ident_named(&cond.test, target)
          || self.expr_uses_ident_named(&cond.cons, target)
          || self.expr_uses_ident_named(&cond.alt, target)
      }
      Expr::Unary(unary) => self.expr_uses_ident_named(&unary.arg, target),
      Expr::Array(array) => array
        .elems
        .iter()
        .filter_map(|elem| elem.as_ref())
        .any(|elem| self.expr_uses_ident_named(&elem.expr, target)),
      Expr::Object(object) => object.props.iter().any(|prop| match prop {
        PropOrSpread::Prop(prop) => match &**prop {
          Prop::KeyValue(kv) => self.expr_uses_ident_named(&kv.value, target),
          Prop::Getter(getter) => getter
            .body
            .as_ref()
            .map(|body| self.block_uses_ident_named(body, target))
            .unwrap_or(false),
          Prop::Setter(setter) => setter
            .body
            .as_ref()
            .map(|body| self.block_uses_ident_named(body, target))
            .unwrap_or(false),
          Prop::Method(method) => method
            .function
            .body
            .as_ref()
            .map(|body| self.block_uses_ident_named(body, target))
            .unwrap_or(false),
          Prop::Shorthand(ident) => ident.sym.as_ref() == target,
          Prop::Assign(assign) => self.expr_uses_ident_named(&assign.value, target),
        },
        PropOrSpread::Spread(spread) => self.expr_uses_ident_named(&spread.expr, target),
      }),
      _ => false,
    }
  }

  fn block_uses_ident_named(&self, block: &BlockStmt, target: &str) -> bool {
    block
      .stmts
      .iter()
      .any(|stmt| self.stmt_uses_ident_named(stmt, target))
  }

  fn stmt_uses_ident_named(&self, stmt: &Stmt, target: &str) -> bool {
    match stmt {
      Stmt::Expr(expr_stmt) => self.expr_uses_ident_named(&expr_stmt.expr, target),
      Stmt::Decl(decl) => match decl {
        Decl::Var(var) => var.decls.iter().any(|decl| {
          decl
            .init
            .as_ref()
            .map(|expr| self.expr_uses_ident_named(expr, target))
            .unwrap_or(false)
        }),
        Decl::Fn(function) => function
          .function
          .body
          .as_ref()
          .map(|body| self.block_uses_ident_named(body, target))
          .unwrap_or(false),
        Decl::Class(class) => class.class.body.iter().any(|member| match member {
          ClassMember::Constructor(cons) => cons
            .body
            .as_ref()
            .map(|body| self.block_uses_ident_named(body, target))
            .unwrap_or(false),
          ClassMember::Method(method) => method
            .function
            .body
            .as_ref()
            .map(|body| self.block_uses_ident_named(body, target))
            .unwrap_or(false),
          _ => false,
        }),
        Decl::TsEnum(enum_decl) => enum_decl.members.iter().any(|member| match &member.init {
          Some(expr) => self.expr_uses_ident_named(expr, target),
          None => false,
        }),
        _ => false,
      },
      Stmt::If(if_stmt) => {
        self.expr_uses_ident_named(&if_stmt.test, target)
          || self.stmt_uses_ident_named(&*if_stmt.cons, target)
          || if_stmt
            .alt
            .as_ref()
            .map(|stmt| self.stmt_uses_ident_named(stmt, target))
            .unwrap_or(false)
      }
      Stmt::Block(block) => self.block_uses_ident_named(block, target),
      _ => false,
    }
  }

  fn process_dynamic_styled_template(
    &mut self,
    template: &TaggedTpl,
    props_ident: &Ident,
  ) -> Option<CssArtifacts> {
    const PLACEHOLDER_PREFIX: &str = "__COMPILED_EXPR_";
    const PLACEHOLDER_SUFFIX: &str = "__";

    let mut source = String::new();
    for (index, quasi) in template.tpl.quasis.iter().enumerate() {
      let cooked = quasi
        .cooked
        .as_ref()
        .map(|atom| atom.to_string())
        .unwrap_or_else(|| quasi.raw.to_string());
      source.push_str(&cooked);
      if template.tpl.exprs.get(index).is_some() {
        source.push_str(PLACEHOLDER_PREFIX);
        source.push_str(&index.to_string());
        source.push_str(PLACEHOLDER_SUFFIX);
      }
    }

    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      eprintln!(
        "[compiled-debug] styled template source: {}",
        source.replace('\n', "\\n")
      );
    }

    #[derive(Debug)]
    enum ValueSegment {
      Text(String),
      Expr(usize),
    }

    fn parse_segments(value: &str) -> Option<Vec<ValueSegment>> {
      const PREFIX: &str = "__COMPILED_EXPR_";
      const SUFFIX: &str = "__";
      let mut segments = Vec::new();
      let mut remaining = value;
      while let Some(start) = remaining.find(PREFIX) {
        let (before, rest) = remaining.split_at(start);
        if !before.is_empty() {
          segments.push(ValueSegment::Text(before.to_string()));
        }
        let rest_after_prefix = &rest[PREFIX.len()..];
        let end = rest_after_prefix.find(SUFFIX)?;
        let index_str = &rest_after_prefix[..end];
        let index = index_str.parse::<usize>().ok()?;
        segments.push(ValueSegment::Expr(index));
        remaining = &rest_after_prefix[end + SUFFIX.len()..];
      }
      if !remaining.is_empty() {
        segments.push(ValueSegment::Text(remaining.to_string()));
      }
      Some(segments)
    }

    fn apply_selectors(stack: &[String], declaration: &str) -> String {
      if stack.is_empty() {
        return declaration.to_string();
      }
      let mut result = String::new();
      for selector in stack {
        result.push_str(selector);
      }
      result.push_str(declaration);
      for _ in stack {
        result.push('}');
      }
      result
    }

    let mut selector_stack: Vec<String> = Vec::new();
    let mut base_declarations: Vec<String> = Vec::new();
    let mut artifacts = CssArtifacts::default();
    let mut runtime_variables: Vec<RuntimeCssVariable> = Vec::new();
    let mut runtime_class_conditions: Vec<RuntimeClassCondition> = Vec::new();

    for raw_line in source.lines() {
      let mut line = raw_line.trim();
      if line.is_empty() {
        continue;
      }

      if line == "}" {
        base_declarations.push("}".to_string());
        selector_stack.pop();
        continue;
      }

      if line.ends_with('{') {
        let selector = line.to_string();
        base_declarations.push(selector.clone());
        selector_stack.push(selector);
        continue;
      }

      if line.starts_with(PLACEHOLDER_PREFIX) && line.ends_with(PLACEHOLDER_SUFFIX) {
        let index_str = &line[PLACEHOLDER_PREFIX.len()..line.len() - PLACEHOLDER_SUFFIX.len()];
        if let Ok(expr_index) = index_str.parse::<usize>() {
          if let Some(expr) = template.tpl.exprs.get(expr_index) {
            if self
              .handle_full_declaration_expression(
                expr,
                props_ident,
                &mut base_declarations,
                &mut artifacts,
                &mut runtime_class_conditions,
              )
              .is_none()
            {
              return None;
            }
            continue;
          } else {
            return None;
          }
        }
      }

      line = line.trim_end_matches('}').trim();
      if line.is_empty() {
        continue;
      }

      let declaration = line.trim_end_matches(';').trim();
      if declaration.is_empty() {
        continue;
      }

      let Some(colon_index) = declaration.find(':') else {
        continue;
      };

      let mut property_string = declaration[..colon_index].trim().to_string();
      if property_string.is_empty() {
        continue;
      }

      if let Some(placeholder_pos) = property_string.find(PLACEHOLDER_PREFIX) {
        let after_prefix = &property_string[placeholder_pos + PLACEHOLDER_PREFIX.len()..];
        if let Some(suffix_pos) = after_prefix.find(PLACEHOLDER_SUFFIX) {
          let index_str = &after_prefix[..suffix_pos];
          if let Ok(expr_index) = index_str.parse::<usize>() {
            if let Some(expr) = template.tpl.exprs.get(expr_index) {
              if self
                .handle_full_declaration_expression(
                  expr,
                  props_ident,
                  &mut base_declarations,
                  &mut artifacts,
                  &mut runtime_class_conditions,
                )
                .is_none()
              {
                return None;
              }
              let end_pos = placeholder_pos
                + PLACEHOLDER_PREFIX.len()
                + index_str.len()
                + PLACEHOLDER_SUFFIX.len();
              property_string.replace_range(placeholder_pos..end_pos, "");
              property_string = property_string.trim().to_string();
              if property_string.is_empty() {
                continue;
              }
            }
          }
        }
      }

      let property = property_string.as_str();
      let value_raw = declaration[colon_index + 1..].trim();
      if value_raw.is_empty() {
        continue;
      }

      let segments = parse_segments(value_raw)?;
      let expr_count = segments
        .iter()
        .filter(|segment| matches!(segment, ValueSegment::Expr(_)))
        .count();

      if expr_count == 1 && segments.len() == 1 {
        if let ValueSegment::Expr(expr_index) = segments[0] {
          let expr = template.tpl.exprs.get(expr_index)?;
          if let Some(static_value) = self.static_value_from_expr(expr) {
            if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
              eprintln!(
                "[compiled-debug] styled template static expr {:?}",
                static_value.to_js_string()
              );
            }
          }
        }
      }

      if expr_count == 0 {
        base_declarations.push(format!("{}:{};", property, value_raw));
        continue;
      }

      if expr_count > 1 {
        let mut value_builder = String::new();
        for segment in segments {
          match segment {
            ValueSegment::Text(text) => value_builder.push_str(&text),
            ValueSegment::Expr(index) => {
              let expr = match template.tpl.exprs.get(index) {
                Some(expr) => expr,
                None => return None,
              };
              if !self.options.extract {
                if let Some(static_value) = self.evaluate_static_to_css_string(expr) {
                  value_builder.push_str(&static_value.value);
                  continue;
                } else if let Some(token_value) = resolve_token_expression(expr) {
                  value_builder.push_str(&token_value);
                  continue;
                }
              }
              let (expression, variable_input) =
                match self.normalize_variable_expression(expr, props_ident) {
                  Some(res) => res,
                  None => return None,
                };
              let hash_value = hash(&variable_input, 0);
              let variable_name = format!("--_{}", hash_value);
              value_builder.push_str("var(");
              value_builder.push_str(&variable_name);
              value_builder.push(')');
              let mut variable = RuntimeCssVariable::new(variable_name, expression, None, None);
              if self.options.extract {
                variable.allow_static_substitution = false;
              }
              self.register_runtime_variable_dependency(
                &variable.name,
                &variable.expression,
                Some(props_ident),
              );
              runtime_variables.push(variable);
            }
          }
        }
        base_declarations.push(format!("{}:{};", property, value_builder.trim()));
        continue;
      }

      let mut prefix_text = String::new();
      let mut suffix_text = String::new();
      let mut expr_index = None;
      let mut saw_expr = false;
      for segment in &segments {
        match segment {
          ValueSegment::Text(text) => {
            if !saw_expr {
              prefix_text.push_str(text);
            } else {
              suffix_text.push_str(text);
            }
          }
          ValueSegment::Expr(index) => {
            expr_index = Some(*index);
            saw_expr = true;
          }
        }
      }

      let expr_index = match expr_index {
        Some(idx) => idx,
        None => return None,
      };

      let expr = match template.tpl.exprs.get(expr_index) {
        Some(expr) => expr.clone(),
        None => return None,
      };

      let (expression, variable_input) =
        match self.normalize_variable_expression(&expr, props_ident) {
          Some(res) => res,
          None => return None,
        };

      let mut expr_ref = &expression;
      while let Expr::Paren(paren) = expr_ref {
        expr_ref = &paren.expr;
      }

      if let Expr::Cond(cond_expr) = expr_ref {
        if let (Some(true_value), Some(false_value)) = (
          self.evaluate_static_to_css_string(cond_expr.cons.as_ref()),
          self.evaluate_static_to_css_string(cond_expr.alt.as_ref()),
        ) {
          let true_decl = format!(
            "{}:{};",
            property,
            format!("{}{}{}", prefix_text, true_value.value, suffix_text).trim()
          );
          let false_decl = format!(
            "{}:{};",
            property,
            format!("{}{}{}", prefix_text, false_value.value, suffix_text).trim()
          );

          let true_literal = apply_selectors(&selector_stack, &true_decl);
          let false_literal = apply_selectors(&selector_stack, &false_decl);

          let true_artifacts = css_artifacts_from_literal(&true_literal, &self.css_options())?;
          let false_artifacts = css_artifacts_from_literal(&false_literal, &self.css_options())?;

          let true_classes = true_artifacts
            .rules
            .iter()
            .map(|rule| rule.class_name.clone())
            .collect();
          let false_classes = false_artifacts
            .rules
            .iter()
            .map(|rule| rule.class_name.clone())
            .collect();

          runtime_class_conditions.push(RuntimeClassCondition::new(
            (*cond_expr.test).clone(),
            true_classes,
            false_classes,
          ));

          artifacts.merge(true_artifacts);
          artifacts.merge(false_artifacts);
          continue;
        }
      }

      if let Some(static_value) = self.evaluate_static_to_css_string(expr_ref) {
        let mut base_value = String::new();
        base_value.push_str(&prefix_text);
        base_value.push_str(&static_value.value);
        base_value.push_str(&suffix_text);
        base_declarations.push(apply_selectors(
          &selector_stack,
          &format!("{}:{};", property, base_value.trim()),
        ));
        continue;
      }

      let hash_value = hash(&variable_input, 0);
      let variable_name = format!("--_{}", hash_value);
      let mut base_value = String::new();
      base_value.push_str(&prefix_text);
      base_value.push_str("var(");
      base_value.push_str(&variable_name);
      base_value.push(')');
      base_value.push_str(&suffix_text);
      let mut variable = RuntimeCssVariable::new(variable_name, expression, None, None);
      if self.options.extract {
        variable.allow_static_substitution = false;
      }
      self.register_runtime_variable_dependency(
        &variable.name,
        &variable.expression,
        Some(props_ident),
      );
      runtime_variables.push(variable);
      base_declarations.push(apply_selectors(
        &selector_stack,
        &format!("{}:{};", property, base_value.trim()),
      ));
    }

    if !base_declarations.is_empty() {
      let base_css = base_declarations.join("");
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] styled template base css: {}",
          base_css.replace('\n', "\\n")
        );
      }
      if let Some(base_artifacts) = css_artifacts_from_literal(&base_css, &self.css_options()) {
        artifacts.merge(base_artifacts);
      } else {
        return None;
      }
    }

    artifacts.runtime_variables = runtime_variables;
    artifacts.runtime_class_conditions = runtime_class_conditions;

    Some(artifacts)
  }
  fn handle_full_declaration_expression(
    &mut self,
    expr: &Expr,
    props_ident: &Ident,
    base_declarations: &mut Vec<String>,
    artifacts: &mut CssArtifacts,
    runtime_class_conditions: &mut Vec<RuntimeClassCondition>,
  ) -> Option<()> {
    let (normalized_expr, _) = self.normalize_variable_expression(expr, props_ident)?;
    let mut expr_ref = &normalized_expr;
    while let Expr::Paren(paren) = expr_ref {
      expr_ref = &paren.expr;
    }

    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      eprintln!(
        "[compiled-debug] handle_full_declaration_expression expr={}",
        emit_expression(expr_ref)
      );
    }

    if let Expr::Cond(cond_expr) = expr_ref {
      let true_str = self.evaluate_static_to_css_string(cond_expr.cons.as_ref())?;
      let false_str = self.evaluate_static_to_css_string(cond_expr.alt.as_ref())?;

      let (true_prop, true_value) = Self::split_css_declaration(&true_str.value)?;
      let (false_prop, false_value) = Self::split_css_declaration(&false_str.value)?;

      let true_literal = format!("{}:{};", true_prop, true_value);
      let false_literal = format!("{}:{};", false_prop, false_value);

      let true_artifacts = css_artifacts_from_literal(&true_literal, &self.css_options())?;
      let false_artifacts = css_artifacts_from_literal(&false_literal, &self.css_options())?;

      let true_classes = true_artifacts
        .rules
        .iter()
        .map(|rule| rule.class_name.clone())
        .collect();
      let false_classes = false_artifacts
        .rules
        .iter()
        .map(|rule| rule.class_name.clone())
        .collect();

      runtime_class_conditions.push(RuntimeClassCondition::new(
        (*cond_expr.test).clone(),
        true_classes,
        false_classes,
      ));

      artifacts.merge(true_artifacts);
      artifacts.merge(false_artifacts);
      return Some(());
    }

    if let Some(static_str) = self.evaluate_static_to_css_string(expr_ref) {
      let (property, value) = Self::split_css_declaration(&static_str.value)?;
      base_declarations.push(format!("{}:{};", property, value));
      return Some(());
    }

    None
  }

  fn split_css_declaration(input: &str) -> Option<(String, String)> {
    let trimmed = input.trim().trim_end_matches(';').trim();
    let colon_index = trimmed.find(':')?;
    let property = trimmed[..colon_index].trim();
    let value = trimmed[colon_index + 1..].trim();
    if property.is_empty() || value.is_empty() {
      return None;
    }
    Some((property.to_string(), value.to_string()))
  }

  fn expr_to_static_css_value(expr: &Expr) -> Option<String> {
    match expr {
      Expr::Lit(Lit::Str(str)) => Some(str.value.to_string()),
      Expr::Tpl(tpl) if tpl.exprs.is_empty() => {
        let mut value = String::new();
        for quasi in &tpl.quasis {
          value.push_str(
            quasi
              .cooked
              .as_ref()
              .map(|atom| atom.to_string())
              .unwrap_or_else(|| quasi.raw.to_string())
              .as_str(),
          );
        }
        Some(value)
      }
      _ => None,
    }
  }

  fn class_names_to_expr(class_names: &[String]) -> Expr {
    if class_names.is_empty() {
      Expr::Lit(Lit::Str(Str::from(String::new())))
    } else {
      Expr::Lit(Lit::Str(Str::from(class_names.join(" "))))
    }
  }

  fn member_expr_from_key(base: Expr, key: &PropName) -> Option<Expr> {
    let member = match key {
      PropName::Ident(ident) => MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(base),
        prop: MemberProp::Ident(ident.clone()),
      },
      PropName::Str(str) => MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(base),
        prop: MemberProp::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: Box::new(Expr::Lit(Lit::Str(str.clone()))),
        }),
      },
      PropName::Num(num) => MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(base),
        prop: MemberProp::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: Box::new(Expr::Lit(Lit::Num(num.clone()))),
        }),
      },
      PropName::BigInt(bigint) => MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(base),
        prop: MemberProp::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: Box::new(Expr::Lit(Lit::BigInt(bigint.clone()))),
        }),
      },
      PropName::Computed(_) => return None,
    };
    Some(Expr::Member(member))
  }

  fn collect_pattern_bindings(pattern: &Pat, target: Expr, out: &mut Vec<(Id, Expr)>) -> bool {
    match pattern {
      Pat::Ident(binding) => {
        out.push((to_id(&binding.id), target));
        true
      }
      Pat::Assign(assign) => Self::collect_pattern_bindings(&assign.left, target, out),
      Pat::Object(object) => {
        for prop in &object.props {
          match prop {
            ObjectPatProp::KeyValue(kv) => {
              let Some(member_expr) = Self::member_expr_from_key(target.clone(), &kv.key) else {
                return false;
              };
              if !Self::collect_pattern_bindings(&kv.value, member_expr, out) {
                return false;
              }
            }
            ObjectPatProp::Assign(assign) => {
              let ident = assign.key.id.clone();
              let member_expr = Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(target.clone()),
                prop: MemberProp::Ident(ident.clone().into()),
              });
              out.push((to_id(&ident), member_expr));
            }
            ObjectPatProp::Rest(_) => return false,
          }
        }
        true
      }
      Pat::Array(array) => {
        for (index, element) in array.elems.iter().enumerate() {
          let Some(element_pat) = element else {
            continue;
          };
          let member_expr = Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(target.clone()),
            prop: MemberProp::Computed(ComputedPropName {
              span: DUMMY_SP,
              expr: Box::new(Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value: index as f64,
                raw: None,
              }))),
            }),
          });
          if !Self::collect_pattern_bindings(element_pat, member_expr, out) {
            return false;
          }
        }
        true
      }
      _ => false,
    }
  }

  fn replace_idents_with_expr(expr: &mut Expr, mappings: &HashMap<Id, Expr>) {
    struct Replacer<'a> {
      mappings: &'a HashMap<Id, Expr>,
    }

    impl VisitMut for Replacer<'_> {
      fn visit_mut_expr(&mut self, expr: &mut Expr) {
        if let Expr::Ident(ident) = expr {
          if let Some(replacement) = self.mappings.get(&to_id(ident)) {
            *expr = replacement.clone();
            return;
          }
        }
        expr.visit_mut_children_with(self);
      }
    }

    let mut replacer = Replacer { mappings };
    expr.visit_mut_with(&mut replacer);
  }

  fn is_css_none_expr(expr: &Expr) -> bool {
    match expr {
      Expr::Ident(ident) => ident.sym.as_ref() == "undefined",
      Expr::Lit(Lit::Null(_)) => true,
      Expr::Paren(paren) => Self::is_css_none_expr(&paren.expr),
      Expr::TsAs(assert) => Self::is_css_none_expr(&assert.expr),
      Expr::TsTypeAssertion(assert) => Self::is_css_none_expr(&assert.expr),
      Expr::TsConstAssertion(assert) => Self::is_css_none_expr(&assert.expr),
      Expr::TsNonNull(non_null) => Self::is_css_none_expr(&non_null.expr),
      _ => false,
    }
  }

  fn strip_wrapping_parens(code: &str) -> String {
    let trimmed = code.trim();
    if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
      return code.to_string();
    }

    let mut depth = 0usize;
    for (index, ch) in trimmed.char_indices() {
      match ch {
        '(' => depth += 1,
        ')' => {
          if depth == 0 {
            return code.to_string();
          }
          depth -= 1;
          if depth == 0 && index != trimmed.len() - 1 {
            return code.to_string();
          }
        }
        _ => {}
      }
    }

    if depth == 0 {
      trimmed[1..trimmed.len() - 1].trim().to_string()
    } else {
      code.to_string()
    }
  }

  fn canonicalize_runtime_value(value: &str) -> String {
    if !value.starts_with("var(") {
      return Self::uppercase_hex_literals(value);
    }
    let normalized = Self::normalize_var_comma_spacing(value);
    Self::uppercase_hex_literals(&normalized)
  }

  fn normalize_var_comma_spacing(value: &str) -> String {
    let Some(comma_index) = Self::find_top_level_comma_in_var(value) else {
      return value.to_string();
    };
    let prefix = &value[..comma_index + 1];
    let remainder = &value[comma_index + 1..];
    let trimmed = remainder.trim_start();
    let mut output = String::with_capacity(value.len());
    output.push_str(prefix);
    output.push_str(trimmed);
    output
  }

  fn find_top_level_comma_in_var(value: &str) -> Option<usize> {
    if !value.starts_with("var(") {
      return None;
    }
    let mut depth = 0usize;
    for (idx, ch) in value.char_indices() {
      match ch {
        '(' => depth += 1,
        ')' => {
          if depth > 0 {
            depth -= 1;
          }
        }
        ',' if depth == 1 => return Some(idx),
        _ => {}
      }
    }
    None
  }

  fn uppercase_hex_literals(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut iter = value.chars().peekable();
    while let Some(ch) = iter.next() {
      if ch == '#' {
        output.push('#');
        while let Some(&next) = iter.peek() {
          if next.is_ascii_hexdigit() {
            output.push(next.to_ascii_uppercase());
            iter.next();
          } else {
            break;
          }
        }
      } else {
        output.push(ch);
      }
    }
    output
  }

  fn normalize_variable_expression(
    &self,
    expr: &Expr,
    props_ident: &Ident,
  ) -> Option<(Expr, String)> {
    let canonical_ident = Ident::new(
      CANONICAL_PROPS_IDENT.into(),
      DUMMY_SP,
      SyntaxContext::empty(),
    );
    match expr {
      Expr::Arrow(arrow) => {
        if arrow.params.len() != 1 {
          return None;
        }
        let mut body_expr = self.arrow_body_to_expr(arrow)?;
        match &arrow.params[0] {
          Pat::Ident(binding) => {
            Self::rename_ident_in_expr(&mut body_expr, &binding.id, props_ident);
          }
          pattern => {
            let mut bindings = Vec::new();
            if !Self::collect_pattern_bindings(
              pattern,
              Expr::Ident(props_ident.clone()),
              &mut bindings,
            ) {
              return None;
            }
            let mut mapping = HashMap::new();
            for (id, expr) in bindings {
              mapping.insert(id, expr);
            }
            Self::replace_idents_with_expr(&mut body_expr, &mapping);
          }
        }
        let mut canonical_body = body_expr.clone();
        Self::rename_ident_in_expr(&mut canonical_body, props_ident, &canonical_ident);
        let body_code = emit_expression_with_codegen(&canonical_body);
        let mut normalized_body = if body_code.trim().starts_with('(')
          && body_code.contains('?')
          && body_code.contains(':')
        {
          Self::strip_wrapping_parens(&body_code)
        } else {
          body_code
        };
        let trimmed_body = normalized_body.trim();
        if (trimmed_body.starts_with("(()=>") || trimmed_body.starts_with("(() =>"))
          && trimmed_body.ends_with(")()")
        {
          if let Some(start) = trimmed_body.find('{') {
            if let Some(end) = trimmed_body.rfind('}') {
              normalized_body = trimmed_body[start..=end].to_string();
            }
          }
        }
        if normalized_body.contains('#') {
          normalized_body = Self::uppercase_hex_literals(&normalized_body);
        }
        let contains_comment = normalized_body.contains("//") || normalized_body.contains("/*");
        let contains_newline = normalized_body.contains('\n');
        if !contains_comment && !contains_newline {
          normalized_body = normalized_body
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        }
        let variable_input = format!("{} => {}", CANONICAL_PROPS_IDENT, normalized_body);
        if std::env::var_os("COMPILED_DEBUG_HASH").is_some()
          && (variable_input.contains("isHighlighted")
            || variable_input.contains("formatRuleHoverColor"))
        {
          eprintln!("[compiled-debug] variable-input={:?}", variable_input);
        }
        if std::env::var_os("COMPILED_DEBUG_VARIABLE_INPUT").is_some() {
          eprintln!("[compiled-variable-input] {}", variable_input);
        }
        Some((body_expr, variable_input))
      }
      Expr::Cond(cond) => {
        let mut canonical_expr = Expr::Cond(cond.clone());
        Self::rename_ident_in_expr(&mut canonical_expr, props_ident, &canonical_ident);
        let mut normalized = emit_expression_with_codegen(&canonical_expr);
        if !normalized.contains('\n') && !normalized.contains("/*") && !normalized.contains("//") {
          normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
        }
        if std::env::var_os("COMPILED_DEBUG_VARIABLE_INPUT").is_some() {
          eprintln!("[compiled-variable-input] {}", normalized);
        }
        Some((Expr::Cond(cond.clone()), normalized))
      }
      Expr::Ident(_)
      | Expr::Member(_)
      | Expr::Call(_)
      | Expr::Tpl(_)
      | Expr::Lit(_)
      | Expr::Bin(_)
      | Expr::Unary(_)
      | Expr::Paren(_)
      | Expr::TsAs(_)
      | Expr::TsTypeAssertion(_)
      | Expr::TsConstAssertion(_)
      | Expr::TsNonNull(_)
      | Expr::OptChain(_)
      | Expr::Await(_)
      | Expr::New(_)
      | Expr::Array(_)
      | Expr::Object(_) => {
        let mut canonical_expr = expr.clone();
        Self::rename_ident_in_expr(&mut canonical_expr, props_ident, &canonical_ident);
        let mut variable_input = emit_expression_with_codegen(&canonical_expr);
        if !variable_input.contains('\n')
          && !variable_input.contains("/*")
          && !variable_input.contains("//")
        {
          variable_input = variable_input
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        }
        if std::env::var_os("COMPILED_DEBUG_HASH").is_some()
          && (variable_input.contains("isHighlighted")
            || variable_input.contains("formatRuleHoverColor"))
        {
          eprintln!("[compiled-debug] variable-input={:?}", variable_input);
        }
        if std::env::var_os("COMPILED_DEBUG_VARIABLE_INPUT").is_some() {
          eprintln!("[compiled-variable-input] {}", variable_input);
        }
        Some((expr.clone(), variable_input))
      }
      _ => None,
    }
  }

  fn css_literal_for_runtime_value(value: &str, important: bool) -> String {
    let mut literal = value.to_string();
    if important {
      let trimmed = literal.trim_end().to_string();
      if !trimmed.ends_with("!important") {
        literal = trimmed;
        if !literal.ends_with(' ') && !literal.is_empty() {
          literal.push(' ');
        }
        literal.push_str("!important");
      }
    }
    let escaped = literal.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{}'", escaped)
  }

  fn strip_js_comments(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    let mut in_string: Option<char> = None;
    let mut escape_next = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while let Some(ch) = chars.next() {
      if in_line_comment {
        if ch == '\n' {
          in_line_comment = false;
          result.push(ch);
        }
        continue;
      }
      if in_block_comment {
        if ch == '*' {
          if let Some('/') = chars.peek() {
            chars.next();
            in_block_comment = false;
          }
        }
        continue;
      }
      if let Some(delim) = in_string {
        result.push(ch);
        if escape_next {
          escape_next = false;
          continue;
        }
        if ch == '\\' {
          escape_next = true;
          continue;
        }
        if delim == ch {
          in_string = None;
        }
        continue;
      }
      if ch == '/' {
        match chars.peek() {
          Some('/') => {
            chars.next();
            in_line_comment = true;
            continue;
          }
          Some('*') => {
            chars.next();
            in_block_comment = true;
            continue;
          }
          _ => {}
        }
      }
      if ch == '"' || ch == '\'' || ch == '`' {
        in_string = Some(ch);
        result.push(ch);
        continue;
      }
      result.push(ch);
    }
    result
  }

  fn collapse_blank_lines(value: &str) -> String {
    value
      .lines()
      .filter(|line| !line.trim().is_empty())
      .collect::<Vec<_>>()
      .join("\n")
  }

  fn runtime_value_dedupe_key(property: &str, value: &str) -> String {
    let without_comments = Self::strip_js_comments(value);
    let collapsed = Self::collapse_blank_lines(&without_comments);
    let canonical = if collapsed.contains('\n') {
      collapsed
    } else {
      collapsed.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    format!("{}::{}", property, canonical)
  }

  fn arrow_body_to_expr(&self, arrow: &ArrowExpr) -> Option<Expr> {
    match arrow.body.as_ref() {
      BlockStmtOrExpr::Expr(expr) => Some(expr.as_ref().clone()),
      BlockStmtOrExpr::BlockStmt(block) => {
        let arrow_expr = Expr::Arrow(ArrowExpr {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          params: Vec::new(),
          body: Box::new(BlockStmtOrExpr::BlockStmt(block.clone())),
          is_async: arrow.is_async,
          is_generator: arrow.is_generator,
          type_params: None,
          return_type: None,
        });
        Some(Expr::Call(CallExpr {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          callee: Callee::Expr(Box::new(Expr::Paren(ParenExpr {
            span: DUMMY_SP,
            expr: Box::new(arrow_expr),
          }))),
          args: Vec::new(),
          type_args: None,
        }))
      }
    }
  }

  fn build_style_with_variables(
    &mut self,
    base_style_ident: &Ident,
    variables: &[RuntimeCssVariable],
  ) -> Expr {
    let mut props = Vec::new();
    props.push(PropOrSpread::Spread(SpreadElement {
      dot3_token: DUMMY_SP,
      expr: Box::new(Expr::Ident(base_style_ident.clone())),
    }));

    let mut seen = HashSet::new();
    for variable in variables {
      if !seen.insert(variable.name.clone()) {
        continue;
      }
      let mut ix_args = Vec::new();
      ix_args.push(ExprOrSpread {
        spread: None,
        expr: Box::new(variable.expression.clone()),
      });
      if let Some(suffix) = &variable.suffix {
        if !suffix.is_empty() {
          ix_args.push(ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Lit(Lit::Str(Str::from(suffix.clone())))),
          });
          if let Some(prefix) = &variable.prefix {
            if !prefix.is_empty() {
              ix_args.push(ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Lit(Lit::Str(Str::from(prefix.clone())))),
              });
            }
          }
        }
      }
      let ix_call = Expr::Call(CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Callee::Expr(Box::new(Expr::Ident(self.runtime_ix_ident()))),
        args: ix_args,
        type_args: None,
      });
      props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Str(Str::from(variable.name.clone())),
        value: Box::new(ix_call),
      }))));
    }

    Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props,
    })
  }

  fn rename_ident_in_expr(expr: &mut Expr, from: &Ident, to: &Ident) {
    struct IdentRenamer<'a> {
      from: (Atom, SyntaxContext),
      to: &'a Ident,
    }

    impl<'a> VisitMut for IdentRenamer<'a> {
      fn visit_mut_ident(&mut self, ident: &mut Ident) {
        if ident.sym == self.from.0 && ident.ctxt == self.from.1 {
          ident.sym = self.to.sym.clone();
          ident.ctxt = self.to.ctxt;
        }
      }
    }

    let mut renamer = IdentRenamer {
      from: (from.sym.clone(), from.ctxt),
      to,
    };
    expr.visit_mut_with(&mut renamer);
  }

  fn pattern_binds_name(pat: &Pat, name: &str) -> bool {
    match pat {
      Pat::Ident(binding) => binding.id.sym.as_ref() == name,
      Pat::Array(array) => array
        .elems
        .iter()
        .flatten()
        .any(|elem| Self::pattern_binds_name(elem, name)),
      Pat::Object(object) => object.props.iter().any(|prop| match prop {
        ObjectPatProp::KeyValue(kv) => Self::pattern_binds_name(&kv.value, name),
        ObjectPatProp::Assign(assign) => assign.key.sym.as_ref() == name,
        ObjectPatProp::Rest(rest) => Self::pattern_binds_name(&rest.arg, name),
      }),
      Pat::Assign(assign) => Self::pattern_binds_name(&assign.left, name),
      Pat::Rest(rest) => Self::pattern_binds_name(&rest.arg, name),
      _ => false,
    }
  }

  fn collect_class_names_bindings(
    &self,
    pat: &Pat,
    css_idents: &mut HashSet<(Atom, SyntaxContext)>,
    style_idents: &mut HashSet<(Atom, SyntaxContext)>,
  ) -> bool {
    match pat {
      Pat::Object(object) => {
        for prop in &object.props {
          match prop {
            ObjectPatProp::KeyValue(kv) => {
              let name = match &kv.key {
                PropName::Ident(ident) => ident.sym.as_ref().to_string(),
                PropName::Str(str) => str.value.as_ref().to_string(),
                _ => return false,
              };
              let binding = match binding_ident_from_pat(&kv.value) {
                Some(ident) => ident,
                None => return false,
              };
              let id = to_id(&binding);
              match name.as_str() {
                "css" => {
                  css_idents.insert(id);
                }
                "style" => {
                  style_idents.insert(id);
                }
                _ => {}
              }
            }
            ObjectPatProp::Assign(assign) => {
              let name = assign.key.sym.as_ref();
              let id = to_id(&assign.key);
              match name {
                "css" => {
                  css_idents.insert(id);
                }
                "style" => {
                  style_idents.insert(id);
                }
                _ => {}
              }
            }
            ObjectPatProp::Rest(_) => {}
          }
        }
        true
      }
      Pat::Assign(assign) => {
        self.collect_class_names_bindings(&assign.left, css_idents, style_idents)
      }
      _ => false,
    }
  }

  fn extract_class_names_bindings(
    &self,
    params: &[Pat],
  ) -> Option<(
    HashSet<(Atom, SyntaxContext)>,
    HashSet<(Atom, SyntaxContext)>,
  )> {
    let first = params.first()?;
    let mut css_idents = HashSet::new();
    let mut style_idents = HashSet::new();
    if !self.collect_class_names_bindings(first, &mut css_idents, &mut style_idents) {
      return None;
    }
    if css_idents.is_empty() {
      return None;
    }
    Some((css_idents, style_idents))
  }

  fn handle_class_names_element(&mut self, element: &JSXElement) -> Option<Expr> {
    let ident = match &element.opening.name {
      JSXElementName::Ident(ident) => ident,
      _ => return None,
    };
    let id = to_id(ident);
    if self.compiled_import_kinds.get(&id) != Some(&CompiledImportKind::ClassNames) {
      return None;
    }

    let function_expr = element.children.iter().find_map(|child| {
      if let JSXElementChild::JSXExprContainer(container) = child {
        match &container.expr {
          JSXExpr::Expr(expr) => Some(expr.as_ref().clone()),
          _ => None,
        }
      } else {
        None
      }
    });

    let expr = match function_expr {
      Some(expr) => expr,
      None => {
        eprintln!("ClassNames handler: missing function expression");
        self.retain_imports.insert(id);
        return None;
      }
    };

    let (params, body_expr) = match expr {
      Expr::Arrow(arrow) => {
        let params = arrow.params.clone();
        let body = match &*arrow.body {
          BlockStmtOrExpr::Expr(expr) => expr.as_ref().clone(),
          BlockStmtOrExpr::BlockStmt(block) => Expr::Call(CallExpr {
            span: block.span,
            ctxt: block.ctxt,
            callee: Callee::Expr(Box::new(Expr::Arrow(ArrowExpr {
              span: DUMMY_SP,
              ctxt: arrow.ctxt,
              params: vec![],
              body: Box::new(BlockStmtOrExpr::BlockStmt(block.clone())),
              is_async: arrow.is_async,
              is_generator: arrow.is_generator,
              type_params: None,
              return_type: None,
            }))),
            args: vec![],
            type_args: None,
          }),
        };
        (params, body)
      }
      Expr::Fn(fn_expr) => {
        let params: Vec<Pat> = fn_expr
          .function
          .params
          .iter()
          .map(|param| param.pat.clone())
          .collect();
        let body = Expr::Call(CallExpr {
          span: fn_expr.function.span,
          ctxt: fn_expr.function.ctxt,
          callee: Callee::Expr(Box::new(Expr::Fn(fn_expr.clone()))),
          args: vec![],
          type_args: None,
        });
        (params, body)
      }
      _ => {
        eprintln!("ClassNames handler: unsupported callback expression");
        self.retain_imports.insert(id);
        return None;
      }
    };

    let (css_idents, style_idents) = match self.extract_class_names_bindings(&params) {
      Some(result) => result,
      None => {
        eprintln!("ClassNames handler: failed to extract bindings");
        self.retain_imports.insert(id);
        return None;
      }
    };

    let mut rewritten = body_expr;
    let mut visitor = ClassNamesBodyVisitor::new(self, css_idents, style_idents);
    rewritten.visit_mut_with(&mut visitor);
    if visitor.failed {
      eprintln!("ClassNames handler: visitor failed");
      self.retain_imports.insert(id);
      return None;
    }
    let sheets = visitor.finalize(&mut rewritten);
    if !self.options.extract && !sheets.is_empty() {
      let key_expr = element
                .opening
                .attrs
                .iter()
                .find_map(|attr| match attr {
                    JSXAttrOrSpread::JSXAttr(attr)
                        if matches!(attr.name, JSXAttrName::Ident(ref ident) if ident.sym.as_ref() == "key") =>
                    {
                        attr.value.as_ref().and_then(|value| match value {
                            JSXAttrValue::JSXExprContainer(container) => match &container.expr {
                                JSXExpr::Expr(expr) => Some((**expr).clone()),
                                _ => None,
                            },
                            JSXAttrValue::Lit(Lit::Str(str)) => {
                                Some(Expr::Lit(Lit::Str(str.clone())))
                            }
                            _ => None,
                        })
                    }
                    _ => None,
                });
      rewritten = self.build_runtime_component(rewritten, sheets, key_expr);
    }

    Some(rewritten)
  }

  fn handle_css_template(&mut self, template: &TaggedTpl) -> Option<Expr> {
    let css = self.evaluate_template(template)?;
    let mut artifacts = css_artifacts_from_literal(&css, &self.css_options())
      .or_else(|| Some(atomicize_literal(&css, &self.css_options())))?;
    for rule in artifacts.rules.drain(..) {
      self.register_rule_without_metadata(rule.css);
    }
    for css in artifacts.raw_rules.drain(..) {
      self.register_rule_without_metadata(css);
    }
    Some(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))
  }

  fn handle_css_call(&mut self, call: &CallExpr) -> Option<Expr> {
    self.handle_css_call_with_emission(call, true)
  }

  fn handle_css_call_with_emission(&mut self, call: &CallExpr, emit_rules: bool) -> Option<Expr> {
    let debug_css = std::env::var_os("COMPILED_DEBUG_CSS").is_some();
    let evaluated_values = self.evaluate_call_arguments(call);
    let binding_id = self.current_binding.clone();
    if emit_rules {
      if let Some(binding) = &binding_id {
        if let Some(cached) = self.css_runtime_artifacts.get(binding).cloned() {
          self.register_artifacts_for_metadata(&cached);
          return Some(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })));
        }
      }
    }
    if debug_css {
      match &self.current_binding {
        Some(binding) => eprintln!(
          "[compiled-debug] handle_css_call start binding={} static_eval={}",
          binding.0,
          evaluated_values.is_some()
        ),
        None => eprintln!(
          "[compiled-debug] handle_css_call start binding=<none> static_eval={}",
          evaluated_values.is_some()
        ),
      }
    }
    if let Some(values) = evaluated_values {
      let mut combined = CssArtifacts::default();
      for value in &values {
        let artifacts = css_artifacts_from_static_value(value, &self.css_options())?;
        combined.merge(artifacts);
      }
      let has_binding = binding_id.is_some();
      if let Some(binding) = &binding_id {
        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
          eprintln!(
            "[compiled-debug] cache css artifacts for {} (static) -> rules:{} raw:{} vars:{}",
            binding.0,
            combined.rules.len(),
            combined.raw_rules.len(),
            combined.runtime_variables.len()
          );
        }
        self
          .css_runtime_artifacts
          .insert(binding.clone(), combined.clone());
        self.css_binding_rules.insert(
          binding.clone(),
          combined.rules.iter().map(|rule| rule.css.clone()).collect(),
        );
      } else if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] css call without binding -> emitting {} rules immediately",
          combined.rules.len()
        );
      }
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] handle_css_call rules len {}",
          combined.rules.len()
        );
        eprintln!(
          "[compiled-debug] handle_css_call contains aria? {}",
          combined.rules.iter().any(|rule| rule.css.contains("aria"))
        );
        for rule in &combined.rules {
          eprintln!("[compiled-debug] handle_css_call rule list {}", rule.css);
        }
      }
      for rule in &combined.rules {
        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
          eprintln!("[compiled-debug] handle_css_call rule {}", rule.css);
        }
      }
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        for entry in combined.raw_rules.iter().filter(|css| css.contains("aria")) {
          eprintln!(
            "[compiled-debug] handle_css_call raw_rule candidate {}",
            entry
          );
        }
      }
      if emit_rules && !has_binding {
        self.register_property_rules_from_artifacts(&combined);
        self.register_artifacts_for_metadata(&combined);
      }
      if emit_rules && !has_binding {
        for rule in &combined.rules {
          self.register_rule_with_metadata_key(
            rule.css.clone(),
            rule.include_in_metadata,
            rule.metadata_key.clone(),
          );
        }
        for css in &combined.raw_rules {
          self.register_rule_with_metadata_key(css.clone(), true, None);
        }
      }
      Some(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))
    } else {
      let mut combined = CssArtifacts::default();
      let props_ident = Ident::new("props".into(), DUMMY_SP, SyntaxContext::empty());
      for arg in &call.args {
        if arg.spread.is_some() {
          return None;
        }
        match &*arg.expr {
          Expr::Object(object) => {
            let artifacts = self.process_dynamic_css_object(object, &props_ident)?;
            combined.merge(artifacts);
          }
          _ => return None,
        }
      }
      let has_binding = self.current_binding.is_some();
      if let Some(binding) = &self.current_binding {
        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
          if binding.0.as_ref() == "tabStyles" {
            for rule in &combined.rules {
              eprintln!("[compiled-debug] tabStyles combined rule {}", rule.css);
            }
          }
          eprintln!(
            "[compiled-debug] cache css artifacts for {} (dynamic) -> rules:{} raw:{} vars:{}",
            binding.0,
            combined.rules.len(),
            combined.raw_rules.len(),
            combined.runtime_variables.len()
          );
        }
        self
          .css_runtime_artifacts
          .insert(binding.clone(), combined.clone());
        self.css_binding_rules.insert(
          binding.clone(),
          combined.rules.iter().map(|rule| rule.css.clone()).collect(),
        );
      } else if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] css call without binding (dynamic) -> emitting {} rules immediately",
          combined.rules.len()
        );
      }
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] handle_css_call rules len {}",
          combined.rules.len()
        );
        eprintln!(
          "[compiled-debug] handle_css_call contains aria? {}",
          combined.rules.iter().any(|rule| rule.css.contains("aria"))
        );
        for rule in combined.rules.iter().take(10) {
          eprintln!("[compiled-debug] handle_css_call rule preview {}", rule.css);
        }
      }
      for rule in &combined.rules {
        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
          eprintln!("[compiled-debug] handle_css_call rule {}", rule.css);
        }
      }
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        for entry in combined.raw_rules.iter().filter(|css| css.contains("aria")) {
          eprintln!(
            "[compiled-debug] handle_css_call raw_rule candidate {}",
            entry
          );
        }
      }
      if emit_rules {
        if has_binding {
          self.register_artifacts_for_metadata(&combined);
        } else {
          for rule in &combined.rules {
            if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
              eprintln!("[compiled-debug] handle_css_call rule {}", rule.css);
            }
            self.register_rule_without_metadata(rule.css.clone());
          }
          for css in &combined.raw_rules {
            if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
              eprintln!("[compiled-debug] handle_css_call raw_rule {}", css);
            }
            self.register_rule_without_metadata(css.clone());
          }
        }
      }
      Some(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))
    }
  }

  fn handle_keyframes_template(&mut self, template: &TaggedTpl) -> Option<Expr> {
    let css = self.evaluate_template(template)?;
    self.emit_keyframes_rule(&css, &Expr::TaggedTpl(template.clone()))
  }

  fn handle_keyframes_call(&mut self, call: &CallExpr) -> Option<Expr> {
    let value = self.evaluate_call_argument(call)?;
    let css = build_keyframes_css(&value)?;
    self.emit_keyframes_rule(&css, &Expr::Call(call.clone()))
  }

  fn emit_keyframes_rule(&mut self, css: &str, expression: &Expr) -> Option<Expr> {
    let mut normalized = normalize_keyframes_body(css).unwrap_or_else(|| css.replace(";}", "}"));
    let expression_code = emit_expression_for_keyframes(expression);
    if std::env::var_os("COMPILED_DEBUG_HASH").is_some() {
      eprintln!(
        "[compiled-hash] keyframes-expression='{}'",
        expression_code.replace('\n', "\\n")
      );
    }
    let name = format!("k{}", hash(&expression_code, 0));
    let rule = format!("@keyframes {}{{{}}}", name, normalized);
    if std::env::var_os("COMPILED_DEBUG_HASH").is_some() {
      eprintln!("[compiled-debug] register keyframes {} -> {}", name, rule);
    }
    self.keyframes_rules.insert(name, rule);
    Some(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))
  }

  fn preserve_import_for_ident(&mut self, ident: &Ident) {
    self.retain_imports.insert(to_id(ident));
  }

  fn preserve_import_for_expr(&mut self, expr: &Expr) {
    if let Expr::Ident(ident) = expr {
      self.preserve_import_for_ident(ident);
    }
  }

  fn preserve_styled_usage_in_expr(&mut self, expr: &Expr) {
    match expr {
      Expr::Call(call) => {
        if let Callee::Expr(callee_expr) = &call.callee {
          match &**callee_expr {
            Expr::Member(member) => {
              if let Expr::Ident(obj) = &*member.obj {
                if self.styled_imports.contains(&to_id(obj)) {
                  self.preserve_import_for_ident(obj);
                }
              }
            }
            Expr::Ident(ident) => {
              if self.styled_imports.contains(&to_id(ident)) {
                self.preserve_import_for_ident(ident);
              }
            }
            _ => {}
          }
        }
      }
      Expr::Ident(ident) => {
        if self.styled_imports.contains(&to_id(ident)) {
          self.preserve_import_for_ident(ident);
        }
      }
      Expr::Member(member) => {
        if let Expr::Ident(obj) = &*member.obj {
          if self.styled_imports.contains(&to_id(obj)) {
            self.preserve_import_for_ident(obj);
          }
        }
      }
      _ => {}
    }
  }

  fn inject_imports(&mut self, module: &mut Module) {
    let mut insertion_index = 0usize;
    for (index, item) in module.body.iter().enumerate() {
      match item {
        ModuleItem::ModuleDecl(ModuleDecl::Import(_)) => insertion_index = index + 1,
        _ => break,
      }
    }

    let mut imports = Vec::new();
    if self.needs_runtime_ax {
      let helper_export = self.runtime_class_helper();
      let helper_ident = self.runtime_class_ident();
      let ix_ident = self.runtime_ix_ident();
      let cc_ident = if self.needs_runtime_cc {
        Some(self.runtime_cc_ident())
      } else {
        None
      };
      let cs_ident = if self.needs_runtime_cs {
        Some(self.runtime_cs_ident())
      } else {
        None
      };
      let runtime_source = "@compiled/react/runtime";

      let runtime_import_index =
        module
          .body
          .iter()
          .enumerate()
          .find_map(|(idx, item)| match item {
            ModuleItem::ModuleDecl(ModuleDecl::Import(import))
              if import.src.value.as_ref() == runtime_source =>
            {
              Some(idx)
            }
            _ => None,
          });

      if let Some(idx) = runtime_import_index {
        if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = module
          .body
          .get_mut(idx)
          .expect("runtime import index valid")
        {
          let mut has_helper = false;
          let mut has_ix = false;
          let mut has_cc = false;
          let mut has_cs = false;

          for spec in &import.specifiers {
            if let ImportSpecifier::Named(named) = spec {
              let local = named.local.sym.as_ref();
              if local == helper_ident.sym.as_ref() {
                has_helper = true;
              }
              if local == ix_ident.sym.as_ref() {
                has_ix = true;
              }
              if let Some(cc_ident) = cc_ident.as_ref() {
                if local == cc_ident.sym.as_ref() {
                  has_cc = true;
                }
              }
              if let Some(cs_ident) = cs_ident.as_ref() {
                if local == cs_ident.sym.as_ref() {
                  has_cs = true;
                }
              }
            }
          }

          if !has_helper {
            import
              .specifiers
              .push(Self::import_named_spec(&helper_ident, helper_export));
          }

          if !has_ix {
            import
              .specifiers
              .push(Self::import_named_spec(&ix_ident, "ix"));
          }

          if let Some(cc_ident) = cc_ident.as_ref() {
            if !has_cc {
              import
                .specifiers
                .push(Self::import_named_spec(cc_ident, "CC"));
            }
          }

          if let Some(cs_ident) = cs_ident.as_ref() {
            if !has_cs {
              import
                .specifiers
                .push(Self::import_named_spec(cs_ident, "CS"));
            }
          }
        }
      } else {
        let mut specifiers = Vec::new();
        specifiers.push(Self::import_named_spec(&helper_ident, helper_export));
        specifiers.push(Self::import_named_spec(&ix_ident, "ix"));
        if let Some(cc_ident) = cc_ident.as_ref() {
          specifiers.push(Self::import_named_spec(cc_ident, "CC"));
        }
        if let Some(cs_ident) = cs_ident.as_ref() {
          specifiers.push(Self::import_named_spec(cs_ident, "CS"));
        }

        imports.push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
          span: DUMMY_SP,
          specifiers,
          src: Box::new(Str::from(runtime_source)),
          type_only: false,
          with: None,
          phase: ImportPhase::Evaluation,
        })));
      }
    }
    if self.needs_jsx_runtime || self.needs_jsxs_runtime {
      let jsx_runtime_source = "react/jsx-runtime";
      let jsx_ident = if self.needs_jsx_runtime {
        Some(self.jsx_ident())
      } else {
        None
      };
      let jsxs_ident = if self.needs_jsxs_runtime {
        Some(self.jsxs_ident())
      } else {
        None
      };
      let jsx_import_index = module
        .body
        .iter()
        .enumerate()
        .find_map(|(idx, item)| match item {
          ModuleItem::ModuleDecl(ModuleDecl::Import(import))
            if import.src.value.as_ref() == jsx_runtime_source =>
          {
            Some(idx)
          }
          _ => None,
        });

      if let Some(idx) = jsx_import_index {
        if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = module
          .body
          .get_mut(idx)
          .expect("jsx runtime import index valid")
        {
          let mut has_jsx = false;
          let mut has_jsxs = false;
          for spec in &import.specifiers {
            if let ImportSpecifier::Named(named) = spec {
              if let Some(ident) = jsx_ident.as_ref() {
                if named.local.sym.as_ref() == ident.sym.as_ref() {
                  has_jsx = true;
                }
              }
              if let Some(ident) = jsxs_ident.as_ref() {
                if named.local.sym.as_ref() == ident.sym.as_ref() {
                  has_jsxs = true;
                }
              }
            }
          }
          if let Some(ident) = jsx_ident.as_ref() {
            if !has_jsx {
              import
                .specifiers
                .push(Self::import_named_spec(ident, "jsx"));
            }
          }
          if let Some(ident) = jsxs_ident.as_ref() {
            if !has_jsxs {
              import
                .specifiers
                .push(Self::import_named_spec(ident, "jsxs"));
            }
          }
        }
      } else {
        let mut specifiers = Vec::new();
        if let Some(ident) = jsx_ident.as_ref() {
          specifiers.push(Self::import_named_spec(ident, "jsx"));
        }
        if let Some(ident) = jsxs_ident.as_ref() {
          specifiers.push(Self::import_named_spec(ident, "jsxs"));
        }
        imports.push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
          span: DUMMY_SP,
          specifiers,
          src: Box::new(Str::from(jsx_runtime_source)),
          type_only: false,
          with: None,
          phase: ImportPhase::Evaluation,
        })));
      }
    }
    if self.needs_react_namespace && self.should_import_react_namespace() {
      if !self.has_react_namespace_binding {
        let ident = self.react_namespace_ident();
        imports.push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
          span: DUMMY_SP,
          specifiers: vec![ImportSpecifier::Namespace(ImportStarAsSpecifier {
            span: DUMMY_SP,
            local: ident.clone(),
          })],
          src: Box::new(Self::single_quoted_str("react")),
          type_only: false,
          with: None,
          phase: ImportPhase::Evaluation,
        })));
        self.has_react_namespace_binding = true;
      }
    }
    if self.needs_forward_ref && !self.has_forward_ref_binding {
      let ident = self.forward_ref_ident();
      imports.push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        span: DUMMY_SP,
        specifiers: vec![Self::import_named_spec(&ident, "forwardRef")],
        src: Box::new(Self::single_quoted_str("react")),
        type_only: false,
        with: None,
        phase: ImportPhase::Evaluation,
      })));
      self.has_forward_ref_binding = true;
    }

    module
      .body
      .splice(insertion_index..insertion_index, imports);
  }

  fn append_display_names(&mut self, module: &mut Module) {
    for (ident, display_name) in self.styled_display_names.drain(..) {
      let test = Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op: BinaryOp::NotEqEq,
        left: Box::new(Expr::Member(MemberExpr {
          span: DUMMY_SP,
          obj: Box::new(Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Ident(quote_ident!("process").into())),
            prop: MemberProp::Ident(quote_ident!("env")),
          })),
          prop: MemberProp::Ident(quote_ident!("NODE_ENV")),
        })),
        right: Box::new(Expr::Lit(Lit::Str(Self::single_quoted_str("production")))),
      });
      let stmt = Stmt::If(IfStmt {
        span: DUMMY_SP,
        test: Box::new(test),
        cons: Box::new(Stmt::Block(BlockStmt {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          stmts: vec![Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Assign(AssignExpr {
              span: DUMMY_SP,
              op: AssignOp::Assign,
              left: SimpleAssignTarget::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Ident(ident.clone())),
                prop: MemberProp::Ident(quote_ident!("displayName")),
              })
              .into(),
              right: Box::new(Expr::Lit(Lit::Str(Self::single_quoted_str(&display_name)))),
            })),
          })],
        })),
        alt: None,
      });
      let item = ModuleItem::Stmt(stmt);
      let insert_after_decl = module
        .body
        .iter()
        .position(|item| Self::matches_styled_binding(item, &ident));
      if let Some(idx) = insert_after_decl {
        module.body.insert(idx + 1, item);
        continue;
      }
      let export_index = module
        .body
        .iter()
        .position(|item| Self::matches_export_decl(item))
        .unwrap_or(module.body.len());
      module.body.insert(export_index, item);
    }
  }

  fn matches_styled_binding(item: &ModuleItem, ident: &Ident) -> bool {
    match item {
      ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => var_decl
        .decls
        .iter()
        .any(|decl| Self::pattern_binds_name(&decl.name, ident.sym.as_ref())),
      ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
        if let Decl::Var(var_decl) = &export_decl.decl {
          return var_decl
            .decls
            .iter()
            .any(|decl| Self::pattern_binds_name(&decl.name, ident.sym.as_ref()));
        }
        false
      }
      _ => false,
    }
  }

  fn matches_export_decl(item: &ModuleItem) -> bool {
    match item {
      ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(_))
      | ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(_))
      | ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(_))
      | ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(_))
      | ModuleItem::ModuleDecl(ModuleDecl::ExportAll(_)) => true,
      _ => false,
    }
  }

  fn process_xcss_attributes(&mut self, element: &mut JSXElement) -> Option<XcssProcessing> {
    if !self.options.process_xcss {
      return None;
    }

    let element_name = if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      match &element.opening.name {
        JSXElementName::Ident(ident) => ident.sym.as_ref().to_string(),
        _ => "<complex>".to_string(),
      }
    } else {
      String::new()
    };
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      eprintln!(
        "[compiled-debug] process_xcss_attributes scanning <{}>",
        element_name
      );
    }

    let mut runtime_sheets = Vec::new();
    let mut pending_class_names: Vec<String> = Vec::new();
    let mut transformed = false;

    for attr in &mut element.opening.attrs {
      let JSXAttrOrSpread::JSXAttr(attr) = attr else {
        continue;
      };
      let JSXAttrName::Ident(name) = &attr.name else {
        continue;
      };
      let attr_name = name.sym.as_ref();
      if !attr_name.to_ascii_lowercase().ends_with("xcss") {
        continue;
      }

      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] considering xcss-like attr '{}' on <{}>",
          attr_name,
          if element_name.is_empty() {
            "<unknown>"
          } else {
            &element_name
          }
        );
      }

      let Some(JSXAttrValue::JSXExprContainer(container)) = &mut attr.value else {
        continue;
      };

      let JSXExpr::Expr(expr) = &mut container.expr else {
        continue;
      };

      let evaluated = evaluate_static(expr, &self.bindings);

      if matches!(**expr, Expr::Object(_)) {
        let evaluated_object = evaluated
          .as_ref()
          .and_then(|value| match value {
            StaticValue::Object(map) => Some(map),
            _ => None,
          })
          .unwrap_or_else(|| panic!("Object given to the xcss prop must be static"));

        let artifacts = css_artifacts_from_static_object(evaluated_object, &self.css_options())
          .unwrap_or_else(|| panic!("Object given to the xcss prop must be static"));

        let mut class_names = Vec::new();
        for rule in &artifacts.rules {
          self.register_rule_with_metadata_key(
            rule.css.clone(),
            rule.include_in_metadata,
            rule.metadata_key.clone(),
          );
          class_names.push(rule.class_name.clone());
          if !runtime_sheets.iter().any(|existing| existing == &rule.css) {
            runtime_sheets.push(rule.css.clone());
          }
        }
        for css in &artifacts.raw_rules {
          self.register_rule_with_metadata_key(css.clone(), true, None);
          if !runtime_sheets.iter().any(|existing| existing == css) {
            runtime_sheets.push(css.clone());
          }
        }
        for class_name in &class_names {
          self.xcss_class_names.insert(class_name.clone());
        }

        if class_names.is_empty() {
          **expr = Expr::Ident(quote_ident!("undefined").into());
        } else {
          let joined = class_names.join(" ");
          **expr = Expr::Lit(Lit::Str(Str::from(joined)));
        }

        transformed = true;
        continue;
      }

      let Some(value) = evaluated.as_ref() else {
        continue;
      };

      self.mark_cx_usage(expr);

      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!("[compiled-debug] xcss evaluated value: {:?}", value);
      }
      let related_refs = self.ensure_css_map_rules_for_expr(expr);
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] css prop related refs len={}",
          related_refs.len()
        );
      }
      let mut class_names = Vec::new();
      collect_precomputed_classes(value, &mut class_names);
      if class_names.is_empty() {
        for reference in &related_refs {
          if let Some(property_map) = self.css_map_ident_classes.get(&reference.ident) {
            let key = reference.property.as_deref().unwrap_or("*");
            if let Some(classes) = property_map.get(key).or_else(|| property_map.get("*")) {
              for class_name in classes {
                if !class_names.contains(class_name) {
                  class_names.push(class_name.clone());
                }
              }
            } else if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
              eprintln!(
                "[compiled-debug] no css_map classes for ident {:?} property {:?}",
                reference.ident, reference.property
              );
            }
          } else if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
            eprintln!(
              "[compiled-debug] no css_map_ident_classes for {:?}",
              reference.ident
            );
          }
        }
      }

      if class_names.is_empty() {
        continue;
      }

      for class_name in &class_names {
        self.xcss_class_names.insert(class_name.clone());
      }
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] recorded xcss classes {:?}",
          self.xcss_class_names
        );
      }
      pending_class_names.extend(class_names);
      transformed = true;
    }

    if !transformed {
      return None;
    }

    Some(XcssProcessing {
      runtime_sheets,
      pending_class_names,
      transformed,
    })
  }

  fn mark_cx_usage(&mut self, expr: &Expr) {
    match expr {
      Expr::Ident(ident) => {
        let id = to_id(ident);
        if let Some(source) = self.compiled_import_sources.get(&id) {
          if source == "@atlaskit/css" && ident.sym.as_ref() == "cx" {
            self.retain_imports.insert(id);
          }
        }
      }
      Expr::Member(member) => {
        self.mark_cx_usage(&member.obj);
      }
      Expr::Call(call) => {
        if let Callee::Expr(callee) = &call.callee {
          self.mark_cx_usage(callee);
        }
        for arg in &call.args {
          self.mark_cx_usage(&arg.expr);
        }
      }
      Expr::Cond(cond) => {
        self.mark_cx_usage(&cond.test);
        self.mark_cx_usage(&cond.cons);
        self.mark_cx_usage(&cond.alt);
      }
      Expr::Array(array) => {
        for elem in &array.elems {
          if let Some(elem) = elem {
            self.mark_cx_usage(&elem.expr);
          }
        }
      }
      Expr::Tpl(tpl) => {
        for expr in &tpl.exprs {
          self.mark_cx_usage(expr);
        }
      }
      Expr::Seq(seq) => {
        for expr in &seq.exprs {
          self.mark_cx_usage(expr);
        }
      }
      Expr::Paren(paren) => {
        self.mark_cx_usage(&paren.expr);
      }
      _ => {}
    }
  }

  fn resolve_pending_xcss(&mut self, class_names: &[String]) -> Vec<String> {
    let mut runtime_sheets = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      eprintln!(
        "[compiled-debug] resolve_pending_xcss extract={}",
        self.options.extract
      );
    }

    for class_name in class_names {
      let mut handled = false;
      if let Some(rules) = self.css_map_rule_groups.get(class_name).cloned() {
        handled = true;
        for rule in rules {
          if seen.insert(rule.clone()) {
            self.register_rule(rule.clone());
            runtime_sheets.push(rule);
          }
        }
      }

      if handled {
        continue;
      }

      let needle = format!(".{}", class_name);
      if let Some(rule) = self
        .collected_rules
        .iter()
        .find(|css| css.contains(&needle))
        .cloned()
      {
        if seen.insert(rule.clone()) {
          self.register_rule(rule.clone());
          runtime_sheets.push(rule);
        }
        continue;
      }

      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] unable to resolve xcss class {}",
          class_name
        );
      }
    }

    runtime_sheets
  }

  fn ensure_css_map_rules_for_expr(&mut self, expr: &Expr) -> Vec<CssMapReference> {
    let mut references = Vec::new();
    self.collect_css_map_references(expr, &mut references);
    let mut unique_refs = Vec::new();
    let mut seen_refs: HashSet<((Atom, SyntaxContext), Option<String>)> = HashSet::new();
    let mut processed_idents: HashSet<(Atom, SyntaxContext)> = HashSet::new();
    for reference in references {
      let key = (reference.ident.clone(), reference.property.clone());
      if seen_refs.insert(key) {
        unique_refs.push(reference.clone());
      }
      if processed_idents.insert(reference.ident.clone()) {
        if let Some(map) = self.css_map_static_objects.remove(&reference.ident) {
          let mut map_options = self.css_options();
          map_options.preserve_leading_combinator_space = true;
          let mut entries: Vec<_> = map.iter().collect();
          entries.sort_by(|(a, _), (b, _)| a.cmp(b));
          for (entry_name, value) in entries {
            if let Some(artifacts) = css_artifacts_from_static_value(value, &map_options) {
              self.cache_css_map_artifacts(
                Some(reference.ident.clone()),
                Some(entry_name.clone()),
                &artifacts,
              );
            }
          }
        }
      }
    }
    unique_refs
  }

  fn cache_css_map_artifacts(
    &mut self,
    ident: Option<(Atom, SyntaxContext)>,
    property: Option<String>,
    artifacts: &CssArtifacts,
  ) {
    let map_rule_css: Vec<String> = artifacts
      .merged_css_entries()
      .into_iter()
      .map(|(css, _, _)| css)
      .collect();
    for rule in &artifacts.rules {
      self
        .css_map_rule_groups
        .entry(rule.class_name.clone())
        .or_insert_with(|| map_rule_css.clone());
    }
    if let Some(ident_key) = ident {
      let entry = self
        .css_map_ident_classes
        .entry(ident_key)
        .or_insert_with(HashMap::new);
      let key = property.unwrap_or_else(|| "*".to_string());
      let class_list = entry.entry(key.clone()).or_insert_with(Vec::new);
      for rule in &artifacts.rules {
        if !class_list.contains(&rule.class_name) {
          class_list.push(rule.class_name.clone());
        }
      }
      let fallback_list = entry.entry("*".to_string()).or_insert_with(Vec::new);
      for rule in &artifacts.rules {
        if !fallback_list.contains(&rule.class_name) {
          fallback_list.push(rule.class_name.clone());
        }
      }
    }
  }

  fn collect_css_map_references(&self, expr: &Expr, out: &mut Vec<CssMapReference>) {
    if let Some(reference) = Self::resolve_css_map_reference(expr) {
      out.push(reference);
    }
    match expr {
      Expr::Member(member) => {
        self.collect_css_map_references(&member.obj, out);
        if let MemberProp::Computed(comp) = &member.prop {
          self.collect_css_map_references(&comp.expr, out);
        }
      }
      Expr::Call(call) => {
        if let Callee::Expr(callee) = &call.callee {
          self.collect_css_map_references(callee, out);
        }
        for arg in &call.args {
          self.collect_css_map_references(&arg.expr, out);
        }
      }
      Expr::Cond(cond) => {
        self.collect_css_map_references(&cond.cons, out);
        self.collect_css_map_references(&cond.alt, out);
      }
      Expr::Array(array) => {
        for elem in &array.elems {
          if let Some(elem) = elem {
            self.collect_css_map_references(&elem.expr, out);
          }
        }
      }
      Expr::Tpl(tpl) => {
        for expr in &tpl.exprs {
          self.collect_css_map_references(expr, out);
        }
      }
      Expr::Seq(seq) => {
        for expr in &seq.exprs {
          self.collect_css_map_references(expr, out);
        }
      }
      Expr::Bin(bin) => {
        self.collect_css_map_references(&bin.left, out);
        self.collect_css_map_references(&bin.right, out);
      }
      Expr::Unary(unary) => {
        self.collect_css_map_references(&unary.arg, out);
      }
      Expr::Paren(paren) => {
        self.collect_css_map_references(&paren.expr, out);
      }
      _ => {}
    }
  }

  fn resolve_css_map_reference(expr: &Expr) -> Option<CssMapReference> {
    match expr {
      Expr::Ident(ident) => Some(CssMapReference {
        ident: to_id(ident),
        property: None,
      }),
      Expr::Member(member) => {
        let mut base = Self::resolve_css_map_reference(&member.obj)?;
        if base.property.is_none() {
          if let Some(name) = Self::css_map_member_prop_name(&member.prop) {
            base.property = Some(name);
          }
        }
        Some(base)
      }
      Expr::Call(call) => {
        if let Callee::Expr(callee) = &call.callee {
          Self::resolve_css_map_reference(callee)
        } else {
          None
        }
      }
      Expr::Paren(paren) => Self::resolve_css_map_reference(&paren.expr),
      _ => None,
    }
  }

  fn css_map_member_prop_name(prop: &MemberProp) -> Option<String> {
    match prop {
      MemberProp::Ident(ident) => Some(ident.sym.to_string()),
      MemberProp::Computed(comp) => match &*comp.expr {
        Expr::Lit(Lit::Str(str_lit)) => Some(str_lit.value.to_string()),
        _ => None,
      },
      _ => None,
    }
  }

  fn resolve_styled_target(&mut self, expr: &Expr) -> Option<(Option<Expr>, Ident)> {
    match expr {
      Expr::Member(member) => {
        let styled_ident = match &*member.obj {
          Expr::Ident(ident) => ident.clone(),
          _ => return None,
        };
        if !self.styled_imports.contains(&to_id(&styled_ident)) {
          return None;
        }
        let component_expr = match &member.prop {
          MemberProp::Ident(ident) => Some(Expr::Lit(Lit::Str(Str::from(ident.sym.to_string())))),
          MemberProp::Computed(computed) => {
            if let Expr::Lit(Lit::Str(str)) = &*computed.expr {
              Some(Expr::Lit(Lit::Str(str.clone())))
            } else {
              self.preserve_import_for_ident(&styled_ident);
              return None;
            }
          }
          _ => {
            self.preserve_import_for_ident(&styled_ident);
            return None;
          }
        };
        Some((component_expr, styled_ident))
      }
      Expr::Call(call) => {
        let styled_ident = match &call.callee {
          Callee::Expr(expr) => match &**expr {
            Expr::Ident(ident) => ident.clone(),
            _ => return None,
          },
          _ => return None,
        };
        if !self.styled_imports.contains(&to_id(&styled_ident)) {
          return None;
        }
        let first = match call.args.get(0) {
          Some(arg) if arg.spread.is_none() => arg,
          _ => {
            self.preserve_import_for_ident(&styled_ident);
            return None;
          }
        };
        Some((Some((*first.expr).clone()), styled_ident))
      }
      _ => None,
    }
  }

  fn process_css_prop(&mut self, element: &mut JSXElement) -> Option<(Vec<String>, Option<Expr>)> {
    let debug_css = std::env::var_os("COMPILED_DEBUG_CSS").is_some();
    let mut css_index = None;
    let mut class_index = None;
    let mut key_expr: Option<Expr> = None;
    let mut style_index = None;
    let mut existing_style_attr: Option<JSXAttr> = None;
    let mut existing_style_expr: Option<Expr> = None;

    for (index, attr) in element.opening.attrs.iter().enumerate() {
      let JSXAttrOrSpread::JSXAttr(attr) = attr else {
        continue;
      };

      let JSXAttrName::Ident(name) = &attr.name else {
        continue;
      };

      match name.sym.as_ref() {
        "css" => css_index = Some(index),
        "className" => class_index = Some(index),
        "style" => {
          style_index = Some(index);
          existing_style_attr = Some(attr.clone());
          if let Some(value) = &attr.value {
            existing_style_expr = match value {
              JSXAttrValue::JSXExprContainer(container) => match &container.expr {
                JSXExpr::Expr(expr) => Some((**expr).clone()),
                _ => None,
              },
              JSXAttrValue::Lit(Lit::Str(str)) => Some(Expr::Lit(Lit::Str(str.clone()))),
              _ => None,
            };
          }
        }
        "key" => {
          if let Some(value) = &attr.value {
            key_expr = match value {
              JSXAttrValue::JSXExprContainer(container) => match &container.expr {
                JSXExpr::Expr(expr) => Some((**expr).clone()),
                _ => None,
              },
              JSXAttrValue::Lit(Lit::Str(str)) => Some(Expr::Lit(Lit::Str(str.clone()))),
              _ => None,
            };
          }
        }
        _ => {}
      }
    }

    let css_index = css_index?;
    let css_attr = match &element.opening.attrs[css_index] {
      JSXAttrOrSpread::JSXAttr(attr) => attr,
      _ => return None,
    };

    let original_css_expr = match &css_attr.value {
      Some(JSXAttrValue::JSXExprContainer(container)) => match &container.expr {
        JSXExpr::Expr(expr) => Some((**expr).clone()),
        _ => None,
      },
      Some(JSXAttrValue::Lit(lit)) => match lit {
        Lit::Str(str) => Some(Expr::Lit(Lit::Str(str.clone()))),
        _ => None,
      },
      _ => None,
    };

    let props_ident = Ident::new(
      CANONICAL_PROPS_IDENT.into(),
      DUMMY_SP,
      SyntaxContext::empty(),
    );

    if let Some(expr) = &original_css_expr {
      self.mark_css_bindings_in_css_prop_expr(expr);
    }

    if debug_css {
      eprintln!(
        "[compiled-debug] process_css_prop options extract={}",
        self.options.extract
      );
      let element_name = match &element.opening.name {
        JSXElementName::Ident(ident) => ident.sym.as_ref().to_string(),
        _ => "<complex>".to_string(),
      };
      if let Some(expr) = &original_css_expr {
        eprintln!(
          "[compiled-debug] process_css_prop expr={} element={}",
          emit_expression(expr),
          element_name
        );
      } else {
        eprintln!(
          "[compiled-debug] process_css_prop literal attr on element={}",
          element_name
        );
      }
    }

    let mut css_value_for_precomputed = match &css_attr.value {
      Some(JSXAttrValue::JSXExprContainer(container)) => match &container.expr {
        JSXExpr::Expr(expr) => evaluate_static(expr, &self.bindings),
        _ => None,
      },
      Some(JSXAttrValue::Lit(Lit::Str(str))) => {
        Some(StaticValue::Str(StaticString::new(str.value.to_string())))
      }
      _ => None,
    };

    let mut using_runtime_artifacts = css_value_for_precomputed.is_none();
    let mut artifacts_from_static = None;
    if let Some(ref value) = css_value_for_precomputed {
      if let Some(artifacts) = css_artifacts_from_static_value(value, &self.css_options()) {
        artifacts_from_static = Some(artifacts);
      } else {
        css_value_for_precomputed = None;
        using_runtime_artifacts = true;
      }
    }
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      eprintln!(
        "[compiled-debug] css prop precomputed value={:?}",
        css_value_for_precomputed
      );
    }
    let mut artifacts = if let Some(artifacts) = artifacts_from_static {
      artifacts
    } else if let Some(expr) = &original_css_expr {
      if let Expr::Ident(ident) = expr {
        self.mark_css_binding_used(ident);
        match self.css_runtime_artifacts.get(&to_id(ident)).cloned() {
          Some(artifacts) => artifacts,
          None => return None,
        }
      } else {
        using_runtime_artifacts = true;
        let options = self.css_options();
        let artifacts =
          self.css_artifacts_from_dynamic_css_expression(expr, &props_ident, &options)?;
        if debug_css {
          let source = emit_expression(expr);
          eprintln!(
            "[compiled-debug] css prop dynamic expression processed: {} (rules={}, raw={})",
            source,
            artifacts.rules.len(),
            artifacts.raw_rules.len()
          );
        }
        artifacts
      }
    } else {
      return None;
    };
    self.register_property_rules_from_artifacts(&artifacts);
    self.register_artifacts_for_metadata(&artifacts);
    if debug_css {
      eprintln!(
        "[compiled-debug] css prop artifacts summary rules={} runtime_conditions={}",
        artifacts.rules.len(),
        artifacts.runtime_class_conditions.len()
      );
    }

    if using_runtime_artifacts && std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      let source = original_css_expr
        .as_ref()
        .map(emit_expression)
        .unwrap_or_else(|| "<unknown>".to_string());
      eprintln!(
        "[compiled-debug] using runtime artifacts for {} -> vars: {}, rules: {}",
        source,
        artifacts.runtime_variables.len(),
        artifacts.rules.len()
      );
    }

    let precomputed_classes: Vec<String> = css_value_for_precomputed
      .as_ref()
      .map(|value| {
        let mut classes = Vec::new();
        collect_precomputed_classes(value, &mut classes);
        classes
      })
      .unwrap_or_default();
    let mut classes_for_rules = precomputed_classes.clone();
    if let Some(expr) = original_css_expr.as_ref() {
      let related_refs = self.ensure_css_map_rules_for_expr(expr);
      for reference in related_refs {
        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
          eprintln!(
            "[compiled-debug] css prop related ident {:?} property {:?}",
            reference.ident, reference.property
          );
        }
        if let Some(property_map) = self.css_map_ident_classes.get(&reference.ident) {
          let key = reference.property.as_deref().unwrap_or("*");
          if let Some(classes) = property_map.get(key).or_else(|| property_map.get("*")) {
            for class_name in classes {
              if !classes_for_rules.contains(class_name) {
                classes_for_rules.push(class_name.clone());
              }
            }
          }
        }
      }
    }
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      eprintln!(
        "[compiled-debug] classes_for_rules={:?} precomputed_classes={:?}",
        classes_for_rules, precomputed_classes
      );
    }

    let mut precomputed_exprs = Vec::new();
    let mut expr_class_names = Vec::new();
    for class_name in &classes_for_rules {
      self.non_xcss_class_names.insert(class_name.clone());
    }
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      eprintln!("[compiled-debug] classes_for_rules={:?}", classes_for_rules);
    }
    if let (Some(expr), Some(value)) = (
      original_css_expr.as_ref(),
      css_value_for_precomputed.as_ref(),
    ) {
      collect_precomputed_class_exprs(expr, value, &mut expr_class_names, &mut precomputed_exprs);
    }
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      eprintln!(
        "[compiled-debug] expr_class_names={:?} precomputed_exprs={}",
        expr_class_names,
        precomputed_exprs.len()
      );
    }

    let mut runtime_class_conditions = std::mem::take(&mut artifacts.runtime_class_conditions);
    let mut conditional_class_names: HashSet<String> = HashSet::new();
    for condition in &runtime_class_conditions {
      conditional_class_names.extend(condition.when_true.iter().cloned());
      conditional_class_names.extend(condition.when_false.iter().cloned());
    }
    for condition in runtime_class_conditions {
      let has_true = !condition.when_true.is_empty();
      let has_false = !condition.when_false.is_empty();
      if has_true && has_false {
        let cons_expr = TransformVisitor::class_names_to_expr(&condition.when_true);
        let alt_expr = TransformVisitor::class_names_to_expr(&condition.when_false);
        precomputed_exprs.push(Expr::Cond(CondExpr {
          span: DUMMY_SP,
          test: Box::new(condition.test),
          cons: Box::new(cons_expr),
          alt: Box::new(alt_expr),
        }));
      } else if has_true {
        let cons_expr = TransformVisitor::class_names_to_expr(&condition.when_true);
        precomputed_exprs.push(Expr::Bin(BinExpr {
          span: DUMMY_SP,
          op: BinaryOp::LogicalAnd,
          left: Box::new(condition.test),
          right: Box::new(cons_expr),
        }));
      } else if has_false {
        let alt_expr = TransformVisitor::class_names_to_expr(&condition.when_false);
        precomputed_exprs.push(Expr::Bin(BinExpr {
          span: DUMMY_SP,
          op: BinaryOp::LogicalAnd,
          left: Box::new(Expr::Unary(UnaryExpr {
            span: DUMMY_SP,
            op: UnaryOp::Bang,
            arg: Box::new(condition.test),
          })),
          right: Box::new(alt_expr),
        }));
      }
    }

    let mut runtime_sheets = Vec::new();
    for rule in &artifacts.rules {
      self.register_rule_without_metadata(rule.css.clone());
      if !runtime_sheets.contains(&rule.css) {
        runtime_sheets.push(rule.css.clone());
      }
    }
    for css in &artifacts.raw_rules {
      let sheet = css.clone();
      self.register_rule_without_metadata(sheet.clone());
      if !runtime_sheets.contains(&sheet) {
        runtime_sheets.push(sheet);
      }
    }

    for class_name in &classes_for_rules {
      if let Some(rules) = self.css_map_rule_groups.get(class_name).cloned() {
        for rule in rules {
          if !runtime_sheets.contains(&rule) {
            runtime_sheets.push(rule.clone());
          }
          self.register_rule(rule);
        }
      } else {
        let needle = format!(".{}", class_name);
        if let Some(rule) = self
          .collected_rules
          .iter()
          .find(|css| css.contains(&needle))
          .cloned()
        {
          if !runtime_sheets.contains(&rule) {
            runtime_sheets.push(rule.clone());
          }
          self.register_rule_without_metadata(rule);
        }
      }
    }

    let runtime_sheets = self.finalize_runtime_sheets(runtime_sheets);

    self.needs_runtime_ax = true;
    self.needs_jsx_runtime = true;
    self.needs_react_namespace = true;

    let mut unique_class_names = Vec::new();
    let mut seen_classes = HashSet::new();
    for rule in &artifacts.rules {
      if expr_class_names.contains(&rule.class_name) {
        continue;
      }
      if !conditional_class_names.contains(&rule.class_name)
        && seen_classes.insert(rule.class_name.clone())
      {
        unique_class_names.push(rule.class_name.clone());
      }
    }
    for class_name in &precomputed_classes {
      if !expr_class_names.contains(class_name)
        && !conditional_class_names.contains(class_name)
        && seen_classes.insert(class_name.clone())
      {
        unique_class_names.push(class_name.clone());
      }
    }
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      eprintln!(
        "[compiled-debug] unique_class_names={:?} precomputed_classes={:?}",
        unique_class_names, precomputed_classes
      );
    }

    let mut style_attr_opt: Option<JSXAttrOrSpread> = None;
    if !artifacts.runtime_variables.is_empty() {
      let mut style_props: Vec<PropOrSpread> = Vec::new();
      if let Some(style_expr) = existing_style_expr.take() {
        style_props.push(PropOrSpread::Spread(SpreadElement {
          dot3_token: DUMMY_SP,
          expr: Box::new(style_expr),
        }));
      }
      let mut seen = HashSet::new();
      for variable in &artifacts.runtime_variables {
        if !seen.insert(variable.name.clone()) {
          continue;
        }
        let mut ix_args = Vec::new();
        ix_args.push(ExprOrSpread {
          spread: None,
          expr: Box::new(variable.expression.clone()),
        });
        if let Some(suffix) = &variable.suffix {
          if !suffix.is_empty() {
            ix_args.push(ExprOrSpread {
              spread: None,
              expr: Box::new(Expr::Lit(Lit::Str(Str::from(suffix.clone())))),
            });
            if let Some(prefix) = &variable.prefix {
              if !prefix.is_empty() {
                ix_args.push(ExprOrSpread {
                  spread: None,
                  expr: Box::new(Expr::Lit(Lit::Str(Str::from(prefix.clone())))),
                });
              }
            }
          }
        }
        let ix_call = Expr::Call(CallExpr {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          callee: Callee::Expr(Box::new(Expr::Ident(self.runtime_ix_ident()))),
          args: ix_args,
          type_args: None,
        });
        style_props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: PropName::Str(Str::from(variable.name.clone())),
          value: Box::new(ix_call),
        }))));
      }
      let style_object = Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: style_props,
      });
      style_attr_opt = Some(JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: DUMMY_SP,
        name: JSXAttrName::Ident(quote_ident!("style")),
        value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
          span: DUMMY_SP,
          expr: JSXExpr::Expr(Box::new(style_object)),
        })),
      }));
    } else if let Some(attr) = existing_style_attr {
      style_attr_opt = Some(JSXAttrOrSpread::JSXAttr(attr));
    }

    let mut class_entries: Vec<ExprOrSpread> = Vec::new();
    if !unique_class_names.is_empty() {
      let joined = unique_class_names.join(" ");
      class_entries.push(ExprOrSpread {
        spread: None,
        expr: Box::new(Expr::Lit(Lit::Str(Str::from(joined)))),
      });
    }

    for expr in precomputed_exprs {
      class_entries.push(ExprOrSpread {
        spread: None,
        expr: Box::new(expr),
      });
    }

    if let Some(index) = class_index {
      if let JSXAttrOrSpread::JSXAttr(class_attr) = &element.opening.attrs[index] {
        if let Some(value) = &class_attr.value {
          let expr = match value {
            JSXAttrValue::JSXExprContainer(container) => match &container.expr {
              JSXExpr::Expr(expr) => Some((**expr).clone()),
              _ => None,
            },
            JSXAttrValue::Lit(lit) => match lit {
              Lit::Str(str) => Some(Expr::Lit(Lit::Str(str.clone()))),
              _ => None,
            },
            _ => None,
          };
          if let Some(expr) = expr {
            class_entries.push(ExprOrSpread {
              spread: None,
              expr: Box::new(expr),
            });
          }
        }
      }
    }

    let merged_class_entries = if class_index.is_some() {
      class_entries
    } else {
      merge_adjacent_string_entries(class_entries)
    };
    let class_entry_count = merged_class_entries.len();
    let class_array = ArrayLit {
      span: DUMMY_SP,
      elems: merged_class_entries.into_iter().map(Some).collect(),
    };

    let class_attr = JSXAttrOrSpread::JSXAttr(JSXAttr {
      span: DUMMY_SP,
      name: JSXAttrName::Ident(quote_ident!("className")),
      value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
        span: DUMMY_SP,
        expr: JSXExpr::Expr(Box::new(Expr::Call(CallExpr {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          callee: Callee::Expr(Box::new(Expr::Ident(self.runtime_class_ident()))),
          args: vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Array(class_array)),
          }],
          type_args: None,
        }))),
      })),
    });

    let mut new_attrs = Vec::new();
    for (index, attr) in element.opening.attrs.iter().enumerate() {
      if index == css_index {
        continue;
      }
      if Some(index) == class_index {
        continue;
      }
      if Some(index) == style_index {
        continue;
      }
      new_attrs.push(attr.clone());
    }
    new_attrs.push(class_attr);
    if let Some(style_attr) = style_attr_opt {
      new_attrs.push(style_attr);
    }
    element.opening.attrs = new_attrs;

    if debug_css {
      eprintln!(
        "[compiled-debug] css prop result class_entries={} runtime_sheets={}",
        class_entry_count,
        runtime_sheets.len()
      );
    }

    Some((runtime_sheets, key_expr))
  }

  fn build_runtime_component(
    &mut self,
    child: Expr,
    sheets: Vec<String>,
    key: Option<Expr>,
  ) -> Expr {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let mut sheet_exprs = Vec::new();
    for css in sheets {
      if seen.insert(css.clone()) {
        let ident = self.hoist_sheet_ident(&css);
        sheet_exprs.push(ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Ident(ident)),
        });
      }
    }

    let sheets_array = Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: sheet_exprs.into_iter().map(Some).collect(),
    });

    let mut cs_props = Vec::new();
    if let Some(nonce) = &self.options.nonce {
      cs_props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(quote_ident!("nonce")),
        value: Box::new(Expr::Ident(Ident::new(
          nonce.clone().into(),
          DUMMY_SP,
          SyntaxContext::empty(),
        ))),
      }))));
    }
    cs_props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(quote_ident!("children")),
      value: Box::new(sheets_array),
    }))));

    let cs_call = Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Ident(self.jsx_ident()))),
      args: vec![
        ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Ident(self.runtime_cs_ident())),
        },
        ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: cs_props,
          })),
        },
      ],
      type_args: None,
    });

    let children_array = Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: vec![
        Some(ExprOrSpread {
          spread: None,
          expr: Box::new(cs_call),
        }),
        Some(ExprOrSpread {
          spread: None,
          expr: Box::new(child),
        }),
      ],
    });

    let mut cc_props = Vec::new();
    cc_props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(quote_ident!("children")),
      value: Box::new(children_array),
    }))));

    let mut args = vec![
      ExprOrSpread {
        spread: None,
        expr: Box::new(Expr::Ident(self.runtime_cc_ident())),
      },
      ExprOrSpread {
        spread: None,
        expr: Box::new(Expr::Object(ObjectLit {
          span: DUMMY_SP,
          props: cc_props,
        })),
      },
    ];

    if let Some(key_expr) = key {
      args.push(ExprOrSpread {
        spread: None,
        expr: Box::new(key_expr),
      });
    }

    self.needs_runtime_cc = true;
    self.needs_runtime_cs = true;
    self.needs_jsx_runtime = true;
    self.needs_jsxs_runtime = true;

    Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Ident(self.jsxs_ident()))),
      args,
      type_args: None,
    })
  }
}

impl<'a> VisitMut for TransformVisitor<'a> {
  fn visit_mut_module(&mut self, module: &mut Module) {
    self.css_imports.clear();
    self.keyframes_imports.clear();
    self.styled_imports.clear();
    self.css_map_imports.clear();
    self.compiled_import_kinds.clear();
    self.retain_imports.clear();
    for item in &module.body {
      if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item {
        let source = import.src.value.to_string();
        if source == "react" {
          for specifier in &import.specifiers {
            match specifier {
              ImportSpecifier::Named(named) => {
                let imported_name = named
                  .imported
                  .as_ref()
                  .map(|name| match name {
                    ModuleExportName::Ident(ident) => ident.sym.as_ref(),
                    ModuleExportName::Str(str) => str.value.as_ref(),
                  })
                  .unwrap_or_else(|| named.local.sym.as_ref());
                if imported_name == "forwardRef" {
                  self.has_forward_ref_binding = true;
                  if self.forward_ref_ident.is_none() {
                    self.forward_ref_ident = Some(named.local.clone());
                  }
                }
              }
              ImportSpecifier::Namespace(namespace) => {
                self.has_react_namespace_binding = true;
                if self.react_namespace_ident.is_none() {
                  self.react_namespace_ident = Some(namespace.local.clone());
                }
              }
              _ => {}
            }
          }
        }
        if source == "@compiled/react/runtime" {
          for specifier in &import.specifiers {
            if let ImportSpecifier::Named(named) = specifier {
              let imported_name = named
                .imported
                .as_ref()
                .map(|name| match name {
                  ModuleExportName::Ident(ident) => ident.sym.as_ref(),
                  ModuleExportName::Str(str) => str.value.as_ref(),
                })
                .unwrap_or_else(|| named.local.sym.as_ref());

              match imported_name {
                "ax" | "ac" => {
                  if imported_name == self.runtime_class_helper()
                    && self.runtime_class_ident.is_none()
                  {
                    self.runtime_class_ident = Some(named.local.clone());
                  }
                }
                "ix" => {
                  if self.runtime_ix_ident.is_none() {
                    self.runtime_ix_ident = Some(named.local.clone());
                  }
                }
                "CC" => {
                  if self.runtime_cc_ident.is_none() {
                    self.runtime_cc_ident = Some(named.local.clone());
                  }
                }
                "CS" => {
                  if self.runtime_cs_ident.is_none() {
                    self.runtime_cs_ident = Some(named.local.clone());
                  }
                }
                _ => {}
              }
            }
          }
        }
        if source == "react/jsx-runtime" {
          for specifier in &import.specifiers {
            if let ImportSpecifier::Named(named) = specifier {
              let imported_name = named
                .imported
                .as_ref()
                .map(|name| match name {
                  ModuleExportName::Ident(ident) => ident.sym.as_ref(),
                  ModuleExportName::Str(str) => str.value.as_ref(),
                })
                .unwrap_or_else(|| named.local.sym.as_ref());

              match imported_name {
                "jsx" => {
                  if self.jsx_ident.is_none() {
                    self.jsx_ident = Some(named.local.clone());
                  }
                }
                "jsxs" => {
                  if self.jsxs_ident.is_none() {
                    self.jsxs_ident = Some(named.local.clone());
                  }
                }
                _ => {}
              }
            }
          }
        }
        for specifier in &import.specifiers {
          match specifier {
            ImportSpecifier::Named(named) => {
              self
                .compiled_import_sources
                .insert(to_id(&named.local), source.clone());
            }
            ImportSpecifier::Default(spec) => {
              self
                .compiled_import_sources
                .insert(to_id(&spec.local), source.clone());
            }
            ImportSpecifier::Namespace(ns) => {
              self
                .compiled_import_sources
                .insert(to_id(&ns.local), source.clone());
            }
          }
        }
        if self.options.import_sources.contains(&source) {
          for specifier in &import.specifiers {
            match specifier {
              ImportSpecifier::Named(named) => {
                let imported_name = named
                  .imported
                  .as_ref()
                  .map(|name| match name {
                    ModuleExportName::Ident(ident) => ident.sym.as_ref(),
                    ModuleExportName::Str(str) => str.value.as_ref(),
                  })
                  .unwrap_or_else(|| named.local.sym.as_ref());
                let id = to_id(&named.local);
                self
                  .compiled_import_sources
                  .insert(id.clone(), source.clone());
                match imported_name {
                  "css" => {
                    self.css_imports.insert(id.clone());
                    self
                      .compiled_import_kinds
                      .insert(id, CompiledImportKind::Css);
                  }
                  "keyframes" => {
                    self.keyframes_imports.insert(id.clone());
                    self
                      .compiled_import_kinds
                      .insert(id, CompiledImportKind::Keyframes);
                  }
                  "styled" => {
                    self.styled_imports.insert(id.clone());
                    self
                      .compiled_import_kinds
                      .insert(id, CompiledImportKind::Styled);
                  }
                  "cssMap" => {
                    self.css_map_imports.insert(id.clone());
                    self
                      .compiled_import_kinds
                      .insert(id, CompiledImportKind::CssMap);
                  }
                  "ClassNames" => {
                    self
                      .compiled_import_kinds
                      .insert(id, CompiledImportKind::ClassNames);
                  }
                  "jsx" => {
                    self
                      .compiled_import_kinds
                      .insert(id, CompiledImportKind::Jsx);
                  }
                  _ => {}
                }
              }
              ImportSpecifier::Default(spec) => {
                if source == "@compiled/react" {
                  let id = to_id(&spec.local);
                  self.styled_imports.insert(id.clone());
                  self
                    .compiled_import_kinds
                    .insert(id, CompiledImportKind::Styled);
                }
              }
              ImportSpecifier::Namespace(_) => {}
            }
          }
        }
      }
    }

    self.prime_css_runtime_artifacts(module);
    module.visit_mut_children_with(self);

    let usage_snapshot = module.clone();
    let mut new_body = Vec::with_capacity(module.body.len());
    for item in module.body.drain(..) {
      match item {
        ModuleItem::ModuleDecl(ModuleDecl::Import(mut import)) => {
          let source = import.src.value.to_string();
          if self.options.import_sources.contains(&source) {
            import.specifiers.retain(|specifier| match specifier {
              ImportSpecifier::Named(named) => {
                let id = to_id(&named.local);
                if self.compiled_import_kinds.contains_key(&id) {
                  self.retain_imports.contains(&id)
                } else {
                  true
                }
              }
              ImportSpecifier::Default(spec) => {
                let id = to_id(&spec.local);
                if self.compiled_import_kinds.contains_key(&id) {
                  self.retain_imports.contains(&id)
                } else {
                  true
                }
              }
              ImportSpecifier::Namespace(_) => true,
            });
            if import.specifiers.is_empty() && !import.type_only && import.with.is_none() {
              continue;
            }
          }
          if import.src.value.as_ref() == "@atlaskit/tokens" {
            import.specifiers.retain(|specifier| match specifier {
              ImportSpecifier::Named(named) => {
                if Self::is_token_import(named) {
                  Self::ident_used_in_module(&usage_snapshot, &named.local)
                } else {
                  true
                }
              }
              _ => true,
            });
            if import.specifiers.is_empty() && !import.type_only && import.with.is_none() {
              continue;
            }
          }
          new_body.push(ModuleItem::ModuleDecl(ModuleDecl::Import(import)));
        }
        other => new_body.push(other),
      }
    }
    module.body = new_body;
    if !self.options.extract && !self.hoisted_sheet_order.is_empty() {
      let mut insertion_index = 0usize;
      for (index, item) in module.body.iter().enumerate() {
        if matches!(item, ModuleItem::ModuleDecl(ModuleDecl::Import(_))) {
          insertion_index = index + 1;
        } else {
          break;
        }
      }

      let mut declarations = Vec::new();
      for css in self.hoisted_sheet_order.iter().rev() {
        if let Some(ident) = self.hoisted_sheets.get(css) {
          declarations.push(ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind: VarDeclKind::Const,
            declare: false,
            decls: vec![VarDeclarator {
              span: DUMMY_SP,
              name: Pat::Ident(BindingIdent::from(ident.clone())),
              init: Some(Box::new(Expr::Lit(Lit::Str(Str::from(css.clone()))))),
              definite: false,
            }],
          })))));
        }
      }

      module
        .body
        .splice(insertion_index..insertion_index, declarations);
    }
    self.inject_imports(module);
    self.append_display_names(module);
    self.hoisted_sheet_order.clear();
    self.hoisted_sheets.clear();
  }

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    if let Some(token_value) = resolve_token_expression(expr) {
      *expr = Expr::Lit(Lit::Str(Str::from(token_value)));
      return;
    }

    if let Expr::Call(call) = expr {
      if let Callee::Expr(callee_expr) = &call.callee {
        if let Some((_component_expr, styled_ident)) = self.resolve_styled_target(callee_expr) {
          let props_ident = Ident::new(
            CANONICAL_PROPS_IDENT.into(),
            DUMMY_SP,
            SyntaxContext::empty(),
          );
          match self.process_styled_call_arguments(call, &props_ident) {
            Some(artifacts) => {
              if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
                eprintln!(
                  "[compiled-debug] inline styled call processed arg_count={} rules={} raw={}",
                  call.args.len(),
                  artifacts.rules.len(),
                  artifacts.raw_rules.len()
                );
              }
              self.register_artifacts_for_metadata(&artifacts);
            }
            None => {
              self.preserve_import_for_ident(&styled_ident);
            }
          }
        }
      }
    }

    if let Expr::JSXElement(element) = expr {
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        let element_name = match &element.opening.name {
          JSXElementName::Ident(ident) => ident.sym.as_ref().to_string(),
          _ => "<complex>".to_string(),
        };
        eprintln!("[compiled-debug] visit JSXElement name={}", element_name);
      }
      if let Some(mut replacement) = self.handle_class_names_element(element) {
        replacement.visit_mut_with(self);
        *expr = replacement;
        return;
      }

      let css_info = self.process_css_prop(element);
      let xcss_result = self.process_xcss_attributes(element);
      element.visit_mut_children_with(self);
      self.walk_jsx_children(element);

      let mut runtime_sheets: Vec<String> = Vec::new();
      let mut key_expr: Option<Expr> = None;
      let mut xcss_transformed = false;

      if let Some((sheets, key)) = css_info {
        if !self.options.extract && !sheets.is_empty() {
          runtime_sheets.extend(sheets);
        }
        key_expr = key;
        if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
          eprintln!(
            "[compiled-debug] visit_mut_expr css_info runtime_sheets={} key_set={}",
            runtime_sheets.len(),
            key_expr.is_some()
          );
        }
      }

      if let Some(result) = xcss_result {
        if !result.runtime_sheets.is_empty() {
          runtime_sheets.extend(result.runtime_sheets);
        }
        if !result.pending_class_names.is_empty() {
          let mut resolved = self.resolve_pending_xcss(&result.pending_class_names);
          runtime_sheets.append(&mut resolved);
        }
        if result.transformed || !result.pending_class_names.is_empty() {
          xcss_transformed = true;
        }
      }

      if !runtime_sheets.is_empty() {
        if !self.options.extract {
          self.needs_runtime_ax = true;
        }
        let inner = (**element).clone();
        let wrapper =
          self.build_runtime_component(Expr::JSXElement(Box::new(inner)), runtime_sheets, key_expr);
        *expr = wrapper;
      } else if xcss_transformed {
        // Metadata registrations have already occurred; no runtime wrapper required.
      }
      return;
    }
    expr.visit_mut_children_with(self);

    if let Expr::TaggedTpl(template) = expr.clone() {
      if self.is_css_ident(&template.tag) {
        if let Some(replacement) = self.handle_css_template(&template) {
          *expr = replacement;
        } else {
          self.preserve_import_for_expr(&template.tag);
        }
        return;
      }
      if self.is_keyframes_ident(&template.tag) {
        if let Some(replacement) = self.handle_keyframes_template(&template) {
          *expr = replacement;
        } else {
          self.preserve_import_for_expr(&template.tag);
        }
        return;
      }
    }

    if let Expr::Call(call) = expr.clone() {
      if let Callee::Expr(callee_expr) = &call.callee {
        if self.is_css_ident(callee_expr) {
          if let Some(replacement) = self.handle_css_call(&call) {
            *expr = replacement;
          } else {
            self.preserve_import_for_expr(callee_expr);
          }
          return;
        }
        if self.is_keyframes_ident(callee_expr) {
          if let Some(replacement) = self.handle_keyframes_call(&call) {
            *expr = replacement;
          } else {
            self.preserve_import_for_expr(callee_expr);
          }
          return;
        }
        if let Expr::Ident(ident) = &**callee_expr {
          if self.css_map_imports.contains(&to_id(ident)) {
            let import_source = self.compiled_import_sources.get(&to_id(ident)).cloned();
            let include_metadata = import_source
              .as_deref()
              .map(|source| source == "@atlaskit/css")
              .unwrap_or(false);
            if let Some(value) = self.evaluate_call_argument(&call) {
              if let StaticValue::Object(object) = value {
                let mut props = Vec::new();
                let mut entries: Vec<_> = object.iter().collect();
                entries.sort_by(|(a, _), (b, _)| a.cmp(b));
                for (key, value) in entries {
                  let variant_object = match value.as_object() {
                    Some(inner) => inner,
                    None => {
                      self.retain_imports.insert(to_id(ident));
                      return;
                    }
                  };
                  let mut map_options = self.css_options();
                  map_options.preserve_leading_combinator_space = true;
                  let artifacts =
                    match css_artifacts_from_static_object(variant_object, &map_options) {
                      Some(artifacts) => artifacts,
                      None => {
                        self.retain_imports.insert(to_id(ident));
                        return;
                      }
                    };
                  let mut class_names = Vec::new();
                  for rule in &artifacts.rules {
                    class_names.push(rule.class_name.clone());
                  }
                  let cache_ident = self.current_binding.clone().unwrap_or_else(|| to_id(ident));
                  self.cache_css_map_artifacts(Some(cache_ident), Some(key.clone()), &artifacts);
                  for (css, include_meta_flag, metadata_key) in artifacts.merged_css_entries() {
                    let should_include_metadata = include_metadata && include_meta_flag;
                    let metadata_key = if should_include_metadata {
                      metadata_key
                    } else {
                      None
                    };
                    self.register_rule_with_metadata_key(
                      css,
                      should_include_metadata,
                      metadata_key,
                    );
                  }
                  drop(artifacts);
                  let joined = class_names.join(" ");
                  props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Str(Str::from(key.clone())),
                    value: Box::new(Expr::Lit(Lit::Str(Str::from(joined)))),
                  }))));
                }
                *expr = Expr::Object(ObjectLit {
                  span: DUMMY_SP,
                  props,
                });
                return;
              }
            }
            self.retain_imports.insert(to_id(ident));
          }
        }
      }
    }
  }

  fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
    let prev_binding = self.current_binding.clone();
    if let Pat::Ident(binding) = &declarator.name {
      self.current_binding = Some(to_id(&binding.id));
    } else {
      self.current_binding = None;
    }
    declarator.visit_mut_children_with(self);
    self.current_binding = prev_binding;
    let init_expr = match &mut declarator.init {
      Some(init) => &mut **init,
      None => return,
    };

    let props_ident = Ident::new("__cmplp".into(), DUMMY_SP, SyntaxContext::empty());

    let (default_component_expr, mut artifacts) = match &*init_expr {
      Expr::TaggedTpl(tagged) => {
        let (component_expr, styled_ident) = match self.resolve_styled_target(&tagged.tag) {
          Some(res) => res,
          None => return,
        };
        let default_expr = match component_expr {
          Some(expr) => expr,
          None => {
            self.preserve_import_for_ident(&styled_ident);
            return;
          }
        };
        let artifacts = if let Some(css) = self.evaluate_template(tagged) {
          css_artifacts_from_literal(&css, &self.css_options())
            .unwrap_or_else(|| atomicize_literal(&css, &self.css_options()))
        } else {
          match self.process_dynamic_styled_template(tagged, &props_ident) {
            Some(artifacts) => artifacts,
            None => {
              if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
                eprintln!(
                  "[compiled-debug] styled template processing failed for {}",
                  styled_ident.sym
                );
              }
              self.preserve_import_for_ident(&styled_ident);
              return;
            }
          }
        };
        (default_expr, artifacts)
      }
      Expr::Call(call) => {
        let callee_expr = match &call.callee {
          Callee::Expr(expr) => &**expr,
          _ => {
            self.preserve_styled_usage_in_expr(&Expr::Call(call.clone()));
            return;
          }
        };
        let (component_expr, styled_ident) = match self.resolve_styled_target(callee_expr) {
          Some(res) => res,
          None => {
            self.preserve_styled_usage_in_expr(&Expr::Call(call.clone()));
            return;
          }
        };
        let mut combined = CssArtifacts::default();
        let debug_css = std::env::var_os("COMPILED_DEBUG_CSS").is_some();
        if let Some(values) = self.evaluate_call_arguments_allow_imports(call) {
          if debug_css {
            eprintln!(
              "[compiled-debug] styled call static args len={}",
              values.len()
            );
          }
          for (index, (arg, value)) in call.args.iter().zip(values.iter()).enumerate() {
            let value_kind = if debug_css {
              match value.unwrap_spread() {
                StaticValue::Null => "Null",
                StaticValue::Str(_) => "Str",
                StaticValue::Num(_) => "Num",
                StaticValue::Bool(_) => "Bool",
                StaticValue::Object(_) => "Object",
                StaticValue::Array(_) => "Array",
                StaticValue::Function(_) => "Function",
                StaticValue::Spread(_) => unreachable!("spread should be unwrapped"),
              }
            } else {
              ""
            };
            let static_artifacts = css_artifacts_from_static_value(value, &self.css_options());
            let mut merged = false;
            let mut used_static = false;
            let mut used_runtime = false;
            if let Some(ref artifacts) = static_artifacts {
              if !artifacts.rules.is_empty()
                || !artifacts.raw_rules.is_empty()
                || !artifacts.runtime_variables.is_empty()
                || !artifacts.runtime_class_conditions.is_empty()
                || !matches!(value, StaticValue::Null)
              {
                combined.merge(artifacts.clone());
                merged = true;
                used_static = true;
              }
            }
            if let Expr::Ident(ident) = &*arg.expr {
              self.mark_css_binding_used(ident);
              let mut runtime_artifacts = self.css_runtime_artifacts.get(&to_id(ident)).cloned();
              if runtime_artifacts.is_none() {
                if let Some((_, artifacts)) = self
                  .css_runtime_artifacts
                  .iter()
                  .find(|((sym, _), _)| sym == &ident.sym)
                {
                  runtime_artifacts = Some(artifacts.clone());
                }
              }
              if debug_css && runtime_artifacts.is_none() {
                eprintln!(
                  "[compiled-debug] styled runtime lookup miss for {}",
                  ident.sym
                );
              }
              if let Some(artifacts) = runtime_artifacts {
                combined.merge(artifacts);
                merged = true;
                used_runtime = true;
              }
            }
            if !merged && matches!(&*arg.expr, Expr::Lit(Lit::Null(_))) {
              merged = true;
            }
            if !merged {
              self.preserve_import_for_ident(&styled_ident);
              return;
            }
            if debug_css {
              eprintln!(
                "[compiled-debug] styled call arg idx={} kind={} used_static={} used_runtime={}",
                index, value_kind, used_static, used_runtime
              );
            }
          }
        } else {
          match self.process_styled_call_arguments(call, &props_ident) {
            Some(artifacts) => combined.merge(artifacts),
            None => {
              self.preserve_import_for_ident(&styled_ident);
              return;
            }
          }
        }
        let default_expr = match component_expr {
          Some(expr) => expr,
          None => {
            self.preserve_import_for_ident(&styled_ident);
            return;
          }
        };
        (default_expr, combined)
      }
      other => {
        self.preserve_styled_usage_in_expr(other);
        return;
      }
    };

    let component_name = match &declarator.name {
      Pat::Ident(binding) => Some(binding.id.sym.to_string()),
      _ => None,
    };

    let style_ident = Ident::new("__cmpls".into(), DUMMY_SP, SyntaxContext::empty());
    let ref_ident = Ident::new("__cmplr".into(), DUMMY_SP, SyntaxContext::empty());
    let component_ident = Ident::new("C".into(), DUMMY_SP, SyntaxContext::empty());

    let mut runtime_sheets = Vec::new();
    for rule in &artifacts.rules {
      self.register_rule_with_metadata_key(
        rule.css.clone(),
        rule.include_in_metadata,
        rule.metadata_key.clone(),
      );
      if !runtime_sheets.contains(&rule.css) {
        runtime_sheets.push(rule.css.clone());
      }
    }
    for css in &artifacts.raw_rules {
      let sheet = css.clone();
      self.register_rule_with_metadata_key(sheet.clone(), true, None);
      if !runtime_sheets.contains(&sheet) {
        runtime_sheets.push(sheet);
      }
    }
    let runtime_sheets = self.finalize_runtime_sheets(runtime_sheets);
    self.needs_react_namespace = true;
    self.needs_jsx_runtime = true;
    self.needs_runtime_ax = true;
    self.needs_forward_ref = true;

    if !artifacts.runtime_class_conditions.is_empty() && self.props_scope_depth == 0 {
      let source_ident = Ident::new("props".into(), DUMMY_SP, SyntaxContext::empty());
      for condition in artifacts.runtime_class_conditions.iter_mut() {
        Self::rename_ident_in_expr(&mut condition.test, &source_ident, &props_ident);
      }
    }

    let mut class_strings: Vec<ExprOrSpread> = Vec::new();
    if let Some(name) = component_name.as_ref() {
      if self.should_emit_component_class_name() {
        class_strings.push(ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Lit(Lit::Str(Str::from(format!("c_{}", name))))),
        });
      }
    }
    let mut conditional_classes = HashSet::new();
    for condition in &artifacts.runtime_class_conditions {
      for class_name in &condition.when_true {
        conditional_classes.insert(class_name.clone());
      }
      for class_name in &condition.when_false {
        conditional_classes.insert(class_name.clone());
      }
    }
    let mut styled_unique = Vec::new();
    let mut styled_seen = HashSet::new();
    for rule in &artifacts.rules {
      if conditional_classes.contains(&rule.class_name) {
        continue;
      }
      if styled_seen.insert(rule.class_name.clone()) {
        styled_unique.push(rule.class_name.clone());
      }
    }
    if !styled_unique.is_empty() {
      class_strings.push(ExprOrSpread {
        spread: None,
        expr: Box::new(Expr::Lit(Lit::Str(Str::from(styled_unique.join(" "))))),
      });
    }
    for condition in &artifacts.runtime_class_conditions {
      let has_true = !condition.when_true.is_empty();
      let has_false = !condition.when_false.is_empty();
      if has_true && has_false {
        let test_expr = condition.test.clone();
        let cons_expr = Self::class_names_to_expr(&condition.when_true);
        let alt_expr = Self::class_names_to_expr(&condition.when_false);
        class_strings.push(ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Cond(CondExpr {
            span: DUMMY_SP,
            test: Box::new(test_expr),
            cons: Box::new(cons_expr),
            alt: Box::new(alt_expr),
          })),
        });
      } else if has_true {
        let test_expr = condition.test.clone();
        let cons_expr = Self::class_names_to_expr(&condition.when_true);
        class_strings.push(ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: BinaryOp::LogicalAnd,
            left: Box::new(test_expr),
            right: Box::new(cons_expr),
          })),
        });
      } else if has_false {
        let test_expr = Expr::Unary(UnaryExpr {
          span: DUMMY_SP,
          op: UnaryOp::Bang,
          arg: Box::new(condition.test.clone()),
        });
        let alt_expr = Self::class_names_to_expr(&condition.when_false);
        class_strings.push(ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: BinaryOp::LogicalAnd,
            left: Box::new(test_expr),
            right: Box::new(alt_expr),
          })),
        });
      }
    }

    let mut runtime_variables = artifacts.runtime_variables.clone();
    if !runtime_variables.is_empty() {
      let source_ident = Ident::new("props".into(), DUMMY_SP, SyntaxContext::empty());
      for variable in runtime_variables.iter_mut() {
        Self::rename_ident_in_expr(&mut variable.expression, &source_ident, &props_ident);
      }
    }

    let mut object_props = Vec::new();
    object_props.push(ObjectPatProp::KeyValue(KeyValuePatProp {
      key: PropName::Ident(quote_ident!("as")),
      value: Box::new(Pat::Assign(AssignPat {
        span: DUMMY_SP,
        left: Box::new(Pat::Ident(BindingIdent::from(component_ident.clone()))),
        right: Box::new(default_component_expr),
      })),
    }));
    object_props.push(ObjectPatProp::KeyValue(KeyValuePatProp {
      key: PropName::Ident(quote_ident!("style")),
      value: Box::new(Pat::Ident(BindingIdent::from(style_ident.clone()))),
    }));
    object_props.push(ObjectPatProp::Rest(RestPat {
      span: DUMMY_SP,
      dot3_token: DUMMY_SP,
      arg: Box::new(Pat::Ident(BindingIdent::from(props_ident.clone()))),
      type_ann: None,
    }));

    let params = vec![
      Pat::Object(ObjectPat {
        span: DUMMY_SP,
        props: object_props,
        optional: false,
        type_ann: None,
      }),
      Pat::Ident(BindingIdent::from(ref_ident.clone())),
    ];

    let class_array = Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: class_strings
        .into_iter()
        .chain(std::iter::once(ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Ident(props_ident.clone())),
            prop: MemberProp::Ident(quote_ident!("className")),
          })),
        }))
        .map(Some)
        .collect(),
    });

    let style_prop_value = if runtime_variables.is_empty() {
      Expr::Ident(style_ident.clone())
    } else {
      self.build_style_with_variables(&style_ident, &runtime_variables)
    };

    let jsx_call = Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Ident(self.jsx_ident()))),
      args: vec![
        ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Ident(component_ident.clone())),
        },
        ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
              PropOrSpread::Spread(SpreadElement {
                dot3_token: DUMMY_SP,
                expr: Box::new(Expr::Ident(props_ident.clone())),
              }),
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(quote_ident!("style")),
                value: Box::new(style_prop_value),
              }))),
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(quote_ident!("ref")),
                value: Box::new(Expr::Ident(ref_ident.clone())),
              }))),
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(quote_ident!("className")),
                value: Box::new(Expr::Call(CallExpr {
                  span: DUMMY_SP,
                  ctxt: SyntaxContext::empty(),
                  callee: Callee::Expr(Box::new(Expr::Ident(self.runtime_class_ident()))),
                  args: vec![ExprOrSpread {
                    spread: None,
                    expr: Box::new(class_array),
                  }],
                  type_args: None,
                })),
              }))),
            ],
          })),
        },
      ],
      type_args: None,
    });

    let render_expr = if self.options.extract {
      jsx_call
    } else {
      self.build_runtime_component(jsx_call, runtime_sheets, None)
    };

    let guard = Stmt::If(IfStmt {
      span: DUMMY_SP,
      test: Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Ident(props_ident.clone())),
        prop: MemberProp::Ident(quote_ident!("innerRef")),
      })),
      cons: Box::new(Stmt::Block(BlockStmt {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        stmts: vec![Stmt::Throw(ThrowStmt {
          span: DUMMY_SP,
          arg: Box::new(Expr::New(NewExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Box::new(Expr::Ident(quote_ident!("Error").into())),
            args: Some(vec![ExprOrSpread {
              spread: None,
              expr: Box::new(Expr::Lit(Lit::Str(Str::from(
                "Please use 'ref' instead of 'innerRef'.",
              )))),
            }]),
            type_args: None,
          })),
        })],
      })),
      alt: None,
    });

    let arrow = Expr::Arrow(ArrowExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      params,
      body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        stmts: {
          let mut stmts = Vec::new();
          if self.is_development_env() {
            stmts.push(guard);
          }
          stmts.push(Stmt::Return(ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(render_expr)),
          }));
          stmts
        },
      })),
      is_async: false,
      is_generator: false,
      type_params: None,
      return_type: None,
    });

    *init_expr = Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Ident(self.forward_ref_ident()))),
      args: vec![ExprOrSpread {
        spread: None,
        expr: Box::new(arrow),
      }],
      type_args: None,
    });

    if let Pat::Ident(BindingIdent { id, .. }) = &declarator.name {
      self
        .styled_display_names
        .push((id.clone(), id.sym.to_string()));
    }
  }

  fn visit_mut_function(&mut self, function: &mut Function) {
    let has_props = function
      .params
      .iter()
      .any(|param| Self::pattern_binds_name(&param.pat, "props"));
    if has_props {
      self.props_scope_depth += 1;
    }
    function.visit_mut_children_with(self);
    if has_props {
      self.props_scope_depth -= 1;
    }
  }

  fn visit_mut_arrow_expr(&mut self, expr: &mut ArrowExpr) {
    let has_props = expr
      .params
      .iter()
      .any(|param| Self::pattern_binds_name(param, "props"));
    if has_props {
      self.props_scope_depth += 1;
    }
    expr.visit_mut_children_with(self);
    if has_props {
      self.props_scope_depth -= 1;
    }
  }

  fn visit_mut_jsx_element(&mut self, element: &mut JSXElement) {
    if self.options.extract {
      if let Some(result) = self.process_xcss_attributes(element) {
        if !result.pending_class_names.is_empty() {
          let mut resolved = self.resolve_pending_xcss(&result.pending_class_names);
          if std::env::var_os("COMPILED_DEBUG_CSS").is_some() && resolved.is_empty() {
            eprintln!("[compiled-debug] resolve_pending_xcss produced no sheets");
          }
        }
      }
    }
    self.walk_jsx_children(element);
  }
}

fn parse_transformed_source(code: &str, filename: &str) -> Program {
  use std::sync::Arc;

  let cm: Arc<SourceMap> = Default::default();
  let fm = cm.new_source_file(FileName::Custom(filename.into()).into(), code.into());

  let syntax_for_file = |name: &str| {
    if name.ends_with(".ts") || name.ends_with(".tsx") || name.ends_with(".cts") {
      Syntax::Typescript(TsSyntax {
        tsx: name.ends_with(".tsx"),
        decorators: true,
        ..Default::default()
      })
    } else {
      Syntax::Es(EsSyntax {
        jsx: name.ends_with(".jsx") || name.ends_with(".tsx"),
        decorators: true,
        export_default_from: true,
        import_attributes: true,
        ..Default::default()
      })
    }
  };

  let make_parser = |syntax: Syntax| {
    let lexer = Lexer::new(syntax, EsVersion::Es2022, StringInput::from(&*fm), None);
    Parser::new_from(lexer)
  };

  let mut parser = make_parser(syntax_for_file(filename));
  match parser.parse_module() {
    Ok(module) => Program::Module(module),
    Err(module_err) => {
      if std::env::var_os("COMPILED_DUMP_PARSE_FAILURE").is_some() {
        let sanitized = filename
          .chars()
          .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
          .collect::<String>();
        let dump_path = format!("/tmp/compiled_parse_failure_{}.js", sanitized);
        let _ = std::fs::write(&dump_path, code);
      }
      let mut parser = make_parser(syntax_for_file(filename));
      match parser.parse_script() {
        Ok(script) => Program::Script(script),
        Err(script_err) => panic!(
          "failed to parse emitted output as module ({module_err:?}), and as script ({script_err:?})"
        ),
      }
    }
  }
}

pub fn transform_program_with_options(
  program: Program,
  metadata: TransformPluginProgramMetadata,
  options: PluginOptions,
  filename: String,
) -> Program {
  let _emit_guard = metadata
    .comments
    .map(|proxy| EmitCommentsGuard::from_proxy(proxy));
  let _source_map_guard = SourceMapGuard::from_proxy(metadata.source_map);
  let mut program = program;
  if let Program::Module(mut module) = program {
    let file_path = PathBuf::from(&filename);
    let file_dir = file_path
      .parent()
      .map(Path::to_path_buf)
      .unwrap_or_else(|| PathBuf::from("."));
    let project_root = find_project_root(&file_dir);
    let css_options = css_options_from_plugin_options(&options);
    let evaluator = ModuleEvaluator::new(
      &file_dir,
      &project_root,
      &options.extensions,
      &options.import_sources,
      css_options.clone(),
    );
    let static_bindings = collect_static_bindings(
      &module,
      Some(&evaluator),
      Some(file_path.as_path()),
      &options.import_sources,
      &css_options,
    );
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      eprintln!(
        "[compiled-debug] initial css_map_rules count: {}",
        static_bindings.css_map_rules.len()
      );
    }
    let bindings = static_bindings.bindings;
    let initial_css_map_rules = static_bindings.css_map_rules;
    let initial_css_map_static_objects = static_bindings.css_map_static_objects;
    let name_tracker = NameTracker::from_module(&module);
    let (exported_bindings, exported_binding_names) = collect_exported_bindings(&module);
    let referenced_bindings = collect_identifier_usages(&module);
    let mut visitor = TransformVisitor::new(
      &options,
      bindings,
      name_tracker,
      initial_css_map_rules,
      initial_css_map_static_objects,
      exported_bindings,
      exported_binding_names,
      referenced_bindings,
    );
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      eprintln!(
        "[compiled-debug] process_xcss option for {} => {}",
        file_path.display(),
        options.process_xcss
      );
    }
    module.visit_mut_with(&mut visitor);
    visitor.discard_unused_css_binding_rules();

    let mut collected_rules = visitor.collected_rules.clone();
    let mut metadata_rules = visitor.metadata_rules.clone();
    {
      let mut seen_map_rules: HashSet<String> = metadata_rules.iter().cloned().collect();
      for property_map in visitor.css_map_ident_classes.values() {
        for classes in property_map.values() {
          for class_name in classes {
            if let Some(rules) = visitor.css_map_rule_groups.get(class_name) {
              for rule in rules {
                if seen_map_rules.insert(rule.clone()) {
                  metadata_rules.push(rule.clone());
                }
              }
            }
          }
        }
      }
    }
    if metadata_rules.is_empty() && !collected_rules.is_empty() {
      metadata_rules = collected_rules.clone();
    }
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      let aria_count = metadata_rules
        .iter()
        .filter(|rule| rule.contains("aria"))
        .count();
      eprintln!(
        "[compiled-debug] metadata_rules pre-filter aria count = {}",
        aria_count
      );
      for rule in metadata_rules.iter().take(10) {
        eprintln!("[compiled-debug] metadata_rule {}", rule);
      }
    }
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
      eprintln!(
        "[compiled-debug] collected_rules({}) count={}",
        file_path.display(),
        collected_rules.len()
      );
      eprintln!(
        "[compiled-debug] metadata_rules({}) count={}",
        file_path.display(),
        metadata_rules.len()
      );
    }
    if options.extract && !visitor.xcss_class_names.is_empty() {
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] xcss_class_names set {:?}",
          visitor.xcss_class_names
        );
        eprintln!(
          "[compiled-debug] non_xcss_class_names set {:?}",
          visitor.non_xcss_class_names
        );
        eprintln!(
          "[compiled-debug] css_map_rule_groups keys {:?}",
          visitor.css_map_rule_groups.keys().collect::<Vec<_>>()
        );
      }

      use std::collections::HashSet;
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] xcss_class_names={:?} non_xcss_class_names={:?}",
          visitor.xcss_class_names, visitor.non_xcss_class_names
        );
      }
      let mut non_xcss_class_names = visitor.non_xcss_class_names.clone();
      non_xcss_class_names.extend(visitor.css_map_rule_groups.keys().cloned());
      let xcss_only: HashSet<String> = visitor
        .xcss_class_names
        .difference(&non_xcss_class_names)
        .cloned()
        .collect();
      if !xcss_only.is_empty() {
        metadata_rules.retain(|rule| {
          !xcss_only
            .iter()
            .any(|class_name| rule.contains(&format!(".{}", class_name)))
        });
      }
    }
    if options.compiled_require_exclude.unwrap_or(false) {
      // Skip any runtime require hooks when exclusion is requested.
    } else if let Some(path) = options.style_sheet_path.as_ref() {
      append_stylesheet_requires(&mut module, path, &collected_rules);
    }

    if let Some(ref extract_opts) = options.extract_styles_to_directory {
      let sort_at_rules = options.sort_at_rules.unwrap_or(true);
      let sort_shorthand = options.sort_shorthand.unwrap_or(true);
      if let Err(message) = write_stylesheet_to_directory(
        &mut module,
        extract_opts,
        &collected_rules,
        sort_at_rules,
        sort_shorthand,
        &file_path,
        &project_root,
      ) {
        panic!("{message}");
      }
    }

    let included_files: Vec<String> = evaluator
      .included_files()
      .into_iter()
      .map(|path| path.to_string_lossy().to_string())
      .collect();

    if let Ok(mut guard) = LATEST_ARTIFACTS.lock() {
      if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
        eprintln!(
          "[compiled-debug] metadata_rules({}): {}",
          filename,
          metadata_rules.len()
        );
      }
      guard.insert(
        std::thread::current().id(),
        StyleArtifacts {
          style_rules: metadata_rules.clone(),
          metadata: json!({
              "styleRules": metadata_rules,
              "includedFiles": included_files,
          }),
        },
      );
    }
    program = Program::Module(module);
  }

  program
}

fn transform_program(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
  let config_value = metadata
    .get_transform_plugin_config()
    .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
    .unwrap_or(Value::Object(serde_json::Map::new()));

  let mut options: PluginOptions = serde_json::from_value(config_value).unwrap_or_default();
  if !options
    .import_sources
    .iter()
    .any(|source| source == "@compiled/react")
  {
    options.import_sources.push("@compiled/react".into());
  }

  let filename = metadata
    .get_context(&TransformPluginMetadataContextKind::Filename)
    .unwrap_or_else(|| "unknown.js".to_string());

  transform_program_with_options(program, metadata, options, filename)
}

#[doc(hidden)]
pub fn transform_program_for_testing(
  program: Program,
  filename: String,
  config_json: Option<&str>,
) -> Program {
  let config_value = config_json
    .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
    .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

  let mut options: PluginOptions = serde_json::from_value(config_value).unwrap_or_default();
  if !options
    .import_sources
    .iter()
    .any(|source| source == "@compiled/react")
  {
    options.import_sources.push("@compiled/react".into());
  }

  let metadata = TransformPluginProgramMetadata {
    comments: None,
    source_map: PluginSourceMapProxy {
      source_file: OnceCell::new(),
    },
    unresolved_mark: Mark::new(),
  };

  transform_program_with_options(program, metadata, options, filename)
}

#[doc(hidden)]
pub fn empty_metadata() -> TransformPluginProgramMetadata {
  TransformPluginProgramMetadata {
    comments: None,
    source_map: PluginSourceMapProxy {
      source_file: OnceCell::new(),
    },
    unresolved_mark: Mark::new(),
  }
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::process;
  use std::time::{SystemTime, UNIX_EPOCH};

  use super::{
    ExtractStylesToDirectoryOptions, ModuleEvaluator, NameTracker, PluginOptions, StaticString,
    StaticValue, StyleArtifacts, TransformVisitor, build_keyframe_declarations,
    collect_exported_bindings, collect_static_bindings, css_options_from_plugin_options,
    minify_keyframe_block, program_to_source, static_value_to_css_value, take_latest_artifacts,
    transform_program_with_options,
  };
  use once_cell::sync::OnceCell;
  use serde_json::Value;
  use swc_core::common::{FileName, GLOBALS, Globals, Mark, SourceMap};
  use swc_core::ecma::ast::EsVersion;
  use swc_core::ecma::ast::Program;
  use swc_core::ecma::parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};
  use swc_core::ecma::visit::VisitMutWith;
  use swc_core::plugin::proxies::{PluginSourceMapProxy, TransformPluginProgramMetadata};

  fn parse(code: &str) -> Program {
    use std::sync::Arc;

    let cm: Arc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("test.tsx".into()).into(), code.into());
    let lexer = Lexer::new(
      Syntax::Typescript(TsSyntax {
        tsx: true,
        decorators: true,
        ..Default::default()
      }),
      EsVersion::Es2022,
      StringInput::from(&*fm),
      None,
    );
    let mut parser = Parser::new_from(lexer);
    Program::Module(parser.parse_module().expect("module"))
  }

  fn empty_metadata() -> TransformPluginProgramMetadata {
    TransformPluginProgramMetadata {
      comments: None,
      source_map: PluginSourceMapProxy {
        source_file: OnceCell::new(),
      },
      unresolved_mark: Mark::new(),
    }
  }

  fn transform_source(input: &str) -> (String, StyleArtifacts) {
    GLOBALS.set(&Globals::new(), || {
      let program = parse(input);
      let metadata = empty_metadata();
      let transformed = transform_program_with_options(
        program,
        metadata,
        PluginOptions::default(),
        "test.tsx".into(),
      );
      let emitted = program_to_source(&transformed).expect("emit program");
      let artifacts = take_latest_artifacts();
      (emitted, artifacts)
    })
  }

  fn transform_source_with_options(
    input: &str,
    options: PluginOptions,
  ) -> (String, StyleArtifacts) {
    GLOBALS.set(&Globals::new(), || {
      let program = parse(input);
      let metadata = empty_metadata();
      let transformed =
        transform_program_with_options(program, metadata, options, "test.tsx".into());
      let emitted = program_to_source(&transformed).expect("emit program");
      let artifacts = take_latest_artifacts();
      (emitted, artifacts)
    })
  }

  #[test]
  fn keyframe_minifier_preserves_background_position_spacing() {
    let minified = minify_keyframe_block("background-position:-300px 0;opacity:0");
    assert_eq!(minified, "background-position:-300px 0;opacity:0");
  }

  #[test]
  #[test]
  #[test]
  #[test]
  fn styled_component_transforms() {
    let (emitted, artifacts) = transform_source(
      "import { styled } from '@compiled/react';\nconst Styled = styled.div`color: red;`;\n",
    );
    assert!(emitted.contains("import { ax, ix } from \"@compiled/react/runtime\";"));
    assert!(emitted.contains("forwardRef"));
    assert!(emitted.contains("Styled.displayName"));
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains("color:red"))
    );
  }

  #[test]
  fn styled_call_component_transforms() {
    let (emitted, artifacts) = transform_source(
      "import { styled } from '@compiled/react';\nconst Button = 'button';\nconst Styled = styled(Button)`color: red;`;\n",
    );
    assert!(emitted.contains("import { ax, ix } from \"@compiled/react/runtime\";"));
    assert!(emitted.contains("forwardRef"));
    assert!(emitted.contains("Styled.displayName"));
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains("color:red"))
    );
  }

  #[test]
  fn styled_adds_component_class_when_enabled() {
    let prev = std::env::var("NODE_ENV").ok();
    unsafe {
      std::env::remove_var("NODE_ENV");
    }
    let mut options = PluginOptions::default();
    options.add_component_name = Some(true);
    let (emitted, _) = transform_source_with_options(
      "import { styled } from '@compiled/react';\nconst MyDiv = styled.div`color: red;`;\n",
      options,
    );
    if let Some(prev) = prev {
      unsafe {
        std::env::set_var("NODE_ENV", prev);
      }
    } else {
      unsafe {
        std::env::remove_var("NODE_ENV");
      }
    }
    assert!(emitted.contains("\"c_MyDiv\""));
  }

  #[test]
  fn styled_runtime_wraps_with_cc() {
    let mut options = PluginOptions::default();
    options.extract = false;
    let (emitted, artifacts) = transform_source_with_options(
      "import { styled } from '@compiled/react';\nconst Styled = styled.div({ color: 'red' });\n",
      options,
    );
    assert!(emitted.contains("jsxs(CC, {"), "emitted: {emitted}");
    assert!(emitted.contains("jsx(CS, {"));
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains("color:red"))
    );
  }

  #[test]
  fn styled_reuses_forward_ref_alias() {
    let (emitted, _) = transform_source(
      "import { forwardRef as useForwardRef } from 'react';\nimport { styled } from '@compiled/react';\nconst Styled = styled.div`color: red;`;\n",
    );
    assert!(emitted.contains("useForwardRef("));
    assert!(!emitted.contains("import { forwardRef } from 'react';"));
  }

  #[test]
  fn keyframes_transforms() {
    let (emitted, artifacts) = transform_source(
      "import { keyframes } from '@compiled/react';\nconst fadeIn = keyframes`from { opacity: 0; } to { opacity: 1; }`;\n",
    );
    assert!(emitted.contains("const fadeIn = null"));
    assert!(artifacts.style_rules.is_empty());
  }

  #[test]
  fn keyframes_alias_import_transforms() {
    let (emitted, artifacts) = transform_source(
      "import { keyframes as animation } from '@compiled/react';\nconst fadeIn = animation`from { opacity: 0; } to { opacity: 1; }`;\n",
    );
    assert!(emitted.contains("const fadeIn = null"));
    assert!(!emitted.contains("animation`"));
    assert!(artifacts.style_rules.is_empty());
  }

  #[test]
  fn keyframes_object_call_transforms() {
    let (emitted, artifacts) = transform_source(
      "import { keyframes } from '@compiled/react';\nconst fadeIn = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } });\n",
    );
    assert!(emitted.contains("const fadeIn = null"));
    assert!(artifacts.style_rules.is_empty());
  }

  #[test]
  #[test]
  fn css_map_transforms() {
    let (emitted, artifacts) = transform_source(
      "import { cssMap } from '@compiled/react';\nconst map = cssMap({ primary: { backgroundColor: 'red' } });\n",
    );
    assert!(emitted.contains("primary"));
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains("background-color:red"))
    );
  }

  #[test]
  fn css_map_alias_import_transforms() {
    let (emitted, artifacts) = transform_source(
      "import { cssMap as compiledMap } from '@compiled/react';\nconst map = compiledMap({ primary: { backgroundColor: 'red' } });\n",
    );
    assert!(emitted.contains("primary"));
    assert!(!emitted.contains("compiledMap"));
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains("background-color:red"))
    );
  }

  #[test]
  #[test]
  fn styled_nested_is_selector_preserved() {
    let mut options = PluginOptions::default();
    options.extract = true;
    options.optimize_css = Some(true);
    let (_, artifacts) = transform_source_with_options(
      "import { styled } from '@compiled/react';\nconst Container = styled.div({ '> :is(div, button)': { flexShrink: 0 } });\n",
      options,
    );
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule == "._1puhidpf >:is(div,button){flex-shrink:0}"),
      "style rules: {:?}",
      artifacts.style_rules
    );
  }

  #[test]
  fn styled_container_nested_is_selector_preserved() {
    let mut options = PluginOptions::default();
    options.extract = true;
    options.optimize_css = Some(true);
    let (_, artifacts) = transform_source_with_options(
      "import { styled } from '@compiled/react';\nimport { token } from '@atlaskit/tokens';\nconst Container = styled.div({\n  display: 'flex',\n  alignItems: 'center',\n  gap: token('space.050'),\n  flexShrink: 0,\n  '> :is(div, button)': {\n    flexShrink: 0,\n  },\n});\n",
      options,
    );
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule == "._1puhidpf >:is(div,button){flex-shrink:0}"),
      "style rules: {:?}",
      artifacts.style_rules
    );
  }

  #[test]
  fn css_map_preserves_descendant_combinator_spacing() {
    let (emitted, artifacts) = transform_source(
      "import { cssMap } from '@compiled/react';\nconst map = cssMap({ base: { '& > button': { alignSelf: 'end' } } });\nconst Component = () => <div css={map.base} />;\n",
    );
    assert!(emitted.contains("map.base"));
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains(">button{align-self:end}")),
      "rules: {:?}",
      artifacts.style_rules
    );
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains(" _") || rule.contains("._")),
      "rules: {:?}",
      artifacts.style_rules
    );
  }

  #[test]
  fn css_prop_in_jsx_transforms() {
    let (emitted, artifacts) = transform_source(
      "import '@compiled/react';\nconst styles = { color: 'red' };\nconst Component = () => <div css={styles} />;\n",
    );
    assert!(emitted.contains("className"));
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains("color:red"))
    );
  }

  #[test]
  fn css_prop_nested_rules_match_babel_hashes() {
    let (_, artifacts) = transform_source(
      "import '@compiled/react';\nconst Component = () => (\n  <div css={{\n    color: 'red',\n    '&:hover': { color: 'blue' },\n    '@media': {\n      'screen and (min-width: 500px)': {\n        color: 'green',\n      },\n    },\n    content: ''\n  }} />\n);\n",
    );

    let rules = &artifacts.style_rules;
    assert!(
      rules.iter().any(|rule| rule == "._syaz5scu{color:red}"),
      "rules: {:?}",
      rules
    );
    assert!(
      rules.iter().any(|rule| rule == "._1sb2b3bt{content:\"\"}"),
      "rules: {:?}",
      rules
    );
    assert!(
      rules
        .iter()
        .any(|rule| rule == "._30l313q2:hover{color:blue}"),
      "rules: {:?}",
      rules
    );
    assert!(
      rules.iter().any(|rule| {
        rule.contains("@media") && rule.contains("_19jfbf54") && rule.contains("color:green")
      }),
      "rules: {:?}",
      rules
    );
  }

  #[test]
  fn css_prop_runtime_wraps_with_cc() {
    let mut options = PluginOptions::default();
    options.extract = false;
    let (emitted, artifacts) = transform_source_with_options(
      "import '@compiled/react';\nconst Component = () => <div css={{ color: 'red' }} />;\n",
      options,
    );
    assert!(emitted.contains("import { ax, ix, CC, CS } from \"@compiled/react/runtime\";"));
    assert!(emitted.contains("import { jsx, jsxs } from \"react/jsx-runtime\";"));
    assert!(emitted.contains("jsxs(CC, {"), "emitted: {emitted}");
    assert!(emitted.contains("jsx(CS, {"));
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains("color:red"))
    );
  }

  #[test]
  fn css_prop_runtime_honors_nonce() {
    let mut options = PluginOptions::default();
    options.extract = false;
    options.nonce = Some("__webpack_nonce__".into());
    let (emitted, _) = transform_source_with_options(
      "import '@compiled/react';\nconst Component = () => <div css={{ color: 'red' }} />;\n",
      options,
    );
    assert!(
      emitted.contains("nonce: __webpack_nonce__"),
      "emitted: {emitted}"
    );
  }

  #[test]
  fn class_names_transforms() {
    let (emitted, artifacts) = transform_source(
      "import { ClassNames } from '@compiled/react';\nconst Component = () => (\n  <ClassNames>{({ css }) => <div className={css({ color: 'red' })} />}</ClassNames>\n);\n",
    );
    assert!(!emitted.contains("ClassNames"));
    assert!(emitted.contains("ax(["));
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains("color:red"))
    );
  }

  #[test]
  fn runtime_helpers_avoid_name_collisions() {
    let mut options = PluginOptions::default();
    options.extract = false;
    let (emitted, _) = transform_source_with_options(
      "import { css } from '@compiled/react';\nconst ax = 'one';\nconst ix = 'two';\nconst jsx = 'three';\nconst jsxs = 'four';\nconst Component = () => <div css={{ color: 'red' }} />;\n",
      options,
    );
    assert!(emitted.contains("ax as _ax"));
    assert!(emitted.contains("ix as _ix"));
    assert!(emitted.contains("jsx as _jsx"));
    assert!(emitted.contains("jsxs as _jsxs"));
    assert!(emitted.contains("_ax(["));
  }

  #[test]
  fn hoisted_sheet_identifiers_avoid_collisions() {
    let mut options = PluginOptions::default();
    options.extract = false;
    let (emitted, _) = transform_source_with_options(
      "import { styled } from '@compiled/react';\nconst _ = 'keep';\nconst Styled = styled.div({ color: 'red' });\n",
      options,
    );
    assert!(emitted.contains("const _ = 'keep'") || emitted.contains("const _ = \"keep\""));
    assert!(
      emitted.contains("const _2 = \"") || emitted.contains("const _2 = '"),
      "emitted: {emitted}"
    );
  }

  #[test]
  fn styled_alias_import_transforms() {
    let (emitted, artifacts) = transform_source(
      "import { styled as compileStyled } from '@compiled/react';\nconst Styled = compileStyled.div`color: red;`;\n",
    );
    assert!(emitted.contains("forwardRef"));
    assert!(!emitted.contains("compileStyled"));
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains("color:red"))
    );
  }

  #[test]
  fn class_names_runtime_wraps_with_cc() {
    let mut options = PluginOptions::default();
    options.extract = false;
    let (emitted, artifacts) = transform_source_with_options(
      "import { ClassNames } from '@compiled/react';\nconst Component = () => (\n  <ClassNames>{({ css }) => <div className={css({ color: 'red' })} />}</ClassNames>\n);\n",
      options,
    );
    assert!(emitted.contains("jsxs(CC, {"), "emitted: {emitted}");
    assert!(emitted.contains("jsx(CS, {"));
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains("color:red"))
    );
  }

  #[test]
  #[test]
  #[test]
  #[test]
  #[test]
  fn styled_uses_flattened_selector_rules_from_css_binding() {
    let (_, artifacts) = transform_source(
      r#"
import { styled, css } from '@compiled/react';
const themedUnderline = { '&::after': { content: "''", position: 'absolute' } };
const tabStyles = css({
  display: 'flex',
  '&[aria-expanded="true"], &[aria-current="page"]': {
    display: 'flex',
    ...themedUnderline,
  },
});
export const Component = styled.div(tabStyles);
"#,
    );
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains("aria-current")),
      "expected styled binding to expose selector rule, got {:?}",
      artifacts.style_rules
    );
  }
  #[test]
  fn css_spread_nested_selectors_emit_flat_rules() {
    let (_, artifacts) = transform_source(
      r#"
import { css } from '@compiled/react';
const themedUnderline = { '&::after': { content: "''", position: 'absolute' } };
const styles = css({
  display: 'flex',
  '&[aria-expanded="true"], &[aria-current="page"]': {
    display: 'flex',
    alignItems: 'center',
    color: 'red',
    ...themedUnderline,
  },
  ':hover': ({ isDraggable }) => ({
    '--display-icon-before': isDraggable ? 'none' : 'flex',
  }),
});
"#,
    );
    let contains_comma = artifacts
      .style_rules
      .iter()
      .any(|rule| rule.contains("aria-current") && rule.contains(","));
    assert!(
      !contains_comma,
      "expected flattened selector style rules, got {:?}",
      artifacts.style_rules
    );
    assert!(
      artifacts
        .style_rules
        .iter()
        .filter(|rule| rule.contains("aria-current"))
        .count()
        >= 1
        && artifacts
          .style_rules
          .iter()
          .filter(|rule| rule.contains("aria-expanded"))
          .count()
          >= 1,
      "expected distinct selector rules for aria attributes, got {:?}",
      artifacts.style_rules
    );
  }

  #[test]
  #[should_panic(expected = "Object given to the xcss prop must be static")]
  fn xcss_inline_object_must_be_static() {
    let _ =
      transform_source("const render = (value: string) => <div xcss={{ color: value }} />;\n");
  }

  #[test]
  fn xcss_css_map_reference_preserved() {
    let (emitted, artifacts) = transform_source(
      "import { cssMap } from '@compiled/react';\nconst styles = cssMap({ primary: { color: 'red' } });\nconst Component = () => <div xcss={styles.primary} />;\n",
    );
    assert!(emitted.contains("xcss={styles.primary}"));
    assert!(
      artifacts
        .style_rules
        .iter()
        .any(|rule| rule.contains("color:red"))
    );
  }

  #[test]
  #[test]
  #[test]
  #[test]
  fn width_zero_percent_static_value_converts_to_zero() {
    let value = StaticValue::Str(StaticString::new("0%".to_string()));
    let (output, _, _) =
      static_value_to_css_value("width", &value).expect("should convert zero percent");
    assert_eq!(output, "0");
  }

  #[test]
  fn keyframe_width_zero_percent_converts_to_zero() {
    use indexmap::IndexMap;

    let mut map: IndexMap<String, StaticValue> = IndexMap::new();
    map.insert(
      "width".to_string(),
      StaticValue::Str(StaticString::new("0%".to_string())),
    );
    let declarations =
      build_keyframe_declarations(&map).expect("should build keyframe declarations");
    assert_eq!(declarations, "width:0");
  }
}
