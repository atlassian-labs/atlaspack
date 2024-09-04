use anyhow::Error;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::plugin::{PluginContext, TransformerPlugin};
use atlaspack_core::types::{Asset, BundleBehavior};

#[derive(Debug)]
pub struct AtlaspackInlineStringTransformerPlugin {}

impl AtlaspackInlineStringTransformerPlugin {
  pub fn new(_ctx: &PluginContext) -> Result<Self, Error> {
    Ok(AtlaspackInlineStringTransformerPlugin {})
  }
}

impl TransformerPlugin for AtlaspackInlineStringTransformerPlugin {
  fn transform(&mut self, asset: Asset) -> Result<TransformResult, Error> {
    let mut asset = asset.clone();

    asset.bundle_behavior = BundleBehavior::Inline;
    asset
      .meta
      .insert(String::from("inlineType"), "string".into());

    Ok(TransformResult {
      asset,
      dependencies: Vec::new(),
      invalidate_on_file_change: Vec::new(),
    })
  }
}

#[cfg(test)]
mod test {}
