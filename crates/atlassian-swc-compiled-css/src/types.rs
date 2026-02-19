use std::cell::{Ref, RefCell, RefMut};
use std::collections::BTreeMap;
use std::env;
use std::fmt;
#[cfg(test)]
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use indexmap::{IndexMap, IndexSet};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use swc_core::common::comments::Comment;
#[cfg(not(test))]
use swc_core::common::errors::ColorConfig;
use swc_core::common::errors::{Emitter, EmitterWriter, Handler};
use swc_core::common::sync::Lrc;
use swc_core::common::{SourceMap, Span};
use swc_core::ecma::ast::{Expr, Ident, Program};

use oxc_resolver::Resolver;

use crate::DEFAULT_IMPORT_SOURCES;
use crate::utils_cache::{Cache, CacheOptions};
use crate::utils_types::PartialBindingWithMeta;

fn normalized_join(root: &Path, segment: &str) -> PathBuf {
  root.join(segment).components().collect()
}

#[cfg(test)]
fn emitter_for_source_map(source_map: Lrc<SourceMap>) -> Box<dyn Emitter> {
  Box::new(EmitterWriter::new(
    Box::new(io::sink()),
    Some(source_map),
    false,
    false,
  ))
}

#[cfg(not(test))]
fn emitter_for_source_map(source_map: Lrc<SourceMap>) -> Box<dyn Emitter> {
  Box::new(EmitterWriter::stderr(
    ColorConfig::Never,
    Some(source_map),
    false,
    false,
  ))
}
/// Mirror of the union used by the Babel plugin for controlling cache behaviour.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum CacheBehavior {
  /// Equivalent to setting the option to a boolean.
  Enabled(bool),
  /// Matches the `'file-pass'` literal supported by the Babel plugin.
  FilePass(String),
}

impl CacheBehavior {
  pub fn is_enabled(&self) -> bool {
    match self {
      CacheBehavior::Enabled(value) => *value,
      CacheBehavior::FilePass(_) => true,
    }
  }

  pub fn is_file_pass(&self) -> bool {
    matches!(self, CacheBehavior::FilePass(_))
  }
}

impl Default for CacheBehavior {
  fn default() -> Self {
    CacheBehavior::Enabled(false)
  }
}

/// Represents a resolver configuration that can be either an inline object or a module string.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ResolverOption {
  Module(String),
  Inline(Value),
}

impl ResolverOption {
  pub fn as_module(&self) -> Option<&str> {
    match self {
      ResolverOption::Module(value) => Some(value.as_str()),
      ResolverOption::Inline(_) => None,
    }
  }
}

/// Normalized resolver stored on the transform state.
#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedResolver {
  Inline(Value),
  Module(String),
}

impl ResolvedResolver {
  pub fn from_option(option: &ResolverOption, root: &Path) -> Self {
    match option {
      ResolverOption::Module(specifier) => {
        if specifier.starts_with('.') {
          let joined = normalized_join(root, specifier);
          ResolvedResolver::Module(joined.to_string_lossy().into_owned())
        } else {
          ResolvedResolver::Module(specifier.clone())
        }
      }
      ResolverOption::Inline(value) => ResolvedResolver::Inline(value.clone()),
    }
  }

  pub fn as_module(&self) -> Option<&str> {
    match self {
      ResolvedResolver::Module(value) => Some(value.as_str()),
      ResolvedResolver::Inline(_) => None,
    }
  }

  pub fn as_inline(&self) -> Option<&Value> {
    match self {
      ResolvedResolver::Inline(value) => Some(value),
      ResolvedResolver::Module(_) => None,
    }
  }
}

/// Rust representation of the Babel plugin options.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct PluginOptions {
  pub cache: Option<CacheBehavior>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_size: Option<usize>,
  pub import_react: Option<bool>,
  pub nonce: Option<String>,
  pub import_sources: Vec<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub on_included_files: Option<Value>,
  pub optimize_css: Option<bool>,
  pub resolver: Option<ResolverOption>,
  pub extensions: Option<Vec<String>>,
  pub parser_babel_plugins: Option<Vec<Value>>,
  pub add_component_name: Option<bool>,
  pub class_name_compression_map: Option<BTreeMap<String, String>>,
  pub process_xcss: Option<bool>,
  pub increase_specificity: Option<bool>,
  pub sort_at_rules: Option<bool>,
  pub class_hash_prefix: Option<String>,
  pub flatten_multiple_selectors: Option<bool>,
  pub extract: Option<bool>,
}

