use std::collections::BTreeMap;

mod serialize;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::Arc;

use derive_builder::Builder;
use serde::Deserialize;
use serde::Serialize;
use serde_repr::Deserialize_repr;
use serde_repr::Serialize_repr;

use crate::hash::IdentifierHasher;
use crate::types::{AssetId, ExportsCondition};

use super::FileType;
use super::bundle::MaybeBundleBehavior;
use super::environment::Environment;
use super::json::JSONObject;
use super::source::SourceLocation;
use super::symbol::Symbol;
use super::target::Target;

#[derive(Hash, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DependencyKind {
  /// Corresponds to ESM import statements
  /// ```skip
  /// import {x} from './dependency';
  /// ```
  Import,
  /// Corresponds to ESM re-export statements
  /// ```skip
  /// export {x} from './dependency';
  /// ```
  Export,
  /// Corresponds to dynamic import statements
  /// ```skip
  /// import('./dependency').then(({x}) => {/* ... */});
  /// ```
  DynamicImport,
  /// Corresponds to CJS require statements
  /// ```skip
  /// const {x} = require('./dependency');
  /// ```
  Require,
  /// Corresponds to conditional import statements
  /// ```skip
  /// const {x} = importCond('condition', './true-dep', './false-dep');
  /// ```
  ConditionalImport,
  /// Corresponds to Worker URL statements
  /// ```skip
  /// const worker = new Worker(
  ///     new URL('./dependency', import.meta.url),
  ///     {type: 'module'}
  /// );
  /// ```
  WebWorker,
  /// Corresponds to ServiceWorker URL statements
  /// ```skip
  /// navigator.serviceWorker.register(
  ///     new URL('./dependency', import.meta.url),
  ///     {type: 'module'}
  /// );
  /// ```
  ServiceWorker,
  /// CSS / WebAudio worklets
  /// ```skip
  /// CSS.paintWorklet.addModule(
  ///   new URL('./dependency', import.meta.url)
  /// );
  /// ```
  Worklet,
  /// URL statements
  /// ```skip
  /// let img = document.createElement('img');
  /// img.src = new URL('hero.jpg', import.meta.url);
  /// document.body.appendChild(img);
  /// ```
  Url,
  /// `fs.readFileSync` statements
  ///
  /// > Calls to fs.readFileSync are replaced with the file's contents if the filepath is statically
  /// > determinable and inside the project root.
  ///
  /// ```skip
  /// import fs from "fs";
  /// import path from "path";
  ///
  /// const data = fs.readFileSync(path.join(__dirname, "data.json"), "utf8");
  /// ```
  ///
  /// * https://parceljs.org/features/node-emulation/#inlining-fs.readfilesync
  File,
  /// `parcelRequire` call.
  Id,
}

impl fmt::Display for DependencyKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}", self)
  }
}

#[allow(clippy::too_many_arguments)]
pub fn create_dependency_id(
  source_asset_id: Option<&AssetId>,
  specifier: &str,
  environment_id: &str,
  target: Option<&Target>,
  pipeline: Option<&str>,
  specifier_type: &SpecifierType,
  bundle_behavior: &MaybeBundleBehavior,
  priority: &Priority,
  package_conditions: &ExportsCondition,
) -> String {
  let mut hasher = IdentifierHasher::new();

  source_asset_id.hash(&mut hasher);
  specifier.hash(&mut hasher);
  environment_id.hash(&mut hasher);
  target.hash(&mut hasher);
  pipeline.hash(&mut hasher);
  specifier_type.hash(&mut hasher);
  bundle_behavior.hash(&mut hasher);
  priority.hash(&mut hasher);
  package_conditions.hash(&mut hasher);

  let hash = hasher.finish();
  format!("{:016x}", hash)
}

/// A dependency denotes a connection between two assets
#[derive(Hash, PartialEq, Clone, Debug, Default, Builder)]
#[builder(build_fn(skip), pattern = "owned", setter(strip_option))]
// Dependencies should not be created directly, so we can ensure that an ID
// exists. DependencyBuilder::build() should be used instead.
#[non_exhaustive]
pub struct Dependency {
  /// Controls the behavior of the bundle the resolved asset is placed into
  ///
  /// This option is used in combination with priority to determine when the bundle is loaded.
  ///
  pub bundle_behavior: MaybeBundleBehavior,

