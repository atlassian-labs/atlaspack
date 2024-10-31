use std::io::Cursor;
use std::sync::Arc;

use anyhow::Error;
use async_trait::async_trait;
use atlaspack_core::diagnostic_error;
use atlaspack_core::plugin::{PluginContext, TransformerPlugin};
use atlaspack_core::plugin::{TransformContext, TransformResult};
use atlaspack_core::types::{Asset, BundleBehavior, Code, FileType};
use image::imageops::FilterType;
use image::{ImageFormat, ImageReader};
use url_search_params::parse_url_search_params;

#[derive(Debug)]
pub struct AtlaspackImageTransformerPlugin {}

impl AtlaspackImageTransformerPlugin {
  pub fn new(_ctx: &PluginContext) -> Self {
    AtlaspackImageTransformerPlugin {}
  }
}

#[async_trait]
impl TransformerPlugin for AtlaspackImageTransformerPlugin {
  async fn transform(
    &self,
    _context: TransformContext,
    asset: Asset,
  ) -> Result<TransformResult, Error> {
    let mut asset = asset.clone();

    if asset.bundle_behavior.is_none() {
      asset.bundle_behavior = Some(BundleBehavior::Isolated);
    }

    // TODO: Optimize this in resolver / change asset query type
    let query = asset
      .query
      .clone()
      .map(|q| parse_url_search_params(&q[1..]))
      .unwrap_or_default();

    let width = query
      .get("width")
      .map_or(Ok(None), |w| w.parse::<u32>().map(Some))?;

    let height = query
      .get("height")
      .map_or(Ok(None), |h| h.parse::<u32>().map(Some))?;

    let target_file_type = query.get("as").map(|f| FileType::from_extension(f));

    if width.is_some() || height.is_some() || target_file_type.is_some() {
      let format = image_format(&asset.file_type)?;
      let target_format = target_file_type
        .clone()
        .map_or(Ok(None), |f| image_format(&f).map(Some))?
        .unwrap_or(format);

      let img = ImageReader::with_format(Cursor::new(asset.code.bytes()), format).decode()?;
      let filter = FilterType::Triangle;
      let img = if let (Some(width), Some(height)) = (width, height) {
        img.resize_exact(width, height, filter)
      } else if let Some(width) = width {
        img.resize(width, img.height(), filter)
      } else if let Some(height) = height {
        img.resize(img.width(), height, filter)
      } else {
        img
      };

      let mut bytes: Vec<u8> = Vec::new();
      img.write_to(&mut Cursor::new(&mut bytes), target_format)?;

      asset.code = Arc::new(Code::new(bytes));
      asset.file_type = target_file_type.unwrap_or(asset.file_type);
    }

    Ok(TransformResult {
      asset,
      ..Default::default()
    })
  }
}

fn image_format(file_type: &FileType) -> Result<ImageFormat, Error> {
  match file_type {
    FileType::Avif => Ok(ImageFormat::Avif),
    FileType::Gif => Ok(ImageFormat::Gif),
    FileType::Jpeg => Ok(ImageFormat::Jpeg),
    FileType::Png => Ok(ImageFormat::Png),
    FileType::Tiff => Ok(ImageFormat::Tiff),
    FileType::WebP => Ok(ImageFormat::WebP),
    _ => Err(diagnostic_error!(
      "Unsupported image file type: {}",
      file_type.extension()
    )),
  }
}

#[cfg(test)]
mod tests {
  use std::{path::PathBuf, sync::Arc};

  use atlaspack_core::{
    config_loader::ConfigLoader,
    plugin::{PluginLogger, PluginOptions},
  };
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;

  use super::*;

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_image_asset() {
    let file_system = Arc::new(InMemoryFileSystem::default());
    let plugin = AtlaspackImageTransformerPlugin::new(&PluginContext {
      config: Arc::new(ConfigLoader {
        fs: file_system.clone(),
        project_root: PathBuf::default(),
        search_path: PathBuf::default(),
      }),
      file_system,
      logger: PluginLogger::default(),
      options: Arc::new(PluginOptions::default()),
    });

    let asset = Asset::default();
    let context = TransformContext::default();

    assert_ne!(asset.bundle_behavior, Some(BundleBehavior::Isolated));
    assert_eq!(
      plugin
        .transform(context, asset)
        .await
        .map_err(|e| e.to_string()),
      Ok(TransformResult {
        asset: Asset {
          bundle_behavior: Some(BundleBehavior::Isolated),
          ..Asset::default()
        },
        ..Default::default()
      })
    );
  }
}
