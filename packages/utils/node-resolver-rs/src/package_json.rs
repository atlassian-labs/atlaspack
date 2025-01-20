use std::borrow::Cow;
use std::cmp::Ordering;
use std::ops::Range;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use bitflags::bitflags;
use glob_match::glob_match;
use glob_match::glob_match_with_captures;
use indexmap::IndexMap;
use serde::Deserialize;

pub use atlaspack_core::types::ExportsCondition;

use crate::path::resolve_path;
use crate::specifier::decode_path;
use crate::specifier::Specifier;
use crate::specifier::SpecifierType;

bitflags! {
  #[derive(serde::Serialize)]
  pub struct Fields: u8 {
    const MAIN = 1 << 0;
    const MODULE = 1 << 1;
    const SOURCE = 1 << 2;
    const BROWSER = 1 << 3;
    const ALIAS = 1 << 4;
    const TSCONFIG = 1 << 5;
    const TYPES = 1 << 6;
  }
}

#[derive(serde::Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
  #[serde(skip)]
  pub path: PathBuf,
  #[serde(default, deserialize_with = "ok_or_default")]
  pub name: String,
  #[serde(default, deserialize_with = "ok_or_default")]
  pub version: Option<String>,
  #[serde(rename = "type", default, deserialize_with = "ok_or_default")]
  pub module_type: ModuleType,
  #[serde(default, deserialize_with = "ok_or_default")]
  main: Option<String>,
  #[serde(default, deserialize_with = "ok_or_default")]
  module: Option<String>,
  #[serde(default, deserialize_with = "ok_or_default")]
  tsconfig: Option<String>,
  #[serde(default, deserialize_with = "ok_or_default")]
  types: Option<String>,
  #[serde(default, deserialize_with = "ok_or_default")]
  pub source: SourceField,
  #[serde(default, deserialize_with = "ok_or_default")]
  browser: BrowserField,
  #[serde(default, deserialize_with = "ok_or_default")]
  alias: IndexMap<Specifier, AliasValue>,
  #[serde(default, deserialize_with = "ok_or_default")]
  exports: ExportsField,
  #[serde(default, deserialize_with = "ok_or_default")]
  imports: IndexMap<ExportsKey, ExportsField>,
  #[serde(default, deserialize_with = "ok_or_default")]
  side_effects: SideEffects,
}