impl Default for PluginOptions {
  fn default() -> Self {
    Self {
      cache: None,
      max_size: None,
      import_react: None,
      nonce: None,
      import_sources: Default::default(),
      on_included_files: None,
      optimize_css: None,
      resolver: None,
      extensions: None,
      parser_babel_plugins: None,
      add_component_name: None,
      class_name_compression_map: None,
      process_xcss: None,
      increase_specificity: None,
      sort_at_rules: None,
      class_hash_prefix: None,
      flatten_multiple_selectors: None,
      extract: None,
    }
  }
}

impl From<&crate::config::CompiledCssInJsConfig> for PluginOptions {
  fn from(config: &crate::config::CompiledCssInJsConfig) -> Self {
    Self {
      cache: None,
      max_size: None,
      import_react: config.import_react,
      nonce: config.nonce.clone(),
      import_sources: config.import_sources.clone(),
      on_included_files: None,
      optimize_css: config.optimize_css,
      resolver: None,
      extensions: config.extensions.clone(),
      parser_babel_plugins: None,
      add_component_name: config.add_component_name,
      class_name_compression_map: None,
      process_xcss: config.process_xcss,
      increase_specificity: config.increase_specificity,
      sort_at_rules: config.sort_at_rules,
      class_hash_prefix: config.class_hash_prefix.clone(),
      flatten_multiple_selectors: config.flatten_multiple_selectors,
      extract: config.extract,
    }
  }
}

/// Metadata returned from the transform.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransformMetadata {
  pub included_files: Vec<String>,
  pub style_rules: Vec<String>,
  #[serde(skip_serializing_if = "Vec::is_empty", default)]
  pub diagnostics: Vec<crate::errors::TransformError>,
}

/// Result of a transform run containing the mutated program and collected metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct TransformOutput {
  pub program: Program,
  pub metadata: TransformMetadata,
}

impl TransformOutput {
  pub fn empty(program: Program) -> Self {
    Self {
      program,
      metadata: TransformMetadata::default(),
    }
  }
}

/// Represents the file-level information tracked during a transform.
#[derive(Clone)]
pub struct TransformFile {
  pub source_map: Lrc<SourceMap>,
  pub comments: Vec<Comment>,
  pub filename: Option<String>,
  pub cwd: PathBuf,
  pub root: PathBuf,
  pub loc: Option<TransformFileLocation>,
}

/// Location metadata exposed by Babel's `BabelFile.loc` helper.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransformFileLocation {
  pub filename: String,
}

/// Options used to construct `TransformFile` instances.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TransformFileOptions {
  pub filename: Option<String>,
  pub cwd: Option<PathBuf>,
  pub root: Option<PathBuf>,
  pub loc_filename: Option<String>,
}

impl TransformFile {
  pub fn new(source_map: Lrc<SourceMap>, comments: Vec<Comment>) -> Self {
    Self::transform_compiled_with_options(source_map, comments, TransformFileOptions::default())
  }

  pub fn transform_compiled_with_options(
    source_map: Lrc<SourceMap>,
    comments: Vec<Comment>,
    options: TransformFileOptions,
  ) -> Self {
    let TransformFileOptions {
      filename,
      cwd,
      root,
      loc_filename,
    } = options;

    let cwd_path = cwd
      .or_else(|| env::current_dir().ok())
      .unwrap_or_else(|| PathBuf::from("."));
    let root_path = root.unwrap_or_else(|| cwd_path.clone());
    let loc = loc_filename
      .or_else(|| filename.as_ref().cloned())
      .map(|filename| TransformFileLocation { filename });

    Self {
      source_map,
      comments,
      filename,
      cwd: cwd_path,
      root: root_path,
      loc,
    }
  }
}

