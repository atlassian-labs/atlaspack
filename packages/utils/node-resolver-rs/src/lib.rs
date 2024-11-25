use std::borrow::Cow;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use bitflags::bitflags;
use once_cell::unsync::OnceCell;

pub use atlaspack_core::types::IncludeNodeModules;
#[cfg(not(target_arch = "wasm32"))]
pub use atlaspack_filesystem::os_file_system::OsFileSystem;
pub use atlaspack_filesystem::FileSystem;
pub use cache::Cache;
pub use cache::CacheCow;
pub use error::ResolverError;
pub use invalidations::*;
use package_json::AliasValue;
pub use package_json::ExportsCondition;
use package_json::ExportsResolution;
pub use package_json::Fields;
pub use package_json::ModuleType;
use package_json::PackageJson;
pub use package_json::PackageJsonError;
use parking_lot::RwLock;
use parking_lot::RwLockReadGuard;
pub use specifier::parse_package_specifier;
pub use specifier::parse_scheme;
pub use specifier::Specifier;
pub use specifier::SpecifierError;
pub use specifier::SpecifierType;
use tsconfig::TsConfig;

use crate::path::resolve_path;

mod builtins;
mod cache;
mod error;
mod invalidations;
mod package_json;
mod path;
mod specifier;
mod tsconfig;
mod url_to_path;

bitflags! {
  pub struct Flags: u16 {
    /// Atlaspack-style absolute paths resolved relative to project root.
    const ABSOLUTE_SPECIFIERS = 1 << 0;
    /// Atlaspack-style tilde specifiers resolved relative to nearest module root.
    const TILDE_SPECIFIERS = 1 << 1;
    /// The `npm:` scheme.
    const NPM_SCHEME = 1 << 2;
    /// The "alias" field in package.json.
    const ALIASES = 1 << 3;
    /// The settings in tsconfig.json.
    const TSCONFIG = 1 << 4;
    /// The "exports" and "imports" fields in package.json.
    const EXPORTS = 1 << 5;
    /// Directory index files, e.g. index.js.
    const DIR_INDEX = 1 << 6;
    /// Optional extensions in specifiers, using the `extensions` setting.
    const OPTIONAL_EXTENSIONS = 1 << 7;
    /// Whether extensions are replaced in specifiers, e.g. `./foo.js` -> `./foo.ts`.
    /// This also allows omitting the `.ts` and `.tsx` extensions when outside node_modules.
    const TYPESCRIPT_EXTENSIONS = 1 << 8;
    /// Whether to allow omitting the extension when resolving the same file type.
    const PARENT_EXTENSION = 1 << 9;
    /// Whether to allow optional extensions in the "exports" field.
    const EXPORTS_OPTIONAL_EXTENSIONS = 1 << 10;
    /// Enable the graphql ESM upgrade fix
    const GRAPHQL_ESM_UPGRADE = 1 << 11;

    /// Default Node settings for CommonJS.
    const NODE_CJS = Self::EXPORTS.bits | Self::DIR_INDEX.bits | Self::OPTIONAL_EXTENSIONS.bits;
    /// Default Node settings for ESM.
    const NODE_ESM = Self::EXPORTS.bits;
    /// Default TypeScript settings.
    const TYPESCRIPT = Self::TSCONFIG.bits | Self::EXPORTS.bits | Self::DIR_INDEX.bits | Self::OPTIONAL_EXTENSIONS.bits | Self::TYPESCRIPT_EXTENSIONS.bits | Self::EXPORTS_OPTIONAL_EXTENSIONS.bits;
  }
}

type ResolveModuleDir = dyn Fn(&str, &Path) -> Result<PathBuf, ResolverError> + Send + Sync;

pub struct Resolver<'a> {
  pub project_root: Cow<'a, Path>,
  pub extensions: Extensions<'a>,
  pub index_file: &'a str,
  pub entries: Fields,
  pub flags: Flags,
  pub include_node_modules: Cow<'a, IncludeNodeModules>,
  pub conditions: ExportsCondition,
  pub module_dir_resolver: Option<Arc<ResolveModuleDir>>,
  pub cache: CacheCow<'a>,
}

pub enum Extensions<'a> {
  Borrowed(&'a [&'a str]),
  Owned(Vec<String>),
}

impl<'a> Extensions<'a> {
  fn iter(&self) -> impl Iterator<Item = &str> {
    match self {
      Extensions::Borrowed(v) => itertools::Either::Left(v.iter().copied()),
      Extensions::Owned(v) => itertools::Either::Right(v.iter().map(|s| s.as_str())),
    }
  }
}

#[derive(Default, Debug)]
pub struct ResolveOptions {
  pub conditions: ExportsCondition,
  pub custom_conditions: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Resolution {
  /// Resolved to a file path.
  Path(PathBuf),
  /// Resolved to a runtime builtin module.
  Builtin(String),
  /// Resolved to an external module that should not be bundled.
  External,
  /// Resolved to an empty module (e.g. `false` in the package.json#browser field).
  Empty,
  /// Resolved to a global variable.
  Global(String),
}

pub struct ResolveResult {
  pub result: Result<(Resolution, Option<String>), ResolverError>,
  pub invalidations: Invalidations,
}

impl<'a> Resolver<'a> {
  pub fn node(project_root: Cow<'a, Path>, cache: CacheCow<'a>) -> Self {
    Self {
      project_root,
      extensions: Extensions::Borrowed(&["js", "json", "node"]),
      index_file: "index",
      entries: Fields::MAIN,
      flags: Flags::NODE_CJS,
      cache,
      include_node_modules: Cow::Owned(IncludeNodeModules::default()),
      conditions: ExportsCondition::NODE,
      module_dir_resolver: None,
    }
  }

  pub fn node_esm(project_root: Cow<'a, Path>, cache: CacheCow<'a>) -> Self {
    Self {
      project_root,
      extensions: Extensions::Borrowed(&[]),
      index_file: "index",
      entries: Fields::MAIN,
      flags: Flags::NODE_ESM,
      cache,
      include_node_modules: Cow::Owned(IncludeNodeModules::default()),
      conditions: ExportsCondition::NODE,
      module_dir_resolver: None,
    }
  }

  pub fn atlaspack(project_root: Cow<'a, Path>, cache: CacheCow<'a>) -> Self {
    let mut flags = Flags::all();
    flags.set(Flags::GRAPHQL_ESM_UPGRADE, false);

    Self {
      project_root,
      extensions: Extensions::Borrowed(&["mjs", "js", "jsx", "cjs", "json"]),
      index_file: "index",
      entries: Fields::MAIN | Fields::SOURCE | Fields::BROWSER | Fields::MODULE,
      flags,
      cache,
      include_node_modules: Cow::Owned(IncludeNodeModules::default()),
      conditions: ExportsCondition::empty(),
      module_dir_resolver: None,
    }
  }

  pub fn resolve(
    &self,
    specifier: &str,
    from: &Path,
    specifier_type: SpecifierType,
  ) -> ResolveResult {
    self.resolve_with_options(specifier, from, specifier_type, Default::default())
  }

  pub fn resolve_with_options(
    &self,
    specifier: &str,
    from: &Path,
    specifier_type: SpecifierType,
    options: ResolveOptions,
  ) -> ResolveResult {
    tracing::trace!(%specifier, ?from, ?specifier_type, "Resolving specifier");
    let invalidations = Invalidations::default();
    let result =
      self.resolve_with_invalidations(specifier, from, specifier_type, &invalidations, options);

    ResolveResult {
      result,
      invalidations,
    }
  }

  pub fn resolve_with_invalidations(
    &self,
    specifier: &str,
    from: &Path,
    specifier_type: SpecifierType,
    invalidations: &Invalidations,
    options: ResolveOptions,
  ) -> Result<(Resolution, Option<String>), ResolverError> {
    let (specifier, query) = match Specifier::parse(specifier, specifier_type, self.flags) {
      Ok(s) => s,
      Err(e) => return Err(e.into()),
    };
    let mut request = ResolveRequest::new(self, &specifier, specifier_type, from, invalidations);
    if !options.conditions.is_empty() || !options.custom_conditions.is_empty() {
      // If custom conditions are defined, these override the default conditions inferred from the specifier type.
      request.conditions = self.conditions | options.conditions;
      request.custom_conditions = options.custom_conditions.as_slice();
    }

    match request.resolve() {
      Ok(r) => Ok((r, query.map(|q| q.to_owned()))),
      Err(r) => Err(r),
    }
  }