fn ok_or_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
  T: serde::Deserialize<'de> + Default,
  D: serde::Deserializer<'de>,
{
  Ok(T::deserialize(deserializer).unwrap_or_default())
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ModuleType {
  Module,
  Json,
  #[default]
  #[serde(other)]
  CommonJs,
}

#[derive(serde::Deserialize, Debug, Default)]
#[serde(untagged)]
pub enum BrowserField {
  #[default]
  None,
  String(String),
  Map(IndexMap<Specifier, AliasValue>),
}

#[derive(serde::Deserialize, Debug, Default)]
#[serde(untagged)]
pub enum SourceField {
  #[default]
  None,
  String(String),
  Map(IndexMap<Specifier, AliasValue>),
  Array(Vec<String>),
  Bool(bool),
}

#[derive(serde::Deserialize, Debug, Default, PartialEq)]
#[serde(untagged)]
pub enum ExportsField {
  #[default]
  None,
  String(String),
  Array(Vec<ExportsField>),
  Map(IndexMap<ExportsKey, ExportsField>),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ExportsKey {
  Main,
  Pattern(String),
  Condition(ExportsCondition),
  CustomCondition(String),
}

impl<'a> From<&'a str> for ExportsKey {
  fn from(key: &'a str) -> Self {
    if key == "." {
      ExportsKey::Main
    } else if let Some(key) = key.strip_prefix("./") {
      ExportsKey::Pattern(key.to_string())
    } else if let Some(key) = key.strip_prefix('#') {
      ExportsKey::Pattern(key.to_string())
    } else if let Ok(c) = ExportsCondition::try_from(key) {
      ExportsKey::Condition(c)
    } else {
      ExportsKey::CustomCondition(key.to_string())
    }
  }
}

impl<'de> Deserialize<'de> for ExportsKey {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let s: &'de str = Deserialize::deserialize(deserializer)?;
    Ok(ExportsKey::from(s))
  }
}

#[derive(serde::Deserialize, Clone, PartialEq, Debug)]
#[serde(untagged)]
pub enum AliasValue {
  Specifier(Specifier),
  Bool(bool),
  Global { global: String },
}

#[derive(serde::Deserialize, Clone, Default, PartialEq, Debug)]
#[serde(untagged)]
pub enum SideEffects {
  #[default]
  None,
  Boolean(bool),
  String(String),
  Array(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum PackageJsonError {
  InvalidPackageTarget,
  PackagePathNotExported,
  InvalidSpecifier,
  ImportNotDefined,
}

#[derive(Debug, PartialEq)]
pub enum ExportsResolution<'a> {
  None,
  Path(PathBuf),
  Package(Cow<'a, str>),
}

impl PackageJson {
  pub fn parse(path: PathBuf, data: &str) -> serde_json::Result<PackageJson> {
    let mut parsed: PackageJson = serde_json::from_str(data)?;
    parsed.path = path;
    Ok(parsed)
  }

  pub fn entries(&self, fields: Fields) -> EntryIter {
    EntryIter {
      package: self,
      fields,
    }
  }

  pub fn source(&self) -> Option<PathBuf> {
    match &self.source {
      SourceField::None | SourceField::Array(_) | SourceField::Bool(_) => None,
      SourceField::String(source) => Some(resolve_path(&self.path, source)),
      SourceField::Map(map) => {
        match map.get(&Specifier::Package(self.name.clone(), String::from(""))) {
          Some(AliasValue::Specifier(Specifier::Relative(s))) => Some(resolve_path(&self.path, s)),
          _ => None,
        }
      }
    }
  }

  pub fn has_exports(&self) -> bool {
    self.exports != ExportsField::None
  }

  pub fn resolve_package_exports(
    &self,
    subpath: &str,
    conditions: ExportsCondition,
    custom_conditions: &[String],
  ) -> Result<PathBuf, PackageJsonError> {
    // If exports is an Object with both a key starting with "." and a key not starting with ".", throw an Invalid Package Configuration error.
    if let ExportsField::Map(map) = &self.exports {
      let mut has_conditions = false;
      let mut has_patterns = false;
      for key in map.keys() {
        has_conditions = has_conditions
          || matches!(
            key,
            ExportsKey::Condition(..) | ExportsKey::CustomCondition(..)
          );
        has_patterns = has_patterns || matches!(key, ExportsKey::Pattern(..) | ExportsKey::Main);
        if has_conditions && has_patterns {
          return Err(PackageJsonError::InvalidPackageTarget);
        }
      }
    }

    if subpath.is_empty() {
      let mut main_export = &ExportsField::None;
      match &self.exports {
        ExportsField::None | ExportsField::String(_) | ExportsField::Array(_) => {
          main_export = &self.exports;
        }
        ExportsField::Map(map) => {
          if let Some(v) = map.get(&ExportsKey::Main) {
            main_export = v;
          } else if !map.keys().any(|k| matches!(k, ExportsKey::Pattern(_))) {
            main_export = &self.exports;
          }
        }
      }

      if main_export != &ExportsField::None {
        match self.resolve_package_target(main_export, "", false, conditions, custom_conditions)? {
          ExportsResolution::Path(path) => return Ok(path),
          ExportsResolution::None | ExportsResolution::Package(..) => {}
        }
      }
    } else if let ExportsField::Map(exports) = &self.exports {
      // All exports must start with "." at this point.
      match self.resolve_package_imports_exports(
        subpath,
        exports,
        false,
        conditions,
        custom_conditions,
      )? {
        ExportsResolution::Path(path) => return Ok(path),
        ExportsResolution::None | ExportsResolution::Package(..) => {}
      }
    }

    Err(PackageJsonError::PackagePathNotExported)
  }

  pub fn resolve_package_imports(
    &self,
    specifier: &str,
    conditions: ExportsCondition,
    custom_conditions: &[String],
  ) -> Result<ExportsResolution<'_>, PackageJsonError> {
    if specifier == "#" || specifier.starts_with("#/") {
      return Err(PackageJsonError::InvalidSpecifier);
    }

    match self.resolve_package_imports_exports(
      specifier,
      &self.imports,
      true,
      conditions,
      custom_conditions,
    )? {
      ExportsResolution::None => {}
      res => return Ok(res),
    }

    Err(PackageJsonError::ImportNotDefined)
  }

  fn resolve_package_target(
    &self,
    target: &ExportsField,
    pattern_match: &str,
    is_imports: bool,
    conditions: ExportsCondition,
    custom_conditions: &[String],
  ) -> Result<ExportsResolution<'_>, PackageJsonError> {
    match target {
      ExportsField::String(target) => {
        if !target.starts_with("./") {
          if !is_imports || target.starts_with("../") || target.starts_with('/') {
            return Err(PackageJsonError::InvalidPackageTarget);
          }

          if !pattern_match.is_empty() {
            let target = target.replace('*', pattern_match);
            return Ok(ExportsResolution::Package(Cow::Owned(target)));
          }

          return Ok(ExportsResolution::Package(Cow::Owned(target.clone())));
        }

        let target = if pattern_match.is_empty() {
          Cow::Borrowed(target)
        } else {
          Cow::Owned(target.replace('*', pattern_match))
        };

        // If target split on "/" or "\" contains any "", ".", "..", or "node_modules" segments after
        // the first "." segment, case insensitive and including percent encoded variants,
        // throw an Invalid Package Target error.
        let target_path = decode_path(target.as_ref(), SpecifierType::Esm).0;
        if target_path
          .components()
          .enumerate()
          .any(|(index, c)| match c {
            Component::ParentDir => true,
            Component::CurDir => index > 0,
            Component::Normal(c) => c.eq_ignore_ascii_case("node_modules"),
            _ => false,
          })
        {
          return Err(PackageJsonError::InvalidPackageTarget);
        }

        let resolved_target = resolve_path(&self.path, &target_path);
        return Ok(ExportsResolution::Path(resolved_target));
      }
      ExportsField::Map(target) => {
        // We must iterate in object insertion order.
        for (key, value) in target {
          let matches = match key {
            ExportsKey::Condition(key) => {
              *key == ExportsCondition::DEFAULT || conditions.contains(*key)
            }
            ExportsKey::CustomCondition(key) => custom_conditions.iter().any(|k| k == key),
            _ => false,
          };
          if matches {
            match self.resolve_package_target(
              value,
              pattern_match,
              is_imports,
              conditions,
              custom_conditions,
            )? {
              ExportsResolution::None => continue,
              res => return Ok(res),
            }
          }
        }
      }
      ExportsField::Array(target) => {
        if target.is_empty() {
          return Err(PackageJsonError::PackagePathNotExported);
        }

        for item in target {
          match self.resolve_package_target(
            item,
            pattern_match,
            is_imports,
            conditions,
            custom_conditions,
          ) {
            Err(_) | Ok(ExportsResolution::None) => continue,
            Ok(res) => return Ok(res),
          }
        }
      }
      ExportsField::None => return Ok(ExportsResolution::None),
    }

    Ok(ExportsResolution::None)
  }

  fn resolve_package_imports_exports(
    &self,
    match_key: &str,
    match_obj: &IndexMap<ExportsKey, ExportsField>,
    is_imports: bool,
    conditions: ExportsCondition,
    custom_conditions: &[String],
  ) -> Result<ExportsResolution<'_>, PackageJsonError> {
    let pattern = ExportsKey::Pattern(match_key.to_string());
    if let Some(target) = match_obj.get(&pattern) {
      if !match_key.contains('*') {
        return self.resolve_package_target(target, "", is_imports, conditions, custom_conditions);
      }
    }

    let mut best_key = "";
    let mut best_match = "";
    for key in match_obj.keys() {
      if let ExportsKey::Pattern(key) = key {
        if let Some((pattern_base, pattern_trailer)) = key.split_once('*') {
          if match_key.starts_with(pattern_base)
            && !pattern_trailer.contains('*')
            && (pattern_trailer.is_empty()
              || (match_key.len() >= key.len() && match_key.ends_with(pattern_trailer)))
            && pattern_key_compare(best_key, key) == Ordering::Greater
          {
            best_key = key;
            best_match = &match_key[pattern_base.len()..match_key.len() - pattern_trailer.len()];
          }
        }
      }
    }

    if !best_key.is_empty() {
      return self.resolve_package_target(
        &match_obj[&ExportsKey::Pattern(best_key.to_string())],
        best_match,
        is_imports,
        conditions,
        custom_conditions,
      );
    }

    Ok(ExportsResolution::None)
  }

  pub fn resolve_aliases<'b>(
    &'b self,
    specifier: &'b Specifier,
    fields: Fields,
  ) -> Option<Cow<'b, AliasValue>> {
    if fields.contains(Fields::SOURCE) {
      if let SourceField::Map(source) = &self.source {
        match self.resolve_alias(source, specifier) {
          None => {}
          res => return res,
        }
      }
    }

    if fields.contains(Fields::ALIAS) {
      match self.resolve_alias(&self.alias, specifier) {
        None => {}
        res => return res,
      }
    }

    if fields.contains(Fields::BROWSER) {
      if let BrowserField::Map(browser) = &self.browser {
        match self.resolve_alias(browser, specifier) {
          None => {}
          res => return res,
        }
      }
    }

    None
  }

  fn resolve_alias<'b>(
    &'b self,
    map: &'b IndexMap<Specifier, AliasValue>,
    specifier: &'b Specifier,
  ) -> Option<Cow<'b, AliasValue>> {
    if let Some(alias) = self.lookup_alias(map, specifier) {
      return Some(alias);
    }

    if let Specifier::Package(package, subpath) = specifier {
      if let Some(alias) =
        self.lookup_alias(map, &Specifier::Package(package.clone(), String::from("")))
      {
        match alias.as_ref() {
          AliasValue::Specifier(base) => {
            // Join the subpath back onto the resolved alias.
            match base {
              Specifier::Package(base_pkg, base_subpath) => {
                let subpath = if !base_subpath.is_empty() && !subpath.is_empty() {
                  format!("{}/{}", base_subpath, subpath)
                } else if !subpath.is_empty() {
                  subpath.clone()
                } else {
                  return Some(alias);
                };
                return Some(Cow::Owned(AliasValue::Specifier(Specifier::Package(
                  base_pkg.clone(),
                  subpath,
                ))));
              }
              Specifier::Relative(path) => {
                if subpath.is_empty() {
                  return Some(alias);
                } else {
                  return Some(Cow::Owned(AliasValue::Specifier(Specifier::Relative(
                    path.join(subpath),
                  ))));
                }
              }
              Specifier::Absolute(path) => {
                if subpath.is_empty() {
                  return Some(alias);
                } else {
                  return Some(Cow::Owned(AliasValue::Specifier(Specifier::Absolute(
                    path.join(subpath),
                  ))));
                }
              }
              Specifier::Tilde(path) => {
                if subpath.is_empty() {
                  return Some(alias);
                } else {
                  return Some(Cow::Owned(AliasValue::Specifier(Specifier::Tilde(
                    path.join(subpath),
                  ))));
                }
              }
              _ => return Some(alias),
            }
          }
          _ => return Some(alias),
        };
      }
    }

    None
  }

  fn lookup_alias<'b>(
    &self,
    map: &'b IndexMap<Specifier, AliasValue>,
    specifier: &Specifier,
  ) -> Option<Cow<'b, AliasValue>> {
    if let Some(value) = map.get(specifier) {
      return Some(Cow::Borrowed(value));
    }

    // Match glob aliases.
    for (key, value) in map {
      let (glob, path) = match (key, specifier) {
        (Specifier::Relative(glob), Specifier::Relative(path))
        | (Specifier::Absolute(glob), Specifier::Absolute(path))
        | (Specifier::Tilde(glob), Specifier::Tilde(path)) => (
          glob.as_os_str().to_string_lossy(),
          path.as_os_str().to_string_lossy(),
        ),
        (Specifier::Package(module_a, glob), Specifier::Package(module_b, path))
          if module_a == module_b =>
        {
          (Cow::Borrowed(glob.as_ref()), Cow::Borrowed(path.as_ref()))
        }
        (pkg_a @ Specifier::Package(..), pkg_b @ Specifier::Package(..)) => {
          // Glob could be in the package name, e.g. "@internal/*"
          (pkg_a.to_string(), pkg_b.to_string())
        }
        _ => continue,
      };

      if let Some(captures) = glob_match_with_captures(&glob, &path) {
        let res = match value {
          AliasValue::Specifier(specifier) => AliasValue::Specifier(match specifier {
            Specifier::Relative(r) => {
              Specifier::Relative(replace_path_captures(r, &path, &captures)?)
            }
            Specifier::Absolute(r) => {
              Specifier::Absolute(replace_path_captures(r, &path, &captures)?)
            }
            Specifier::Tilde(r) => Specifier::Tilde(replace_path_captures(r, &path, &captures)?),
            Specifier::Package(module, subpath) => Specifier::Package(
              module.clone(),
              replace_captures(subpath, &path, &captures).to_string(),
            ),
            _ => return Some(Cow::Borrowed(value)),
          }),
          _ => return Some(Cow::Borrowed(value)),
        };

        return Some(Cow::Owned(res));
      }
    }

    None
  }

  pub fn has_side_effects(&self, path: &Path) -> bool {
    let path = path
      .strip_prefix(self.path.parent().unwrap())
      .ok()
      .and_then(|path| path.as_os_str().to_str());

    let path = match path {
      Some(p) => p,
      None => return true,
    };

    fn side_effects_glob_matches(glob: &str, path: &str) -> bool {
      // Trim leading "./"
      let glob = glob.strip_prefix("./").unwrap_or(glob);

      // If the glob does not contain any '/' characters, prefix with "**/" to match webpack.
      let glob = if !glob.contains('/') {
        Cow::Owned(format!("**/{}", glob))
      } else {
        Cow::Borrowed(glob)
      };

      glob_match(glob.as_ref(), path)
    }

    match &self.side_effects {
      SideEffects::None => true,
      SideEffects::Boolean(b) => *b,
      SideEffects::String(glob) => side_effects_glob_matches(glob, path),
      SideEffects::Array(globs) => globs
        .iter()
        .any(|glob| side_effects_glob_matches(glob, path)),
    }
  }
}

