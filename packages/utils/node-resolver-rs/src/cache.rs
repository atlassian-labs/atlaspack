use rayon::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::types::File;
use atlaspack_filesystem::{FileSystemRealPathCache, FileSystemRef};
use atlaspack_shared_map::SharedHashMap;

use crate::package_json::PackageJson;
use crate::package_json::SourceField;
use crate::tsconfig::TsConfig;
use crate::tsconfig::TsConfigWrapper;
use crate::ResolverError;

pub struct Cache {
  pub fs: FileSystemRef,
  /// These map paths to parsed config files. They aren't really 'static, but Rust doens't have a good
  /// way to associate a lifetime with owned data stored in the same struct. We only vend temporary references
  /// from our public methods so this is ok for now. FrozenMap is an append only map, which doesn't require &mut
  /// to insert into. Since each value is in a Box, it won't move and therefore references are stable.
  packages: SharedHashMap<PathBuf, Arc<Result<Arc<PackageJson>, ResolverError>>>,
  package_duplicates: SharedHashMap<PathBuf, Arc<PackageJson>>,
  tsconfigs: SharedHashMap<PathBuf, Arc<Result<Arc<TsConfigWrapper>, ResolverError>>>,
  // In particular just the is_dir_cache spends around 8% of the time on a large project resolution
  // hashing paths. Instead of using a hashmap we should try a trie here.
  is_dir_cache: SharedHashMap<PathBuf, bool>,
  is_file_cache: SharedHashMap<PathBuf, bool>,
  realpath_cache: FileSystemRealPathCache,
}

impl fmt::Debug for Cache {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Cache").finish()
  }
}

#[allow(clippy::large_enum_variant)]
/// Special Cow implementation for a Cache that doesn't require Clone.
pub enum CacheCow<'a> {
  Borrowed(&'a Cache),
  Owned(Cache),
}

impl<'a> Deref for CacheCow<'a> {
  type Target = Cache;

  fn deref(&self) -> &Self::Target {
    match self {
      CacheCow::Borrowed(c) => c,
      CacheCow::Owned(c) => c,
    }
  }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct JsonError {
  pub file: File,
  pub line: usize,
  pub column: usize,
  pub message: String,
}

impl JsonError {
  fn new(file: File, err: serde_json::Error) -> JsonError {
    JsonError {
      file,
      line: err.line(),
      column: err.column(),
      message: err.to_string(),
    }
  }

  fn json5(
    file: File,
    serde_json5::Error::Message { msg, location }: serde_json5::Error,
  ) -> JsonError {
    JsonError {
      file,
      line: location.as_ref().map(|l| l.line).unwrap_or(0),
      column: location.as_ref().map(|l| l.column).unwrap_or(0),
      message: msg.to_string(),
    }
  }
}

impl Cache {
  pub fn new(fs: FileSystemRef) -> Self {
    Self {
      fs,
      packages: SharedHashMap::new(),
      package_duplicates: SharedHashMap::new(),
      tsconfigs: SharedHashMap::new(),
      is_file_cache: SharedHashMap::new(),
      is_dir_cache: SharedHashMap::new(),
      realpath_cache: FileSystemRealPathCache::default(),
    }
  }

  pub fn is_file(&self, path: &Path) -> bool {
    if let Some(is_file) = self.is_file_cache.get(path) {
      return is_file;
    }

    let is_file = self.fs.is_file(path);
    self.is_file_cache.insert(path.to_path_buf(), is_file);
    is_file
  }

  pub fn is_dir(&self, path: &Path) -> bool {
    if let Some(is_file) = self.is_dir_cache.get(path) {
      return is_file;
    }

    let is_file = self.fs.is_dir(path);
    self.is_dir_cache.insert(path.to_path_buf(), is_file);
    is_file
  }

  pub fn canonicalize(&self, path: &Path) -> Result<PathBuf, ResolverError> {
    Ok(self.fs.canonicalize(path, &self.realpath_cache)?)
  }

  #[tracing::instrument(level = "info", skip_all)]
  pub fn scan_package_duplicates(&self, root_dir: &Path) {
    let mut package_json_files = find_package_json_files(&root_dir.join("node_modules"));
    package_json_files.sort_by(|a, b| {
      let a_len = a.to_string_lossy().len();
      let b_len = b.to_string_lossy().len();

      if a_len == b_len {
        a.cmp(b)
      } else {
        a_len.cmp(&b_len)
      }
    });
    tracing::info!("Results {:?}", package_json_files.len());

    let mut packages_by_version: HashMap<String, Arc<PackageJson>> = HashMap::new();
    let mut count = 0;
    let packages: Vec<Arc<Result<Arc<PackageJson>, ResolverError>>> = package_json_files
      .par_iter()
      .map(|path| {
        let package = read_and_parse_package(&self.fs, &self.realpath_cache, path);
        let entry = Arc::new(package.map(Arc::new));
        // While we have the package.json we may as well hydrate the cache for
        // later
        self.packages.insert(path.clone(), entry.clone());
        entry
      })
      .collect();

    for entry in packages.iter() {
      if let Ok(package_json) = entry.as_ref() {
        if let Some(version) = package_json.version.clone() {
          let dedupe_key = format!("{}@{}", package_json.name, version);

          if let Some(existing) = packages_by_version.get(&dedupe_key) {
            self
              .package_duplicates
              .insert(package_json.path.clone(), existing.clone());
            count += 1;
          } else {
            packages_by_version.insert(dedupe_key.clone(), package_json.clone());
          }
        }
      }
    }
    tracing::info!("{} packages marked as duplicate", count,);
  }