  pub fn resolve_side_effects(
    &self,
    path: &Path,
    invalidations: &Invalidations,
  ) -> Result<bool, ResolverError> {
    if let Some(package) = self.find_package(path.parent().unwrap(), invalidations)? {
      Ok(package.has_side_effects(path))
    } else {
      Ok(true)
    }
  }

  pub fn resolve_module_type(
    &self,
    path: &Path,
    invalidations: &Invalidations,
  ) -> Result<ModuleType, ResolverError> {
    if let Some(ext) = path.extension() {
      if ext == "mjs" {
        return Ok(ModuleType::Module);
      }

      if ext == "cjs" || ext == "node" {
        return Ok(ModuleType::CommonJs);
      }

      if ext == "json" {
        return Ok(ModuleType::Json);
      }

      if ext == "js" {
        if let Some(package) = self.find_package(path.parent().unwrap(), invalidations)? {
          return Ok(package.module_type);
        }
      }
    }

    Ok(ModuleType::CommonJs)
  }

  fn find_package(
    &self,
    from: &Path,
    invalidations: &Invalidations,
  ) -> Result<Option<Arc<PackageJson>>, ResolverError> {
    if let Some(path) = self.find_ancestor_file(from, "package.json", invalidations) {
      let package = self.cache.read_package(Cow::Owned(path)).deref().clone()?;
      return Ok(Some(package));
    }

    Ok(None)
  }

  fn find_ancestor_file(
    &self,
    from: &Path,
    filename: &str,
    invalidations: &Invalidations,
  ) -> Option<PathBuf> {
    let mut first = true;
    for dir in from.ancestors() {
      if let Some(filename) = dir.file_name() {
        if filename == "node_modules" {
          break;
        }
      }

      let file = dir.join(filename);
      if self.cache.is_file(&file) {
        invalidations.invalidate_on_file_change(&file);
        return Some(file);
      }

      if dir == self.project_root {
        break;
      }

      if first {
        invalidations.invalidate_on_file_create_above(filename, from);
      }

      first = false;
    }

    None
  }
}

struct ResolveRequest<'a> {
  resolver: &'a Resolver<'a>,
  specifier: &'a Specifier,
  specifier_type: SpecifierType,
  from: &'a Path,
  flags: RequestFlags,
  tsconfig: OnceCell<Option<Arc<RwLock<TsConfig>>>>,
  root_package: OnceCell<Option<Arc<PackageJson>>>,
  invalidations: &'a Invalidations,
  conditions: ExportsCondition,
  custom_conditions: &'a [String],
  priority_extension: Option<&'a str>,
}

bitflags! {
  struct RequestFlags: u8 {
    const IN_TS_FILE = 1 << 0;
    const IN_JS_FILE = 1 << 1;
    const IN_NODE_MODULES = 1 << 2;
  }
}

impl<'a> ResolveRequest<'a> {
  fn new(
    resolver: &'a Resolver<'a>,
    specifier: &'a Specifier,
    mut specifier_type: SpecifierType,
    from: &'a Path,
    invalidations: &'a Invalidations,
  ) -> Self {
    let mut flags = RequestFlags::empty();
    if let Some(ext) = from.extension() {
      if ext == "ts" || ext == "tsx" || ext == "mts" || ext == "cts" {
        flags |= RequestFlags::IN_TS_FILE;
      } else if ext == "js" || ext == "jsx" || ext == "mjs" || ext == "cjs" {
        flags |= RequestFlags::IN_JS_FILE;
      }
    }

    if from.components().any(|c| c.as_os_str() == "node_modules") {
      flags |= RequestFlags::IN_NODE_MODULES;
    }

    // Replace the specifier type for `npm:` URLs so we resolve it like a module.
    if specifier_type == SpecifierType::Url && matches!(specifier, Specifier::Package(..)) {
      specifier_type = SpecifierType::Esm;
    }

    // Add "import" or "require" condition to global conditions based on specifier type.
    // Also add the "module" condition if the "module" entry field is enabled.
    let mut conditions = resolver.conditions;
    let module_condition = if resolver.entries.contains(Fields::MODULE) {
      ExportsCondition::MODULE
    } else {
      ExportsCondition::empty()
    };
    match specifier_type {
      SpecifierType::Esm => conditions |= ExportsCondition::IMPORT | module_condition,
      SpecifierType::Cjs => conditions |= ExportsCondition::REQUIRE | module_condition,
      _ => {}
    }

    // Store the parent file extension so we can prioritize it even in sub-requests.
    let priority_extension = if resolver.flags.contains(Flags::PARENT_EXTENSION) {
      from.extension().and_then(|ext| ext.to_str())
    } else {
      None
    };

    Self {
      resolver,
      specifier,
      specifier_type,
      from,
      flags,
      tsconfig: OnceCell::new(),
      root_package: OnceCell::new(),
      invalidations,
      conditions,
      custom_conditions: &[],
      priority_extension,
    }
  }

  fn resolve_aliases(
    &self,
    package: &PackageJson,
    specifier: &Specifier,
    fields: Fields,
  ) -> Result<Option<Resolution>, ResolverError> {
    // Don't resolve alias if it came from the package.json itself (i.e. another alias).
    if self.from == package.path {
      return Ok(None);
    }

    match package.resolve_aliases(specifier, fields) {
      Some(alias) => match alias.as_ref() {
        AliasValue::Specifier(specifier) => {
          let mut req = ResolveRequest::new(
            self.resolver,
            specifier,
            SpecifierType::Cjs,
            &package.path,
            self.invalidations,
          );
          req.priority_extension = self.priority_extension;
          req.conditions = self.conditions;
          req.custom_conditions = self.custom_conditions;
          let resolved = req.resolve()?;
          Ok(Some(resolved))
        }
        AliasValue::Bool(false) => Ok(Some(Resolution::Empty)),
        AliasValue::Bool(true) => Ok(None),
        AliasValue::Global { global } => Ok(Some(Resolution::Global((*global).to_owned()))),
      },
      None => Ok(None),
    }
  }

  fn root_package(&self) -> Result<Option<Arc<PackageJson>>, ResolverError> {
    self
      .root_package
      .get_or_try_init(|| self.find_package(&self.resolver.project_root))
      .cloned()
  }

  fn resolve(&self) -> Result<Resolution, ResolverError> {
    match &self.specifier {
      Specifier::Relative(specifier) => {
        // Relative path
        self.resolve_relative(specifier, self.from)
      }
      Specifier::Tilde(specifier) if self.resolver.flags.contains(Flags::TILDE_SPECIFIERS) => {
        // Tilde path. Resolve relative to nearest node_modules directory,
        // the nearest directory with package.json or the project root - whichever comes first.
        if let Some(p) = self.find_ancestor_file(self.from, "package.json") {
          return self.resolve_relative(specifier, &p);
        }

        Err(ResolverError::PackageJsonNotFound {
          from: self.from.to_owned(),
        })
      }
      Specifier::Absolute(specifier) => {
        // In Atlaspack mode, absolute paths are actually relative to the project root.
        if self.resolver.flags.contains(Flags::ABSOLUTE_SPECIFIERS) {
          self.resolve_relative(
            specifier.strip_prefix("/").unwrap(),
            &self.resolver.project_root.join("index"),
          )
        } else if let Some(res) = self.load_path(specifier, None)? {
          Ok(res)
        } else {
          Err(ResolverError::FileNotFound {
            relative: specifier.to_owned(),
            from: PathBuf::from("/"),
          })
        }
      }
      Specifier::Hash(hash) => {
        if self.specifier_type == SpecifierType::Url {
          // An ID-only URL, e.g. `url(#clip-path)` for CSS rules. Ignore.
          Ok(Resolution::External)
        } else if self.specifier_type == SpecifierType::Esm
          && self.resolver.flags.contains(Flags::EXPORTS)
        {
          // An internal package #import specifier.
          let package = self.find_package(self.from.parent().unwrap_or(self.from))?;
          if let Some(package) = package {
            let res = package
              .resolve_package_imports(hash, self.conditions, self.custom_conditions)
              .map_err(|error| ResolverError::PackageJsonError {
                error,
                module: package.name.to_owned(),
                path: package.path.clone(),
              })?;
            match res {
              ExportsResolution::Path(path) => {
                // Extensionless specifiers are not supported in the imports field.
                if let Some(res) = self.try_file_without_aliases(&path)? {
                  return Ok(res);
                }
              }
              ExportsResolution::Package(specifier) => {
                let (module, subpath) = parse_package_specifier(&specifier)?;
                // TODO: should this follow aliases??
                return self.resolve_bare(module, subpath);
              }
              _ => {}
            }
          }

          Err(ResolverError::PackageJsonNotFound {
            from: self.from.to_owned(),
          })
        } else {
          Err(ResolverError::UnknownError)
        }
      }
      Specifier::Package(module, subpath) => {
        // Bare specifier.
        self.resolve_bare(module, subpath)
      }
      Specifier::Builtin(builtin) => {
        if let Some(res) = self.resolve_package_aliases_and_tsconfig_paths(self.specifier)? {
          return Ok(res);
        }
        Ok(Resolution::Builtin(builtin.to_owned()))
      }
      Specifier::Url(url) => {
        if self.specifier_type == SpecifierType::Url {
          Ok(Resolution::External)
        } else {
          let (scheme, _) = parse_scheme(url)?;
          Err(ResolverError::UnknownScheme {
            scheme: scheme.into_owned(),
          })
        }
      }
      _ => Err(ResolverError::UnknownError),
    }
  }

