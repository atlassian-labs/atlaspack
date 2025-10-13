use atlaspack_core::types::Asset;
use regex::RegexSet;

pub struct Conditions {
  code_match: Option<RegexSet>,
}

impl Conditions {
  pub fn new(code_match: Option<Vec<String>>) -> anyhow::Result<Self> {
    let code_match = if let Some(patterns) = code_match {
      Some(RegexSet::new(&patterns)?)
    } else {
      None
    };

    Ok(Self { code_match })
  }

  pub fn should_run(&self, asset: &Asset) -> anyhow::Result<bool> {
    if let Some(code_match) = &self.code_match {
      return Ok(code_match.is_match(asset.code.as_str()?));
    }

    Ok(true)
  }
}
