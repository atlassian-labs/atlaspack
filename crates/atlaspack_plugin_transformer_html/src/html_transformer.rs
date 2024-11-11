use std::io::BufReader;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Error;
use async_trait::async_trait;
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
pub struct AtlaspackHtmlTransformerPlugin {
  project_root: PathBuf,
}

impl AtlaspackHtmlTransformerPlugin {
  pub fn new(ctx: &PluginContext) -> Self {
    AtlaspackHtmlTransformerPlugin {
      project_root: ctx.options.project_root.clone(),
    }
  }
}

#[async_trait]
impl TransformerPlugin for AtlaspackHtmlTransformerPlugin {
  async fn transform(
    &self,
    context: TransformContext,
    input: Asset,
  ) -> Result<TransformResult, Error> {
    let bytes: &[u8] = input.code.bytes();
    let mut dom = parse_html(bytes)?;
    let context = HTMLTransformationContext {
      // TODO: Where is this?
      enable_hmr: false,
      env: context.env().clone(),
      project_root: self.project_root.clone(),
      side_effects: input.side_effects,
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
    asset.code = Code::new(serialize_html(dom)?);

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
  pub project_root: PathBuf,
  pub side_effects: bool,
  pub source_asset_id: AssetId,
  pub source_path: Option<PathBuf>,
}

#[derive(Debug, PartialEq)]
struct HtmlTransformation {
  dependencies: Vec<Arc<Dependency>>,
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
    dependencies: dependencies_visitor
      .dependencies
      .into_iter()
      .map(|d| Arc::new(d))
      .collect(),
    discovered_assets,
  }
}

#[cfg(test)]
mod test {
  use atlaspack_core::types::{FileType, JSONObject, SourceType};
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
    "#;

    let mut dom = parse_html(bytes.as_bytes()).unwrap();

    run_html_transformations(transformation_context(), &mut dom);

    let html = String::from_utf8(serialize_html(dom).unwrap()).unwrap();

    assert_eq!(
      &normalize_html(&html),
      &normalize_html(
        r#"
          <html>
            <body>
              <script src="966f7b31c3f6c3fc"></script>
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
    "#;

    let mut dom = parse_html(bytes.as_bytes()).unwrap();

    run_html_transformations(transformation_context(), &mut dom);

    let html = String::from_utf8(serialize_html(dom).unwrap()).unwrap();

    assert_eq!(
      &normalize_html(&html),
      &normalize_html(
        r#"
          <html>
            <head>
              <link href="492e1268e5326028" rel="manifest" />
            </head>
            <body></body>
          </html>
        "#
      )
    );
  }

  #[test]
  fn transforms_inline_script_tag() {
    let script = String::from(
      "
        const main = () => {
          console.log('test');
        }
      ",
    )
    .trim()
    .to_string();

    let bytes = html_body(&format!(
      r#"
        <script type="text/javascript">{script}</script>
      "#,
    ));

    let mut dom = parse_html(bytes.as_bytes()).unwrap();
    let context = transformation_context();

    let transformation = run_html_transformations(context.clone(), &mut dom);
    let html = String::from_utf8(serialize_html(dom).unwrap()).unwrap();

    assert_eq!(
      &normalize_html(&html),
      &normalize_html(&html_body(&format!(
        r#"
          <script data-parcel-key="16f87d7beed96467">{script}</script>
        "#
      )))
    );

    let env = Arc::new(Environment {
      source_type: SourceType::Script,
      ..Environment::default()
    });

    assert_eq!(
      transformation,
      HtmlTransformation {
        dependencies: vec![Dependency {
          bundle_behavior: Some(BundleBehavior::Inline),
          env: env.clone(),
          source_asset_id: Some(String::from("test")),
          source_asset_type: Some(FileType::Html),
          source_path: Some(PathBuf::from("main.html")),
          specifier: String::from("16f87d7beed96467"),
          ..Dependency::default()
        }],
        discovered_assets: vec![AssetWithDependencies {
          asset: Asset {
            bundle_behavior: Some(BundleBehavior::Inline),
            code: Code::from(script),
            env: env.clone(),
            file_path: PathBuf::from("main.html"),
            file_type: FileType::Js,
            id: String::from("b0deada2a458cc5f"),
            is_bundle_splittable: true,
            is_source: true,
            meta: JSONObject::from_iter([(String::from("type"), "tag".into())]),
            unique_key: Some(String::from("16f87d7beed96467")),
            ..Asset::default()
          },
          dependencies: Vec::new()
        }],
      }
    );
  }

  #[test]
  fn transforms_inline_style_tag() {
    let bytes = html_body(
      "
        <style>
          a { color: blue; }
        </style>
      ",
    );

    let mut dom = parse_html(bytes.as_bytes()).unwrap();
    let context = transformation_context();

    let transformation = run_html_transformations(context.clone(), &mut dom);
    let html = String::from_utf8(serialize_html(dom).unwrap()).unwrap();

    assert_eq!(
      &normalize_html(&html),
      &normalize_html(&html_body(
        r#"
          <style data-parcel-key="16f87d7beed96467">
            a { color: blue; }
          </style>
        "#
      ))
    );

    assert_eq!(
      transformation,
      HtmlTransformation {
        dependencies: vec![Dependency {
          source_asset_id: Some(String::from("test")),
          source_asset_type: Some(FileType::Html),
          source_path: Some(PathBuf::from("main.html")),
          specifier: String::from("16f87d7beed96467"),
          ..Dependency::default()
        }],
        discovered_assets: vec![AssetWithDependencies {
          asset: Asset {
            bundle_behavior: Some(BundleBehavior::Inline),
            code: Code::from(String::from("\n          a { color: blue; }\n        ")),
            file_path: PathBuf::from("main.html"),
            file_type: FileType::Css,
            id: String::from("9ee0b2d6680a3e8d"),
            is_bundle_splittable: true,
            is_source: true,
            meta: JSONObject::from_iter([(String::from("type"), "tag".into())]),
            unique_key: Some(String::from("16f87d7beed96467")),
            ..Asset::default()
          },
          dependencies: Vec::new()
        }],
      }
    );
  }

  #[test]
  fn inserts_hmr_tag() {
    let bytes = html_body("");
    let context = HTMLTransformationContext {
      enable_hmr: true,
      ..transformation_context()
    };

    let mut dom = parse_html(bytes.as_bytes()).unwrap();

    run_html_transformations(context, &mut dom);

    let html = String::from_utf8(serialize_html(dom).unwrap()).unwrap();

    assert_eq!(
      &normalize_html(&html),
      &normalize_html(&html_body(r#"<script src="8321472594eb517f"></script>"#))
    );
  }

  fn transformation_context() -> HTMLTransformationContext {
    let mut context = HTMLTransformationContext::default();

    Arc::get_mut(&mut context.env).unwrap().should_optimize = false;
    Arc::get_mut(&mut context.env).unwrap().should_scope_hoist = false;
    context.source_path = Some(PathBuf::from("main.html"));
    context.source_asset_id = String::from("test");

    context
  }

  fn html_body(body: &str) -> String {
    format!(
      r#"
        <html>
          <body>
            {}
          </body>
        </html>
      "#,
      body.trim()
    )
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