  fn find_ancestor_file(&self, from: &Path, filename: &str) -> Option<PathBuf> {
    let from = from.parent().unwrap();
    self
      .resolver
      .find_ancestor_file(from, filename, self.invalidations)
  }

  fn find_package(&self, from: &Path) -> Result<Option<Arc<PackageJson>>, ResolverError> {
    self.resolver.find_package(from, self.invalidations)
  }

  fn resolve_relative(&self, specifier: &Path, from: &Path) -> Result<Resolution, ResolverError> {
    // Resolve aliases from the nearest package.json.
    let path = resolve_path(from, specifier);
    let package = if self.resolver.flags.contains(Flags::ALIASES) {
      self.find_package(path.parent().unwrap())?
    } else {
      None
    };

    if let Some(res) = self.load_path(&path, package.as_deref())? {
      return Ok(res);
    }

    Err(ResolverError::FileNotFound {
      relative: specifier.to_owned(),
      from: from.to_owned(),
    })
  }

  fn resolve_bare(&self, module: &str, subpath: &str) -> Result<Resolution, ResolverError> {
    let include = match self.resolver.include_node_modules.as_ref() {
      IncludeNodeModules::Bool(b) => *b,
      IncludeNodeModules::Array(a) => a.iter().any(|v| v == module),
      IncludeNodeModules::Map(m) => *m.get(module).unwrap_or(&true),
    };

    if !include {
      return Ok(Resolution::External);
    }

    // Try aliases and tsconfig paths first.
    let specifier = Specifier::Package(String::from(module), String::from(subpath));
    if let Some(res) = self.resolve_package_aliases_and_tsconfig_paths(&specifier)? {
      return Ok(res);
    }

    self.resolve_node_module(module, subpath)
  }

  fn resolve_package_aliases_and_tsconfig_paths(
    &self,
    specifier: &Specifier,
  ) -> Result<Option<Resolution>, ResolverError> {
    if self.resolver.flags.contains(Flags::ALIASES) {
      // First, check for an alias in the root package.json.
      if let Some(package) = self.root_package()? {
        if let Some(res) = self.resolve_aliases(&package, specifier, Fields::ALIAS)? {
          return Ok(Some(res));
        }
      }

      // Next, try the local package.json.
      if let Some(package) = self.find_package(self.from.parent().unwrap_or(self.from))? {
        let mut fields = Fields::ALIAS;
        if self.resolver.entries.contains(Fields::BROWSER) {
          fields |= Fields::BROWSER;
        }
        if let Some(res) = self.resolve_aliases(&package, specifier, fields)? {
          return Ok(Some(res));
        }
      }
    }

    // Next, check tsconfig.json for the paths and baseUrl options.
    self.resolve_tsconfig_paths()
  }

  fn resolve_node_module(&self, module: &str, subpath: &str) -> Result<Resolution, ResolverError> {
    // If there is a custom module directory resolver (e.g. Yarn PnP), use that.
    if let Some(module_dir_resolver) = &self.resolver.module_dir_resolver {
      let package_dir = module_dir_resolver(module, self.from)?;
      return self.resolve_package(package_dir, module, subpath);
    } else {
      self.invalidations.invalidate_on_file_create_above(
        format!("node_modules/{}", module),
        self.from.parent().unwrap_or(self.from),
      );

      for dir in self.from.ancestors() {
        // Skip over node_modules directories
        if let Some(filename) = dir.file_name() {
          if filename == "node_modules" {
            continue;
          }
        }

        let package_dir = dir.join("node_modules").join(module);
        if self.resolver.cache.is_dir(&package_dir) {
          return self.resolve_package(package_dir, module, subpath);
        }
      }
    }

    // NODE_PATH??

    Err(ResolverError::ModuleNotFound {
      module: module.to_owned(),
    })
  }

  fn resolve_package(
    &self,
    package_dir: PathBuf,
    module: &str,
    subpath: &str,
  ) -> Result<Resolution, ResolverError> {
    let package_path = package_dir.join("package.json");
    let package = self.invalidations.read(&package_path, || {
      self
        .resolver
        .cache
        .read_package(Cow::Borrowed(&package_path))
        .deref()
        .clone()
    });

    let package = match package {
      Ok(package) => package,
      Err(ResolverError::IOError(_)) | Err(ResolverError::FileNotFound { .. }) => {
        // No package.json in node_modules is probably invalid but we have tests for it...
        if self.resolver.flags.contains(Flags::DIR_INDEX) {
          if let Some(res) = self.load_file(&package_dir.join(self.resolver.index_file), None)? {
            return Ok(res);
          }
        }

        return Err(ResolverError::ModuleNotFound {
          module: module.to_owned(),
        });
      }
      Err(err) => return Err(err),
    };

    // Try the "source" field first, if present.
    if self.resolver.entries.contains(Fields::SOURCE) && subpath.is_empty() {
      if let Some(source) = package.source() {
        if let Some(res) = self.load_path(&source, Some(&*package))? {
          return Ok(res);
        }
      }
    }

    // If the exports field is present, use the Node ESM algorithm.
    // Otherwise, fall back to classic CJS resolution.
    if self.resolver.flags.contains(Flags::EXPORTS) && package.has_exports() {
      let path = package
        .resolve_package_exports(subpath, self.conditions, self.custom_conditions)
        .map_err(|e| ResolverError::PackageJsonError {
          module: package.name.to_owned(),
          path: package.path.clone(),
          error: e,
        })?;

      // Extensionless specifiers are not supported in the exports field
      // according to the Node spec (for both ESM and CJS). However, webpack
      // didn't follow this, so there are many packages that rely on it (e.g. underscore).
      if self
        .resolver
        .flags
        .contains(Flags::EXPORTS_OPTIONAL_EXTENSIONS)
      {
        if let Some(res) = self.load_file(&path, Some(&*package))? {
          return Ok(res);
        }
      } else if let Some(res) = self.try_file_without_aliases(&path)? {
        return Ok(res);
      }

      // TODO: track location of resolved field
      Err(ResolverError::ModuleSubpathNotFound {
        module: module.to_owned(),
        path,
        package_path: package.path.clone(),
      })
    } else if !subpath.is_empty() {
      let mut package_sub_path = package.path.parent().unwrap().to_owned();
      package_sub_path.push(subpath);

      if let Some(res) = self.load_path(&package_sub_path, Some(&*package))? {
        return Ok(res);
      }

      Err(ResolverError::ModuleSubpathNotFound {
        module: module.to_owned(),
        path: package_sub_path,
        package_path: package.path.clone(),
      })
    } else {
      #[allow(clippy::explicit_auto_deref)]
      let res = self.try_package_entries(&*package);
      if let Ok(Some(res)) = res {
        return Ok(res);
      }

      // Node ESM doesn't allow directory imports.
      if self.resolver.flags.contains(Flags::DIR_INDEX) {
        let package_dir = package.path.parent().unwrap().to_owned();
        if let Some(res) =
          self.load_file(&package_dir.join(self.resolver.index_file), Some(&*package))?
        {
          return Ok(res);
        }
      }

      res?;

      Err(ResolverError::ModuleSubpathNotFound {
        module: module.to_owned(),
        path: package_dir.join(self.resolver.index_file),
        package_path: package.path.clone(),
      })
    }
  }