  /// The environment of the dependency
  pub env: Arc<Environment>,

  #[builder(setter(skip))]
  pub id: String,

  /// The location within the source file where the dependency was found
  pub loc: Option<SourceLocation>,

  /// Plugin-specific metadata for the dependency
  pub meta: JSONObject,

  /// A list of custom conditions to use when resolving package.json "exports" and "imports"
  ///
  /// This will be combined with the conditions from the environment. However, it overrides the default "import" and "require" conditions inferred from the specifierType. To include those in addition to custom conditions, explicitly add them to this list.
  ///
  pub package_conditions: ExportsCondition,

  /// The pipeline defined in .parcelrc that the dependency should be processed with
  pub pipeline: Option<String>,

  /// Determines when the dependency should be loaded
  pub priority: Priority,

  /// The semver version range expected for the dependency
  pub range: Option<String>,

  /// The file path where the dependency should be resolved from
  ///
  /// By default, this is the path of the source file where the dependency was specified.
  ///
  pub resolve_from: Option<PathBuf>,

  /// The id of the asset with this dependency
  pub source_asset_id: Option<AssetId>,

  /// The file path of the asset with this dependency
  pub source_path: Option<PathBuf>,

  /// The import or export specifier that connects two assets together
  pub specifier: String,

  /// How the specifier should be interpreted
  pub specifier_type: SpecifierType,

  pub source_asset_type: Option<FileType>,

  /// These are the "Symbols" this dependency has which are used in import sites.
  ///
  /// We might want to split this information from this type.
  pub symbols: Option<Vec<Symbol>>,

  /// The target associated with an entry, if any
  pub target: Option<Box<Target>>,

  /// Whether the dependency is an entry
  pub is_entry: bool,

  /// Whether the dependency is optional
  ///
  /// If an optional dependency cannot be resolved, it will not fail the build.
  ///
  pub is_optional: bool,

  /// Indicates that the name should be stable over time, even when the content of the bundle changes
  ///
  /// When the dependency is a bundle entry (priority is "parallel" or "lazy"), this controls the
  /// naming of that bundle.
  ///
  /// This is useful for entries that a user would manually enter the URL for, as well as for
  /// things like service workers or RSS feeds, where the URL must remain consistent over time.
  ///
  pub needs_stable_name: bool,

  pub should_wrap: bool,

  /// Whether this dependency object corresponds to an ESM import/export statement or to a dynamic
  /// import expression.
  pub is_esm: bool,

  pub placeholder: Option<String>,

  /// Whether this dependency is a webworker
  pub is_webworker: bool,

  /// The kind of dependency (e.g., "Require", "Import", etc.)
  pub kind: Option<DependencyKind>,

  /// Symbol name for promise-based imports
  pub promise_symbol: Option<String>,

  /// Import attributes for this dependency
  pub import_attributes: BTreeMap<String, bool>,

  /// Media query for CSS imports
  pub media: Option<String>,

  /// Whether this is a CSS import
  pub is_css_import: bool,

  /// Chunk name from magic comment
  pub chunk_name_magic_comment: Option<String>,
}