impl Default for TransformFile {
  fn default() -> Self {
    Self::new(Lrc::new(SourceMap::default()), Vec::new())
  }
}

impl fmt::Debug for TransformFile {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("TransformFile")
      .field("filename", &self.filename)
      .field("cwd", &self.cwd)
      .field("root", &self.root)
      .field("comments", &self.comments)
      .finish()
  }
}

/// Shared pragma flags toggled during a transform.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PragmaFlags {
  pub jsx: bool,
  pub jsx_import_source: bool,
  pub classic_jsx_pragma_is_compiled: bool,
  pub classic_jsx_pragma_local_name: Option<String>,
}

/// Tracks discovered compiled imports for the current file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompiledImports {
  pub class_names: Vec<String>,
  pub css: Vec<String>,
  pub keyframes: Vec<String>,
  pub styled: Vec<String>,
  pub css_map: Vec<String>,
}

/// Tracks compiled runtime imports that have already been inserted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportedCompiledImports {
  pub css: Option<String>,
}

/// Represents a cleanup action scheduled for visitor exit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CleanupAction {
  Replace,
  Remove,
}

/// Placeholder for the Babel `NodePath` cleanup entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathCleanup {
  pub action: CleanupAction,
  pub span: Span,
}

/// Tracks spans that have already been transformed to avoid duplicate work.
#[derive(Clone, Debug, Default)]
pub struct TransformCache {
  spans: IndexSet<Span>,
}

impl TransformCache {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn has(&self, span: Span) -> bool {
    self.spans.contains(&span)
  }

  pub fn set(&mut self, span: Span) {
    self.spans.insert(span);
  }

  pub fn clear(&mut self) {
    self.spans.clear();
  }

  pub fn len(&self) -> usize {
    self.spans.len()
  }

  pub fn is_empty(&self) -> bool {
    self.spans.is_empty()
  }
}

/// Core transform state shared across visitors.
pub struct TransformState {
  pub compiled_imports: Option<CompiledImports>,
  pub uses_xcss: bool,
  pub uses_runtime_wrappers: bool,
  pub imported_compiled_imports: ImportedCompiledImports,
  pub import_sources: Vec<String>,
  pub pragma: PragmaFlags,
  pub paths_to_cleanup: Vec<PathCleanup>,
  pub opts: PluginOptions,
  pub file: TransformFile,
  pub included_files: Vec<String>,
  pub module_scope: SharedScope,
  pub module_cache: Rc<RefCell<IndexMap<String, CachedModule>>>,
  pub sheets: IndexMap<String, Ident>,
  pub style_rules: IndexSet<String>,
  pub sheet_identifier_counter: usize,
  pub cache: SharedCache,
  pub css_map: IndexMap<String, Vec<String>>,
  pub ignore_member_expressions: IndexSet<String>,
  pub resolver: Option<ResolvedResolver>,
  pub module_resolver: Option<Resolver>,
  pub transform_cache: TransformCache,
  pub filename: Option<String>,
  pub cwd: PathBuf,
  pub root: PathBuf,
  pub handler: Lrc<Handler>,
  pub diagnostics: Vec<crate::errors::TransformError>,
}

impl fmt::Debug for TransformState {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("TransformState")
      .field("compiled_imports", &self.compiled_imports)
      .field("uses_xcss", &self.uses_xcss)
      .field("imported_compiled_imports", &self.imported_compiled_imports)
      .field("import_sources", &self.import_sources)
      .field("pragma", &self.pragma)
      .field("paths_to_cleanup", &self.paths_to_cleanup)
      .field("opts", &self.opts)
      .field("included_files", &self.included_files)
      .field("sheet_identifier_counter", &self.sheet_identifier_counter)
      .finish()
  }
}

static GLOBAL_CACHE: OnceCell<SharedCache> = OnceCell::new();

/// Shared cache handle mirroring the Babel plugin behaviour.
pub type SharedCache = Arc<Mutex<Cache<Value>>>;