  pub fn read_package(&self, path: Cow<Path>) -> Arc<Result<Arc<PackageJson>, ResolverError>> {
    if let Some(pkg) = self.package_duplicates.get(path.as_ref()) {
      tracing::info!("Deduplicating package import: {:?} -> {:?}", path, pkg.path);
      return Arc::new(Ok(pkg.clone()));
    }

    if let Some(pkg) = self.packages.get(path.as_ref()) {
      return pkg.clone();
    }

    let path = path.into_owned();
    let package: Result<PackageJson, ResolverError> =
      read_and_parse_package(&self.fs, &self.realpath_cache, &path);

    // Since we have exclusive access to packages,
    let entry = Arc::new(package.map(Arc::new));
    self.packages.insert(path.clone(), entry.clone());

    entry.clone()
  }

  pub fn read_tsconfig<F: FnOnce(&mut TsConfigWrapper) -> Result<(), ResolverError>>(
    &self,
    path: &Path,
    process: F,
  ) -> Arc<Result<Arc<TsConfigWrapper>, ResolverError>> {
    if let Some(tsconfig) = self.tsconfigs.get(path) {
      return tsconfig.clone();
    }

    fn read_tsconfig<F: FnOnce(&mut TsConfigWrapper) -> Result<(), ResolverError>>(
      fs: &FileSystemRef,
      path: &Path,
      process: F,
    ) -> Result<TsConfigWrapper, ResolverError> {
      let data = fs.read_to_string(path)?;
      let mut tsconfig = TsConfig::parse(path.to_owned(), &data).map_err(|e| {
        JsonError::json5(
          File {
            contents: data,
            path: path.to_owned(),
          },
          e,
        )
      })?;
      process(&mut tsconfig)?;
      Ok(tsconfig)
    }

    // Since we have exclusive access to tsconfigs, it should be impossible for the get to fail
    // after insert
    let tsconfig = read_tsconfig(&self.fs, path, process).map(Arc::new);
    let tsconfig = Arc::new(tsconfig);
    self.tsconfigs.insert(PathBuf::from(path), tsconfig.clone());

    tsconfig
  }
}

fn read_and_parse_package<'a>(
  fs: &'a FileSystemRef,
  realpath_cache: &'a FileSystemRealPathCache,
  path: &Path,
) -> Result<PackageJson, ResolverError> {
  let contents: String = fs.read_to_string(path)?;
  let mut pkg = PackageJson::parse(PathBuf::from(path), &contents).map_err(|e| {
    JsonError::new(
      File {
        path: PathBuf::from(path),
        contents,
      },
      e,
    )
  })?;

  // If the package has a `source` field, make sure
  // - the package is behind symlinks
  // - and the realpath to the packages does not includes `node_modules`.
  // Since such package is likely a pre-compiled module
  // installed with package managers, rather than including a source code.
  if !matches!(pkg.source, SourceField::None) {
    let realpath = fs.canonicalize(&pkg.path, realpath_cache)?;
    if realpath == pkg.path
      || realpath
        .components()
        .any(|c| c.as_os_str() == "node_modules")
    {
      pkg.source = SourceField::None;
    }
  }

  Ok(pkg)
}

fn find_package_json_files(base_path: &Path) -> Vec<PathBuf> {
  let mut package_json_files = Vec::new();
  let should_traverse = base_path.file_name().is_some_and(|dir_name| {
    dir_name == "node_modules" || dir_name.to_string_lossy().starts_with("@")
  });

  if let Ok(entries) = fs::read_dir(base_path) {
    // Collect entries to avoid borrowing issues during parallel iteration
    let entries: Vec<_> = entries.filter_map(Result::ok).collect();

    // Use parallel iterator for entries
    let found_files: Vec<PathBuf> = entries
      .par_iter()
      .flat_map(|entry| {
        let path = entry.path();
        if path.is_dir() && (should_traverse || path.ends_with("node_modules")) {
          // If it's a directory, recursively find package.json files in parallel
          find_package_json_files(&path)
        } else if path
          .file_name()
          .is_some_and(|file_name| file_name == "package.json")
        {
          // If it's a package.json file, add it to the list
          vec![path]
        } else {
          vec![]
        }
      })
      .collect();

    package_json_files.extend(found_files);
  }
  package_json_files
}
