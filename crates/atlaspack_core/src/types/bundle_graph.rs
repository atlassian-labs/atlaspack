//! Backwards compatible re-exports for bundle graph JS types.
//!
//! The canonical location for these types is:
//! `atlaspack_core::bundle_graph::bundle_graph_from_js::types`.
//!
//! Keeping this module avoids widespread import churn while we migrate bundle graph
//! implementations.

pub use crate::bundle_graph::bundle_graph_from_js::types::*;
