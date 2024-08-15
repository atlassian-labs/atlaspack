use std::borrow::Cow;
use std::path::is_separator;
use std::path::PathBuf;

use percent_encoding::percent_decode_str;

use crate::builtins::BUILTINS;
use crate::url_to_path::url_to_path;
use crate::Flags;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SpecifierType {
  Esm,
  Cjs,
  Url,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum SpecifierError {
  EmptySpecifier,
  InvalidPackageSpecifier,
  #[serde(serialize_with = "serialize_url_error")]
  UrlError(url::ParseError),
  InvalidFileUrl,
}

impl From<url::ParseError> for SpecifierError {
  fn from(value: url::ParseError) -> Self {
    SpecifierError::UrlError(value)
  }
}

fn serialize_url_error<S>(value: &url::ParseError, serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  use serde::Serialize;
  value.to_string().serialize(serializer)
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Specifier {
  Relative(PathBuf),
  Absolute(PathBuf),
  Tilde(PathBuf),
  Hash(String),
  Package(String, String),
  Builtin(String),
  Url(String),
}

impl Specifier {
  pub fn parse(
    specifier: &str,
    specifier_type: SpecifierType,
    flags: Flags,
  ) -> Result<(Specifier, Option<&str>), SpecifierError> {
    if specifier.is_empty() {
      return Err(SpecifierError::EmptySpecifier);
    }

    Ok(match specifier.as_bytes()[0] {
      b'.' => {
        let specifier = if let Some(specifier) = specifier.strip_prefix("./") {
          specifier.trim_start_matches('/')
        } else {
          specifier
        };
        let (path, query) = decode_path(specifier, specifier_type);
        (Specifier::Relative(path), query)
      }
      b'~' => {
        let mut specifier = &specifier[1..];
        if !specifier.is_empty() && is_separator(specifier.as_bytes()[0] as char) {
          specifier = &specifier[1..];
        }
        let (path, query) = decode_path(specifier, specifier_type);
        (Specifier::Tilde(path), query)
      }
      b'/' => {
        if specifier.starts_with("//") && specifier_type == SpecifierType::Url {
          // A protocol-relative URL, e.g `url('//example.com/foo.png')`.
          (Specifier::Url(String::from(specifier)), None)
        } else {
          let (path, query) = decode_path(specifier, specifier_type);
          (Specifier::Absolute(path), query)
        }
      }
      b'#' => (Specifier::Hash(String::from(&specifier[1..])), None),
      _ => {
        // Bare specifier.
        match specifier_type {
          SpecifierType::Url | SpecifierType::Esm => {
            // Check if there is a scheme first.
            if let Ok((scheme, rest)) = parse_scheme(specifier) {
              let (path, rest) = parse_path(rest);
              let (query, _) = parse_query(rest);
              match scheme.as_ref() {
                "npm" if flags.contains(Flags::NPM_SCHEME) => {
                  if BUILTINS.contains(&path) {
                    return Ok((Specifier::Builtin(String::from(path)), None));
                  }

                  (
                    parse_package(percent_decode_str(path).decode_utf8_lossy())?,
                    query,
                  )
                }
                "node" => {
                  // Node does not URL decode or support query params here.
                  // See https://github.com/nodejs/node/issues/39710.
                  (Specifier::Builtin(String::from(path)), None)
                }
                "file" => (Specifier::Absolute(url_to_path(specifier)?), query),
                _ => (Specifier::Url(String::from(specifier)), None),
              }
            } else {
              // If not, then parse as an npm package if this is an ESM specifier,
              // otherwise treat this as a relative path.
              let (path, rest) = parse_path(specifier);
              if specifier_type == SpecifierType::Esm {
                if BUILTINS.contains(&path) {
                  return Ok((Specifier::Builtin(String::from(path)), None));
                }

                let (query, _) = parse_query(rest);
                (
                  parse_package(percent_decode_str(path).decode_utf8_lossy())?,
                  query,
                )
              } else {
                let (path, query) = decode_path(specifier, specifier_type);
                (Specifier::Relative(path), query)
              }
            }
          }
          SpecifierType::Cjs => {
            if let Some(node_prefixed) = specifier.strip_prefix("node:") {
              return Ok((Specifier::Builtin(String::from(node_prefixed)), None));
            }

            if BUILTINS.contains(&specifier) {
              (Specifier::Builtin(String::from(specifier)), None)
            } else {
              #[cfg(windows)]
              if !flags.contains(Flags::ABSOLUTE_SPECIFIERS) {
                let path = std::path::PathBuf::from(specifier);
                if path.is_absolute() {
                  return Ok((Specifier::Absolute(path), None));
                }
              }

              (parse_package(Cow::Borrowed(specifier))?, None)
            }
          }
        }
      }
    })
  }

  pub fn to_string<'a>(&'a self) -> Cow<'a, str> {
    match self {
      Specifier::Relative(path) | Specifier::Absolute(path) | Specifier::Tilde(path) => {
        path.as_os_str().to_string_lossy()
      }
      Specifier::Hash(path) => Cow::Borrowed(path),
      Specifier::Package(module, subpath) => {
        if subpath.is_empty() {
          Cow::Borrowed(module)
        } else {
          Cow::Owned(format!("{}/{}", module, subpath))
        }
      }
      Specifier::Builtin(builtin) => Cow::Borrowed(builtin),
      Specifier::Url(url) => Cow::Borrowed(url),
    }
  }
}