impl TransformState {
  pub fn new(file: TransformFile, opts: PluginOptions) -> Self {
    let filename = file.filename.clone();
    let cwd = file.cwd.clone();
    let root = file.root.clone();
    let import_sources = Self::resolve_import_sources(&file, &opts);
    let handler = Self::handler_for_source_map(&file.source_map);
    let resolver = opts
      .resolver
      .as_ref()
      .map(|resolver_option| ResolvedResolver::from_option(resolver_option, &root));

    let cache_behavior = opts.cache.clone();
    let cache_enabled = cache_behavior
      .as_ref()
      .map(CacheBehavior::is_enabled)
      .unwrap_or(false);
    let use_global_cache = matches!(cache_behavior, Some(CacheBehavior::Enabled(true)));
    let max_size = opts.max_size;

    let cache_handle = if use_global_cache {
      GLOBAL_CACHE
        .get_or_init(|| Arc::new(Mutex::new(Cache::new())))
        .clone()
    } else {
      Arc::new(Mutex::new(Cache::new()))
    };

    {
      let mut cache = cache_handle
        .lock()
        .expect("global cache lock should not be poisoned");
      cache.initialize(CacheOptions {
        cache: Some(cache_enabled),
        max_size,
      });
    }

    Self {
      compiled_imports: None,
      uses_xcss: false,
      uses_runtime_wrappers: false,
      imported_compiled_imports: ImportedCompiledImports::default(),
      import_sources,
      pragma: PragmaFlags::default(),
      paths_to_cleanup: Vec::new(),
      opts,
      file,
      included_files: Vec::new(),
      module_scope: new_scope(),
      module_cache: Rc::new(RefCell::new(IndexMap::new())),
      sheets: IndexMap::new(),
      style_rules: IndexSet::new(),
      sheet_identifier_counter: 0,
      cache: cache_handle,
      css_map: IndexMap::new(),
      ignore_member_expressions: IndexSet::new(),
      resolver,
      module_resolver: None,
      transform_cache: TransformCache::default(),
      filename,
      cwd,
      root,
      handler,
      diagnostics: Vec::new(),
    }
  }

  pub fn file(&self) -> &TransformFile {
    &self.file
  }

  fn handler_for_source_map(source_map: &Lrc<SourceMap>) -> Lrc<Handler> {
    let handler = Handler::with_emitter(true, false, emitter_for_source_map(source_map.clone()));
    Lrc::new(handler)
  }

  pub fn replace_file(&mut self, file: TransformFile) {
    self.filename = file.filename.clone();
    self.cwd = file.cwd.clone();
    self.root = file.root.clone();
    self.file = file;
    self.handler = Self::handler_for_source_map(&self.file.source_map);
    self.import_sources = Self::resolve_import_sources(&self.file, &self.opts);
    self.resolver = self
      .opts
      .resolver
      .as_ref()
      .map(|resolver_option| ResolvedResolver::from_option(resolver_option, &self.root));
    self.module_resolver = None;
  }

  fn resolve_import_sources(file: &TransformFile, opts: &PluginOptions) -> Vec<String> {
    let resolved_sources = opts.import_sources.iter().map(|origin| {
      if origin.starts_with('.') {
        let joined = normalized_join(&file.root, origin.as_str());
        joined.to_string_lossy().into_owned()
      } else {
        origin.clone()
      }
    });

    DEFAULT_IMPORT_SOURCES
      .iter()
      .map(|s| s.to_string())
      .chain(resolved_sources)
      .collect()
  }

  pub fn enqueue_cleanup(&mut self, action: CleanupAction, span: Span) {
    if self
      .paths_to_cleanup
      .iter()
      .any(|entry| entry.span == span && entry.action == action)
    {
      return;
    }

    self.paths_to_cleanup.push(PathCleanup { action, span });
  }
}

/// Shared pointer to the transform state, allowing metadata clones to mutate it.
pub type SharedTransformState = Rc<RefCell<TransformState>>;

/// Contextual metadata threaded through helper utilities during traversal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetadataContext {
  Root,
  Keyframes { keyframe: String },
  Fragment,
}

