use atlaspack_core::types::Asset;
use atlaspack_core::types::AssetWithDependencies;
use atlaspack_core::types::Dependency;
use atlaspack_memoization_cache::CacheResponse;
use atlaspack_memoization_cache::Cacheable;
use serde::Deserialize;
use serde::Serialize;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::ser::{SerializeStruct, Serializer};
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;

use crate::plugins::PluginsRef;
use crate::plugins::TransformerPipeline;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum PipelineResult {
  Complete(Asset, Vec<Dependency>),
  TypeChange(Asset, Vec<Dependency>),
}

pub struct RunPipelineInput {
  pub asset: Asset,
  pub pipeline: TransformerPipeline,
  pub plugins: PluginsRef,
  pub project_root: PathBuf,
}

#[derive(Debug, PartialEq)]
pub struct RunPipelineOutput {
  pub invalidations: Vec<PathBuf>,
  pub discovered_assets: Vec<AssetWithDependencies>,
  pub pipeline_result: PipelineResult,
  pub project_root: PathBuf,
  cache_bailout: bool,
  /// Names of transformers that caused bailouts in this pipeline.
  pub bailout_transformers: Vec<String>,
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn run_pipeline(input: RunPipelineInput) -> anyhow::Result<RunPipelineOutput> {
  let RunPipelineInput {
    asset,
    pipeline,
    plugins,
    project_root,
  } = input;

  let mut current_asset = asset.clone();
  let mut dependencies = Vec::new();
  let mut discovered_assets = Vec::new();
  let mut invalidations = Vec::new();

  let original_asset_type = current_asset.file_type.clone();

  let mut cache_bailout = false;
  let mut bailout_transformers = Vec::new();

  for transformer in pipeline.transformers() {
    let transform_result = transformer.transform(current_asset).await?;

    if transform_result.cache_bailout {
      cache_bailout = true;
    }

    // Collect transformer names that caused bailouts
    if let Some(transformer_name) = transform_result.bailout_transformer {
      bailout_transformers.push(transformer_name);
    }

    current_asset = transform_result.asset;

    dependencies.extend(transform_result.dependencies);
    invalidations.extend(transform_result.invalidate_on_file_change);
    discovered_assets.extend(transform_result.discovered_assets);

    // If the Asset has changed type then we may need to trigger a different pipeline
    if current_asset.file_type != original_asset_type {
      // When the Asset changes file_type we need to regenerate its id
      current_asset.update_id(&project_root);

      let next_pipeline = plugins.transformers(&current_asset).await?;

      if pipeline.should_run_new_pipeline(&next_pipeline) {
        tracing::debug!(
          "Asset requires pipeline switch. File: {} from {} to {}",
          current_asset.file_path.display(),
          original_asset_type.extension(),
          current_asset.file_type.extension()
        );

        return Ok(RunPipelineOutput {
          invalidations,
          discovered_assets,
          pipeline_result: PipelineResult::TypeChange(current_asset, dependencies),
          project_root: project_root.clone(),
          cache_bailout,
          bailout_transformers,
        });
      }
    }
  }

  Ok(RunPipelineOutput {
    invalidations,
    discovered_assets,
    pipeline_result: PipelineResult::Complete(current_asset, dependencies),
    project_root: project_root.clone(),
    cache_bailout,
    bailout_transformers,
  })
}

impl Cacheable for RunPipelineInput {
  fn cache_key(&self) -> Option<(String, String)> {
    let mut hasher = atlaspack_core::hash::IdentifierHasher::default();
    self.asset.hash(&mut hasher);

    // If the pipeline has no cache key then it is uncachable
    self.pipeline.cache_key?.hash(&mut hasher);

    self.project_root.hash(&mut hasher);

    // Ignore plugins from the cache key

    let label = format!(
      "run_pipeline|{}",
      self
        .asset
        .file_path
        .strip_prefix(&self.project_root)
        .unwrap_or(&self.asset.file_path)
        .display()
    );
    let cache_key = format!("{}|{}", &label, hasher.finish());

    Some((label, cache_key))
  }
}

impl CacheResponse for RunPipelineOutput {
  fn should_bailout(&self) -> bool {
    self.cache_bailout
  }
}

// We need a custom serializer for RunPipelineOutput to serialize the code and
// maps separately
impl Serialize for RunPipelineOutput {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut state = serializer.serialize_struct("RunPipelineOutput", 6)?;
    state.serialize_field("invalidations", &self.invalidations)?;
    state.serialize_field("discovered_assets", &self.discovered_assets)?;
    state.serialize_field("pipeline_result", &self.pipeline_result)?;
    state.serialize_field("project_root", &self.project_root)?;

    let mut code: Vec<&[u8]> = Vec::with_capacity(self.discovered_assets.len() + 1);
    let mut maps: Vec<Option<Vec<u8>>> = Vec::with_capacity(self.discovered_assets.len() + 1);

    match &self.pipeline_result {
      PipelineResult::Complete(asset, _) | PipelineResult::TypeChange(asset, _) => {
        code.push(asset.code.bytes());

        if let Some(map) = &asset.map {
          maps.push(Some(map.to_buffer_vec().map_err(|error| {
            serde::ser::Error::custom(format!(
              "Failed to serialize source map to buffer: {}",
              error
            ))
          })?));
        } else {
          maps.push(None);
        }
      }
    }