impl DependencyBuilder {
  pub fn build(self) -> Dependency {
    // These properties are required to generate an ID
    let specifier = self.specifier.expect("specifier is required");
    let env = self.env.expect("env is required");
    let specifier_type = self.specifier_type.expect("specifier_type is required");
    let priority = self.priority.expect("priority is required");

    // These are part of ID generation, but can be optional
    let source_asset_id = self.source_asset_id.flatten();
    let target = self.target.flatten();
    let pipeline = self.pipeline.flatten();
    let bundle_behavior = self.bundle_behavior.unwrap_or_default();
    let package_conditions = self.package_conditions.unwrap_or_default();

    let id = create_dependency_id(
      source_asset_id.as_ref(),
      &specifier,
      &env.id(),
      target.as_deref(),
      pipeline.as_deref(),
      &specifier_type,
      &bundle_behavior,
      &priority,
      &package_conditions,
    );

    Dependency {
      id,

      // Mandatory ID fields
      specifier,
      env,
      specifier_type,
      priority,
      package_conditions,

      // Optional ID fields
      pipeline,
      source_asset_id,
      target,

      // These properties are either optional or safe to default
      bundle_behavior: self.bundle_behavior.unwrap_or_default(),
      loc: self.loc.flatten(),
      meta: self.meta.unwrap_or_default(),
      range: self.range.flatten(),
      resolve_from: self.resolve_from.flatten(),
      source_path: self.source_path.flatten(),
      source_asset_type: self.source_asset_type.flatten(),
      symbols: self.symbols.flatten(),
      is_entry: self.is_entry.unwrap_or_default(),
      is_optional: self.is_optional.unwrap_or_default(),
      needs_stable_name: self.needs_stable_name.unwrap_or_default(),
      should_wrap: self.should_wrap.unwrap_or_default(),
      is_esm: self.is_esm.unwrap_or_default(),
      placeholder: self.placeholder.flatten(),
      is_webworker: self.is_webworker.unwrap_or_default(),
      kind: self.kind.flatten(),
      promise_symbol: self.promise_symbol.flatten(),
      import_attributes: self.import_attributes.unwrap_or_default(),
      media: self.media.flatten(),
      is_css_import: self.is_css_import.unwrap_or_default(),
      chunk_name_magic_comment: self.chunk_name_magic_comment.flatten(),
    }
  }

  pub fn source_path_option(self, source_path: Option<PathBuf>) -> Self {
    if let Some(source_path) = source_path {
      self.source_path(source_path)
    } else {
      self
    }
  }

  pub fn placeholder_option(self, placeholder: Option<String>) -> Self {
    if let Some(placeholder) = placeholder {
      self.placeholder(placeholder)
    } else {
      self
    }
  }

  pub fn media_option(self, media: Option<String>) -> Self {
    if let Some(media) = media {
      self.media(media)
    } else {
      self
    }
  }
}

impl Dependency {
  pub fn id(&self) -> String {
    self.id.clone()
  }

  pub fn entry(entry: String, target: Target) -> Dependency {
    let is_library = target.env.is_library;

    let mut dep_builder = DependencyBuilder::default()
      .env(target.env.clone())
      .is_entry(true)
      .needs_stable_name(true)
      .specifier(entry)
      .specifier_type(SpecifierType::Url)
      .target(Box::new(target))
      .priority(Priority::default());

    if is_library {
      dep_builder = dep_builder.symbols(vec![Symbol {
        exported: "*".into(),
        is_esm_export: false,
        is_weak: true,
        loc: None,
        local: "*".into(),
        self_referenced: false,
        is_static_binding_safe: false,
      }])
    }

    dep_builder.build()
  }
}

#[derive(Clone, Debug, Deserialize, Hash, Serialize)]
pub struct ImportAttribute {
  pub key: String,
  pub value: bool,
}

/// Determines when a dependency should load
#[derive(Clone, Copy, Debug, Deserialize_repr, Eq, Hash, PartialEq, Serialize_repr)]
#[serde(rename_all = "lowercase")]
#[repr(u32)]
#[derive(Default)]
pub enum Priority {
  /// Resolves the dependency synchronously, placing the resolved asset in the same bundle as the parent or another bundle that is already on the page
  #[default]
  Sync = 0,
  /// Places the dependency in a separate bundle loaded in parallel with the current bundle
  Parallel = 1,
  /// The dependency should be placed in a separate bundle that is loaded later
  Lazy = 2,
  /// The dependency should be placed in a separate bundle that is loaded conditionally
  Conditional = 3,
}

/// The type of the import specifier
#[derive(Clone, Copy, Debug, Deserialize_repr, Eq, Hash, PartialEq, Serialize_repr)]
#[repr(u8)]
#[derive(Default)]
pub enum SpecifierType {
  /// An ES Module specifier
  ///
  /// This is parsed as an URL, but bare specifiers are treated as node_modules.
  ///
  #[default]
  Esm = 0,

  /// A CommonJS specifier
  ///
  /// This is not parsed as an URL.
  ///
  CommonJS = 1,

  /// A URL that works as in a browser
  ///
  /// Bare specifiers are treated as relative URLs.
  ///
  Url = 2,

  /// A custom specifier that must be handled by a custom resolver plugin
  Custom = 3,
}
