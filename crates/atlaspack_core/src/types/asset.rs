use std::collections::HashSet;
use std::ffi::OsStr;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::str;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use serde_json::json;

use crate::project_path::to_project_path;

use super::bundle::MaybeBundleBehavior;
use super::environment::Environment;
use super::file_type::FileType;
use super::json::JSONObject;
use super::symbol::Symbol;
use super::Dependency;
use super::{BundleBehavior, SourceMap};

pub type AssetId = String;

/// The source code for an asset.
///
/// TODO: This should be called contents now that it's bytes
/// TODO: This should be an enum and represent cases where the bytes are on disk
#[derive(PartialEq, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", transparent)]
pub struct Code {
  inner: Vec<u8>,
}

impl Code {
  pub fn new(bytes: Vec<u8>) -> Self {
    Self { inner: bytes }
  }

  pub fn bytes(&self) -> &[u8] {
    &self.inner
  }

  pub fn get_mut(&mut self) -> &mut Vec<u8> {
    &mut self.inner
  }

  pub fn as_str(&self) -> anyhow::Result<&str> {
    str::from_utf8(&self.inner)
      .map_err(|e| anyhow::Error::new(e).context("Failed to convert code to UTF8 str"))
  }

  pub fn size(&self) -> u32 {
    self.inner.len() as u32
  }
}

impl Deref for Code {
  type Target = Vec<u8>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl DerefMut for Code {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl Display for Code {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self.inner)
  }
}

impl Debug for Code {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self.as_str().unwrap())
  }
}

impl From<String> for Code {
  fn from(value: String) -> Self {
    Self {
      inner: value.into_bytes(),
    }
  }
}

impl From<&str> for Code {
  fn from(value: &str) -> Self {
    Self {
      inner: value.to_owned().into_bytes(),
    }
  }
}

#[derive(Debug)]
pub struct CreateAssetIdParams<'a> {
  pub code: Option<&'a str>,
  pub environment_id: &'a str,
  /// All paths should be normalized to a project relative string to generate a consistent hash.
  pub file_path: &'a str,
  pub file_type: &'a FileType,
  pub pipeline: Option<&'a str>,
  pub query: Option<&'a str>,
  /// This should be set to None if it's equal to the asset-id and set by the
  /// constructor otherwise the values will differ. See [`Asset::new`] for more.
  pub unique_key: Option<&'a str>,
}

pub fn create_asset_id(params: CreateAssetIdParams) -> String {
  tracing::debug!(?params, "Creating asset id");

  let CreateAssetIdParams {
    code,
    environment_id,
    file_path,
    file_type,
    pipeline,
    query,
    unique_key,
  } = params;

  let mut hasher = crate::hash::IdentifierHasher::default();

  environment_id.hash(&mut hasher);
  file_path.hash(&mut hasher);
  pipeline.hash(&mut hasher);
  code.hash(&mut hasher);
  query.hash(&mut hasher);
  file_type.hash(&mut hasher);
  unique_key.hash(&mut hasher);

  // Ids must be 16 characters for scope hoisting to replace imports correctly in REPLACEMENT_RE
  format!("{:016x}", hasher.finish())
}