  fn try_package_entries(
    &self,
    package: &PackageJson,
  ) -> Result<Option<Resolution>, ResolverError> {
    // Try all entry fields.
    if let Some((entry, field)) = package.entries(self.resolver.entries).next() {
      if let Some(res) = self.load_path(&entry, Some(package))? {
        return Ok(Some(res));
      } else {
        return Err(ResolverError::ModuleEntryNotFound {
          module: package.name.to_owned(),
          entry_path: entry,
          package_path: package.path.clone(),
          field,
        });
      }
    }

    Ok(None)
  }

  fn load_path(
    &self,
    path: &Path,
    package: Option<&PackageJson>,
  ) -> Result<Option<Resolution>, ResolverError> {
    // Urls and Node ESM do not resolve directory index files.
    let can_load_directory =
      self.resolver.flags.contains(Flags::DIR_INDEX) && self.specifier_type != SpecifierType::Url;

    // If path ends with / only try loading as a directory.
    let is_directory = can_load_directory
      && path
        .as_os_str()
        .to_str()
        .map(|s| s.ends_with('/'))
        .unwrap_or(false);

    if !is_directory {
      if let Some(res) = self
        .load_file(path, package)?
        .map(|res| self.upgrade_graphql_path_to_esm(package, res))
      {
        return Ok(Some(res));
      }
    }

    // Urls and Node ESM do not resolve directory index files.
    if can_load_directory {
      return Ok(
        self
          .load_directory(path, package)?
          .map(|res| self.upgrade_graphql_path_to_esm(package, res)),
      );
    }

    Ok(None)
  }

  // The 'graphql' package contains both '.js' and '.mjs' versions of all it's
  // files. When importing 'graphql' and targetting ESM then you should resolve
  // the '.mjs' version. However, as the package doesn't have an exports key,
  // when importing subpaths like 'graphql/error' you will resolve the '.js'
  // version. This can cause issues as you end up with mismatched constructors.
  //
  // This function attempts to upgrade any '.js' imports in the 'graphql'
  // package to '.mjs' to avoid this issue.
  fn upgrade_graphql_path_to_esm(
    &self,
    package: Option<&PackageJson>,
    res: Resolution,
  ) -> Resolution {
    if self.resolver.flags.contains(Flags::GRAPHQL_ESM_UPGRADE)
      && package.is_some_and(|package| package.name == "graphql")
      && matches!(&res, Resolution::Path(path) if path.extension().is_some_and(|extension| extension == "js"))
    {
      if let Resolution::Path(path) = &res {
        let esm_path = path.with_extension("mjs");

        if let Ok(Some(res)) = self.load_file(&esm_path, package) {
          tracing::info!(
            "Upgraded graphql import to mjs. {:?} to {:?}",
            path,
            esm_path
          );
          return res;
        } else {
          tracing::info!(
            "Failed to upgrade graphql import to mjs {:?}. Tried {:?}",
            path,
            esm_path
          );
        }
      }
    }

    res
  }

  fn load_file(
    &self,
    path: &Path,
    package: Option<&PackageJson>,
  ) -> Result<Option<Resolution>, ResolverError> {
    // First try the path as is.
    // TypeScript only supports resolving specifiers ending with `.ts` or `.tsx`
    // in a certain mode, but we always allow it.
    // If there is no extension in the original specifier, only check aliases
    // here and delay checking for an extensionless file until later (since this is unlikely).
    if let Some(res) = self.try_suffixes(path, "", package, path.extension().is_none())? {
      return Ok(Some(res));
    }

    // TypeScript allows a specifier like "./foo.js" to resolve to "./foo.ts".
    // TSC does this before trying to append an extension. We match this
    // rather than matching "./foo.js.ts", which seems more unlikely.
    // However, if "./foo.js" exists we will resolve to it (above), unlike TSC.
    // This is to match Node and other bundlers.
    if self.resolver.flags.contains(Flags::TYPESCRIPT_EXTENSIONS)
      && self.flags.contains(RequestFlags::IN_TS_FILE)
      && !self.flags.contains(RequestFlags::IN_NODE_MODULES)
      && self.specifier_type != SpecifierType::Url
    {
      if let Some(ext) = path.extension() {
        // TODO: would be nice if there was a way to do this without cloning
        // but OsStr doesn't let you create a slice.
        let without_extension = &path.with_extension("");
        let extensions: Option<&[&str]> = if ext == "js" || ext == "jsx" {
          // TSC always prioritizes .ts over .tsx, even when the original extension was .jsx.
          Some(&["ts", "tsx"])
        } else if ext == "mjs" {
          Some(&["mts"])
        } else if ext == "cjs" {
          Some(&["cts"])
        } else {
          None
        };

        let res = if let Some(extensions) = extensions {
          self.try_extensions(
            without_extension,
            package,
            &Extensions::Borrowed(extensions),
            false,
          )?
        } else {
          None
        };

        if res.is_some() {
          return Ok(res);
        }
      }
    }

    // Try adding the same extension as in the parent file first.
    if let Some(ext) = self.priority_extension {
      // Use try_suffixes here to skip the specifier_type check.
      // This is reproducing a bug in the old version of the Atlaspack resolver
      // where URL dependencies could omit the extension if it was the same as the parent.
      // TODO: Revert this in the next major version.
      if let Some(res) = self.try_suffixes(path, ext, package, false)? {
        return Ok(Some(res));
      }
    }

    // Try adding typescript extensions if outside node_modules.
    if self
      .resolver
      .flags
      .contains(Flags::TYPESCRIPT_EXTENSIONS | Flags::OPTIONAL_EXTENSIONS)
      && !self.flags.contains(RequestFlags::IN_NODE_MODULES)
    {
      if let Some(res) =
        self.try_extensions(path, package, &Extensions::Borrowed(&["ts", "tsx"]), true)?
      {
        return Ok(Some(res));
      }
    }

    // Try appending the configured extensions.
    if let Some(res) = self.try_extensions(path, package, &self.resolver.extensions, true)? {
      return Ok(Some(res));
    }

    // If there is no extension in the specifier, try an extensionless file as a last resort.
    if path.extension().is_none() {
      if let Some(res) = self.try_suffixes(path, "", package, false)? {
        return Ok(Some(res));
      }
    }

    Ok(None)
  }

  fn try_extensions(
    &self,
    path: &Path,
    package: Option<&PackageJson>,
    extensions: &Extensions,
    skip_parent: bool,
  ) -> Result<Option<Resolution>, ResolverError> {
    if self.resolver.flags.contains(Flags::OPTIONAL_EXTENSIONS)
      && self.specifier_type != SpecifierType::Url
    {
      // Try appending each extension.
      for ext in extensions.iter() {
        // Skip parent extension if we already tried it.
        if skip_parent
          && self.resolver.flags.contains(Flags::PARENT_EXTENSION)
          && matches!(self.from.extension(), Some(e) if e == ext)
        {
          continue;
        }

        if let Some(res) = self.try_suffixes(path, ext, package, false)? {
          return Ok(Some(res));
        }
      }
    }

    Ok(None)
  }

