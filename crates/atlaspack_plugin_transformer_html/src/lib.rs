pub mod attrs;
pub mod dom_visitor;
mod hmr_visitor;
mod html_dependencies_visitor;
mod html_transformer;

pub use crate::html_transformer::parse_html;
pub use crate::html_transformer::serialize_html;
pub use crate::html_transformer::AtlaspackHtmlTransformerPlugin;
