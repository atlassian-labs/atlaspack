use std::{rc::Rc, sync::Arc};

use html5ever::serialize::serialize;
use markup5ever::{expanded_name, local_name, namespace_url, ns};
use markup5ever_rcdom::{Handle, NodeData, SerializableHandle};

use atlaspack_core::types::{
  Asset, AssetWithDependencies, BundleBehavior, Code, Dependency, FileType, JSONObject,
  OutputFormat, Priority, SourceType, SpecifierType,
};

use crate::{
  attrs::Attrs,
  dom_visitor::{DomTraversalOperation, DomVisitor},
  html_transformer::HTMLTransformationContext,
};

/// Find all <script ...>, <link ...>, <a ...> etc. tags and create dependencies that correspond
/// to them.
#[derive(Default)]
pub struct HtmlDependenciesVisitor {
  context: Rc<HTMLTransformationContext>,
  pub dependencies: Vec<Dependency>,
  pub discovered_assets: Vec<AssetWithDependencies>,
}

impl HtmlDependenciesVisitor {
  pub fn new(context: Rc<HTMLTransformationContext>) -> Self {
    HtmlDependenciesVisitor {
      context,
      ..Default::default()
    }
  }
}

impl DomVisitor for HtmlDependenciesVisitor {
  fn visit_node(&mut self, node: Handle) -> DomTraversalOperation {
    match &node.data {
      NodeData::Element { name, attrs, .. } => match name.expanded() {
        expanded_name!(html "link") => {
          let mut attrs = attrs.borrow_mut();
          let mut attrs = Attrs::new(&mut *attrs);

          if let Some(href) = attrs.get(expanded_name!("", "href")) {
            let rel = attrs.get(expanded_name!("", "rel"));
            if rel.map(|r| r.to_string()).is_some_and(|r| r != "manifest") {
              return DomTraversalOperation::Continue;
            }

            let dependency = Dependency {
              env: self.context.env.clone(),
              priority: Priority::Lazy,
              source_asset_id: Some(self.context.source_asset_id.clone()),
              source_asset_type: Some(FileType::Html),
              source_path: self.context.source_path.clone(),
              specifier: href.to_string(),
              specifier_type: SpecifierType::Url,
              ..Dependency::default()
            };

            let dependency_id = dependency.id();
            self.dependencies.push(dependency);
            attrs.set(expanded_name!("", "href"), &dependency_id);
          }
        }
        // TODO: Handle meta
        expanded_name!(html "meta") => {}
        // TODO: Inline scripts
        expanded_name!(html "script") => {
          let mut attrs = attrs.borrow_mut();
          let mut attrs = Attrs::new(&mut *attrs);
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
              bundle_behavior: if source_type == SourceType::Script
                && attrs.get(expanded_name!("", "async")).is_some()
              {
                Some(BundleBehavior::Isolated)
              } else {
                None
              },
              env: self.context.env.clone(),
              is_esm: source_type == SourceType::Module,
              priority: Priority::Parallel,
              source_asset_id: Some(self.context.source_asset_id.clone()),
              source_asset_type: Some(FileType::Html),
              source_path: self.context.source_path.clone(),
              specifier: src_string,
              specifier_type: SpecifierType::Url,
              ..Default::default()
            };

            let dependency_id = dependency.id();
            self.dependencies.push(dependency);
            attrs.set(expanded_name!("", "src"), &dependency_id);
          }
        }
        expanded_name!(html "style") => {
          let mut attrs = attrs.borrow_mut();
          let attrs = Attrs::new(&mut *attrs);

          let type_attr = attrs.get(expanded_name!("", "type"));
          let file_type = type_attr
            .and_then(|t| t.split("/").collect::<Vec<&str>>().get(1).cloned())
            .map(|ext| FileType::from_extension(ext))
            .unwrap_or(FileType::Css);

          let specifier = format!(
            "{}:{}",
            self.context.source_asset_id.clone(),
            self.discovered_assets.len()
          );

          self.dependencies.push(Dependency {
            env: self.context.env.clone(),
            source_asset_id: Some(self.context.source_asset_id.clone()),
            source_asset_type: Some(FileType::Html),
            source_path: self.context.source_path.clone(),
            specifier: specifier.clone(),
            specifier_type: SpecifierType::Esm,
            ..Dependency::default()
          });

          let handle = SerializableHandle::from(node.clone());
          let mut styles = Vec::new();

          serialize(&mut styles, &handle, Default::default())
            .expect("Inline style serialization failed");

          // TODO: How to best handle id construction?
          self.discovered_assets.push(AssetWithDependencies {
            asset: Asset {
              bundle_behavior: Some(BundleBehavior::Inline),
              code: Arc::new(Code::new(styles)),
              env: self.context.env.clone(),
              file_type,
              meta: JSONObject::from_iter([(String::from("type"), "tag".into())]),
              unique_key: Some(specifier),
              ..Asset::default()
            },
            dependencies: Vec::new(),
          });

          return DomTraversalOperation::Stop;
        }
        _ => {}
      },
      _ => {}
    }

    DomTraversalOperation::Continue
  }
}
