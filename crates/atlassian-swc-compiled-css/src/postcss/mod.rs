//! Native PostCSS pipeline replica.
//!
//! This module mirrors `packages/css/src` by exposing `transform` and `sort`
//! entry points plus the plugin tree used by the Babel implementation.

pub mod plugins;
pub mod sort;
pub mod transform;

#[cfg(feature = "postcss_engine")]
pub mod postcss_pipeline;
pub mod utils;
pub mod value_parser;

#[allow(unused_imports)]
pub use sort::{sort_atomic_style_sheet, SortOptions};
#[allow(unused_imports)]
pub use transform::{transform_css, TransformCssOptions, TransformCssResult};