/// An asset is a file or part of a file that may represent any data type including source code, binary data, etc.
///
/// Note that assets may exist in the file system or virtually.
///
#[derive(Default, PartialEq, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
  /// The main identify hash for the asset. It is consistent for the entire
  /// build and between builds.
  pub id: AssetId,

  /// Controls which bundle the asset is placed into
  pub bundle_behavior: MaybeBundleBehavior,

  /// The environment of the asset
  pub env: Arc<Environment>,

  /// The file path to the asset
  pub file_path: PathBuf,

  /// The file type of the asset, which may change during transformation
  #[serde(rename = "type")]
  pub file_type: FileType,

  /// The code of this asset, initially read from disk, then becoming the
  /// transformed output
  #[serde(skip_serializing)]
  pub code: Code,

  /// The source map for the asset
  #[serde(skip_serializing)]
  pub map: Option<SourceMap>,

  /// Plugin specific metadata for the asset
  pub meta: JSONObject,

  /// The pipeline defined in .parcelrc that the asset should be processed with
  pub pipeline: Option<String>,

  /// The transformer options for the asset from the dependency query string
  pub query: Option<String>,

  /// Statistics about the asset
  pub stats: AssetStats,

  /// The symbols that the asset exports
  pub symbols: Option<Vec<Symbol>>,

  /// A unique key that identifies an asset
  ///
  /// When a transformer returns multiple assets, it can give them unique keys to identify them.
  /// This can be used to find assets during packaging, or to create dependencies between multiple
  /// assets returned by a transformer by using the unique key as the dependency specifier.
  ///
  /// This is optional because only when transformers add identifiable assets we should add this.
  ///
  /// We should not add this set to the asset ID.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub unique_key: Option<String>,

  /// Whether this asset can be omitted if none of its exports are being used
  ///
  /// This is initially set by the resolver, but can be overridden by transformers.
  ///
  pub side_effects: bool,

  /// Indicates if the asset is used as a bundle entry
  ///
  /// This controls whether a bundle can be split into multiple, or whether all of the
  /// dependencies must be placed in a single bundle.
  ///
  pub is_bundle_splittable: bool,

  /// Whether this asset is part of the project, and not an external dependency
  ///
  /// This indicates that transformation using the project configuration should be applied.
  ///
  pub is_source: bool,

  /// True if the Asset's code was returned from a resolver rather than being
  /// read from disk.
  #[serde(skip_serializing)]
  pub is_virtual: bool,

  /// True if the asset has CommonJS exports
  pub has_cjs_exports: bool,

  /// This is true unless the module is a CommonJS module that does non-static access of the
  /// `this`, `exports` or `module.exports` objects. For example if the module uses code like
  /// `module.exports[key] = 10`.
  pub static_exports: bool,

  /// TODO: MISSING DOCUMENTATION
  pub should_wrap: bool,

  /// TODO: MISSING DOCUMENTATION
  pub has_node_replacements: bool,

  /// True if this is a 'constant module', meaning it only exports constant assignment statements,
  /// on this case this module may be inlined on its usage depending on whether it is only used
  /// once and the atlaspack configuration.
  ///
  /// An example of a 'constant module' would be:
  ///
  /// ```skip
  /// export const MY_CONSTANT = 'some-value';
  /// ```
  pub is_constant_module: bool,

  /// Contains all conditional imports for an asset
  ///
  /// This includes the condition key and the dependency placeholders
  pub conditions: HashSet<Condition>,

  pub config_path: Option<String>,
  pub config_key_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AssetWithDependencies {
  pub asset: Asset,
  pub dependencies: Vec<Dependency>,
}

impl Asset {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    code: Code,
    is_virtual: bool,
    env: Arc<Environment>,
    file_path: PathBuf,
    pipeline: Option<String>,
    project_root: &Path,
    query: Option<String>,
    side_effects: bool,
  ) -> anyhow::Result<Self> {
    let file_type =
      FileType::from_extension(file_path.extension().and_then(|s| s.to_str()).unwrap_or(""));

    let is_source = !file_path
      .ancestors()
      .any(|p| p.file_name() == Some(OsStr::new("node_modules")));

    let virtual_code = if is_virtual {
      Some(code.as_str()?)
    } else {
      None
    };
    let id = create_asset_id(CreateAssetIdParams {
      code: virtual_code,
      environment_id: &env.id(),
      file_path: &to_project_path(project_root, &file_path).to_string_lossy(),
      file_type: &file_type,
      pipeline: pipeline.as_deref(),
      query: query.as_deref(),
      unique_key: None,
    });

    Ok(Self {
      code,
      env,
      file_path,
      file_type,
      id,
      is_bundle_splittable: true,
      is_source,
      pipeline,
      query,
      side_effects,
      unique_key: None,
      is_virtual,
      ..Asset::default()
    })
  }

  #[allow(clippy::too_many_arguments)]
  pub fn new_inline(
    code: Code,
    env: Arc<Environment>,
    file_path: PathBuf,
    file_type: FileType,
    meta: JSONObject,
    project_root: &Path,
    side_effects: bool,
    unique_key: Option<String>,
  ) -> Self {
    let id = create_asset_id(CreateAssetIdParams {
      code: None,
      environment_id: &env.id(),
      file_path: &to_project_path(project_root, &file_path).to_string_lossy(),
      file_type: &file_type,
      pipeline: None,
      query: None,
      unique_key: unique_key.as_deref(),
    });

    let is_source = !file_path
      .ancestors()
      .any(|p| p.file_name() == Some(OsStr::new("node_modules")));

    Self {
      bundle_behavior: Some(BundleBehavior::Inline),
      code,
      env,
      file_path,
      file_type,
      id,
      is_bundle_splittable: true,
      is_source,
      meta,
      side_effects,
      unique_key,
      ..Asset::default()
    }
  }

  pub fn new_discovered(
    code: String,
    file_type: FileType,
    project_root: &Path,
    source_asset: &Asset,
    unique_key: Option<String>,
  ) -> Self {
    let id = create_asset_id(CreateAssetIdParams {
      code: None,
      environment_id: &source_asset.env.id(),
      file_path: &to_project_path(project_root, &source_asset.file_path).to_string_lossy(),
      file_type: &file_type,
      pipeline: None,
      query: None,
      unique_key: unique_key.as_deref(),
    });

    Self {
      code: Code::from(code),
      file_type,
      id,
      unique_key,
      ..source_asset.clone()
    }
  }

  pub fn update_id(&mut self, project_root: &Path) {
    self.id = create_asset_id(CreateAssetIdParams {
      code: None,
      environment_id: &self.env.id(),
      file_path: &to_project_path(project_root, &self.file_path).to_string_lossy(),
      file_type: &self.file_type,
      pipeline: self.pipeline.as_deref(),
      query: self.query.as_deref(),
      unique_key: self.unique_key.as_deref(),
    });
  }

  pub fn set_interpreter(&mut self, shebang: impl Into<serde_json::Value>) {
    self.meta.insert("interpreter".into(), shebang.into());
  }

  pub fn set_meta_id(&mut self, id: impl Into<serde_json::Value>) {
    self.meta.insert("id".into(), id.into());
  }

  pub fn set_has_cjs_exports(&mut self, value: bool) {
    self.meta.insert("hasCJSExports".into(), value.into());
    self.has_cjs_exports = value;
  }

  pub fn set_static_exports(&mut self, value: bool) {
    self.meta.insert("staticExports".into(), value.into());
    self.static_exports = value;
  }

  pub fn set_should_wrap(&mut self, value: bool) {
    self.meta.insert("shouldWrap".into(), value.into());
    self.should_wrap = value;
  }

  pub fn set_is_constant_module(&mut self, is_constant_module: bool) {
    self.is_constant_module = is_constant_module;
    if is_constant_module {
      self.meta.insert("isConstantModule".into(), true.into());
    }
  }

  pub fn set_has_node_replacements(&mut self, has_node_replacements: bool) {
    self.has_node_replacements = has_node_replacements;
    if has_node_replacements {
      self
        .meta
        // This is intentionally snake_case as that's what it was originally.
        .insert("has_node_replacements".into(), true.into());
    }
  }

  pub fn set_conditions(&mut self, conditions: HashSet<Condition>) {
    self.conditions = conditions.clone();
    self.meta.insert("conditions".into(), json!(conditions));
  }
}

