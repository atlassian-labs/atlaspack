use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use atlaspack_core::types::Environment;
use markup5ever::{ExpandedName, LocalName, expanded_name, local_name, namespace_url, ns};
use markup5ever_rcdom::{Handle, NodeData};
use regex::Regex;

use atlaspack_core::{
  hash::IdentifierHasher,
  types::{
    Asset, AssetWithDependencies, BundleBehavior, Dependency, DependencyBuilder, FileType,
    Priority, SourceType, SpecifierType,
  },
};

use crate::{
  attrs::Attrs,
  dom_visitor::{DomTraversalOperation, DomVisitor},
  svg_transformer::SVGTransformationContext,
};

/// Find all SVG dependencies including functional IRIs, href attributes, and inline assets
#[derive(Default)]
pub struct SvgDependenciesVisitor {
  context: Rc<SVGTransformationContext>,
  pub dependencies: Vec<Dependency>,
  pub discovered_assets: Vec<AssetWithDependencies>,
  func_iri_regex: Option<Regex>,
  pub errors: Vec<String>,
}

impl SvgDependenciesVisitor {
  pub fn new(context: Rc<SVGTransformationContext>) -> Self {
    let func_iri_regex =
      Regex::new(r#"url\(\s*(?:['"]([^'"\\]*(?:\\.[^'"\\]*)*)['"]|([^)]*?))\s*(?:\s+[^)]+)?\)"#)
        .ok();

    SvgDependenciesVisitor {
      context,
      func_iri_regex,
      errors: Vec::new(),
      ..Default::default()
    }
  }

  fn add_url_dependency_with_options(
    &mut self,
    specifier: String,
    priority: Priority,
    needs_stable_name: bool,
  ) -> String {
    let mut dependency_builder = DependencyBuilder::default()
      .env(self.context.env.clone())
      .priority(priority)
      .source_asset_id(self.context.source_asset_id.clone())
      .source_asset_type(FileType::Other("svg".to_string()))
      .source_path_option(self.context.source_path.clone())
      .specifier(specifier)
      .specifier_type(SpecifierType::Url);

    if needs_stable_name {
      dependency_builder = dependency_builder.needs_stable_name(true);
    }

    let dependency = dependency_builder.build();
    let dependency_id = dependency.id();
    self.dependencies.push(dependency);
    dependency_id
  }

  fn process_functional_iri_attributes(&mut self, attrs: &mut Attrs) {
    // List of attributes that can contain functional IRI references
    let func_iri_attrs = [
      expanded_name!("", "fill"),
      expanded_name!("", "stroke"),
      expanded_name!("", "clip-path"),
      expanded_name!("", "color-profile"),
      expanded_name!("", "cursor"),
      expanded_name!("", "filter"),
      expanded_name!("", "marker"),
      expanded_name!("", "marker-start"),
      expanded_name!("", "marker-mid"),
      expanded_name!("", "marker-end"),
      expanded_name!("", "mask"),
      // SVG2 attributes that may not be predefined
      ExpandedName {
        ns: &ns!(),
        local: &LocalName::from("shape-inside"),
      },
      ExpandedName {
        ns: &ns!(),
        local: &LocalName::from("shape-subtract"),
      },
      ExpandedName {
        ns: &ns!(),
        local: &LocalName::from("mask-image"),
      },
    ];

    for attr_name in &func_iri_attrs {
      if let Some(attr_value) = attrs.get(*attr_name) {
        let attr_str = attr_value.to_string();

        // Parse all url() references in the attribute value and rewrite them
        if let Some(ref regex) = self.func_iri_regex {
          let mut updated_value = attr_str.clone();

          // Collect all URLs first, then process them in reverse order to avoid offset issues
          let mut matches = Vec::new();
          for caps in regex.captures_iter(&attr_str) {
            if let Some(url_match) = caps.get(1).or_else(|| caps.get(2)) {
              let url = url_match.as_str().trim();
              let unescaped_url = url.replace("\\'", "'").replace("\\\"", "\"");
              if !unescaped_url.is_empty() {
                matches.push((url_match.start(), url_match.end(), unescaped_url));
              }
            }
          }

          // Process matches in reverse order to avoid offset issues
          for (start, end, unescaped_url) in matches.into_iter().rev() {
            // Create dependency with default options (like JS transformer)
            let dependency = DependencyBuilder::default()
              .env(self.context.env.clone())
              .priority(Priority::Sync)
              .source_asset_id(self.context.source_asset_id.clone())
              .source_asset_type(FileType::Other("svg".to_string()))
              .source_path_option(self.context.source_path.clone())
              .specifier(unescaped_url)
              .specifier_type(SpecifierType::Url)
              .build();

            let dependency_id = dependency.id();
            self.dependencies.push(dependency);

            // Replace only the URL part, preserving modifiers like fallback()
            // Always wrap dependency ID in single quotes like JS transformer
            updated_value.replace_range(start..end, &format!("'{}'", dependency_id));
          }

          // Update the attribute if it was modified
          if updated_value != attr_str {
            attrs.set(*attr_name, &updated_value);
          }
        }
      }
    }
  }

  fn process_href_attributes(
    &mut self,
    attrs: &mut Attrs,
    element_name: &str,
  ) -> Result<(), String> {
    // Elements that support href/xlink:href attributes
    let href_elements = [
      "a",
      "altGlyph",
      "animate",
      "animateMotion",
      "animateTransform",
      "circle",
      "cursor",
      "defs",
      "desc",
      "ellipse",
      "feImage",
      "filter",
      "font-face-uri",
      "foreignObject",
      "g",
      "glyphRef",
      "image",
      "line",
      "linearGradient",
      "mpath",
      "path",
      "pattern",
      "polygon",
      "polyline",
      "radialGradient",
      "rect",
      "script",
      "set",
      "stop",
      "style",
      "svg",
      "switch",
      "symbol",
      "text",
      "textPath",
      "title",
      "tref",
      "tspan",
      "use",
      "view",
      "color-profile",
    ];

    if !href_elements.contains(&element_name) {
      return Ok(());
    }

    // Process both href and xlink:href (both create dependencies, href takes precedence for URL rewriting)

    // Process href first
    if let Some(href) = attrs.get(expanded_name!("", "href")) {
      let href_str = href.to_string();
      if href_str.is_empty() {
        return Err(format!("'href' should not be empty string"));
      }

      // Skip fragment-only references and absolute paths
      if !href_str.starts_with("#") && !href_str.starts_with("/") {
        // Use stable name for <a> tags (like JS transformer)
        let needs_stable_name = element_name == "a";
        let dependency_id =
          self.add_url_dependency_with_options(href_str, Priority::Sync, needs_stable_name);
        attrs.set(expanded_name!("", "href"), &dependency_id);
      }
    }

    // Also process xlink:href (even if href exists)
    if let Some(xlink_href) = attrs.get(ExpandedName {
      ns: &namespace_url!("http://www.w3.org/1999/xlink"),
      local: &local_name!("href"),
    }) {
      let href_str = xlink_href.to_string();
      if href_str.is_empty() {
        return Err(format!("'href' should not be empty string"));
      }

      // Skip fragment-only references and absolute paths
      if !href_str.starts_with("#") && !href_str.starts_with("/") {
        // Use stable name for <a> tags (like JS transformer)
        let needs_stable_name = element_name == "a";
        let dependency_id =
          self.add_url_dependency_with_options(href_str, Priority::Sync, needs_stable_name);
        let xlink_href_name = ExpandedName {
          ns: &namespace_url!("http://www.w3.org/1999/xlink"),
          local: &local_name!("href"),
        };
        attrs.set(xlink_href_name, &dependency_id);
      }
    }

    Ok(())
  }

  fn inline_asset_id(&self) -> String {
    let mut hasher = IdentifierHasher::default();
    self.context.source_asset_id.hash(&mut hasher);
    self.discovered_assets.len().hash(&mut hasher);
    // Ids must be 16 characters for scope hoisting to replace imports correctly
    format!("{:016x}", hasher.finish())
  }

  fn determine_script_type(&self, attrs: &Attrs) -> (FileType, SourceType) {
    if let Some(script_type) = attrs.get(expanded_name!("", "type")) {
      let script_type_str = script_type.to_string();

      // Handle known script types
      match script_type_str.as_str() {
        "application/ecmascript" | "application/javascript" | "text/javascript" => {
          (FileType::Js, SourceType::Script)
        }
        "module" => (FileType::Js, SourceType::Module),
        _ => {
          // Fallback: split by '/' and take the last part
          if let Some(last_part) = script_type_str.split('/').next_back() {
            (FileType::from_extension(last_part), SourceType::Script)
          } else {
            (FileType::Js, SourceType::Script)
          }
        }
      }
    } else {
      (FileType::Js, SourceType::Script)
    }
  }

  fn determine_style_type(&self, attrs: &Attrs) -> FileType {
    if let Some(style_type) = attrs.get(expanded_name!("", "type")) {
      let style_type_str = style_type.to_string();

      // Split by '/' and take the last part (e.g., "text/scss" -> "scss")
      if let Some(last_part) = style_type_str.split('/').next_back() {
        match last_part {
          "scss" => FileType::Other("scss".to_string()),
          "sass" => FileType::Other("sass".to_string()),
          _ => FileType::Css,
        }
      } else {
        FileType::Css
      }
    } else {
      FileType::Css
    }
  }

  fn process_style_attribute_as_asset(&mut self, attrs: &mut Attrs) {
    // Create CSS asset for style attributes to match JS transformer behavior
    if let Some(style_content) = attrs.get(expanded_name!("", "style")) {
      let style_str = style_content.to_string();
      if !style_str.trim().is_empty() {
        // Create dependency and corresponding CSS asset
        let specifier = self.inline_asset_id();

        let new_dependency = DependencyBuilder::default()
          .env(self.context.env.clone())
          .source_asset_id(self.context.source_asset_id.clone())
          .source_asset_type(FileType::Other("svg".to_string()))
          .source_path_option(self.context.source_path.clone())
          .specifier(specifier.clone())
          .specifier_type(SpecifierType::default())
          .priority(Priority::Sync)
          .bundle_behavior(Some(BundleBehavior::Inline))
          .build();

        self.dependencies.push(new_dependency);

        // Create CSS asset for style attribute using new_discovered to inherit source asset identity
        // Wrap style attribute content in a valid CSS rule for lightningcss
        let css_content = format!("* {{ {} }}", style_str);

        // Create a temporary source asset to pass to new_discovered
        let source_asset = Asset {
          file_path: self
            .context
            .source_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("index.svg")),
          env: self.context.env.clone(),
          side_effects: self.context.side_effects,
          ..Asset::default()
        };

        let mut new_asset = Asset::new_discovered(
          css_content,
          FileType::Css,
          &self.context.project_root,
          &source_asset,
          Some(specifier),
        );
        new_asset.bundle_behavior = Some(BundleBehavior::Inline);

        self.discovered_assets.push(AssetWithDependencies {
          asset: new_asset,
          dependencies: Vec::new(),
        });
      }
    }
  }
}

