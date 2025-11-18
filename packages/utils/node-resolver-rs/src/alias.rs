use std::{
  borrow::Cow,
  path::{Path, PathBuf},
};

use glob_match::glob_match_with_captures;
use indexmap::IndexMap;
use std::ops::Range;

use crate::Specifier;

#[derive(serde::Deserialize, Clone, PartialEq, Debug)]
#[serde(untagged)]
pub enum AliasValue {
  Specifier(Specifier),
  Bool(bool),
  Global { global: String },
}

#[derive(serde::Deserialize, Clone, Debug, Default)]
#[serde(transparent)]
pub struct AliasMap {
  map: IndexMap<Specifier, AliasValue>,
}

impl AliasMap {
  pub fn get(&self, specifier: &Specifier) -> Option<&AliasValue> {
    self.map.get(specifier)
  }

  pub fn resolve_alias(&self, specifier: &Specifier) -> Option<Cow<'_, AliasValue>> {
    if let Some(alias) = self.lookup_alias(specifier) {
      return Some(alias);
    }

    if let Specifier::Package(package, subpath) = specifier
      && let Some(alias) = self.lookup_alias(&Specifier::Package(package.clone(), String::from("")))
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

    None
  }

  fn lookup_alias(&self, specifier: &Specifier) -> Option<Cow<'_, AliasValue>> {
    if let Some(value) = self.map.get(specifier) {
      return Some(Cow::Borrowed(value));
    }

    // Match glob aliases.
    for (key, value) in &self.map {
      let (glob, path) = match (key, specifier) {
        (Specifier::Relative(glob), Specifier::Relative(path))
        | (Specifier::Absolute(glob), Specifier::Absolute(path))
        | (Specifier::Tilde(glob), Specifier::Tilde(path)) => (
          glob.as_os_str().to_string_lossy().into_owned(),
          path.as_os_str().to_string_lossy().into_owned(),
        ),
        (Specifier::Package(module_a, glob), Specifier::Package(module_b, path))
          if module_a == module_b =>
        {
          (glob.clone(), path.clone())
        }
        (Specifier::Package(module_a, subpath_a), Specifier::Package(module_b, subpath_b)) => {
          // Glob could be in the package name, e.g. "@internal/*"
          (
            format!("{}/{}", module_a, subpath_a),
            format!("{}/{}", module_b, subpath_b),
          )
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

    if end != idx
      && let Ok(capture_index) = s[idx + 1..end + 1].parse::<usize>()
      && capture_index > 0
      && capture_index - 1 < captures.len()
    {
      res.replace_range(idx..end + 1, &path[captures[capture_index - 1].clone()]);
    }
  }

  res
}

#[cfg(test)]
mod tests {
  use super::*;

  #[allow(clippy::single_range_in_vec_init)]
  #[test]
  fn test_replace_captures() {
    assert_eq!(
      replace_captures("test/$1/$2", "foo/bar/baz", &vec![4..7, 8..11]),
      "test/bar/baz"
    );
    assert_eq!(
      replace_captures("test/$1/$2", "foo/bar/baz", &vec![4..7]),
      "test/bar/$2"
    );
    assert_eq!(
      replace_captures("test/$1/$2/$3", "foo/bar/baz", &vec![4..7, 8..11]),
      "test/bar/baz/$3"
    );
    assert_eq!(
      replace_captures("test/$1/$2/$", "foo/bar/baz", &vec![4..7, 8..11]),
      "test/bar/baz/$"
    );
    assert_eq!(
      replace_captures("te$st/$1/$2", "foo/bar/baz", &vec![4..7, 8..11]),
      "te$st/bar/baz"
    );
  }
}
