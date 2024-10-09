use std::io::BufReader;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Error;
use html5ever::serialize::SerializeOpts;
use html5ever::tendril::TendrilSink;
use html5ever::{serialize, ParseOpts};
use markup5ever_rcdom::{RcDom, SerializableHandle};

use atlaspack_core::plugin::{PluginContext, TransformContext, TransformResult, TransformerPlugin};
use atlaspack_core::types::{
  Asset, AssetId, AssetWithDependencies, BundleBehavior, Code, Dependency, Environment,
};

use crate::dom_visitor::walk;
use crate::hmr_visitor::HMRVisitor;
use crate::html_dependencies_visitor::HtmlDependenciesVisitor;

#[derive(Debug)]
pub struct AtlaspackHtmlTransformerPlugin {}

impl AtlaspackHtmlTransformerPlugin {
  pub fn new(_ctx: &PluginContext) -> Self {
    AtlaspackHtmlTransformerPlugin {}
  }
}

impl TransformerPlugin for AtlaspackHtmlTransformerPlugin {
  fn transform(
    &mut self,
    context: TransformContext,
    input: Asset,
  ) -> Result<TransformResult, Error> {
    let bytes: &[u8] = input.code.bytes();
    let mut dom = parse_html(bytes)?;
    let context = HTMLTransformationContext {
      // TODO: Where is this?
      enable_hmr: false,
      env: context.env().clone(),
      source_asset_id: input.id.clone(),
      source_path: Some(input.file_path.clone()),
    };

    let HtmlTransformation {
      dependencies,
      discovered_assets,
      ..
    } = run_html_transformations(context, &mut dom);

    let mut asset = input;
    asset.bundle_behavior = Some(BundleBehavior::Isolated);
    asset.code = Arc::new(Code::new(serialize_html(dom)?));

    Ok(TransformResult {
      asset,
      dependencies,
      discovered_assets,
      ..Default::default()
    })
  }
}

fn serialize_html(dom: RcDom) -> Result<Vec<u8>, Error> {
  let document: SerializableHandle = dom.document.clone().into();
  let mut output_bytes = vec![];
  let options = SerializeOpts::default();
  serialize(&mut output_bytes, &document, options)?;
  Ok(output_bytes)
}

fn parse_html(bytes: &[u8]) -> Result<RcDom, Error> {
  let mut bytes = BufReader::new(bytes);
  let options = ParseOpts::default();
  let dom = RcDom::default();
  let dom = html5ever::parse_document(dom, options)
    .from_utf8()
    .read_from(&mut bytes)?;
  Ok(dom)
}

#[derive(Clone, Default)]
pub struct HTMLTransformationContext {
  pub enable_hmr: bool,
  pub env: Arc<Environment>,
  pub source_asset_id: AssetId,
  pub source_path: Option<PathBuf>,
}

#[derive(Debug, PartialEq)]
struct HtmlTransformation {
  dependencies: Vec<Dependency>,
  discovered_assets: Vec<AssetWithDependencies>,
}

/// 'Purer' entry-point for all HTML transformations. Do split transformations
/// into smaller functions/visitors rather than doing everything in one pass.
fn run_html_transformations(
  context: HTMLTransformationContext,
  dom: &mut RcDom,
) -> HtmlTransformation {
  let node = dom.document.clone();
  let context = Rc::new(context);

  // Note that HTML5EVER rc-dom uses interior mutability, so these Rc<...>
  // values are actually mutable and changing at each step.
  let mut dependencies_visitor = HtmlDependenciesVisitor::new(context.clone());
  walk(node.clone(), &mut dependencies_visitor);

  let discovered_assets = dependencies_visitor.discovered_assets;

  if context.enable_hmr {
    let mut hmr_visitor = HMRVisitor::new(context);
    walk(node.clone(), &mut hmr_visitor);
    // TODO This creates an infinite loop, but should be added properly later
    // if let Some(asset) = hmr_visitor.hmr_asset {
    //   discovered_assets.push(asset);
    // }
  }

  HtmlTransformation {
    dependencies: dependencies_visitor.dependencies,
    discovered_assets,
  }
}

