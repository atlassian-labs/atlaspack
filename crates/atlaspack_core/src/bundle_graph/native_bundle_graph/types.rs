use std::sync::Arc;

use crate::types::{Asset, Bundle, Dependency, Target};

pub type NodeId = usize;

/// Edge types in the native bundle graph.
///
/// Numeric values match the JS bundle graph edge types.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum NativeBundleGraphEdgeType {
  #[default]
  Null = 1,
  Contains = 2,
  Bundle = 3,
  References = 4,
  InternalAsync = 5,
  Conditional = 6,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum NativeBundleGraphNode {
  Root,
  Asset(Arc<Asset>),
  Dependency(Arc<Dependency>),
  BundleGroup {
    target: Target,
    entry_asset_id: String,
  },
  Bundle(Bundle),
}