  fn try_suffixes(
    &self,
    path: &Path,
    ext: &str,
    package: Option<&PackageJson>,
    alias_only: bool,
  ) -> Result<Option<Resolution>, ResolverError> {
    // TypeScript supports a moduleSuffixes option in tsconfig.json which allows suffixes
    // such as ".ios" to be appended just before the last extension.
    let mut module_suffixes = vec![String::from("")];

    if let Some(tsconfig) = self.tsconfig_read()? {
      if let Some(module_suffixs) = tsconfig.module_suffixes.as_ref() {
        module_suffixes = module_suffixs.clone()
      };
    }

    for suffix in module_suffixes {
      let mut p = if !suffix.is_empty() {
        // The suffix is placed before the _last_ extension. If we will be appending
        // another extension later, then we only need to append the suffix first.
        // Otherwise, we need to remove the original extension so we can add the suffix.
        // TODO: TypeScript only removes certain extensions here...
        let original_ext = path.extension();
        let mut s = if ext.is_empty() && original_ext.is_some() {
          path.with_extension("").into_os_string()
        } else {
          path.into()
        };

        // Append the suffix (this is not necessarily an extension).
        s.push(suffix);

        // Re-add the original extension if we removed it earlier.
        if ext.is_empty() {
          if let Some(original_ext) = original_ext {
            s.push(".");
            s.push(original_ext);
          }
        }

        Cow::Owned(PathBuf::from(s))
      } else {
        Cow::Borrowed(path)
      };

      if !ext.is_empty() {
        // Append the extension.
        let mut s = p.into_owned().into_os_string();
        s.push(".");
        s.push(ext);
        p = Cow::Owned(PathBuf::from(s));
      }

      if let Some(res) = self.try_file(p.as_ref(), package, alias_only)? {
        return Ok(Some(res));
      }
    }

    Ok(None)
  }

  fn try_file(
    &self,
    path: &Path,
    package: Option<&PackageJson>,
    alias_only: bool,
  ) -> Result<Option<Resolution>, ResolverError> {
    if self.resolver.flags.contains(Flags::ALIASES) {
      // Check the project root package.json first.
      if let Some(package) = self.root_package()? {
        if let Ok(s) = path.strip_prefix(package.path.parent().unwrap()) {
          let specifier = Specifier::Relative(s.to_path_buf());
          if let Some(res) = self.resolve_aliases(&package, &specifier, Fields::ALIAS)? {
            return Ok(Some(res));
          }
        }
      }

      // Next try the local package.json.
      if let Some(package) = package {
        if let Ok(s) = path.strip_prefix(package.path.parent().unwrap()) {
          let specifier = Specifier::Relative(s.to_path_buf());
          let mut fields = Fields::ALIAS;
          if self.resolver.entries.contains(Fields::BROWSER) {
            fields |= Fields::BROWSER;
          }
          if let Some(res) = self.resolve_aliases(package, &specifier, fields)? {
            return Ok(Some(res));
          }
        }
      }
    }

    if alias_only {
      return Ok(None);
    }

    self.try_file_without_aliases(path)
  }

  fn try_file_without_aliases(&self, path: &Path) -> Result<Option<Resolution>, ResolverError> {
    if self.resolver.cache.is_file(path) {
      Ok(Some(Resolution::Path(
        self.resolver.cache.canonicalize(path)?,
      )))
    } else {
      self.invalidations.invalidate_on_file_create(path);
      Ok(None)
    }
  }

  fn load_directory(
    &self,
    dir: &Path,
    parent_package: Option<&PackageJson>,
  ) -> Result<Option<Resolution>, ResolverError> {
    // Check if there is a package.json in this directory, and if so, use its entries.
    // Note that the "exports" field is NOT used here - only in resolve_node_module.
    let path = dir.join("package.json");
    let mut res = Ok(None);
    let package = if let Ok(package) = self.invalidations.read(&path, || {
      self
        .resolver
        .cache
        .read_package(Cow::Borrowed(&path))
        .deref()
        .clone()
    }) {
      res = self.try_package_entries(&package);
      if matches!(res, Ok(Some(_))) {
        return res;
      }
      Some(package)
    } else {
      None
    };

    // If no package.json, or no entries, try an index file with all possible extensions.
    if self.resolver.flags.contains(Flags::DIR_INDEX) && self.resolver.cache.is_dir(dir) {
      let target = package.as_deref().or(parent_package);
      return self.load_file(&dir.join(self.resolver.index_file), target);
    }

    res
  }

  fn resolve_tsconfig_paths(&self) -> Result<Option<Resolution>, ResolverError> {
    if let Some(tsconfig) = self.tsconfig_read()? {
      for path in tsconfig.paths(self.specifier) {
        // TODO: should aliases apply to tsconfig paths??
        if let Some(res) = self.load_path(&path, None)? {
          return Ok(Some(res));
        }
      }
    }

    Ok(None)
  }

  fn tsconfig_read(&self) -> Result<Option<RwLockReadGuard<'_, TsConfig>>, ResolverError> {
    if let Some(tsconfig) = self.tsconfig()? {
      Ok(Some(tsconfig.read()))
    } else {
      Ok(None)
    }
  }

  fn tsconfig(&self) -> Result<&Option<Arc<RwLock<TsConfig>>>, ResolverError> {
    if self.resolver.flags.contains(Flags::TSCONFIG)
      && self
        .flags
        .intersects(RequestFlags::IN_TS_FILE | RequestFlags::IN_JS_FILE)
      && !self.flags.contains(RequestFlags::IN_NODE_MODULES)
    {
      self.tsconfig.get_or_try_init(|| {
        if let Some(path) = self.find_ancestor_file(self.from, "tsconfig.json") {
          let tsconfig = self.read_tsconfig(path)?;
          return Ok(Some(tsconfig));
        }

        Ok(None)
      })
    } else {
      Ok(&None)
    }
  }

  fn read_tsconfig(&self, path: PathBuf) -> Result<Arc<RwLock<TsConfig>>, ResolverError> {
    let tsconfig = self.invalidations.read(&path, || {
      let tsconfig = self.resolver.cache.read_tsconfig(&path, |tsconfig| {
        let extends = tsconfig.extends.read();
        let mut compiler_options = tsconfig.compiler_options.write();

        for i in 0..extends.len() {
          let path = match &extends[i] {
            Specifier::Absolute(path) => path.to_owned(),
            Specifier::Relative(path) => {
              let mut absolute_path = resolve_path(&compiler_options.path, path);

              // TypeScript allows "." and ".." to implicitly refer to a tsconfig.json file.
              if path == Path::new(".") || path == Path::new("..") {
                absolute_path.push("tsconfig.json");
              }

              let mut exists = self.resolver.cache.fs.is_file(&absolute_path);

              // If the file doesn't exist, and doesn't end with `.json`, try appending the extension.
              if !exists {
                let try_extension = match absolute_path.extension() {
                  None => true,
                  Some(ext) => ext != "json",
                };

                if try_extension {
                  let mut os_str = absolute_path.into_os_string();
                  os_str.push(".json");
                  absolute_path = PathBuf::from(os_str);
                  exists = self.resolver.cache.fs.is_file(&absolute_path)
                }
              }

              if !exists {
                return Err(ResolverError::TsConfigExtendsNotFound {
                  tsconfig: compiler_options.path.clone(),
                  error: Box::new(ResolverError::FileNotFound {
                    relative: path.to_path_buf(),
                    from: compiler_options.path.clone(),
                  }),
                });
              }

              absolute_path
            }
            specifier @ Specifier::Package(..) => {
              let resolver = Resolver {
                project_root: Cow::Borrowed(&self.resolver.project_root),
                extensions: Extensions::Borrowed(&["json"]),
                index_file: "tsconfig.json",
                entries: Fields::TSCONFIG,
                flags: Flags::NODE_CJS,
                cache: CacheCow::Borrowed(&self.resolver.cache),
                include_node_modules: Cow::Owned(IncludeNodeModules::default()),
                conditions: ExportsCondition::TYPES,
                module_dir_resolver: self.resolver.module_dir_resolver.clone(),
              };

              let req = ResolveRequest::new(
                &resolver,
                specifier,
                SpecifierType::Cjs,
                &compiler_options.path,
                self.invalidations,
              );

              let res = req
                .resolve()
                .map_err(|err| ResolverError::TsConfigExtendsNotFound {
                  tsconfig: compiler_options.path.clone(),
                  error: Box::new(err),
                })?;

              if let Resolution::Path(res) = res {
                res
              } else {
                return Err(ResolverError::TsConfigExtendsNotFound {
                  tsconfig: compiler_options.path.clone(),
                  error: Box::new(ResolverError::UnknownError),
                });
              }
            }
            _ => return Ok(()),
          };

          let tsconfig = self.read_tsconfig(path)?;
          let extended = tsconfig.read();
          compiler_options.extend(&extended);
        }

        Ok(())
      });
      tsconfig.deref().clone()
    })?;

    Ok(tsconfig.compiler_options.clone())
  }
}