fn replace_path_captures(s: &Path, path: &str, captures: &Vec<Range<usize>>) -> Option<PathBuf> {
  Some(PathBuf::from(replace_captures(
    s.as_os_str().to_str()?,
    path,
    captures,
  )))
}

/// Inserts captures matched in a glob against `path` using a pattern string.
/// Replacements are inserted using JS-like $N syntax, e.g. $1 for the first capture.
#[allow(clippy::ptr_arg)]
fn replace_captures(s: &str, path: &str, captures: &Vec<Range<usize>>) -> String {
  let mut res = s.to_string();
  let bytes = s.as_bytes();
  for (idx, _) in s.match_indices('$').rev() {
    let mut end = idx;
    while end + 1 < bytes.len() && bytes[end + 1].is_ascii_digit() {
      end += 1;
    }

    if end != idx {
      if let Ok(capture_index) = s[idx + 1..end + 1].parse::<usize>() {
        if capture_index > 0 && capture_index - 1 < captures.len() {
          res.replace_range(idx..end + 1, &path[captures[capture_index - 1].clone()]);
        }
      }
    }
  }

  res
}

fn pattern_key_compare(a: &str, b: &str) -> Ordering {
  let a_pos = a.chars().position(|c| c == '*');
  let b_pos = b.chars().position(|c| c == '*');
  let base_length_a = a_pos.map_or(a.len(), |p| p + 1);
  let base_length_b = b_pos.map_or(b.len(), |p| p + 1);
  let cmp = base_length_b.cmp(&base_length_a);
  if cmp != Ordering::Equal {
    return cmp;
  }

  if a_pos.is_none() {
    return Ordering::Greater;
  }

  if b_pos.is_none() {
    return Ordering::Less;
  }

  b.len().cmp(&a.len())
}

