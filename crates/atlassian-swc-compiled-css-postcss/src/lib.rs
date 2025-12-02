pub mod ast;
pub mod css_syntax_error;
pub mod from_json;
pub mod input;
pub mod list;
pub mod parse;
pub mod processor;
pub mod result;
pub mod source_map;
pub mod stringifier;
pub mod terminal_highlight;
pub mod to_json;
pub mod vendor;
pub mod warn_once;

pub use ast::nodes::*;
pub use ast::{PositionByOptions, RangeByOptions};
pub use css_syntax_error::{CssSyntaxError, ErrorInput};
pub use from_json::{from_json, from_json_str, FromJsonError, FromJsonOutput};
pub use list::{comma, space, split};
pub use parse::parse;
pub use processor::{
  plugin, BuiltPlugin, CustomParser, CustomStringifier, IntoParser, IntoPlugin, IntoStringifier,
  LazyResult, NoWorkResult, Plugin, PluginBuilder, ProcessOptions, ProcessResult, Processor,
  SyntaxOptions,
};
pub use result::{Message, ProcessorMetadata, Result, ResultOptions, Warning, WarningOptions};
pub use stringifier::stringify;
pub use to_json::{to_json, to_json_nodes};
pub use vendor::{prefix as vendor_prefix, unprefixed as vendor_unprefixed};

pub fn postcss() -> Processor {
  Processor::new()
}

pub fn postcss_with_plugins<I, P>(plugins: I) -> Processor
where
  I: IntoIterator<Item = P>,
  P: IntoPlugin,
{
  Processor::from_plugins(plugins)
}

pub fn root() -> ast::nodes::Root {
  ast::nodes::Root::new()
}

pub fn document() -> ast::nodes::Document {
  ast::nodes::Document::new()
}

pub fn rule(selector: impl Into<String>) -> ast::nodes::Rule {
  ast::nodes::Rule::new(selector)
}

pub fn at_rule(name: impl Into<String>) -> ast::nodes::AtRule {
  ast::nodes::AtRule::new(name)
}

pub fn decl(prop: impl Into<String>, value: impl Into<String>) -> ast::nodes::Declaration {
  ast::nodes::Declaration::new(prop, value)
}

pub fn comment(text: impl Into<String>) -> ast::nodes::Comment {
  ast::nodes::Comment::new(text)
}
