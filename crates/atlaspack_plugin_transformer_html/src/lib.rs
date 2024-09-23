use std::cell::RefMut;
use std::io::BufReader;
use std::ops::DerefMut;
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

use atlaspack_core::plugin::{PluginContext, TransformResult, TransformerPlugin};
use atlaspack_core::types::{
  Asset, AssetId, BundleBehavior, Code, Dependency, Environment, OutputFormat, Priority,
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
  fn transform(&mut self, input: Asset) -> Result<TransformResult, Error> {
    let bytes: &[u8] = input.code.bytes();
    let mut dom = parse_html(bytes)?;
    let context = ExtractDependenciesContext {
      source_path: Some(input.file_path.clone()),
    };
    let ExtractDependenciesOutput { dependencies, .. } = extract_dependencies(context, &mut dom);
    let output_bytes = serialize_html(dom)?;

    let mut asset = input;
    asset.bundle_behavior = Some(BundleBehavior::Isolated);
    asset.code = Arc::new(Code::new(output_bytes));

    Ok(TransformResult {
      asset,
      dependencies,
      invalidate_on_file_change: Vec::new(),
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

trait DomVisitor {
  fn visit_node(&mut self, node: &Node);
}

fn walk(node: Rc<Node>, visitor: &mut impl DomVisitor) {
  let mut queue = vec![node.clone()];
  while let Some(node) = queue.pop() {
    visitor.visit_node(&node);
    let borrow = node.children.borrow();
    for child in borrow.iter() {
      queue.push(child.clone());
    }
  }
}

struct AttrWrapper<'a> {
  attributes: RefMut<'a, Vec<Attribute>>,
}

impl<'a> AttrWrapper<'a> {
  pub fn new(attributes: RefMut<'a, Vec<Attribute>>) -> Self {
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
    let attributes = self.attributes.deref_mut();
    *attributes = attributes
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
      let attributes = self.attributes.deref_mut();
      attributes.push(Attribute {
        name: QualName::new(None, name.ns.clone(), name.local.clone()),
        value: value.into(),
      });
    }
  }
}

#[derive(Default)]
struct ExtractDependencies {
  context: ExtractDependenciesContext,
  should_scope_hoist: bool,
  dependencies: Vec<Dependency>,
  source_asset_id: Option<AssetId>,
  env: Arc<Environment>,
}

impl ExtractDependencies {
  fn new(context: ExtractDependenciesContext) -> Self {
    ExtractDependencies {
      context,
      ..Default::default()
    }
  }
}

impl DomVisitor for ExtractDependencies {
  fn visit_node(&mut self, node: &Node) {
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
          let attrs = attrs.borrow_mut();
          let mut attrs = AttrWrapper::new(attrs);
          if let Some(src_value) = attrs.get(expanded_name!("", "src")) {
            let src_string = src_value.to_string();
            let source_type = if attrs.get(expanded_name!("", "type")) == Some(&"module".into()) {
              SourceType::Module
            } else {
              SourceType::Script
            };

            let mut _output_format = OutputFormat::Global;
            if self.should_scope_hoist {
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
              source_asset_id: self.source_asset_id.clone(),
              env: self.env.clone(),
              source_path: self.context.source_path.clone(),
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
  }
}

struct ExtractDependenciesOutput {
  dependencies: Vec<Dependency>,
}

#[derive(Default)]
struct ExtractDependenciesContext {
  source_path: Option<PathBuf>,
}

fn extract_dependencies(
  context: ExtractDependenciesContext,
  dom: &mut RcDom,
) -> ExtractDependenciesOutput {
  let node = dom.document.clone();
  let mut visitor = ExtractDependencies::new(context);
  walk(node, &mut visitor);
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
    let context = ExtractDependenciesContext::default();
    extract_dependencies(context, &mut dom);
    let html = String::from_utf8(serialize_html(dom).unwrap()).unwrap();
    assert_eq!(
      &normalize_html(&html),
      &normalize_html(
        r#"
<html>
  <body>
    <script src="4d82d7c15de63fc0"></script>
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