/// Shared scope map that mirrors Babel's `NodePath` binding storage.
pub type SharedScope = Rc<RefCell<IndexMap<String, PartialBindingWithMeta>>>;

fn new_scope() -> SharedScope {
  Rc::new(RefCell::new(IndexMap::new()))
}

/// Cached module data used when resolving imported bindings.
#[derive(Clone, Debug)]
pub struct CachedModule {
  pub program: Program,
  pub state: SharedTransformState,
}

/// Metadata wrapper that mirrors the Babel helpers.
#[derive(Clone, Debug)]
pub struct Metadata {
  pub state: SharedTransformState,
  pub context: MetadataContext,
  pub parent_span: Option<Span>,
  pub own_span: Option<Span>,
  pub parent_scope: SharedScope,
  pub own_scope: Option<SharedScope>,
  pub parent_expr: Option<Box<Expr>>,
}

impl Metadata {
  pub fn new(state: SharedTransformState) -> Self {
    let parent_scope = {
      let state_ref = state.borrow();
      state_ref.module_scope.clone()
    };
    Self {
      state,
      context: MetadataContext::Root,
      parent_span: None,
      own_span: None,
      parent_scope,
      own_scope: None,
      parent_expr: None,
    }
  }

  pub fn with_context(&self, context: MetadataContext) -> Self {
    Self {
      context,
      ..self.clone()
    }
  }

  pub fn with_parent_span(&self, parent_span: Option<Span>) -> Self {
    Self {
      parent_span,
      ..self.clone()
    }
  }

  pub fn with_own_span(&self, own_span: Option<Span>) -> Self {
    Self {
      own_span,
      ..self.clone()
    }
  }

  pub fn state(&self) -> Ref<'_, TransformState> {
    self.state.borrow()
  }

  pub fn state_mut(&self) -> RefMut<'_, TransformState> {
    self.state.borrow_mut()
  }

  pub fn with_parent_scope(&self, parent_scope: SharedScope) -> Self {
    Self {
      parent_scope,
      ..self.clone()
    }
  }

  pub fn with_own_scope(&self, own_scope: Option<SharedScope>) -> Self {
    Self {
      own_scope,
      ..self.clone()
    }
  }

  pub fn with_parent_expr(&self, parent_expr: Option<&Expr>) -> Self {
    Self {
      parent_expr: parent_expr.map(|expr| Box::new(expr.clone())),
      ..self.clone()
    }
  }

  pub fn parent_expr(&self) -> Option<&Expr> {
    self.parent_expr.as_deref()
  }

  pub fn parent_scope(&self) -> SharedScope {
    self.parent_scope.clone()
  }

  pub fn own_scope(&self) -> Option<SharedScope> {
    self.own_scope.clone()
  }

  pub fn insert_parent_binding(&self, name: impl Into<String>, binding: PartialBindingWithMeta) {
    self.parent_scope.borrow_mut().insert(name.into(), binding);
  }

  pub fn add_diagnostic(&self, diagnostic: crate::errors::TransformError) {
    self.state.borrow_mut().diagnostics.push(diagnostic);
  }

  pub fn insert_own_binding(&self, name: impl Into<String>, binding: PartialBindingWithMeta) {
    if let Some(scope) = &self.own_scope {
      scope.borrow_mut().insert(name.into(), binding);
    }
  }

  pub fn allocate_own_scope(&self) -> SharedScope {
    new_scope()
  }
}

