use std::hash::{Hash, Hasher};
use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use atlaspack_core::{
  bundle_graph::bundle_graph_from_js::BundleGraphFromJs,
  debug_tools::DebugTools,
  package_result::PackageResult,
  types::{Bundle, FileType},
};
use atlaspack_packager_js::{JsPackager, PackagingContext};
use parking_lot::RwLock;

use crate::{
  request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError},
  requests::RequestResult,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PackageRequestOutput {
  pub bundle_contents: Vec<u8>,
  pub map_contents: Option<Vec<u8>>,
}

#[allow(dead_code)]
pub struct PackageRequest {
  pub bundle: Bundle,
  pub bundle_graph: Arc<RwLock<BundleGraphFromJs>>,
}

impl std::fmt::Debug for PackageRequest {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("PackageRequest")
      .field("bundle", &self.bundle)
      .finish_non_exhaustive()
  }
}

impl Hash for PackageRequest {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.bundle.hash(state);
  }
}

#[async_trait]
impl Request for PackageRequest {
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let result: Result<PackageResult, RunRequestError> = match self.bundle.bundle_type {
      FileType::Js => {
        let packager = JsPackager::new(
          PackagingContext {
            db: Arc::clone(&request_context.db),
            cache: None,
            project_root: request_context.project_root,
            debug_tools: DebugTools::default(),
          },
          Arc::clone(&self.bundle_graph),
        );
        packager.package(&self.bundle.id)
      }
      _ => Err(anyhow!(
        "Unsupported bundle type: {:?}",
        self.bundle.bundle_type
      )),
    };

    match result {
      Ok(package_result) => Ok(ResultAndInvalidations {
        result: RequestResult::Package(PackageRequestOutput {
          bundle_contents: package_result
            .bundle_info
            .bundle_contents
            .expect("Bundle contents are required when packaging"),
          map_contents: package_result.bundle_info.map_contents,
        }),
        invalidations: vec![],
      }),
      Err(error) => Err(error),
    }
  }
}

#[cfg(test)]
mod tests {
  use atlaspack_core::bundle_graph::bundle_graph_from_js::BundleGraphFromJs;
  use atlaspack_core::types::{Environment, Target};
  use parking_lot::RwLock;

  use crate::test_utils::{RequestTrackerTestOptions, request_tracker};

  use super::*;

  fn mock_bundle(bundle_type: FileType) -> Bundle {
    Bundle {
      bundle_behavior: None,
      bundle_type,
      entry_asset_ids: vec![],
      env: Environment::default(),
      hash_reference: String::new(),
      id: String::new(),
      is_splittable: None,
      main_entry_id: None,
      manual_shared_bundle: None,
      name: None,
      needs_stable_name: None,
      pipeline: None,
      public_id: None,
      target: Target::default(),
    }
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_package_request_output() {
    let request = PackageRequest {
      bundle: mock_bundle(FileType::Js),
      bundle_graph: Arc::new(RwLock::new(BundleGraphFromJs::default())),
    };
    let output = request_tracker(RequestTrackerTestOptions::default())
      .run_request(request)
      .await;
    eprintln!(
      "output: {:?}",
      match &**output.as_ref().unwrap() {
        RequestResult::Package(output) => output,
        _ => panic!("Expected PackageRequestOutput"),
      }
    );
    assert!(output.is_ok());
  }
}