/// Statistics that pertain to an asset
#[derive(PartialEq, Clone, Debug, Default, Deserialize, Serialize)]
pub struct AssetStats {
  pub size: u32,
  pub time: u32,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Condition {
  pub key: String,
  pub if_true_placeholder: Option<String>,
  pub if_false_placeholder: Option<String>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn new_produces_stable_ids() {
    let env = Arc::new(Environment::default());
    let project_root = PathBuf::from("project_root");

    let asset_1 = Asset::new(
      Code::from("function hello() {}"),
      false,
      env.clone(),
      project_root.join("test.js"),
      None,
      &project_root,
      None,
      false,
    )
    .unwrap();

    let asset_2 = Asset::new(
      Code::from("function helloButDifferent() {}"),
      false,
      env.clone(),
      project_root.join("test.js"),
      None,
      &project_root,
      None,
      false,
    )
    .unwrap();

    // This nº should not change across runs / compilation
    assert_eq!(asset_1.id, "91d0d64458c223d1");
    assert_eq!(asset_1.id, asset_2.id);
  }

  #[test]
  fn new_creates_asset_ids_relative_to_project_root() {
    let env = Arc::new(Environment::default());
    let project_root = PathBuf::from("project_root");

    let asset = Asset::new(
      Code::default(),
      false,
      env.clone(),
      project_root.join("test.js"),
      None,
      &project_root,
      None,
      false,
    )
    .unwrap();

    assert_eq!(
      asset.id,
      create_asset_id(CreateAssetIdParams {
        code: None,
        environment_id: &env.id(),
        file_path: "test.js",
        file_type: &FileType::Js,
        pipeline: None,
        query: None,
        unique_key: None,
      })
    );
  }

  #[test]
  fn new_inline_creates_asset_ids_relative_to_project_root() {
    let env = Arc::new(Environment::default());
    let project_root = PathBuf::from("project_root");

    let inline_asset = Asset::new_inline(
      Code::default(),
      env.clone(),
      project_root.join("test.js"),
      FileType::Js,
      JSONObject::default(),
      &project_root,
      false,
      None,
    );

    assert_eq!(
      inline_asset.id,
      create_asset_id(CreateAssetIdParams {
        code: None,
        environment_id: &env.id(),
        file_path: "test.js",
        file_type: &FileType::Js,
        pipeline: None,
        query: None,
        unique_key: None,
      })
    );
  }

  #[test]
  fn new_discovered_creates_asset_ids_relative_to_project_root() {
    let project_root = PathBuf::from("project_root");
    let source_asset = Asset {
      file_path: project_root.join("test.js"),
      file_type: FileType::Js,
      ..Asset::default()
    };

    let discovered_asset = Asset::new_discovered(
      String::default(),
      FileType::Css,
      &project_root,
      &source_asset,
      None,
    );

    assert_eq!(
      discovered_asset.id,
      create_asset_id(CreateAssetIdParams {
        code: None,
        environment_id: &source_asset.env.id(),
        file_path: "test.js",
        file_type: &FileType::Css,
        pipeline: None,
        query: None,
        unique_key: None,
      })
    );
  }
}