#[cfg(test)]
mod test {
  use atlaspack_core::types::{FileType, JSONObject};
  use pretty_assertions::assert_eq;

  use super::*;

  #[test]
  fn transforms_external_script_tag() {
    let bytes = r#"
      <html>
        <body>
          <script src="input.js"></script>
        </body>
      </html>
    "#
    .trim();

    let mut dom = parse_html(bytes.as_bytes()).unwrap();

    run_html_transformations(transformation_context(), &mut dom);

    let html = String::from_utf8(serialize_html(dom).unwrap()).unwrap();
    assert_eq!(
      &normalize_html(&html),
      &normalize_html(
        r#"
          <html>
            <body>
              <script src="5ad926ae3ae30d11"></script>
            </body>
          </html>
        "#
      )
    );
  }

  #[test]
  fn transforms_manifest_link_tag() {
    let bytes = r#"
      <html>
        <head>
          <link href="manifest.json" rel="manifest" />
        </head>
      </html>
    "#
    .trim();

    let mut dom = parse_html(bytes.as_bytes()).unwrap();

    run_html_transformations(transformation_context(), &mut dom);

    let html = String::from_utf8(serialize_html(dom).unwrap()).unwrap();
    assert_eq!(
      &normalize_html(&html),
      &normalize_html(
        r#"
          <html>
            <head>
              <link href="b7e43ce21336e8fb" rel="manifest" />
            </head>
            <body></body>
          </html>
        "#
      )
    );
  }

  #[test]
  fn transforms_inline_style_tag() {
    let bytes = r#"
      <html>
        <body>
          <style>
            a { color: blue; }
          </style>
        </body>
      </html>
    "#
    .trim();

    let mut dom = parse_html(bytes.as_bytes()).unwrap();
    let context = transformation_context();

    let transformation = run_html_transformations(context.clone(), &mut dom);

    let html = String::from_utf8(serialize_html(dom).unwrap()).unwrap();
    assert_eq!(&normalize_html(&html), &normalize_html(&bytes));

    assert_eq!(
      transformation,
      HtmlTransformation {
        dependencies: vec![Dependency {
          source_asset_id: Some(String::from("test")),
          source_asset_type: Some(FileType::Html),
          specifier: String::from("test:0"),
          ..Dependency::default()
        }],
        discovered_assets: vec![AssetWithDependencies {
          asset: Asset {
            bundle_behavior: Some(BundleBehavior::Inline),
            code: Arc::new(Code::from(String::from(
              "\n            a { color: blue; }\n          "
            ))),
            file_type: FileType::Css,
            meta: JSONObject::from_iter([(String::from("type"), "tag".into())]),
            unique_key: Some(String::from("test:0")),
            ..Asset::default()
          },
          dependencies: Vec::new()
        }],
      }
    );
  }

  #[test]
  fn inserts_hmr_tag() {
    let bytes = r#"
      <html>
        <body>
        </body>
      </html>
    "#
    .trim();

    let context = HTMLTransformationContext {
      enable_hmr: true,
      ..transformation_context()
    };

    let mut dom = parse_html(bytes.as_bytes()).unwrap();

    run_html_transformations(context, &mut dom);

    let html = String::from_utf8(serialize_html(dom).unwrap()).unwrap();
    assert_eq!(
      &normalize_html(&html),
      &normalize_html(
        r#"
          <html>
            <body>
              <script src="aaa87d49a66b51d6"></script>
            </body>
          </html>
        "#
      )
    );
  }

  fn transformation_context() -> HTMLTransformationContext {
    let mut context = HTMLTransformationContext::default();

    Arc::get_mut(&mut context.env).unwrap().should_optimize = false;
    Arc::get_mut(&mut context.env).unwrap().should_scope_hoist = false;
    context.source_asset_id = String::from("test");

    context
  }

  fn normalize_html(html: &str) -> String {
    let html = parse_html(html.as_bytes()).unwrap();
    let output = String::from_utf8(serialize_html(html).unwrap()).unwrap();
    output
      .lines()
      .map(|line| line.trim())
      .filter(|line| !line.is_empty())
      .collect()
  }
}
