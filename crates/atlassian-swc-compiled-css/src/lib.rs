#![allow(clippy::all)]
#![allow(dead_code)]

pub mod config;
pub mod errors;
pub mod migration_hash;

pub use config::CompiledCssInJsTransformConfig;

#[path = "babel-plugin.rs"]
mod babel_plugin;
#[path = "index.rs"]
mod index_module;
mod types;

#[path = "class-names/index.rs"]
mod class_names;
#[path = "constants.rs"]
mod constants;
#[path = "css-map/index.rs"]
mod css_map;
#[path = "css-map/process-selectors.rs"]
mod css_map_process_selectors;
#[path = "css-prop/index.rs"]
mod css_prop;
#[path = "postcss/mod.rs"]
mod postcss;
#[path = "styled/index.rs"]
mod styled;
#[path = "xcss-prop/index.rs"]
mod xcss_prop;

#[path = "utils/append-runtime-imports.rs"]
mod utils_append_runtime_imports;
#[path = "utils/ast.rs"]
mod utils_ast;
#[path = "utils/build-compiled-component.rs"]
mod utils_build_compiled_component;
#[path = "utils/build-css-variables.rs"]
mod utils_build_css_variables;
#[path = "utils/build-display-name.rs"]
mod utils_build_display_name;
#[path = "utils/build-styled-component.rs"]
mod utils_build_styled_component;
#[path = "utils/cache.rs"]
mod utils_cache;
#[path = "utils/comments.rs"]
mod utils_comments;
#[path = "utils/compress-class-names-for-runtime.rs"]
mod utils_compress_class_names_for_runtime;
#[path = "utils/constants.rs"]
mod utils_constants;
#[path = "utils/create-result-pair.rs"]
mod utils_create_result_pair;
#[path = "utils/css.rs"]
mod utils_css;
#[path = "utils/css-builders.rs"]
mod utils_css_builders;
#[path = "utils/css-map.rs"]
mod utils_css_map;
#[path = "utils/evaluate-expression.rs"]
mod utils_evaluate_expression;
#[path = "utils/find-open-selectors.rs"]
mod utils_find_open_selectors;
#[path = "utils/get-jsx-attribute.rs"]
mod utils_get_jsx_attribute;
#[path = "utils/get-runtime-class-name-library.rs"]
mod utils_get_runtime_class_name_library;
#[path = "utils/has-numeric-value.rs"]
mod utils_has_numeric_value;
#[path = "utils/hash.rs"]
mod utils_hash;
#[path = "utils/hoist-sheet.rs"]
mod utils_hoist_sheet;
#[path = "utils/is-compiled.rs"]
mod utils_is_compiled;
#[path = "utils/is-empty.rs"]
mod utils_is_empty;
#[path = "utils/is-jsx-function.rs"]
mod utils_is_jsx_function;
#[path = "utils/is-prop-valid.rs"]
mod utils_is_prop_valid;
#[path = "utils/is-prop-valid-data.rs"]
mod utils_is_prop_valid_data;
#[path = "utils/manipulate-template-literal.rs"]
mod utils_manipulate_template_literal;
#[path = "utils/__mocks__/cache.rs"]
mod utils_mocks_cache;
#[path = "utils/module_scope.rs"]
mod utils_module_scope;
#[path = "utils/normalize-props-usage.rs"]
mod utils_normalize_props_usage;
#[path = "utils/object-property-to-string.rs"]
mod utils_object_property_to_string;
#[path = "utils/preserve-leading-comments.rs"]
mod utils_preserve_leading_comments;
#[path = "utils/resolve-binding.rs"]
mod utils_resolve_binding;
#[path = "utils/transform-css-items.rs"]
mod utils_transform_css_items;
#[path = "utils/traverse-expression/index.rs"]
mod utils_traverse_expression;
#[path = "utils/traverse-expression/traverse-binary-expression.rs"]
mod utils_traverse_expression_traverse_binary_expression;
#[path = "utils/traverse-expression/traverse-call-expression.rs"]
mod utils_traverse_expression_traverse_call_expression;
#[path = "utils/traverse-expression/traverse-function.rs"]
mod utils_traverse_expression_traverse_function;
#[path = "utils/traverse-expression/traverse-identifier.rs"]
mod utils_traverse_expression_traverse_identifier;
#[path = "utils/traverse-expression/traverse-member-expression/index.rs"]
mod utils_traverse_expression_traverse_member_expression;
#[path = "utils/traverse-expression/traverse-member-expression/traverse-access-path/index.rs"]
mod utils_traverse_expression_traverse_member_expression_traverse_access_path;
#[path = "utils/traverse-expression/traverse-member-expression/traverse-access-path/evaluate-path/index.rs"]
mod utils_traverse_expression_traverse_member_expression_traverse_access_path_evaluate_path;
#[path = "utils/traverse-expression/traverse-member-expression/traverse-access-path/evaluate-path/namespace-import.rs"]
mod utils_traverse_expression_traverse_member_expression_traverse_access_path_evaluate_path_namespace_import;
#[path = "utils/traverse-expression/traverse-member-expression/traverse-access-path/evaluate-path/object.rs"]
mod utils_traverse_expression_traverse_member_expression_traverse_access_path_evaluate_path_object;
#[path = "utils/traverse-expression/traverse-member-expression/traverse-access-path/resolve-expression/index.rs"]
mod utils_traverse_expression_traverse_member_expression_traverse_access_path_resolve_expression;
#[path = "utils/traverse-expression/traverse-member-expression/traverse-access-path/resolve-expression/function-args.rs"]
mod utils_traverse_expression_traverse_member_expression_traverse_access_path_resolve_expression_function_args;
#[path = "utils/traverse-expression/traverse-member-expression/traverse-access-path/resolve-expression/identifier.rs"]
mod utils_traverse_expression_traverse_member_expression_traverse_access_path_resolve_expression_identifier;
#[path = "utils/traverse-expression/traverse-unary-expression.rs"]
mod utils_traverse_expression_traverse_unary_expression;
#[path = "utils/traversers/index.rs"]
mod utils_traversers;
#[path = "utils/traversers/get-export.rs"]
mod utils_traversers_get_export;
#[path = "utils/traversers/object.rs"]
mod utils_traversers_object;
#[path = "utils/traversers/set_imported_compiled_imports.rs"]
mod utils_traversers_set_imported_compiled_imports;
#[path = "utils/traversers/types.rs"]
mod utils_traversers_types;
#[path = "utils/types.rs"]
mod utils_types;

pub use errors::{TransformError, TransformResult};
pub use index_module::*;
pub use postcss::{SortOptions, sort_atomic_style_sheet};
pub use types::*;