/// Tag information used when building styled components.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TagType {
  InBuiltComponent,
  UserDefinedComponent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
  pub name: String,
  pub tag_type: TagType,
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::env;
  use std::path::{Path, PathBuf};
  use std::sync::Arc;
  use std::sync::atomic::{AtomicUsize, Ordering};

  use swc_core::common::sync::Lrc;
  use swc_core::common::{BytePos, SourceMap};

  static GLOBAL_CACHE_KEY_COUNTER: AtomicUsize = AtomicUsize::new(0);
  static FILE_PASS_CACHE_KEY_COUNTER: AtomicUsize = AtomicUsize::new(0);

  #[test]
  fn merges_default_import_sources_with_relative_entries() {
    let cm: Lrc<SourceMap> = Default::default();
    let cwd = env::current_dir().expect("current dir");
    let root = cwd.join("compiled-tests");

    let file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      TransformFileOptions {
        cwd: Some(cwd.clone()),
        root: Some(root.clone()),
        filename: Some(root.join("file.tsx").to_string_lossy().into_owned()),
        ..TransformFileOptions::default()
      },
    );

    let options = PluginOptions {
      import_sources: vec!["./relative/module".into(), "@scope/package".into()],
      ..PluginOptions::default()
    };

    let state = TransformState::new(file, options);

    let mut expected = vec!["@compiled/react".to_string(), "@atlaskit/css".to_string()];
    expected.push(root.join("relative/module").to_string_lossy().into_owned());
    expected.push("@scope/package".into());

    assert_eq!(state.import_sources, expected);
  }

  #[test]
  fn normalizes_relative_resolver_modules_against_root() {
    let cm: Lrc<SourceMap> = Default::default();
    let cwd = env::current_dir().expect("current dir");
    let root = cwd.join("resolver-root");

    let file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      TransformFileOptions {
        cwd: Some(cwd.clone()),
        root: Some(root.clone()),
        ..TransformFileOptions::default()
      },
    );

    let options = PluginOptions {
      resolver: Some(ResolverOption::Module("./custom/resolver.js".into())),
      ..PluginOptions::default()
    };

    let state = TransformState::new(file, options);
    let resolver = state.resolver.expect("resolver should be initialized");
    let expected = root
      .join("custom/resolver.js")
      .to_string_lossy()
      .into_owned();

    assert_eq!(resolver.as_module(), Some(expected.as_str()));
  }

  #[test]
  fn replace_file_refreshes_metadata_and_resolver() {
    let cm: Lrc<SourceMap> = Default::default();
    let cwd = env::current_dir().expect("current dir");
    let first_cwd = cwd.join("first-cwd");
    let second_cwd = cwd.join("second-cwd");
    let first_root = cwd.join("first-root");
    let second_root = cwd.join("second-root");

    let first_file = TransformFile::transform_compiled_with_options(
      cm.clone(),
      Vec::new(),
      TransformFileOptions {
        cwd: Some(first_cwd.clone()),
        root: Some(first_root.clone()),
        filename: Some(first_root.join("file.tsx").to_string_lossy().into_owned()),
        ..TransformFileOptions::default()
      },
    );

    let options = PluginOptions {
      import_sources: vec!["./relative/module".into()],
      resolver: Some(ResolverOption::Module("./custom/resolver.js".into())),
      ..PluginOptions::default()
    };

    let mut state = TransformState::new(first_file, options.clone());

    assert_eq!(state.cwd, first_cwd);
    assert_eq!(state.root, first_root);
    let expected_first_source = first_root.join("relative/module");
    assert!(
      state
        .import_sources
        .iter()
        .any(|source| Path::new(source) == expected_first_source)
    );
    let initial_resolver = state
      .resolver
      .as_ref()
      .and_then(ResolvedResolver::as_module)
      .map(PathBuf::from)
      .expect("expected resolver to be initialized");
    assert!(initial_resolver.starts_with(&first_root));

    let second_file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      TransformFileOptions {
        cwd: Some(second_cwd.clone()),
        root: Some(second_root.clone()),
        filename: Some(second_root.join("file.tsx").to_string_lossy().into_owned()),
        ..TransformFileOptions::default()
      },
    );

    state.replace_file(second_file);

    let expected_filename = second_root.join("file.tsx");
    assert_eq!(
      state
        .filename
        .as_ref()
        .map(PathBuf::from)
        .expect("filename should be set"),
      expected_filename
    );
    assert_eq!(state.cwd, second_cwd);
    assert_eq!(state.root, second_root);

    let expected_second_source = second_root.join("relative/module");
    assert!(
      state
        .import_sources
        .iter()
        .any(|source| Path::new(source) == expected_second_source)
    );

    let refreshed_resolver = state
      .resolver
      .as_ref()
      .and_then(ResolvedResolver::as_module)
      .map(PathBuf::from)
      .expect("resolver should be refreshed");
    assert!(refreshed_resolver.starts_with(&second_root));
    assert!(refreshed_resolver.ends_with("custom/resolver.js"));
    assert!(state.module_resolver.is_none());
  }

  #[test]
  fn initializes_cache_based_on_behavior_flag() {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm, Vec::new());

    let options = PluginOptions {
      cache: Some(CacheBehavior::Enabled(true)),
      ..PluginOptions::default()
    };

    let state = TransformState::new(file, options);

    let inserted = {
      let mut cache = state.cache.lock().expect("cache lock");
      cache.load(Some("namespace"), "cache-key", || Value::from("first"))
    };
    assert_eq!(inserted, Value::from("first"));

    let cached = {
      let mut cache = state.cache.lock().expect("cache lock");
      cache.load(Some("namespace"), "cache-key", || Value::from("second"))
    };
    assert_eq!(cached, Value::from("first"));
  }

  #[test]
  fn reuses_global_cache_when_enabled() {
    let cm: Lrc<SourceMap> = Default::default();
    let first_file = TransformFile::new(cm.clone(), Vec::new());
    let second_file = TransformFile::new(cm, Vec::new());

    let options = PluginOptions {
      cache: Some(CacheBehavior::Enabled(true)),
      ..PluginOptions::default()
    };

    let counter = Arc::new(AtomicUsize::new(0));
    let cache_key = format!(
      "global-cache-key-{}",
      GLOBAL_CACHE_KEY_COUNTER.fetch_add(1, Ordering::SeqCst)
    );

    let first_state = TransformState::new(first_file, options.clone());
    {
      let counter = counter.clone();
      let mut cache = first_state.cache.lock().expect("cache lock");
      let value = cache.load(Some("namespace"), &cache_key, || {
        counter.fetch_add(1, Ordering::SeqCst);
        Value::from("first")
      });
      assert_eq!(value, Value::from("first"));
    }

    let second_state = TransformState::new(second_file, options);
    {
      let counter = counter.clone();
      let mut cache = second_state.cache.lock().expect("cache lock");
      let value = cache.load(Some("namespace"), &cache_key, || {
        counter.fetch_add(1, Ordering::SeqCst);
        Value::from("second")
      });
      assert_eq!(value, Value::from("first"));
    }

    assert_eq!(counter.load(Ordering::SeqCst), 1);
  }

  #[test]
  fn file_pass_cache_isolated_per_state() {
    let cm: Lrc<SourceMap> = Default::default();
    let first_file = TransformFile::new(cm.clone(), Vec::new());
    let second_file = TransformFile::new(cm, Vec::new());

    let options = PluginOptions {
      cache: Some(CacheBehavior::FilePass("file-pass".into())),
      ..PluginOptions::default()
    };

    let counter = Arc::new(AtomicUsize::new(0));
    let cache_key = format!(
      "file-pass-cache-key-{}",
      FILE_PASS_CACHE_KEY_COUNTER.fetch_add(1, Ordering::SeqCst)
    );

    let first_state = TransformState::new(first_file, options.clone());
    {
      let counter = counter.clone();
      let mut cache = first_state.cache.lock().expect("cache lock");
      let value = cache.load(Some("namespace"), &cache_key, || {
        counter.fetch_add(1, Ordering::SeqCst);
        Value::from("first")
      });
      assert_eq!(value, Value::from("first"));
    }

    let second_state = TransformState::new(second_file, options);
    {
      let counter = counter.clone();
      let mut cache = second_state.cache.lock().expect("cache lock");
      let value = cache.load(Some("namespace"), &cache_key, || {
        counter.fetch_add(1, Ordering::SeqCst);
        Value::from("second")
      });
      assert_eq!(value, Value::from("second"));
    }

    assert_eq!(counter.load(Ordering::SeqCst), 2);
  }

  #[test]
  fn transform_cache_tracks_spans() {
    let mut cache = TransformCache::default();
    let span = Span::new(BytePos(1), BytePos(5));

    assert!(!cache.has(span));
    cache.set(span);
    assert!(cache.has(span));
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
  }
}
