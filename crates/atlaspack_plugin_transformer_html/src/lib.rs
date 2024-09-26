use std::cell::RefCell;
use std::io::BufReader;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Error;
use html5ever::serialize::SerializeOpts;
use html5ever::tendril::fmt::UTF8;
use html5ever::tendril::TendrilSink;
use html5ever::{serialize, ParseOpts};
use markup5ever::tendril::Tendril;
use markup5ever::{
  expanded_name, local_name, namespace_url, ns, Attribute, ExpandedName, QualName,
};
use markup5ever_rcdom::{Node, NodeData, RcDom, SerializableHandle};

use atlaspack_core::plugin::{PluginContext, TransformContext, TransformResult, TransformerPlugin};
use atlaspack_core::types::{
  Asset, AssetId, BundleBehavior, Code, Dependency, Environment, FileType, OutputFormat, Priority,
  SourceType, SpecifierType,
};

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
      source_asset_id: Some(input.id.clone()),
      source_path: Some(input.file_path.clone()),
      env: context.env().clone(),
      // TODO: Where is this?
      enable_hmr: false,
    };
    let ExtractDependenciesOutput { dependencies, .. } =
      run_html_transformations(context, &mut dom);
    let output_bytes = serialize_html(dom)?;

    let mut asset = input;
    asset.bundle_behavior = Some(BundleBehavior::Isolated);
    asset.code = Arc::new(Code::new(output_bytes));

    Ok(TransformResult {
      asset,
      dependencies,
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

#[derive(PartialEq, Eq)]
enum DomTraversalOperation {
  Continue,
  Stop,
}

trait DomVisitor {
  fn visit_node(&mut self, node: &Node) -> DomTraversalOperation;
}

fn walk(node: Rc<Node>, visitor: &mut impl DomVisitor) {
  let mut queue = vec![node.clone()];
  while let Some(node) = queue.pop() {
    let operation = visitor.visit_node(&node);
    if operation == DomTraversalOperation::Stop {
      break;
    }
    let borrow = node.children.borrow();
    for child in borrow.iter() {
      queue.push(child.clone());
    }
  }
}

struct AttrWrapper<'a> {
  attributes: &'a mut Vec<Attribute>,
}

impl<'a> AttrWrapper<'a> {
  pub fn new(attributes: &'a mut Vec<Attribute>) -> Self {
    Self { attributes }
  }

  pub fn get(&self, name: ExpandedName) -> Option<&Tendril<UTF8>> {
    self
      .attributes
      .iter()
      .find(|attr| attr.name.expanded() == name)
      .map(|attr| &attr.value)
  }

  pub fn delete(&mut self, name: ExpandedName) {
    *self.attributes = self
      .attributes
      .iter()
      .filter(|attr| attr.name.expanded() != name)
      .cloned()
      .collect();
  }

  fn set(&mut self, name: ExpandedName, value: &str) {
    if let Some(attribute) = self
      .attributes
      .iter_mut()
      .find(|attr| attr.name.expanded() == name)
    {
      attribute.value = value.into();
    } else {
      self.attributes.push(Attribute {
        name: QualName::new(None, name.ns.clone(), name.local.clone()),
        value: value.into(),
      });
    }
  }
}

/// Find all <script ...>, <link ...>, <a ...> etc. tags and create dependencies
/// that correspond to them.
#[derive(Default)]
struct ExtractDependencies {
  context: Rc<HTMLTransformationContext>,
  dependencies: Vec<Dependency>,
}

impl ExtractDependencies {
  fn new(context: Rc<HTMLTransformationContext>) -> Self {
    ExtractDependencies {
      context,
      ..Default::default()
    }
  }
}

impl DomVisitor for ExtractDependencies {
  fn visit_node(&mut self, node: &Node) -> DomTraversalOperation {
    match &node.data {
      NodeData::Document => {}
      NodeData::Doctype { .. } => {}
      NodeData::Text { .. } => {}
      NodeData::Comment { .. } => {}
      NodeData::Element { name, attrs, .. } => match name.expanded() {
        // TODO: Handle meta
        expanded_name!(html "meta") => {}
        // TODO: Handle link
        expanded_name!(html "link") => {}
        expanded_name!(html "script") => {
          let mut attrs = attrs.borrow_mut();
          let mut attrs = AttrWrapper::new(&mut *attrs);
          if let Some(src_value) = attrs.get(expanded_name!("", "src")) {
            let src_string = src_value.to_string();
            let source_type = if attrs.get(expanded_name!("", "type")) == Some(&"module".into()) {
              SourceType::Module
            } else {
              SourceType::Script
            };

            let mut _output_format = OutputFormat::Global;
            if self.context.env.should_scope_hoist {
              _output_format = OutputFormat::EsModule;
            } else {
              if source_type == SourceType::Module {
                attrs.set(expanded_name!("", "defer"), "");
              }
              attrs.delete(expanded_name!("", "type"));
            }

            // TODO: Support non-ESM browsers

            let dependency = Dependency {
              specifier: src_string,
              specifier_type: SpecifierType::Url,
              priority: Priority::Parallel,
              source_asset_type: Some(FileType::Html),
              source_path: self.context.source_path.clone(),
              source_asset_id: self.context.source_asset_id.clone(),
              env: self.context.env.clone(),
              is_esm: source_type == SourceType::Module,
              bundle_behavior: if source_type == SourceType::Script
                && attrs.get(expanded_name!("", "async")).is_some()
              {
                Some(BundleBehavior::Isolated)
              } else {
                None
              },
              ..Default::default()
            };
            let dependency_id = dependency.id();
            self.dependencies.push(dependency);
            attrs.set(expanded_name!("", "src"), &dependency_id);
          }
        }
        _ => {}
      },
      NodeData::ProcessingInstruction { .. } => {}
    }

    DomTraversalOperation::Continue
  }
}

struct ExtractDependenciesOutput {
  dependencies: Vec<Dependency>,
}

#[derive(Default)]
struct HTMLTransformationContext {
  source_path: Option<PathBuf>,
  source_asset_id: Option<AssetId>,
  env: Arc<Environment>,
  enable_hmr: bool,
}

/// Insert a tag for HMR and create its dependency/asset
struct HMRVisitor {
  context: Rc<HTMLTransformationContext>,
}

impl HMRVisitor {
  fn new(context: Rc<HTMLTransformationContext>) -> Self {
    Self { context }
  }
}

impl DomVisitor for HMRVisitor {
  fn visit_node(&mut self, node: &Node) -> DomTraversalOperation {
    match &node.data {
      NodeData::Element { name, .. } => {
        if name.expanded() == expanded_name!(html "body") {
          let mut children = node.children.borrow_mut();
          let mut attrs = vec![];
          {
            let mut attrs = AttrWrapper::new(&mut attrs);
            let dependency = Dependency {
              specifier: "".to_owned(),
              specifier_type: SpecifierType::Url,
              priority: Priority::Parallel,
              source_asset_id: self.context.source_asset_id.clone(),
              env: self.context.env.clone(),
              source_asset_type: Some(FileType::Html),
              source_path: self.context.source_path.clone(),
              ..Default::default()
            };
            let dependency_id = dependency.id();
            attrs.set(expanded_name!("", "src"), &dependency_id);
          }

          let script_node = Node::new(NodeData::Element {
            name: QualName::new(None, ns!(html), local_name!("script")),
            attrs: RefCell::new(attrs),
            template_contents: RefCell::new(None),
            mathml_annotation_xml_integration_point: false,
          });
          children.push(script_node);
          DomTraversalOperation::Stop
        } else {
          DomTraversalOperation::Continue
        }
      }
      _ => DomTraversalOperation::Continue,
    }
  }
}

/// 'Purer' entry-point for all HTML transformations. Do split transformations
/// into smaller functions/visitors rather than doing everything in one pass.
fn run_html_transformations(
  context: HTMLTransformationContext,
  dom: &mut RcDom,
) -> ExtractDependenciesOutput {
  let node = dom.document.clone();
  let context = Rc::new(context);

  // Note that HTML5EVER rc-dom uses interior mutability, so these Rc<...>
  // values are actually mutable and changing at each step.
  let mut visitor = ExtractDependencies::new(context.clone());
  walk(node.clone(), &mut visitor);

  if context.enable_hmr {
    let mut hmr_visitor = HMRVisitor::new(context);
    walk(node, &mut hmr_visitor);
  }

  ExtractDependenciesOutput {
    dependencies: visitor.dependencies,
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_transform_simple_script_tag() {
    let bytes = r#"
<html>
  <body>
    <script src="input.js"></script>
  </body>
</html>
    "#
    .trim();
    let mut dom = parse_html(bytes.as_bytes()).unwrap();
    let mut context = HTMLTransformationContext::default();
    Arc::get_mut(&mut context.env).unwrap().should_optimize = false;
    Arc::get_mut(&mut context.env).unwrap().should_scope_hoist = false;

    run_html_transformations(context, &mut dom);
    let html = String::from_utf8(serialize_html(dom).unwrap()).unwrap();
    assert_eq!(
      &normalize_html(&html),
      &normalize_html(
        r#"
<html>
  <body>
    <script src="58210649f694bf7e"></script>
  </body>
</html>
    "#
      )
    );
  }

  #[test]
  fn test_insert_hmr_tag() {
    let bytes = r#"
<html>
  <body>
  </body>
</html>
    "#
    .trim();
    let mut dom = parse_html(bytes.as_bytes()).unwrap();
    let mut context = HTMLTransformationContext::default();
    context.enable_hmr = true;

    run_html_transformations(context, &mut dom);
    let html = String::from_utf8(serialize_html(dom).unwrap()).unwrap();
    assert_eq!(
      &normalize_html(&html),
      &normalize_html(
        r#"
<html>
  <body>
    <script src="e4b9d27bade2678d"></script>
  </body>
</html>
    "#
      )
    );
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