#[cfg(test)]
mod tests {
  use std::collections::{BTreeMap, HashSet};

  use super::*;

  fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .parent()
      .unwrap()
      .join("node-resolver-core/test/fixture")
  }

  fn test_resolver<'a>() -> Resolver<'a> {
    Resolver::atlaspack(
      root().into(),
      CacheCow::Owned(Cache::new(Arc::new(OsFileSystem))),
    )
  }

  fn node_resolver<'a>() -> Resolver<'a> {
    Resolver::node(
      root().into(),
      CacheCow::Owned(Cache::new(Arc::new(OsFileSystem))),
    )
  }

  #[test]
  fn relative() {
    assert_eq!(
      test_resolver()
        .resolve("./bar.js", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(".///bar.js", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("./bar", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("~/bar", &root().join("nested/test.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("~bar", &root().join("nested/test.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "~/bar",
          &root().join("node_modules/foo/nested/baz.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("./nested", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("nested/index.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("./bar?foo=2", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("./bar?foo=2", &root().join("foo.js"), SpecifierType::Cjs)
        .result
        .unwrap_err(),
      ResolverError::FileNotFound {
        relative: "bar?foo=2".into(),
        from: root().join("foo.js")
      },
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./foo",
          &root().join("priority/index.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("priority/foo.js"))
    );

    let invalidations = test_resolver()
      .resolve("./bar", &root().join("foo.js"), SpecifierType::Esm)
      .invalidations;
    assert_eq!(
      invalidations
        .invalidate_on_file_create
        .read()
        .unwrap()
        .iter()
        .collect::<HashSet<_>>(),
      HashSet::new()
    );
    assert_eq!(
      invalidations
        .invalidate_on_file_change
        .read()
        .unwrap()
        .iter()
        .cloned()
        .collect::<HashSet<_>>(),
      HashSet::from([root().join("package.json"), root().join("tsconfig.json")])
    );
  }

  #[test]
  fn test_absolute() {
    assert_eq!(
      test_resolver()
        .resolve("/bar", &root().join("nested/test.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "/bar",
          &root().join("node_modules/foo/index.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );

    #[cfg(not(windows))]
    {
      assert_eq!(
        test_resolver()
          .resolve(
            "file:///bar",
            &root().join("nested/test.js"),
            SpecifierType::Esm
          )
          .result
          .unwrap()
          .0,
        Resolution::Path(root().join("bar.js"))
      );
      assert_eq!(
        node_resolver()
          .resolve(
            root().join("foo.js").to_str().unwrap(),
            &root().join("nested/test.js"),
            SpecifierType::Esm
          )
          .result
          .unwrap()
          .0,
        Resolution::Path(root().join("foo.js"))
      );
      assert_eq!(
        node_resolver()
          .resolve(
            &format!("file://{}", root().join("foo.js").to_str().unwrap()),
            &root().join("nested/test.js"),
            SpecifierType::Esm
          )
          .result
          .unwrap()
          .0,
        Resolution::Path(root().join("foo.js"))
      );
    }
  }

  #[test]
  fn node_modules() {
    assert_eq!(
      test_resolver()
        .resolve("foo", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/index.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("package-main", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-main/main.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("package-module", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-module/module.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-browser",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-browser/browser.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-fallback",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-fallback/index.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-main-directory",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-main-directory/nested/index.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("foo/nested/baz", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/nested/baz.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("@scope/pkg", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/@scope/pkg/index.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "@scope/pkg/foo/bar",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/@scope/pkg/foo/bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "foo/with space.mjs",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/with space.mjs"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "foo/with%20space.mjs",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/with space.mjs"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "foo/with space.mjs",
          &root().join("foo.js"),
          SpecifierType::Cjs
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/with space.mjs"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "foo/with%20space.mjs",
          &root().join("foo.js"),
          SpecifierType::Cjs
        )
        .result
        .unwrap_err(),
      ResolverError::ModuleSubpathNotFound {
        module: "foo".into(),
        path: root().join("node_modules/foo/with%20space.mjs"),
        package_path: root().join("node_modules/foo/package.json")
      },
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "@scope/pkg?foo=2",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/@scope/pkg/index.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "@scope/pkg?foo=2",
          &root().join("foo.js"),
          SpecifierType::Cjs
        )
        .result
        .unwrap_err(),
      ResolverError::ModuleNotFound {
        module: "@scope/pkg?foo=2".into()
      },
    );

    let invalidations = test_resolver()
      .resolve("foo", &root().join("foo.js"), SpecifierType::Esm)
      .invalidations;
    assert_eq!(
      invalidations
        .invalidate_on_file_create
        .read()
        .unwrap()
        .iter()
        .cloned()
        .collect::<HashSet<_>>(),
      HashSet::from([FileCreateInvalidation::FileName {
        file_name: "node_modules/foo".into(),
        above: root()
      },])
    );
    assert_eq!(
      invalidations
        .invalidate_on_file_change
        .read()
        .unwrap()
        .iter()
        .cloned()
        .collect::<HashSet<_>>(),
      HashSet::from([
        root().join("node_modules/foo/package.json"),
        root().join("package.json"),
        root().join("tsconfig.json")
      ])
    );
  }

  #[test]
  fn browser_field() {
    assert_eq!(
      test_resolver()
        .resolve(
          "package-browser-alias",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-browser-alias/browser.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-browser-alias/foo",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-browser-alias/bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./foo",
          &root().join("node_modules/package-browser-alias/browser.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-browser-alias/bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./nested",
          &root().join("node_modules/package-browser-alias/browser.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(
        root().join("node_modules/package-browser-alias/subfolder1/subfolder2/subfile.js")
      )
    );
  }

  #[test]
  fn local_aliases() {
    assert_eq!(
      test_resolver()
        .resolve(
          "package-alias/foo",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-alias/bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./foo",
          &root().join("node_modules/package-alias/browser.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-alias/bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./lib/test",
          &root().join("node_modules/package-alias-glob/browser.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-alias-glob/src/test.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-browser-exclude",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Empty
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./lib/test",
          &root().join("node_modules/package-alias-glob/index.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-alias-glob/src/test.js"))
    );

    let invalidations = test_resolver()
      .resolve(
        "package-alias/foo",
        &root().join("foo.js"),
        SpecifierType::Esm,
      )
      .invalidations;
    assert_eq!(
      invalidations
        .invalidate_on_file_create
        .read()
        .unwrap()
        .iter()
        .cloned()
        .collect::<HashSet<_>>(),
      HashSet::from([FileCreateInvalidation::FileName {
        file_name: "node_modules/package-alias".into(),
        above: root()
      },])
    );
    assert_eq!(
      invalidations
        .invalidate_on_file_change
        .read()
        .unwrap()
        .iter()
        .cloned()
        .collect::<HashSet<_>>(),
      HashSet::from([
        root().join("node_modules/package-alias/package.json"),
        root().join("package.json"),
        root().join("tsconfig.json")
      ])
    );
  }

  #[test]
  fn global_aliases() {
    assert_eq!(
      test_resolver()
        .resolve("aliased", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/index.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "aliased",
          &root().join("node_modules/package-alias/foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/index.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "aliased/bar",
          &root().join("node_modules/package-alias/foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("aliased-file", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "aliased-file",
          &root().join("node_modules/package-alias/foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "aliasedfolder/test.js",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("nested/test.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("aliasedfolder", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("nested/index.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "aliasedabsolute/test.js",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("nested/test.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "aliasedabsolute",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("nested/index.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("foo/bar", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("glob/bar/test", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("nested/test.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("something", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("nested/test.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "something",
          &root().join("node_modules/package-alias/foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("nested/test.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-alias-exclude",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Empty
    );
    assert_eq!(
      test_resolver()
        .resolve("./baz", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("../baz", &root().join("x/foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("~/baz", &root().join("x/foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./baz",
          &root().join("node_modules/foo/bar.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/baz.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "~/baz",
          &root().join("node_modules/foo/bar.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/baz.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "/baz",
          &root().join("node_modules/foo/bar.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("url", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Empty
    );
  }

  #[test]
  fn test_urls() {
    assert_eq!(
      test_resolver()
        .resolve(
          "http://example.com/foo.png",
          &root().join("foo.js"),
          SpecifierType::Url
        )
        .result
        .unwrap()
        .0,
      Resolution::External
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "//example.com/foo.png",
          &root().join("foo.js"),
          SpecifierType::Url
        )
        .result
        .unwrap()
        .0,
      Resolution::External
    );
    assert_eq!(
      test_resolver()
        .resolve("#hash", &root().join("foo.js"), SpecifierType::Url)
        .result
        .unwrap()
        .0,
      Resolution::External
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "http://example.com/foo.png",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap_err(),
      ResolverError::UnknownScheme {
        scheme: "http".into()
      },
    );
    assert_eq!(
      test_resolver()
        .resolve("bar.js", &root().join("foo.js"), SpecifierType::Url)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    // Reproduce bug for now
    // assert_eq!(
    //   test_resolver()
    //     .resolve("bar", &root().join("foo.js"), SpecifierType::Url)
    //     .result
    //     .unwrap_err(),
    //   ResolverError::FileNotFound {
    //     relative: "bar".into(),
    //     from: root().join("foo.js")
    //   }
    // );
    assert_eq!(
      test_resolver()
        .resolve("bar", &root().join("foo.js"), SpecifierType::Url)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("bar.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("npm:foo", &root().join("foo.js"), SpecifierType::Url)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/index.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve("npm:@scope/pkg", &root().join("foo.js"), SpecifierType::Url)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/@scope/pkg/index.js"))
    );
  }

  #[test]
  fn test_exports() {
    assert_eq!(
      test_resolver()
        .resolve(
          "package-exports",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-exports/main.mjs"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-exports/foo",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      // "browser" field is NOT used.
      Resolution::Path(root().join("node_modules/package-exports/foo.mjs"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-exports/features/test",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-exports/features/test.mjs"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-exports/extensionless-features/test",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-exports/features/test.mjs"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-exports/extensionless-features/test.mjs",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-exports/features/test.mjs"))
    );
    assert_eq!(
      node_resolver()
        .resolve(
          "package-exports/extensionless-features/test",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap_err(),
      ResolverError::ModuleSubpathNotFound {
        module: "package-exports".into(),
        package_path: root().join("node_modules/package-exports/package.json"),
        path: root().join("node_modules/package-exports/features/test"),
      },
    );
    assert_eq!(
      node_resolver()
        .resolve(
          "package-exports/extensionless-features/test",
          &root().join("foo.js"),
          SpecifierType::Cjs
        )
        .result
        .unwrap_err(),
      ResolverError::ModuleSubpathNotFound {
        module: "package-exports".into(),
        package_path: root().join("node_modules/package-exports/package.json"),
        path: root().join("node_modules/package-exports/features/test"),
      },
    );
    assert_eq!(
      node_resolver()
        .resolve(
          "package-exports/extensionless-features/test.mjs",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-exports/features/test.mjs"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-exports/space",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-exports/with space.mjs"))
    );
    // assert_eq!(
    //   test_resolver().resolve("package-exports/with%20space", &root().join("foo.js"), SpecifierType::Esm).unwrap().0,
    //   Resolution::Path(root().join("node_modules/package-exports/with space.mjs"))
    // );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-exports/with space",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap_err(),
      ResolverError::PackageJsonError {
        module: "package-exports".into(),
        path: root().join("node_modules/package-exports/package.json"),
        error: PackageJsonError::PackagePathNotExported
      },
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-exports/internal",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap_err(),
      ResolverError::PackageJsonError {
        module: "package-exports".into(),
        path: root().join("node_modules/package-exports/package.json"),
        error: PackageJsonError::PackagePathNotExported
      },
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-exports/internal.mjs",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap_err(),
      ResolverError::PackageJsonError {
        module: "package-exports".into(),
        path: root().join("node_modules/package-exports/package.json"),
        error: PackageJsonError::PackagePathNotExported
      },
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-exports/invalid",
          &root().join("foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap_err(),
      ResolverError::PackageJsonError {
        module: "package-exports".into(),
        path: root().join("node_modules/package-exports/package.json"),
        error: PackageJsonError::InvalidPackageTarget
      }
    );
  }

  #[test]
  fn test_self_reference() {
    assert_eq!(
      test_resolver()
        .resolve(
          "package-exports",
          &root().join("node_modules/package-exports/foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-exports/main.mjs"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "package-exports/foo",
          &root().join("node_modules/package-exports/foo.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-exports/foo.mjs"))
    );
  }

  #[test]
  fn test_imports() {
    assert_eq!(
      test_resolver()
        .resolve(
          "#internal",
          &root().join("node_modules/package-exports/main.mjs"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/package-exports/internal.mjs"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "#foo",
          &root().join("node_modules/package-exports/main.mjs"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/index.js"))
    );
  }

  #[test]
  fn test_builtins() {
    assert_eq!(
      test_resolver()
        .resolve("zlib", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Builtin("zlib".into())
    );
    assert_eq!(
      test_resolver()
        .resolve("node:zlib", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Builtin("zlib".into())
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "node:fs/promises",
          &root().join("foo.js"),
          SpecifierType::Cjs
        )
        .result
        .unwrap()
        .0,
      Resolution::Builtin("fs/promises".into())
    );
  }

  #[test]
  fn test_tsconfig() {
    assert_eq!(
      test_resolver()
        .resolve("ts-path", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("foo.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "ts-path",
          &root().join("nested/index.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("nested/test.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "foo",
          &root().join("tsconfig/index/index.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/tsconfig-index/foo.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "foo",
          &root().join("tsconfig/field/index.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/tsconfig-field/foo.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "foo",
          &root().join("tsconfig/exports/index.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/tsconfig-exports/foo.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "foo",
          &root().join("tsconfig/extends-extension/index.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("tsconfig/extends-extension/foo.js"))
    );

    let mut extends_node_module_resolver = test_resolver();
    extends_node_module_resolver.include_node_modules = Cow::Owned(IncludeNodeModules::Bool(false));
    assert_eq!(
      extends_node_module_resolver
        .resolve(
          "./bar",
          &root().join("tsconfig/extends-node-module/index.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("tsconfig/extends-node-module/bar.ts"))
    );

    assert_eq!(
      test_resolver()
        .resolve(
          "ts-path",
          &root().join("node_modules/tsconfig-not-used/index.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap_err(),
      ResolverError::ModuleNotFound {
        module: "ts-path".into()
      },
    );
    assert_eq!(
      test_resolver()
        .resolve("ts-path", &root().join("foo.css"), SpecifierType::Esm)
        .result
        .unwrap_err(),
      ResolverError::ModuleNotFound {
        module: "ts-path".into()
      },
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "zlib",
          &root().join("tsconfig/builtins/thing.js"),
          SpecifierType::Cjs
        )
        .result
        .unwrap()
        .0,
      Resolution::Builtin("zlib".into())
    );

    let invalidations = test_resolver()
      .resolve("ts-path", &root().join("foo.js"), SpecifierType::Esm)
      .invalidations;
    assert_eq!(
      invalidations
        .invalidate_on_file_create
        .read()
        .unwrap()
        .iter()
        .cloned()
        .collect::<HashSet<_>>(),
      HashSet::new()
    );
    assert_eq!(
      invalidations
        .invalidate_on_file_change
        .read()
        .unwrap()
        .iter()
        .cloned()
        .collect::<HashSet<_>>(),
      HashSet::from([root().join("package.json"), root().join("tsconfig.json")])
    );
  }

  #[test]
  fn test_module_suffixes() {
    assert_eq!(
      test_resolver()
        .resolve(
          "./a",
          &root().join("tsconfig/suffixes/index.ts"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("tsconfig/suffixes/a.ios.ts"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./a.ts",
          &root().join("tsconfig/suffixes/index.ts"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("tsconfig/suffixes/a.ios.ts"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./b",
          &root().join("tsconfig/suffixes/index.ts"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("tsconfig/suffixes/b.ts"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./b.ts",
          &root().join("tsconfig/suffixes/index.ts"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("tsconfig/suffixes/b.ts"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./c",
          &root().join("tsconfig/suffixes/index.ts"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("tsconfig/suffixes/c-test.ts"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./c.ts",
          &root().join("tsconfig/suffixes/index.ts"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("tsconfig/suffixes/c-test.ts"))
    );
  }

  #[test]
  fn test_tsconfig_parsing() {
    assert_eq!(
      test_resolver()
        .resolve(
          "foo",
          &root().join("tsconfig/trailing-comma/index.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("tsconfig/trailing-comma/bar.js"))
    );
  }

  #[test]
  fn test_ts_extensions() {
    assert_eq!(
      test_resolver()
        .resolve(
          "./a.js",
          &root().join("ts-extensions/index.ts"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("ts-extensions/a.ts"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./a.jsx",
          &root().join("ts-extensions/index.ts"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      // TSC always prioritizes .ts over .tsx
      Resolution::Path(root().join("ts-extensions/a.ts"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./a.mjs",
          &root().join("ts-extensions/index.ts"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("ts-extensions/a.mts"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./a.cjs",
          &root().join("ts-extensions/index.ts"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("ts-extensions/a.cts"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./b.js",
          &root().join("ts-extensions/index.ts"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      // We deviate from TSC here to match Node/bundlers.
      Resolution::Path(root().join("ts-extensions/b.js"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./c.js",
          &root().join("ts-extensions/index.ts"),
          SpecifierType::Esm
        )
        .result
        .unwrap()
        .0,
      // This matches TSC. c.js.ts seems kinda unlikely?
      Resolution::Path(root().join("ts-extensions/c.ts"))
    );
    assert_eq!(
      test_resolver()
        .resolve(
          "./a.js",
          &root().join("ts-extensions/index.js"),
          SpecifierType::Esm
        )
        .result
        .unwrap_err(),
      ResolverError::FileNotFound {
        relative: "a.js".into(),
        from: root().join("ts-extensions/index.js")
      },
    );

    let invalidations = test_resolver()
      .resolve(
        "./a.js",
        &root().join("ts-extensions/index.ts"),
        SpecifierType::Esm,
      )
      .invalidations;
    assert_eq!(
      invalidations
        .invalidate_on_file_create
        .read()
        .unwrap()
        .iter()
        .cloned()
        .collect::<HashSet<_>>(),
      HashSet::from([
        FileCreateInvalidation::Path(root().join("ts-extensions/a.js")),
        FileCreateInvalidation::FileName {
          file_name: "package.json".into(),
          above: root().join("ts-extensions")
        },
        FileCreateInvalidation::FileName {
          file_name: "tsconfig.json".into(),
          above: root().join("ts-extensions")
        },
      ])
    );
    assert_eq!(
      invalidations
        .invalidate_on_file_change
        .read()
        .unwrap()
        .iter()
        .cloned()
        .collect::<HashSet<_>>(),
      HashSet::from([root().join("package.json"), root().join("tsconfig.json")])
    );
  }

  fn resolve_side_effects(specifier: &str, from: &Path) -> bool {
    let resolver = test_resolver();
    let resolved = resolver
      .resolve(specifier, from, SpecifierType::Esm)
      .result
      .unwrap()
      .0;

    if let Resolution::Path(path) = resolved {
      resolver
        .resolve_side_effects(&path, &Invalidations::default())
        .unwrap()
    } else {
      unreachable!()
    }
  }

  #[test]
  fn test_side_effects() {
    assert!(!resolve_side_effects(
      "side-effects-false/src/index.js",
      &root().join("foo.js")
    ));
    assert!(!resolve_side_effects(
      "side-effects-false/src/index",
      &root().join("foo.js")
    ));
    assert!(!resolve_side_effects(
      "side-effects-false/src/",
      &root().join("foo.js")
    ));
    assert!(!resolve_side_effects(
      "side-effects-false",
      &root().join("foo.js")
    ));
    assert!(!resolve_side_effects(
      "side-effects-package-redirect-up/foo/bar",
      &root().join("foo.js")
    ));
    assert!(!resolve_side_effects(
      "side-effects-package-redirect-down/foo/bar",
      &root().join("foo.js")
    ));
    assert!(resolve_side_effects(
      "side-effects-false-glob/a/index",
      &root().join("foo.js")
    ));
    assert!(!resolve_side_effects(
      "side-effects-false-glob/b/index.js",
      &root().join("foo.js")
    ));
    assert!(!resolve_side_effects(
      "side-effects-false-glob/sub/a/index.js",
      &root().join("foo.js")
    ));
    assert!(resolve_side_effects(
      "side-effects-false-glob/sub/index.json",
      &root().join("foo.js")
    ));
  }

  #[test]
  fn test_include_node_modules() {
    let mut resolver = test_resolver();
    resolver.include_node_modules = Cow::Owned(IncludeNodeModules::Bool(false));

    assert_eq!(
      resolver
        .resolve("foo", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::External
    );
    assert_eq!(
      resolver
        .resolve("@scope/pkg", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::External
    );

    resolver.include_node_modules = Cow::Owned(IncludeNodeModules::Array(vec!["foo".into()]));
    assert_eq!(
      resolver
        .resolve("foo", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/foo/index.js"))
    );
    assert_eq!(
      resolver
        .resolve("@scope/pkg", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::External
    );

    resolver.include_node_modules = Cow::Owned(IncludeNodeModules::Map(BTreeMap::from([
      ("foo".into(), false),
      ("@scope/pkg".into(), true),
    ])));
    assert_eq!(
      resolver
        .resolve("foo", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::External
    );
    assert_eq!(
      resolver
        .resolve("@scope/pkg", &root().join("foo.js"), SpecifierType::Esm)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/@scope/pkg/index.js"))
    );
  }

  #[test]
  fn package_deduplication() {
    let cache = CacheCow::Owned(Cache::new(Arc::new(OsFileSystem)));
    cache.scan_package_duplicates(&root()).unwrap();

    let mut resolver = Resolver::node(root().into(), cache);
    resolver.flags.set(Flags::EXPORTS, false);

    assert_eq!(
      resolver
        .resolve(
          "duplicate",
          &root().join("node_modules/duplicate-tester/index.js"),
          SpecifierType::Esm,
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/duplicate/index.js"))
    );

    assert_eq!(
      resolver
        .resolve(
          "duplicate/index.js",
          &root().join("node_modules/duplicate-tester/index.js"),
          SpecifierType::Esm,
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/duplicate/index.js"))
    );

    resolver.flags.set(Flags::EXPORTS, true);

    assert_eq!(
      resolver
        .resolve(
          "duplicate/main",
          &root().join("node_modules/duplicate-tester/index.js"),
          SpecifierType::Esm,
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/duplicate/index.js"))
    );
  }

  #[test]
  fn graphql_esm_upgrade() {
    let mut resolver = test_resolver();
    resolver.flags.set(Flags::GRAPHQL_ESM_UPGRADE, true);

    assert_eq!(
      resolver
        .resolve("graphql", &root().join("foo.js"), SpecifierType::Cjs)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/graphql/index.mjs"))
    );

    assert_eq!(
      resolver
        .resolve("graphql/index", &root().join("foo.js"), SpecifierType::Cjs)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/graphql/index.mjs"))
    );

    assert_eq!(
      resolver
        .resolve("graphql/error", &root().join("foo.js"), SpecifierType::Cjs)
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/graphql/error/index.mjs"))
    );

    assert_eq!(
      resolver
        .resolve(
          "graphql/error/some-error",
          &root().join("foo.js"),
          SpecifierType::Cjs
        )
        .result
        .unwrap()
        .0,
      Resolution::Path(root().join("node_modules/graphql/error/some-error.mjs"))
    );
  }

  // #[test]
  // fn test_visitor() {
  //   let resolved = test_resolver().resolve("unified", &root(), SpecifierType::Esm).unwrap();
  //   println!("{:?}", resolved);
  //   if let Resolution::Path(p) = resolved {
  //     let res = build_esm_graph(
  //       &p,
  //       root()
  //     ).unwrap();
  //     println!("{:?}", res);
  //   }
  // }
}