// https://url.spec.whatwg.org/#scheme-state
// https://github.com/servo/rust-url/blob/1c1e406874b3d2aa6f36c5d2f3a5c2ea74af9efb/url/src/parser.rs#L387
pub fn parse_scheme(input: &str) -> Result<(Cow<'_, str>, &str), ()> {
  if input.is_empty() || !input.starts_with(ascii_alpha) {
    return Err(());
  }
  let mut is_lowercase = true;
  for (i, c) in input.chars().enumerate() {
    match c {
      'A'..='Z' => {
        is_lowercase = false;
      }
      'a'..='z' | '0'..='9' | '+' | '-' | '.' => {}
      ':' => {
        let scheme = &input[0..i];
        let rest = &input[i + 1..];
        return Ok(if is_lowercase {
          (Cow::Borrowed(scheme), rest)
        } else {
          (Cow::Owned(scheme.to_ascii_lowercase()), rest)
        });
      }
      _ => {
        return Err(());
      }
    }
  }

  // EOF before ':'
  Err(())
}

// https://url.spec.whatwg.org/#path-state
fn parse_path(input: &str) -> (&str, &str) {
  // We don't really want to normalize the path (e.g. replacing ".." and "." segments).
  // That is done later. For now, we just need to find the end of the path.
  if let Some(pos) = input.chars().position(|c| c == '?' || c == '#') {
    (&input[0..pos], &input[pos..])
  } else {
    (input, "")
  }
}

// https://url.spec.whatwg.org/#query-state
fn parse_query(input: &str) -> (Option<&str>, &str) {
  if !input.is_empty() && input.as_bytes()[0] == b'?' {
    if let Some(pos) = input.chars().position(|c| c == '#') {
      (Some(&input[0..pos]), &input[pos..])
    } else {
      (Some(input), "")
    }
  } else {
    (None, input)
  }
}

/// https://url.spec.whatwg.org/#ascii-alpha
#[inline]
fn ascii_alpha(ch: char) -> bool {
  ch.is_ascii_alphabetic()
}

fn parse_package(specifier: Cow<'_, str>) -> Result<Specifier, SpecifierError> {
  match specifier {
    Cow::Borrowed(specifier) => {
      let (module, subpath) = parse_package_specifier(specifier)?;
      Ok(Specifier::Package(module.to_string(), subpath.to_string()))
    }
    Cow::Owned(specifier) => {
      let (module, subpath) = parse_package_specifier(&specifier)?;
      Ok(Specifier::Package(module.to_owned(), subpath.to_owned()))
    }
  }
}

pub fn parse_package_specifier(specifier: &str) -> Result<(&str, &str), SpecifierError> {
  let idx = specifier.chars().position(|p| p == '/');
  if specifier.starts_with('@') {
    let idx = idx.ok_or(SpecifierError::InvalidPackageSpecifier)?;
    if let Some(next) = &specifier[idx + 1..].chars().position(|p| p == '/') {
      Ok((
        &specifier[0..idx + 1 + *next],
        &specifier[idx + *next + 2..],
      ))
    } else {
      Ok((specifier, ""))
    }
  } else if let Some(idx) = idx {
    Ok((&specifier[0..idx], &specifier[idx + 1..]))
  } else {
    Ok((specifier, ""))
  }
}

pub fn decode_path(specifier: &str, specifier_type: SpecifierType) -> (PathBuf, Option<&str>) {
  match specifier_type {
    SpecifierType::Url | SpecifierType::Esm => {
      let (path, rest) = parse_path(specifier);
      let (query, _) = parse_query(rest);
      let path = PathBuf::from(percent_decode_str(path).decode_utf8_lossy().to_string());
      (path, query)
    }
    SpecifierType::Cjs => (PathBuf::from(specifier), None),
  }
}

impl From<&str> for Specifier {
  fn from(specifier: &str) -> Self {
    Specifier::parse(specifier, SpecifierType::Cjs, Flags::empty())
      .unwrap()
      .0
  }
}

impl<'de> serde::Deserialize<'de> for Specifier {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    use serde::Deserialize;
    let s: String = Deserialize::deserialize(deserializer)?;
    // Specifiers are only deserialized as part of the "alias" and "browser" fields,
    // so we assume CJS specifiers in Atlaspack mode.
    Specifier::parse(&s, SpecifierType::Cjs, Flags::empty())
      .map(|s| s.0)
      .map_err(|_| serde::de::Error::custom("Invalid specifier"))
  }
}