    for AssetWithDependencies { asset, .. } in &self.discovered_assets {
      code.push(asset.code.bytes());

      if let Some(map) = &asset.map {
        maps.push(Some(map.to_buffer_vec().map_err(|error| {
          serde::ser::Error::custom(format!(
            "Failed to serialize source map to buffer: {}",
            error
          ))
        })?));
      } else {
        maps.push(None);
      }
    }

    state.serialize_field("code", &code)?;
    state.serialize_field("maps", &maps)?;

    state.end()
  }
}

// Custom deserializer for RunPipelineOutput to reconstruct code and maps
impl<'de> Deserialize<'de> for RunPipelineOutput {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    #[derive(Deserialize)]
    #[serde(field_identifier, rename_all = "snake_case")]
    enum Field {
      Invalidations,
      DiscoveredAssets,
      PipelineResult,
      ProjectRoot,
      Code,
      Maps,
    }

    struct RunPipelineOutputVisitor;

    impl<'de> Visitor<'de> for RunPipelineOutputVisitor {
      type Value = RunPipelineOutput;

      fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct RunPipelineOutput")
      }

      fn visit_map<V>(self, mut map: V) -> Result<RunPipelineOutput, V::Error>
      where
        V: MapAccess<'de>,
      {
        let mut invalidations = None;
        let mut discovered_assets: Option<Vec<AssetWithDependencies>> = None;
        let mut pipeline_result = None;
        let mut project_root: Option<PathBuf> = None;
        let mut code = None;
        let mut maps = None;

        while let Some(key) = map.next_key()? {
          match key {
            Field::Invalidations => {
              if invalidations.is_some() {
                return Err(de::Error::duplicate_field("invalidations"));
              }
              invalidations = Some(map.next_value()?);
            }
            Field::DiscoveredAssets => {
              if discovered_assets.is_some() {
                return Err(de::Error::duplicate_field("discovered_assets"));
              }
              discovered_assets = Some(map.next_value()?);
            }
            Field::PipelineResult => {
              if pipeline_result.is_some() {
                return Err(de::Error::duplicate_field("pipeline_result"));
              }
              pipeline_result = Some(map.next_value()?);
            }
            Field::ProjectRoot => {
              if project_root.is_some() {
                return Err(de::Error::duplicate_field("project_root"));
              }
              project_root = Some(map.next_value()?);
            }
            Field::Code => {
              if code.is_some() {
                return Err(de::Error::duplicate_field("code"));
              }
              code = Some(map.next_value::<Vec<Vec<u8>>>()?);
            }
            Field::Maps => {
              if maps.is_some() {
                return Err(de::Error::duplicate_field("maps"));
              }
              maps = Some(map.next_value::<Vec<Option<Vec<u8>>>>()?);
            }
          }
        }

        let invalidations =
          invalidations.ok_or_else(|| de::Error::missing_field("invalidations"))?;
        let mut discovered_assets =
          discovered_assets.ok_or_else(|| de::Error::missing_field("discovered_assets"))?;
        let mut pipeline_result =
          pipeline_result.ok_or_else(|| de::Error::missing_field("pipeline_result"))?;
        let project_root = project_root.ok_or_else(|| de::Error::missing_field("project_root"))?;
        let mut code = code.ok_or_else(|| de::Error::missing_field("code"))?;
        let mut maps = maps.ok_or_else(|| de::Error::missing_field("maps"))?;

        if code.len() != maps.len() {
          return Err(de::Error::custom("code and maps length mismatch"));
        }

        let expected_len = discovered_assets.len() + 1;
        if code.len() != expected_len {
          return Err(de::Error::custom(format!(
            "expected {} code entries, got {}",
            expected_len,
            code.len()
          )));
        }

        // Reconstruct discovered assets' code and maps (in reverse order using pop)
        for AssetWithDependencies { asset, .. } in discovered_assets.iter_mut().rev() {
          let asset_code = code.pop().unwrap(); // Safe because we validated length above
          let asset_map = maps.pop().unwrap(); // Safe because we validated length above

          asset.code = atlaspack_core::types::Code::new(asset_code);

          if let Some(map_data) = asset_map {
            asset.map = Some(
              atlaspack_sourcemap::SourceMap::from_buffer(&project_root, &map_data).map_err(
                |e| de::Error::custom(format!("failed to deserialize source map: {}", e)),
              )?,
            );
          } else {
            asset.map = None;
          }
        }

        // Reconstruct the pipeline result asset's code and map (should be the last remaining item)
        let main_code = code.pop().unwrap(); // Safe because we validated length above
        let main_map = maps.pop().unwrap(); // Safe because we validated length above

        match &mut pipeline_result {
          PipelineResult::Complete(asset, _) | PipelineResult::TypeChange(asset, _) => {
            asset.code = atlaspack_core::types::Code::new(main_code);

            if let Some(map_data) = main_map {
              asset.map = Some(
                atlaspack_sourcemap::SourceMap::from_buffer(&project_root, &map_data).map_err(
                  |e| de::Error::custom(format!("failed to deserialize source map: {}", e)),
                )?,
              );
            } else {
              asset.map = None;
            }
          }
        }

        Ok(RunPipelineOutput {
          invalidations,
          discovered_assets,
          pipeline_result,
          project_root,
          cache_bailout: false, // Default value since it's not serialized
          bailout_transformers: Vec::new(), // Default value since it's not serialized
        })
      }
    }

    const FIELDS: &[&str] = &[
      "invalidations",
      "discovered_assets",
      "pipeline_result",
      "project_root",
      "code",
      "maps",
    ];
    deserializer.deserialize_struct("RunPipelineOutput", FIELDS, RunPipelineOutputVisitor)
  }
}
