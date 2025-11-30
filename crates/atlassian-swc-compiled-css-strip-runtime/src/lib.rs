#![allow(clippy::all)]
#![allow(dead_code)]

#[path = "index.rs"]
mod index_module;
#[path = "strip_runtime.rs"]
mod strip_runtime;
mod types;

#[path = "utils/is-automatic-runtime.rs"]
mod utils_is_automatic_runtime;
#[path = "utils/is-cc-component.rs"]
mod utils_is_cc_component;
#[path = "utils/is-create-element.rs"]
mod utils_is_create_element;
#[path = "utils/remove-style-declarations.rs"]
mod utils_remove_style_declarations;
#[path = "utils/to-uri-component.rs"]
mod utils_to_uri_component;

pub use index_module::*;
pub use types::*;
