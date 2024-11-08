use std::cell::RefCell;
use std::rc::Rc;

use html5ever::namespace_url;
use markup5ever::{expanded_name, local_name, ns, QualName};
use markup5ever_rcdom::{Handle, Node, NodeData};

use atlaspack_core::types::{Asset, Dependency, FileType, Priority, SpecifierType};

use crate::{
  attrs::Attrs,
  dom_visitor::{DomTraversalOperation, DomVisitor},
  html_transformer::HTMLTransformationContext,
};

/// Insert a tag for HMR and create its dependency and asset
#[derive(Default)]
pub struct HMRVisitor {
  pub hmr_asset: Option<Asset>,
  context: Rc<HTMLTransformationContext>,
}

impl HMRVisitor {
  pub fn new(context: Rc<HTMLTransformationContext>) -> Self {
    Self {
      context,
      ..HMRVisitor::default()
    }
  }
}

impl DomVisitor for HMRVisitor {
  fn visit_node(&mut self, node: Handle) -> DomTraversalOperation {
    match &node.data {
      NodeData::Element { name, .. } => {
        if name.expanded() == expanded_name!(html "body") {
          let mut children = node.children.borrow_mut();
          let mut attrs = vec![];
          {
            let mut attrs = Attrs::new(&mut attrs);
            let dependency = Dependency {
              env: self.context.env.clone().into(),
              priority: Priority::Parallel,
              specifier: "".to_owned(),
              specifier_type: SpecifierType::Url,
              source_asset_id: Some(self.context.source_asset_id.clone()),
              source_asset_type: Some(FileType::Html),
              source_path: self.context.source_path.clone(),
              ..Default::default()
            };

            let src = dependency.id();

            self.hmr_asset = Some(Asset {
              file_type: FileType::Js,
              unique_key: Some(src.clone()),
              ..Asset::default()
            });

            attrs.set(expanded_name!("", "src"), &src);
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