pub struct EntryIter<'a> {
  package: &'a PackageJson,
  fields: Fields,
}

impl Iterator for EntryIter<'_> {
  type Item = (PathBuf, &'static str);

  fn next(&mut self) -> Option<Self::Item> {
    if self.fields.contains(Fields::SOURCE) {
      self.fields.remove(Fields::SOURCE);
      if let Some(source) = self.package.source() {
        return Some((source, "source"));
      }
    }

    if self.fields.contains(Fields::TYPES) {
      self.fields.remove(Fields::TYPES);
      if let Some(types) = &self.package.types {
        return Some((resolve_path(&self.package.path, types), "types"));
      }
    }

    if self.fields.contains(Fields::BROWSER) {
      self.fields.remove(Fields::BROWSER);
      match &self.package.browser {
        BrowserField::None => {}
        BrowserField::String(browser) => {
          return Some((resolve_path(&self.package.path, browser), "browser"))
        }
        BrowserField::Map(map) => {
          if let Some(AliasValue::Specifier(Specifier::Relative(s))) = map.get(&Specifier::Package(
            self.package.name.clone(),
            String::from(""),
          )) {
            return Some((resolve_path(&self.package.path, s), "browser"));
          }
        }
      }
    }

    if self.fields.contains(Fields::MODULE) {
      self.fields.remove(Fields::MODULE);
      if let Some(module) = &self.package.module {
        return Some((resolve_path(&self.package.path, module), "module"));
      }
    }

    if self.fields.contains(Fields::MAIN) {
      self.fields.remove(Fields::MAIN);
      if let Some(main) = &self.package.main {
        return Some((resolve_path(&self.package.path, main), "main"));
      }
    }

    if self.fields.contains(Fields::TSCONFIG) {
      self.fields.remove(Fields::TSCONFIG);
      if let Some(tsconfig) = &self.package.tsconfig {
        return Some((resolve_path(&self.package.path, tsconfig), "tsconfig"));
      }
    }

    None
  }
}

#[cfg(test)]
mod tests {
  use indexmap::indexmap;

  use super::*;

  // Based on https://github.com/lukeed/resolve.exports/blob/master/test/resolve.js,
  // https://github.com/privatenumber/resolve-pkg-maps/tree/develop/tests, and
  // https://github.com/webpack/enhanced-resolve/blob/main/test/exportsField.js

