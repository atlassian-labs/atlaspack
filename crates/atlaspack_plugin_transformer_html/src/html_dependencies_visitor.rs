use std::hash::{Hash, Hasher};
use std::rc::Rc;

use html5ever::{serialize::serialize, ExpandedName};
use markup5ever::{expanded_name, local_name, namespace_url, ns, LocalName};
use markup5ever_rcdom::{Handle, NodeData, SerializableHandle};

use atlaspack_core::{
  hash::IdentifierHasher,
  types::{
    Asset, BundleBehavior, Code, Dependency, FileType, JSONObject, OutputFormat, Priority,
    SourceType, SpecifierType,
  },
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
  pub discovered_assets: Vec<Asset>,
}

impl HtmlDependenciesVisitor {
  pub fn new(context: Rc<HTMLTransformationContext>) -> Self {
    HtmlDependenciesVisitor {
      context,
      ..Default::default()
    }
  }

  fn add_url_dependency(&mut self, specifier: String) -> String {
    let dependency = Dependency {
      env: self.context.env.clone(),
      priority: Priority::Lazy,
      source_asset_id: Some(self.context.source_asset_id.clone()),
      source_asset_type: Some(FileType::Html),
      source_path: self.context.source_path.clone(),
      specifier,
      specifier_type: SpecifierType::Url,
      ..Dependency::default()
    };

    let dependency_id = dependency.id();

    self.dependencies.push(dependency);

    dependency_id
  }

  fn add_resource(&mut self, attrs: &mut Attrs, name: ExpandedName) {
    if let Some(url) = attrs.get(name) {
      if url.starts_with("/") {
        return;
      }

      attrs.set(name, &self.add_url_dependency(url.to_string()));
    }
  }

  fn inline_asset_id(&self) -> String {
    let mut hasher = IdentifierHasher::default();

    self.context.source_asset_id.hash(&mut hasher);
    self.discovered_assets.len().hash(&mut hasher);

    // Ids must be 16 characters for scope hoisting to replace imports correctly in REPLACEMENT_RE
    format!("{:016x}", hasher.finish())
  }
}

impl DomVisitor for HtmlDependenciesVisitor {
  fn visit_node(&mut self, node: Handle) -> DomTraversalOperation {
    match &node.data {
      NodeData::Element { name, attrs, .. } => {
        let mut attrs = attrs.borrow_mut();
        let mut attrs = Attrs::new(&mut *attrs);

        match name.expanded() {
          expanded_name!(html "link") => {
            // TODO: imagesrcset
            if let Some(href) = attrs.get(expanded_name!("", "href")) {
              let rel = attrs.get(expanded_name!("", "rel"));
              if rel.map(|r| r.to_string()).is_some_and(|r| r != "manifest") {
                return DomTraversalOperation::Continue;
              }

              attrs.set(
                expanded_name!("", "href"),
                &self.add_url_dependency(href.to_string()),
              );
            }
          }
          // TODO: Handle meta
          expanded_name!(html "meta") => {}
          expanded_name!(html "script") => {
            let src_attr = attrs.get(expanded_name!("", "src")).map(|s| s.to_string());
            let type_attr = attrs.get(expanded_name!("", "type")).map(|t| t.to_string());

            let source_type = if type_attr == Some("module".into()) {
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

            let specifier = match src_attr.as_ref() {
              None => self.inline_asset_id(),
              Some(src) => src.to_string(),
            };

            let dependency = Dependency {
              bundle_behavior: if src_attr.is_none() {
                Some(BundleBehavior::Inline)
              } else if source_type == SourceType::Script
                && attrs.get(expanded_name!("", "async")).is_some()
              {
                Some(BundleBehavior::Isolated)
              } else {
                None
              },
              env: self.context.env.clone(),
              is_esm: source_type == SourceType::Module,
              priority: match src_attr {
                None => Priority::Sync,
                Some(_) => Priority::Parallel,
              },
              source_asset_id: Some(self.context.source_asset_id.clone()),
              source_asset_type: Some(FileType::Html),
              source_path: self.context.source_path.clone(),
              specifier: specifier.clone(),
              specifier_type: match src_attr {
                None => SpecifierType::Esm,
                Some(_) => SpecifierType::Url,
              },
              ..Default::default()
            };

            let dependency_id = dependency.id();
            self.dependencies.push(dependency);

            if src_attr.is_some() {
              attrs.set(expanded_name!("", "src"), &dependency_id);
            } else {
              let handle = SerializableHandle::from(node.clone());
              let mut script = Vec::new();

              serialize(&mut script, &handle, Default::default())
                .expect("Inline script serialization failed");

              attrs.set(
                ExpandedName {
                  ns: &ns!(),
                  local: &LocalName::from("data-parcel-key"),
                },
                &specifier,
              );

              let file_type = type_attr
                .as_ref()
                .and_then(|t| match t.as_str() {
                  "module" => Some(FileType::Js),
                  t => t
                    .split("/")
                    .collect::<Vec<&str>>()
                    .get(1)
                    .cloned()
                    .map(|subtype| match subtype {
                      "javascript" => FileType::Js,
                      ext => FileType::from_extension(ext),
                    }),
                })
                .unwrap_or(FileType::Js);

              self.discovered_assets.push(Asset::new_inline(
                Code::new(script),
                self.context.env.clone(),
                file_type,
                JSONObject::from_iter([(String::from("type"), "tag".into())]),
                Some(specifier),
              ));

              return DomTraversalOperation::Continue;
            }
          }
          expanded_name!(html "style") => {
            let type_attr = attrs.get(expanded_name!("", "type"));
            let file_type = type_attr
              .and_then(|t| t.split("/").collect::<Vec<&str>>().get(1).cloned())
              .map(|ext| FileType::from_extension(ext))
              .unwrap_or(FileType::Css);

            let specifier = self.inline_asset_id();

            attrs.set(
              ExpandedName {
                ns: &ns!(),
                local: &LocalName::from("data-parcel-key"),
              },
              &specifier,
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

            self.discovered_assets.push(Asset::new_inline(
              Code::new(styles),
              self.context.env.clone(),
              file_type,
              JSONObject::from_iter([(String::from("type"), "tag".into())]),
              Some(specifier),
            ));

            return DomTraversalOperation::Continue;
          }
          expanded_name!(html "a") | expanded_name!(svg "image") | expanded_name!(svg "use") => {
            if let Some(href) = attrs.get(expanded_name!("", "href")) {
              if href.starts_with("/") || href.starts_with("#") {
                return DomTraversalOperation::Continue;
              }

              attrs.set(
                expanded_name!("", "href"),
                &self.add_url_dependency(href.to_string()),
              );
            }
          }
          expanded_name!(html "img") | expanded_name!(html "source") => {
            self.add_resource(&mut attrs, expanded_name!("", "src"));
            // TODO: srcset
          }
          expanded_name!(html "audio")
          | expanded_name!(html "embed")
          | expanded_name!(html "iframe")
          | expanded_name!(html "track") => {
            self.add_resource(&mut attrs, expanded_name!("", "src"));
          }
          expanded_name!(html "video") => {
            self.add_resource(&mut attrs, expanded_name!("", "poster"));
            self.add_resource(&mut attrs, expanded_name!("", "src"));
          }
          expanded_name!(html "object") => {
            self.add_resource(&mut attrs, expanded_name!("", "data"));
          }
          _ => {}
        }
      }
      _ => {}
    }

    DomTraversalOperation::Continue
  }
}
