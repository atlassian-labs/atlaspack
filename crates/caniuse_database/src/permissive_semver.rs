use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum VersionParseError {
  #[error("failed to parse version component")]
  ParseIntError(#[from] std::num::ParseIntError),
}

/// A version number like `1.0.0` or `1.2` or `1`.
///
/// Parse with [`Version::parse`] or [`Version::try_from`].
///
/// ## Examples
///
/// ```
/// use caniuse_database::Version;
///
/// let version = Version::parse("1.2.3").unwrap();
/// assert_eq!(&version.components(), &[1, 2, 3]);
/// ```
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct Version {
  components: tinyvec::TinyVec<[u64; 4]>,
}

impl Version {
  /// Parse a version string
  pub fn parse(value: &str) -> Result<Self, VersionParseError> {
    let mut components = tinyvec::tiny_vec!();
    let parts = value.split('.');
    for part in parts {
      if part.is_empty() {
        components.push(0);
      } else {
        components.push(part.parse::<u64>()?);
      }
    }
    Ok(Version { components })
  }

  /// Get the components for this version as a slice
  pub fn components(&self) -> &[u64] {
    self.components.as_slice()
  }
}

impl TryFrom<&str> for Version {
  type Error = VersionParseError;

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    Version::parse(value)
  }
}

impl std::cmp::PartialOrd for Version {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    self.components.partial_cmp(&other.components)
  }
}

/// A version number like `1.0.0` or `1.2` or `1`.
///
/// Parse with [`VersionRange::parse`] or [`VersionRange::try_from`].
///
/// ## Examples
///
/// ```
/// use caniuse_database::{VersionRange, Version};
///
/// // no maximum
/// let range = VersionRange::parse("1.2.3").unwrap();
/// assert_eq!(&range.min().components(), &[1, 2, 3]);
///
/// assert!(range.satisfies(&Version::parse("1.2.3").unwrap()));
///
/// let range = VersionRange::parse("1.2.3-2").unwrap();
/// assert_eq!(&range.min().components(), &[1, 2, 3]);
/// assert_eq!(&range.max().unwrap().components(), &[2]);
///
/// assert!(range.satisfies(&Version::parse("1.9").unwrap()));
/// ```
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct VersionRange {
  min: Version,
  max: Option<Version>,
}

impl VersionRange {
  /// Parse a version range
  pub fn parse(value: &str) -> Result<Self, VersionParseError> {
    let mut parts = value.split('-');
    let min = Version::parse(parts.next().unwrap())?;
    let max = parts.next().map(Version::parse).transpose()?;
    Ok(VersionRange { min, max })
  }

  /// Check if a version satisfies this version range.
  ///
  /// Ranges are always inclusive.
  pub fn satisfies(&self, version: &Version) -> bool {
    version.components >= self.min.components
      && self
        .max
        .as_ref()
        .map_or(true, |max| version.components <= max.components)
  }

  /// Get the minimum bounds for this range
  pub fn min(&self) -> &Version {
    &self.min
  }

  /// Get the maximum bounds for this range
  pub fn max(&self) -> Option<&Version> {
    self.max.as_ref()
  }
}

impl TryFrom<&str> for VersionRange {
  type Error = VersionParseError;

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    VersionRange::parse(value)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_parse_version() {
    let version = Version::parse("1.2.3").unwrap();
    assert_eq!(&version.components.as_slice(), &[1, 2, 3]);
    let version = Version::parse("1").unwrap();
    assert_eq!(&version.components.as_slice(), &[1]);
    let version = Version::parse("1.").unwrap();
    assert_eq!(&version.components.as_slice(), &[1, 0]);
  }

  #[test]
  fn test_parse_version_range() {
    let version = VersionRange::parse("1.2.3").unwrap();
    assert_eq!(
      &version,
      &VersionRange {
        min: Version {
          components: tinyvec::tiny_vec![1, 2, 3]
        },
        max: None,
      }
    );
    let version = VersionRange::parse("1.2.3-2.0").unwrap();
    assert_eq!(
      &version,
      &VersionRange {
        min: Version {
          components: tinyvec::tiny_vec![1, 2, 3]
        },
        max: Some(Version {
          components: tinyvec::tiny_vec![2, 0]
        }),
      }
    );
  }

  #[test]
  fn test_version_ord() {
    let version1 = Version::parse("1.2.3").unwrap();
    let version2 = Version::parse("1.2.4").unwrap();
    assert!(version1 < version2);

    let version1 = Version::parse("1.2.3").unwrap();
    let version2 = Version::parse("1.2.3").unwrap();
    assert!(version1 == version2);

    let version1 = Version::parse("1.2.3").unwrap();
    let version2 = Version::parse("1.2.2").unwrap();
    assert!(version1 > version2);

    let version1 = Version::parse("1.2.3").unwrap();
    let version2 = Version::parse("1.2").unwrap();
    assert!(version1 > version2);

    let version1 = Version::parse("1.2.3").unwrap();
    let version2 = Version::parse("2.0.0.0").unwrap();

    assert!(version1 < version2);
  }
}