  #[test]
  fn exports_string() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::String(String::from("./exports.js")),
      ..PackageJson::default()
    };

    assert_eq!(
      pkg
        .resolve_package_exports("", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/exports.js")
    );
    // assert_eq!(pkg.resolve_package_exports("./exports.js", &[]).unwrap(), PathBuf::from("/foo/exports.js"));
    // assert_eq!(pkg.resolve_package_exports("foobar", &[]).unwrap(), PathBuf::from("/foo/exports.js"));
  }

  #[test]
  fn exports_dot() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Map(indexmap! {
        ".".into() => ExportsField::String(String::from("./exports.js"))
      }),
      ..PackageJson::default()
    };

    assert_eq!(
      pkg
        .resolve_package_exports("", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/exports.js")
    );
    assert!(matches!(
      pkg.resolve_package_exports(".", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::PackagePathNotExported)
    ));
    // assert_eq!(pkg.resolve_package_exports("foobar", &[]).unwrap(), PathBuf::from("/foo/exports.js"));
  }

  #[test]
  fn exports_dot_conditions() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Map(indexmap! {
        ".".into() => ExportsField::Map(indexmap! {
          "import".into() => ExportsField::String(String::from("./import.js")),
          "require".into() => ExportsField::String(String::from("./require.js"))
        })
      }),
      ..PackageJson::default()
    };

    assert_eq!(
      pkg
        .resolve_package_exports(
          "",
          ExportsCondition::IMPORT | ExportsCondition::REQUIRE,
          &[]
        )
        .unwrap(),
      PathBuf::from("/foo/import.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("", ExportsCondition::REQUIRE, &[])
        .unwrap(),
      PathBuf::from("/foo/require.js")
    );
    assert!(matches!(
      pkg.resolve_package_exports("", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::PackagePathNotExported)
    ));
    assert!(matches!(
      pkg.resolve_package_exports("", ExportsCondition::NODE, &[]),
      Err(PackageJsonError::PackagePathNotExported)
    ));
  }

  #[test]
  fn exports_map_string() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Map(indexmap! {
        "./foo".into() => ExportsField::String(String::from("./exports.js")),
        "./.invisible".into() => ExportsField::String(String::from("./.invisible.js")),
        "./".into() => ExportsField::String(String::from("./")),
        "./*".into() => ExportsField::String(String::from("./*.js"))
      }),
      ..PackageJson::default()
    };

    assert_eq!(
      pkg
        .resolve_package_exports("foo", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/exports.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports(".invisible", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/.invisible.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("file", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/file.js")
    );
  }

  #[test]
  fn exports_map_conditions() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Map(indexmap! {
        "./foo".into() => ExportsField::Map(indexmap! {
          "import".into() => ExportsField::String(String::from("./import.js")),
          "require".into() => ExportsField::String(String::from("./require.js"))
        })
      }),
      ..PackageJson::default()
    };

    assert_eq!(
      pkg
        .resolve_package_exports(
          "foo",
          ExportsCondition::IMPORT | ExportsCondition::REQUIRE,
          &[]
        )
        .unwrap(),
      PathBuf::from("/foo/import.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("foo", ExportsCondition::REQUIRE, &[])
        .unwrap(),
      PathBuf::from("/foo/require.js")
    );
    assert!(matches!(
      pkg.resolve_package_exports("foo", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::PackagePathNotExported)
    ));
    assert!(matches!(
      pkg.resolve_package_exports("foo", ExportsCondition::NODE, &[]),
      Err(PackageJsonError::PackagePathNotExported)
    ));
  }

  #[test]
  fn nested_conditions() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Map(indexmap! {
        "node".into() => ExportsField::Map(indexmap! {
          "import".into() => ExportsField::String(String::from("./import.js")),
          "require".into() => ExportsField::String(String::from("./require.js"))
        }),
        "default".into() => ExportsField::String(String::from("./default.js"))
      }),
      ..PackageJson::default()
    };

    assert_eq!(
      pkg
        .resolve_package_exports("", ExportsCondition::NODE | ExportsCondition::IMPORT, &[])
        .unwrap(),
      PathBuf::from("/foo/import.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("", ExportsCondition::NODE | ExportsCondition::REQUIRE, &[])
        .unwrap(),
      PathBuf::from("/foo/require.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("", ExportsCondition::IMPORT, &[])
        .unwrap(),
      PathBuf::from("/foo/default.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/default.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("", ExportsCondition::NODE, &[])
        .unwrap(),
      PathBuf::from("/foo/default.js")
    );
  }

  #[test]
  fn custom_conditions() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Map(indexmap! {
        "custom".into() => ExportsField::String(String::from("./custom.js")),
        "default".into() => ExportsField::String(String::from("./default.js"))
      }),
      ..PackageJson::default()
    };
    assert_eq!(
      pkg
        .resolve_package_exports("", ExportsCondition::NODE, &["custom".into()])
        .unwrap(),
      PathBuf::from("/foo/custom.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("", ExportsCondition::NODE, &[])
        .unwrap(),
      PathBuf::from("/foo/default.js")
    );
  }

  #[test]
  fn subpath_nested_conditions() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Map(indexmap! {
        "./lite".into() => ExportsField::Map(indexmap! {
          "node".into() => ExportsField::Map(indexmap! {
            "import".into() => ExportsField::String(String::from("./node_import.js")),
            "require".into() => ExportsField::String(String::from("./node_require.js"))
          }),
          "browser".into() => ExportsField::Map(indexmap! {
            "import".into() => ExportsField::String(String::from("./browser_import.js")),
            "require".into() => ExportsField::String(String::from("./browser_require.js"))
          }),
        })
      }),
      ..PackageJson::default()
    };

    assert_eq!(
      pkg
        .resolve_package_exports(
          "lite",
          ExportsCondition::NODE | ExportsCondition::IMPORT,
          &[]
        )
        .unwrap(),
      PathBuf::from("/foo/node_import.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports(
          "lite",
          ExportsCondition::NODE | ExportsCondition::REQUIRE,
          &[]
        )
        .unwrap(),
      PathBuf::from("/foo/node_require.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports(
          "lite",
          ExportsCondition::BROWSER | ExportsCondition::IMPORT,
          &[]
        )
        .unwrap(),
      PathBuf::from("/foo/browser_import.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports(
          "lite",
          ExportsCondition::BROWSER | ExportsCondition::REQUIRE,
          &[]
        )
        .unwrap(),
      PathBuf::from("/foo/browser_require.js")
    );
    assert!(matches!(
      pkg.resolve_package_exports("lite", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::PackagePathNotExported)
    ));
  }

  #[test]
  fn subpath_star() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Map(indexmap! {
        "./*".into() => ExportsField::String(String::from("./cheese/*.mjs")),
        "./pizza/*".into() => ExportsField::String(String::from("./pizza/*.mjs")),
        "./burritos/*".into() => ExportsField::String(String::from("./burritos/*/*.mjs")),
        "./literal".into() => ExportsField::String(String::from("./literal/*.js")),
      }),
      ..PackageJson::default()
    };

    assert_eq!(
      pkg
        .resolve_package_exports("hello", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/cheese/hello.mjs")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("hello/world", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/cheese/hello/world.mjs")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("hello.js", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/cheese/hello.js.mjs")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("pizza/test", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/pizza/test.mjs")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("burritos/test", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/burritos/test/test.mjs")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("literal", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/literal/*.js")
    );

    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Map(indexmap! {
        "./*".into() => ExportsField::String(String::from("./*.js")),
        "./*.js".into() => ExportsField::None,
        "./internal/*".into() => ExportsField::None,
      }),
      ..PackageJson::default()
    };
    assert_eq!(
      pkg
        .resolve_package_exports("file", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/file.js")
    );
    assert!(matches!(
      pkg.resolve_package_exports("file.js", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::PackagePathNotExported)
    ));
    assert!(matches!(
      pkg.resolve_package_exports("internal/file", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::PackagePathNotExported)
    ));
  }

  #[test]
  fn exports_null() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Map(indexmap! {
        "./features/*.js".into() => ExportsField::String(String::from("./src/features/*.js")),
        "./features/private-internal/*".into() => ExportsField::None,
      }),
      ..PackageJson::default()
    };

    assert_eq!(
      pkg
        .resolve_package_exports("features/foo.js", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/src/features/foo.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("features/foo/bar.js", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/src/features/foo/bar.js")
    );
    assert!(matches!(
      pkg.resolve_package_exports(
        "features/private-internal/foo.js",
        ExportsCondition::empty(),
        &[]
      ),
      Err(PackageJsonError::PackagePathNotExported)
    ),);
  }

  #[test]
  fn exports_array() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Map(indexmap! {
        "./utils/*".into() => ExportsField::Map(indexmap! {
          "browser".into() => ExportsField::Map(indexmap! {
            "worklet".into() => ExportsField::Array(vec![ExportsField::String(String::from("./*")), ExportsField::String(String::from("./node/*"))]),
            "default".into() => ExportsField::Map(indexmap! {
              "node".into() => ExportsField::String(String::from("./node/*"))
            })
          })
        }),
        "./test/*".into() => ExportsField::Array(vec![ExportsField::String(String::from("lodash/*")), ExportsField::String(String::from("./bar/*"))]),
        "./file".into() => ExportsField::Array(vec![ExportsField::String(String::from("http://a.com")), ExportsField::String(String::from("./file.js"))])
      }),
      ..PackageJson::default()
    };

    assert_eq!(
      pkg
        .resolve_package_exports(
          "utils/index.js",
          ExportsCondition::BROWSER | ExportsCondition::WORKLET,
          &[]
        )
        .unwrap(),
      PathBuf::from("/foo/index.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports(
          "utils/index.js",
          ExportsCondition::BROWSER | ExportsCondition::NODE,
          &[]
        )
        .unwrap(),
      PathBuf::from("/foo/node/index.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("test/index.js", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/bar/index.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("file", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/file.js")
    );
    assert!(matches!(
      pkg.resolve_package_exports("utils/index.js", ExportsCondition::BROWSER, &[]),
      Err(PackageJsonError::PackagePathNotExported)
    ));
    assert!(matches!(
      pkg.resolve_package_exports("dir/file.js", ExportsCondition::BROWSER, &[]),
      Err(PackageJsonError::PackagePathNotExported)
    ));

    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Array(vec![
        ExportsField::Map(indexmap! {
          "node".into() => ExportsField::String(String::from("./a.js"))
        }),
        ExportsField::String(String::from("./b.js")),
      ]),
      ..PackageJson::default()
    };

    assert_eq!(
      pkg
        .resolve_package_exports("", ExportsCondition::empty(), &[])
        .unwrap(),
      PathBuf::from("/foo/b.js")
    );
    assert_eq!(
      pkg
        .resolve_package_exports("", ExportsCondition::NODE, &[])
        .unwrap(),
      PathBuf::from("/foo/a.js")
    );
  }

  #[test]
  fn exports_invalid() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Map(indexmap! {
        "./invalid".into() => ExportsField::String(String::from("../invalid")),
        "./absolute".into() => ExportsField::String(String::from("/absolute")),
        "./package".into() => ExportsField::String(String::from("package")),
        "./utils/index".into() => ExportsField::String(String::from("./src/../index.js")),
        "./dist/*".into() => ExportsField::String(String::from("./src/../../*")),
        "./modules/*".into() => ExportsField::String(String::from("./node_modules/*")),
        "./modules2/*".into() => ExportsField::String(String::from("./NODE_MODULES/*")),
        "./*/*".into() => ExportsField::String(String::from("./file.js"))
      }),
      ..PackageJson::default()
    };

    assert!(matches!(
      pkg.resolve_package_exports("invalid", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::InvalidPackageTarget)
    ));
    assert!(matches!(
      pkg.resolve_package_exports("absolute", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::InvalidPackageTarget)
    ));
    assert!(matches!(
      pkg.resolve_package_exports("package", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::InvalidPackageTarget)
    ));
    assert!(matches!(
      pkg.resolve_package_exports("utils/index", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::InvalidPackageTarget)
    ));
    assert!(matches!(
      pkg.resolve_package_exports("dist/foo", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::InvalidPackageTarget)
    ));
    assert!(matches!(
      pkg.resolve_package_exports("modules/foo", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::InvalidPackageTarget)
    ));
    assert!(matches!(
      pkg.resolve_package_exports("a/b", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::PackagePathNotExported)
    ));
    assert!(matches!(
      pkg.resolve_package_exports("a/*", ExportsCondition::empty(), &[]),
      Err(PackageJsonError::PackagePathNotExported)
    ));

    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      exports: ExportsField::Map(indexmap! {
        ".".into() => ExportsField::String(String::from("./foo.js")),
        "node".into() => ExportsField::String(String::from("./bar.js")),
      }),
      ..PackageJson::default()
    };

    assert!(matches!(
      pkg.resolve_package_exports("", ExportsCondition::NODE, &[]),
      Err(PackageJsonError::InvalidPackageTarget)
    ));
    assert!(matches!(
      pkg.resolve_package_exports("", ExportsCondition::NODE, &[]),
      Err(PackageJsonError::InvalidPackageTarget)
    ));
  }

  #[test]
  fn imports() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      imports: indexmap! {
        "#foo".into() => ExportsField::String(String::from("./foo.mjs")),
        "#internal/*".into() => ExportsField::String(String::from("./src/internal/*.mjs")),
        "#bar".into() => ExportsField::String(String::from("bar")),
      },
      ..PackageJson::default()
    };

    assert_eq!(
      pkg
        .resolve_package_imports("foo", ExportsCondition::empty(), &[])
        .unwrap(),
      ExportsResolution::Path(PathBuf::from("/foo/foo.mjs"))
    );
    assert_eq!(
      pkg
        .resolve_package_imports("internal/foo", ExportsCondition::empty(), &[])
        .unwrap(),
      ExportsResolution::Path(PathBuf::from("/foo/src/internal/foo.mjs"))
    );
    assert_eq!(
      pkg
        .resolve_package_imports("bar", ExportsCondition::empty(), &[])
        .unwrap(),
      ExportsResolution::Package("bar".into())
    );
  }

  #[test]
  fn import_conditions() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      imports: indexmap! {
        "#entry/*".into() => ExportsField::Map(indexmap! {
          "node".into() => ExportsField::String(String::from("./node/*.js")),
          "browser".into() => ExportsField::String(String::from("./browser/*.js"))
        })
      },
      ..PackageJson::default()
    };
    assert_eq!(
      pkg
        .resolve_package_imports("entry/foo", ExportsCondition::NODE, &[])
        .unwrap(),
      ExportsResolution::Path(PathBuf::from("/foo/node/foo.js"))
    );
    assert_eq!(
      pkg
        .resolve_package_imports("entry/foo", ExportsCondition::BROWSER, &[])
        .unwrap(),
      ExportsResolution::Path(PathBuf::from("/foo/browser/foo.js"))
    );
    assert_eq!(
      pkg
        .resolve_package_imports(
          "entry/foo",
          ExportsCondition::NODE | ExportsCondition::BROWSER,
          &[]
        )
        .unwrap(),
      ExportsResolution::Path(PathBuf::from("/foo/node/foo.js"))
    );
  }

  #[test]
  fn aliases() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      alias: indexmap! {
        "./foo.js".into() => AliasValue::Specifier("./foo-alias.js".into()),
        "bar".into()  => AliasValue::Specifier("./bar-alias.js".into()),
        "lodash".into()  => AliasValue::Specifier("my-lodash".into()),
        "lodash/clone".into()  => AliasValue::Specifier("./clone.js".into()),
        "test".into() => AliasValue::Specifier("./test".into()),
        "foo/*".into() => AliasValue::Specifier("bar/$1".into()),
        "./foo/src/**".into() => AliasValue::Specifier("./foo/lib/$1".into()),
        "/foo/src/**".into() => AliasValue::Specifier("/foo/lib/$1".into()),
        "~/foo/src/**".into() => AliasValue::Specifier("~/foo/lib/$1".into()),
        "url".into() => AliasValue::Bool(false),
        "@internal/**".into() => AliasValue::Specifier("./internal/$1".into()),
        "@foo/*/bar/*".into() => AliasValue::Specifier("./test/$1/$2".into()),
      },
      ..PackageJson::default()
    };

    assert_eq!(
      pkg.resolve_aliases(&"./foo.js".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier("./foo-alias.js".into())))
    );
    assert_eq!(
      pkg.resolve_aliases(&"bar".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier("./bar-alias.js".into())))
    );
    assert_eq!(
      pkg.resolve_aliases(&"lodash".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier("my-lodash".into())))
    );
    assert_eq!(
      pkg.resolve_aliases(&"lodash/foo".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier("my-lodash/foo".into())))
    );
    assert_eq!(
      pkg.resolve_aliases(&"lodash/clone".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier("./clone.js".into())))
    );
    assert_eq!(
      pkg.resolve_aliases(&"test".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier("./test".into())))
    );
    assert_eq!(
      pkg.resolve_aliases(&"test/foo".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier("./test/foo".into())))
    );
    assert_eq!(
      pkg.resolve_aliases(&"foo/hi".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier("bar/hi".into())))
    );
    assert_eq!(
      pkg.resolve_aliases(&"./foo/src/a/b".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier("./foo/lib/a/b".into())))
    );
    assert_eq!(
      pkg.resolve_aliases(&"/foo/src/a/b".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier("/foo/lib/a/b".into())))
    );
    assert_eq!(
      pkg.resolve_aliases(&"~/foo/src/a/b".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier("~/foo/lib/a/b".into())))
    );
    assert_eq!(
      pkg.resolve_aliases(&"url".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Bool(false)))
    );
    assert_eq!(
      pkg.resolve_aliases(&"@internal/foo".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier("./internal/foo".into())))
    );
    assert_eq!(
      pkg.resolve_aliases(&"@internal/foo/bar".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier(
        "./internal/foo/bar".into()
      )))
    );
    assert_eq!(
      pkg.resolve_aliases(&"@foo/a/bar/b".into(), Fields::ALIAS),
      Some(Cow::Owned(AliasValue::Specifier("./test/a/b".into())))
    );
  }

  #[allow(clippy::single_range_in_vec_init)]
  #[test]
  fn test_replace_captures() {
    assert_eq!(
      replace_captures("test/$1/$2", "foo/bar/baz", &vec![4..7, 8..11]),
      Cow::Borrowed("test/bar/baz")
    );
    assert_eq!(
      replace_captures("test/$1/$2", "foo/bar/baz", &vec![4..7]),
      Cow::Borrowed("test/bar/$2")
    );
    assert_eq!(
      replace_captures("test/$1/$2/$3", "foo/bar/baz", &vec![4..7, 8..11]),
      Cow::Borrowed("test/bar/baz/$3")
    );
    assert_eq!(
      replace_captures("test/$1/$2/$", "foo/bar/baz", &vec![4..7, 8..11]),
      Cow::Borrowed("test/bar/baz/$")
    );
    assert_eq!(
      replace_captures("te$st/$1/$2", "foo/bar/baz", &vec![4..7, 8..11]),
      Cow::Borrowed("te$st/bar/baz")
    );
  }

  #[test]
  fn side_effects_none() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      ..PackageJson::default()
    };

    assert!(pkg.has_side_effects(Path::new("/foo/index.js")));
    assert!(pkg.has_side_effects(Path::new("/foo/bar/index.js")));
    assert!(pkg.has_side_effects(Path::new("/index.js")));
  }

  #[test]
  fn side_effects_bool() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      side_effects: SideEffects::Boolean(false),
      ..PackageJson::default()
    };

    assert!(!pkg.has_side_effects(Path::new("/foo/index.js")));
    assert!(!pkg.has_side_effects(Path::new("/foo/bar/index.js")));
    assert!(pkg.has_side_effects(Path::new("/index.js")));

    let pkg = PackageJson {
      side_effects: SideEffects::Boolean(true),
      ..pkg
    };

    assert!(pkg.has_side_effects(Path::new("/foo/index.js")));
    assert!(pkg.has_side_effects(Path::new("/foo/bar/index.js")));
    assert!(pkg.has_side_effects(Path::new("/index.js")));
  }

  #[test]
  fn side_effects_glob() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      side_effects: SideEffects::String(String::from("*.css")),
      ..PackageJson::default()
    };

    assert!(pkg.has_side_effects(Path::new("/foo/a.css")));
    assert!(pkg.has_side_effects(Path::new("/foo/bar/baz.css")));
    assert!(pkg.has_side_effects(Path::new("/foo/bar/x/baz.css")));
    assert!(!pkg.has_side_effects(Path::new("/foo/a.js")));
    assert!(!pkg.has_side_effects(Path::new("/foo/bar/baz.js")));
    assert!(pkg.has_side_effects(Path::new("/index.js")));

    let pkg = PackageJson {
      side_effects: SideEffects::String(String::from("bar/*.css")),
      ..pkg
    };

    assert!(!pkg.has_side_effects(Path::new("/foo/a.css")));
    assert!(pkg.has_side_effects(Path::new("/foo/bar/baz.css")));
    assert!(!pkg.has_side_effects(Path::new("/foo/bar/x/baz.css")));
    assert!(!pkg.has_side_effects(Path::new("/foo/a.js")));
    assert!(!pkg.has_side_effects(Path::new("/foo/bar/baz.js")));
    assert!(pkg.has_side_effects(Path::new("/index.js")));

    let pkg = PackageJson {
      side_effects: SideEffects::String(String::from("./bar/*.css")),
      ..pkg
    };

    assert!(!pkg.has_side_effects(Path::new("/foo/a.css")));
    assert!(pkg.has_side_effects(Path::new("/foo/bar/baz.css")));
    assert!(!pkg.has_side_effects(Path::new("/foo/bar/x/baz.css")));
    assert!(!pkg.has_side_effects(Path::new("/foo/a.js")));
    assert!(!pkg.has_side_effects(Path::new("/foo/bar/baz.js")));
    assert!(pkg.has_side_effects(Path::new("/index.js")));
  }

  #[test]
  fn side_effects_array() {
    let pkg = PackageJson {
      path: "/foo/package.json".into(),
      name: String::from("foobar"),
      side_effects: SideEffects::Array(vec![String::from("*.css"), String::from("*.html")]),
      ..PackageJson::default()
    };

    assert!(pkg.has_side_effects(Path::new("/foo/a.css")));
    assert!(pkg.has_side_effects(Path::new("/foo/bar/baz.css")));
    assert!(pkg.has_side_effects(Path::new("/foo/bar/x/baz.css")));
    assert!(pkg.has_side_effects(Path::new("/foo/a.html")));
    assert!(pkg.has_side_effects(Path::new("/foo/bar/baz.html")));
    assert!(pkg.has_side_effects(Path::new("/foo/bar/x/baz.html")));
    assert!(!pkg.has_side_effects(Path::new("/foo/a.js")));
    assert!(!pkg.has_side_effects(Path::new("/foo/bar/baz.js")));
    assert!(pkg.has_side_effects(Path::new("/index.js")));
  }

  #[test]
  fn parsing() {
    let pkg: PackageJson = serde_json::from_str(r#"{"type":"script"}"#).unwrap();
    assert_eq!(pkg.module_type, ModuleType::CommonJs);
    let pkg: PackageJson = serde_json::from_str(r#"{"name":"foo"}"#).unwrap();
    assert_eq!(pkg.module_type, ModuleType::CommonJs);
    let pkg: PackageJson = serde_json::from_str(r#"{"main":false}"#).unwrap();
    assert_eq!(pkg.main, None);
  }
}
