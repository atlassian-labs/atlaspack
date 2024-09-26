use std::collections::HashMap;
use std::str::FromStr;

use browserslist::Distrib;

use super::version::Version;

/// List of targeted browsers
#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct Browsers {
  data: HashMap<String, Version>,
}

impl Browsers {
  pub fn iter(&self) -> impl Iterator<Item = (&String, &Version)> {
    self.data.iter()
  }

  pub fn is_empty(&self) -> bool {
    self.data.is_empty()
  }
}

// TODO: Versions like "all" are not supported.
impl From<Vec<Distrib>> for Browsers {
  fn from(distribs: Vec<Distrib>) -> Self {
    let mut browsers = Browsers::default();
    for distrib in distribs {
      if let Ok(version) = Version::from_str(&*distrib.version()) {
        browsers.data.insert(distrib.name().to_string(), version);
      }
    }

    browsers
  }
}