impl DomVisitor for SvgDependenciesVisitor {
  fn visit_node(&mut self, node: Handle) -> DomTraversalOperation {
    if let NodeData::Element { name, attrs, .. } = &node.data {
      let mut attrs = attrs.borrow_mut();
      let mut attrs = Attrs::new(&mut attrs);
      let element_name = name.local.to_string();

      // Process functional IRI attributes (fill, stroke, etc.)
      self.process_functional_iri_attributes(&mut attrs);

      // Process style attributes as inline CSS assets (like JS implementation)
      self.process_style_attribute_as_asset(&mut attrs);

      match name.expanded() {
        expanded_name!(html "script") | expanded_name!(svg "script") => {
          // Handle external scripts with href attribute (SVG style)
          if let Some(href) = attrs.get(expanded_name!("", "href")) {
            let href_str = href.to_string();
            if href_str.is_empty() {
              self
                .errors
                .push("'href' should not be empty string".to_string());
              return DomTraversalOperation::Stop;
            }
            // Determine source type and create environment like JS transformer
            let source_type = if let Some(script_type) = attrs.get(expanded_name!("", "type")) {
              match script_type.to_string().as_str() {
                "module" => SourceType::Module,
                _ => SourceType::Script,
              }
            } else {
              SourceType::Script
            };

            let env = Arc::new(Environment {
              source_type,
              output_format: atlaspack_core::types::OutputFormat::Global,
              ..(*self.context.env).clone()
            });

            let dependency = DependencyBuilder::default()
              .env(env)
              .priority(Priority::Parallel)
              .source_asset_id(self.context.source_asset_id.clone())
              .source_asset_type(FileType::Other("svg".to_string()))
              .source_path_option(self.context.source_path.clone())
              .specifier(href_str)
              .specifier_type(SpecifierType::Url)
              .build();

            let dependency_id = dependency.id();
            self.dependencies.push(dependency);

            attrs.set(expanded_name!("", "href"), &dependency_id);
            attrs.delete(expanded_name!("", "type"));
            return DomTraversalOperation::Continue;
          }

          // Handle external scripts with xlink:href attribute (SVG style)
          if let Some(xlink_href) = attrs.get(ExpandedName {
            ns: &namespace_url!("http://www.w3.org/1999/xlink"),
            local: &local_name!("href"),
          }) {
            let href_str = xlink_href.to_string();
            if href_str.is_empty() {
              self
                .errors
                .push("'href' should not be empty string".to_string());
              return DomTraversalOperation::Stop;
            }
            // Determine source type and create environment like JS transformer
            let source_type = if let Some(script_type) = attrs.get(expanded_name!("", "type")) {
              match script_type.to_string().as_str() {
                "module" => SourceType::Module,
                _ => SourceType::Script,
              }
            } else {
              SourceType::Script
            };

            let env = Arc::new(Environment {
              source_type,
              output_format: atlaspack_core::types::OutputFormat::Global,
              ..(*self.context.env).clone()
            });

            let dependency = DependencyBuilder::default()
              .env(env)
              .priority(Priority::Parallel)
              .source_asset_id(self.context.source_asset_id.clone())
              .source_asset_type(FileType::Other("svg".to_string()))
              .source_path_option(self.context.source_path.clone())
              .specifier(href_str)
              .specifier_type(SpecifierType::Url)
              .build();

            let dependency_id = dependency.id();
            self.dependencies.push(dependency);

            let xlink_href_name = ExpandedName {
              ns: &namespace_url!("http://www.w3.org/1999/xlink"),
              local: &local_name!("href"),
            };
            attrs.set(xlink_href_name, &dependency_id);
            attrs.delete(expanded_name!("", "type"));
            return DomTraversalOperation::Continue;
          }

          // Handle external scripts with src attribute (HTML style)
          if let Some(src) = attrs.get(expanded_name!("", "src")) {
            let src_str = src.to_string();
            if src_str.is_empty() {
              self
                .errors
                .push("'src' should not be empty string".to_string());
              return DomTraversalOperation::Stop;
            }
            // Determine source type and create environment like JS transformer
            let source_type = if let Some(script_type) = attrs.get(expanded_name!("", "type")) {
              match script_type.to_string().as_str() {
                "module" => SourceType::Module,
                _ => SourceType::Script,
              }
            } else {
              SourceType::Script
            };

            let env = Arc::new(Environment {
              source_type,
              output_format: atlaspack_core::types::OutputFormat::Global,
              ..(*self.context.env).clone()
            });

            let dependency = DependencyBuilder::default()
              .env(env)
              .priority(Priority::Parallel)
              .source_asset_id(self.context.source_asset_id.clone())
              .source_asset_type(FileType::Other("svg".to_string()))
              .source_path_option(self.context.source_path.clone())
              .specifier(src_str)
              .specifier_type(SpecifierType::Url)
              .build();

            let dependency_id = dependency.id();
            self.dependencies.push(dependency);

            attrs.set(expanded_name!("", "src"), &dependency_id);
            attrs.delete(expanded_name!("", "type"));
            return DomTraversalOperation::Continue;
          }

          // Handle inline scripts (only if no href or src)
          let (file_type, source_type) = self.determine_script_type(&attrs);

          // Use existing data-parcel-key or generate new one
          let data_parcel_key_name = ExpandedName {
            ns: &ns!(),
            local: &LocalName::from("data-parcel-key"),
          };
          let specifier = if let Some(existing_key) = attrs.get(data_parcel_key_name) {
            existing_key.to_string()
          } else {
            self.inline_asset_id()
          };

          attrs.set(data_parcel_key_name, &specifier);
          attrs.delete(expanded_name!("", "type"));

          let env = Arc::new(Environment {
            source_type,
            ..(*self.context.env).clone()
          });

          let new_dependency = DependencyBuilder::default()
            .bundle_behavior(Some(BundleBehavior::Inline))
            .env(env.clone())
            .source_asset_id(self.context.source_asset_id.clone())
            .source_asset_type(FileType::Other("svg".to_string()))
            .source_path_option(self.context.source_path.clone())
            .specifier(specifier.clone())
            .specifier_type(SpecifierType::default())
            .priority(Priority::default())
            .build();

          self.dependencies.push(new_dependency);

          // Create a temporary source asset to pass to new_discovered
          let source_asset = Asset {
            file_path: self
              .context
              .source_path
              .clone()
              .unwrap_or_else(|| PathBuf::from("index.svg")),
            env: env.clone(),
            side_effects: self.context.side_effects,
            ..Asset::default()
          };

          let mut new_asset = Asset::new_discovered(
            String::from_utf8_lossy(&text_content(&node)).to_string(),
            file_type,
            &self.context.project_root,
            &source_asset,
            Some(specifier),
          );
          new_asset.bundle_behavior = Some(BundleBehavior::Inline);
          new_asset.css_dependency_type = Some("tag".into());

          self.discovered_assets.push(AssetWithDependencies {
            asset: new_asset,
            dependencies: Vec::new(),
          });
        }
        expanded_name!(html "style") | expanded_name!(svg "style") => {
          let file_type = self.determine_style_type(&attrs);

          // Use existing data-parcel-key or generate new one
          let data_parcel_key_name = ExpandedName {
            ns: &ns!(),
            local: &LocalName::from("data-parcel-key"),
          };
          let specifier = if let Some(existing_key) = attrs.get(data_parcel_key_name) {
            existing_key.to_string()
          } else {
            self.inline_asset_id()
          };

          attrs.set(data_parcel_key_name, &specifier);
          attrs.delete(expanded_name!("", "type"));

          let new_dependency = DependencyBuilder::default()
            .env(self.context.env.clone())
            .source_asset_id(self.context.source_asset_id.clone())
            .source_asset_type(FileType::Other("svg".to_string()))
            .source_path_option(self.context.source_path.clone())
            .specifier(specifier.clone())
            .specifier_type(SpecifierType::default())
            .priority(Priority::Sync)
            .bundle_behavior(Some(BundleBehavior::Inline))
            .build();

          self.dependencies.push(new_dependency);

          // Create a temporary source asset to pass to new_discovered
          let source_asset = Asset {
            file_path: self
              .context
              .source_path
              .clone()
              .unwrap_or_else(|| PathBuf::from("index.svg")),
            env: self.context.env.clone(),
            side_effects: self.context.side_effects,
            ..Asset::default()
          };

          let mut new_asset = Asset::new_discovered(
            String::from_utf8_lossy(&text_content(&node)).to_string(),
            file_type,
            &self.context.project_root,
            &source_asset,
            Some(specifier),
          );
          new_asset.bundle_behavior = Some(BundleBehavior::Inline);
          new_asset.css_dependency_type = Some("tag".into());

          self.discovered_assets.push(AssetWithDependencies {
            asset: new_asset,
            dependencies: Vec::new(),
          });
        }
        _ => {
          // For other elements, process href attributes
          if let Err(error) = self.process_href_attributes(&mut attrs, &element_name) {
            self.errors.push(error);
            return DomTraversalOperation::Stop;
          }
        }
      }
    }

    DomTraversalOperation::Continue
  }
}

/// Retrieves the text content of the node
fn text_content(node: &Handle) -> Vec<u8> {
  let mut content = Vec::new();
  collect_text_content(node, &mut content);
  content
}

fn collect_text_content(node: &Handle, content: &mut Vec<u8>) {
  match &node.data {
    NodeData::Text { contents } => {
      content.extend_from_slice(contents.borrow().as_bytes());
    }
    NodeData::Element { .. } => {
      for child in node.children.borrow().iter() {
        collect_text_content(child, content);
      }
    }
    _ => {}
  }
}
