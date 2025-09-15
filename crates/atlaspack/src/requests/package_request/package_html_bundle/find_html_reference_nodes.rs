use std::{cell::RefCell, collections::HashMap, rc::Rc};

use atlaspack_core::bundle_graph::{BundleGraphEdge, BundleGraphNode};
use atlaspack_plugin_transformer_html::dom_visitor::{DomTraversalOperation, DomVisitor};
use html5ever::{namespace_url, ExpandedName, LocalName};
use markup5ever::{expanded_name, local_name, ns};
use markup5ever_rcdom::{Handle, Node, NodeData};

pub struct RewriteHTMLReferenceParams<'a> {
  pub referenced_paths_by_dependency_id: &'a HashMap<String, Vec<String>>,
  pub contents_by_dependency_specifier: &'a HashMap<String, String>,
}

pub fn rewrite_html_reference(
  params: RewriteHTMLReferenceParams,
  reference: &HTMLReference,
) -> Option<()> {
  // println!("rewrite_html_reference: {:?}", reference);
  match reference.reference_type() {
    HTMLReferenceType::Script => {
      let parent = reference.handle.parent.replace(None);
      let parent = parent.map(|p| p.upgrade()).flatten()?;
      let mut children = parent.children.borrow_mut();

      let index = children
        .iter()
        .position(|child| Rc::as_ptr(child) == Rc::as_ptr(&reference.handle))?;

      let script_node: Handle = children.remove(index);
      let NodeData::Element {
        name,
        attrs,
        template_contents,
        mathml_annotation_xml_integration_point,
      } = &script_node.data
      else {
        return None;
      };

      let new_script_paths = params
        .referenced_paths_by_dependency_id
        .get(&reference.dependency_id)?;
      let new_script_nodes: Vec<Handle> = new_script_paths
        .iter()
        .map(|path| {
          let script_node = Node::new(NodeData::Element {
            name: name.clone(),
            attrs: attrs.clone(),
            template_contents: template_contents.clone(),
            mathml_annotation_xml_integration_point: *mathml_annotation_xml_integration_point,
          });

          {
            let NodeData::Element { attrs, .. } = &script_node.data else {
              return script_node;
            };

            let mut attrs = attrs.borrow_mut();
            let mut attrs = atlaspack_plugin_transformer_html::attrs::Attrs::new(&mut attrs);
            attrs.set(expanded_name!("", "src"), path);
          }

          script_node
        })
        .collect();

      children.splice(index..=index, new_script_nodes);
    }
    HTMLReferenceType::InlineScript => {
      let mut children = reference.handle.children.borrow_mut();
      children.clear();

      // println!(
      //   "rewrite_html_reference: {:?}",
      //   reference.dependency_specifier
      // );
      // println!(
      //   "rewrite_html_reference: {:#?}",
      //   params.contents_by_dependency_specifier
      // );
      let contents = params
        .contents_by_dependency_specifier
        .get(&reference.dependency_specifier)
        .expect("Reference not found");
      children.push(Node::new(NodeData::Text {
        contents: RefCell::new(contents.clone().into()),
      }));

      let NodeData::Element { attrs, .. } = &reference.handle.data else {
        return None;
      };

      let mut attrs = attrs.borrow_mut();
      let mut attrs = atlaspack_plugin_transformer_html::attrs::Attrs::new(&mut attrs);
      attrs.delete(ExpandedName {
        ns: &ns!(),
        local: &LocalName::from("data-parcel-key"),
      });
    }
  }

  Some(())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HTMLReferenceType {
  Script,
  InlineScript,
}

#[derive(Debug)]
pub struct HTMLReference {
  reference_type: HTMLReferenceType,
  handle: Handle,
  dependency_id: String,
  dependency_specifier: String,
}

impl HTMLReference {
  pub fn reference_type(&self) -> &HTMLReferenceType {
    &self.reference_type
  }

  pub fn handle(&self) -> &Handle {
    &self.handle
  }

  pub fn dependency_id(&self) -> &str {
    &self.dependency_id
  }

  pub fn dependency_specifier(&self) -> &str {
    &self.dependency_specifier
  }
}

pub struct FindHTMLReferenceNodesVisitor {
  references: Vec<HTMLReference>,
}

impl FindHTMLReferenceNodesVisitor {
  pub fn new() -> Self {
    FindHTMLReferenceNodesVisitor { references: vec![] }
  }

  pub fn into_references(self) -> Vec<HTMLReference> {
    self.references
  }
}

impl DomVisitor for FindHTMLReferenceNodesVisitor {
  fn visit_node(&mut self, node: Handle) -> DomTraversalOperation {
    match &node.data {
      NodeData::Element { name, attrs, .. } => {
        if name.expanded() == expanded_name!(html "script") {
          let mut attrs = attrs.borrow_mut();
          let attrs = atlaspack_plugin_transformer_html::attrs::Attrs::new(&mut attrs);

          let src_attribute = attrs.get(expanded_name!("", "src"));
          let data_key_attribute = attrs.get(ExpandedName {
            ns: &ns!(),
            local: &LocalName::from("data-parcel-key"),
          });

          if let Some(value) = src_attribute {
            self.references.push(HTMLReference {
              reference_type: HTMLReferenceType::Script,
              handle: node.clone(),
              // TODO: enum
              dependency_specifier: "".to_string(),
              dependency_id: value.to_string(),
            });
          } else if let Some(value) = data_key_attribute {
            self.references.push(HTMLReference {
              reference_type: HTMLReferenceType::InlineScript,
              handle: node.clone(),
              dependency_specifier: value.to_string(),
              dependency_id: "".to_string(),
            });
          }
        }
      }
      _ => {}
    }

    DomTraversalOperation::Continue
  }
}

#[cfg(test)]
mod tests {

  use super::*;

  use std::collections::HashMap;

  use atlaspack_plugin_transformer_html::dom_visitor::walk;

  fn normalize_html(html: &str) -> String {
    let dom = atlaspack_plugin_transformer_html::parse_html(html.as_bytes()).unwrap();
    let output =
      String::from_utf8(atlaspack_plugin_transformer_html::serialize_html(dom).unwrap()).unwrap();
    output
      .lines()
      .map(|line| line.trim())
      .filter(|line| !line.is_empty())
      .collect()
  }

  fn parse(html: &str) -> markup5ever_rcdom::RcDom {
    atlaspack_plugin_transformer_html::parse_html(html.as_bytes()).unwrap()
  }

  #[test]
  fn test_finds_external_script_reference() {
    let html = r#"
      <html>
        <body>
          <script src="dep"></script>
        </body>
      </html>
    "#;

    let dom = parse(html);
    let mut visitor = FindHTMLReferenceNodesVisitor::new();
    walk(dom.document.clone(), &mut visitor);
    let refs = visitor.into_references();

    assert_eq!(refs.len(), 1);
    assert!(matches!(refs[0].reference_type, HTMLReferenceType::Script));
    assert_eq!(refs[0].dependency_id, "dep");
  }

  #[test]
  fn test_finds_inline_script_reference() {
    let html = r#"
      <html>
        <body>
          <script data-parcel-key="abc123">console.log('x');</script>
        </body>
      </html>
    "#;

    let dom = parse(html);
    let mut visitor = FindHTMLReferenceNodesVisitor::new();
    walk(dom.document.clone(), &mut visitor);
    let refs = visitor.into_references();

    assert_eq!(refs.len(), 1);
    assert!(matches!(
      refs[0].reference_type,
      HTMLReferenceType::InlineScript
    ));
    assert_eq!(refs[0].dependency_specifier, "abc123");
  }

  #[test]
  fn test_ignores_non_script_nodes() {
    let html = r#"
      <html>
        <head>
          <link rel="stylesheet" href="dep.css" />
        </head>
        <body>
          <div id="app"></div>
        </body>
      </html>
    "#;

    let dom = parse(html);
    let mut visitor = FindHTMLReferenceNodesVisitor::new();
    walk(dom.document.clone(), &mut visitor);
    let refs = visitor.into_references();

    assert_eq!(refs.len(), 0);
  }

  #[test]
  fn test_multiple_references_and_prefer_src_over_data_key() {
    // Second script has both src and data-parcel-key; src should win.
    let html = r#"
      <html>
        <body>
          <script src="a"></script>
          <script src="b" data-parcel-key="inline"></script>
        </body>
      </html>
    "#;

    let dom = parse(html);
    let mut visitor = FindHTMLReferenceNodesVisitor::new();
    walk(dom.document.clone(), &mut visitor);
    let refs = visitor.into_references();

    assert_eq!(refs.len(), 2);
    assert!(matches!(refs[0].reference_type, HTMLReferenceType::Script));
    assert_eq!(refs[0].dependency_id, "a");
    assert!(matches!(refs[1].reference_type, HTMLReferenceType::Script));
    assert_eq!(refs[1].dependency_id, "b");
  }

  #[test]
  fn test_rewrite_html_reference_with_a_script_mapping_to_a_single_path() {
    let html = r#"
      <html>
        <body>
          <script src="dep"></script>
        </body>
      </html>
    "#;

    let dom = parse(html);
    let mut visitor = FindHTMLReferenceNodesVisitor::new();
    walk(dom.document.clone(), &mut visitor);
    let refs = visitor.into_references();
    assert_eq!(refs.len(), 1);
    assert!(matches!(refs[0].reference_type, HTMLReferenceType::Script));
    assert_eq!(refs[0].dependency_id, "dep");

    let mut referenced_paths_by_dependency_id = HashMap::new();
    referenced_paths_by_dependency_id
      .insert(String::from("dep"), vec![String::from("/static/app.js")]);

    rewrite_html_reference(
      RewriteHTMLReferenceParams {
        referenced_paths_by_dependency_id: &referenced_paths_by_dependency_id,
        contents_by_dependency_specifier: &HashMap::new(),
      },
      &refs[0],
    );

    let output =
      String::from_utf8(atlaspack_plugin_transformer_html::serialize_html(dom).unwrap()).unwrap();

    assert_eq!(
      &normalize_html(&output),
      &normalize_html(
        r#"
          <html>
            <body>
              <script src="/static/app.js"></script>
            </body>
          </html>
        "#,
      )
    );
  }

  #[test]
  fn test_rewrite_html_reference_preserves_other_attributes() {
    let html = r#"
      <html>
        <body>
          <script src="dep" type="module" data-something="1234"></script>
        </body>
      </html>
    "#;

    let dom = parse(html);
    let mut visitor = FindHTMLReferenceNodesVisitor::new();
    walk(dom.document.clone(), &mut visitor);
    let refs = visitor.into_references();
    assert_eq!(refs.len(), 1);
    assert!(matches!(refs[0].reference_type, HTMLReferenceType::Script));
    assert_eq!(refs[0].dependency_id, "dep");

    let mut referenced_paths_by_dependency_id = HashMap::new();
    referenced_paths_by_dependency_id
      .insert(String::from("dep"), vec![String::from("/static/app.js")]);

    rewrite_html_reference(
      RewriteHTMLReferenceParams {
        referenced_paths_by_dependency_id: &referenced_paths_by_dependency_id,
        contents_by_dependency_specifier: &HashMap::new(),
      },
      &refs[0],
    );

    let output =
      String::from_utf8(atlaspack_plugin_transformer_html::serialize_html(dom).unwrap()).unwrap();

    assert_eq!(
      &normalize_html(&output),
      &normalize_html(
        r#"
          <html>
            <body>
              <script src="/static/app.js" type="module" data-something="1234"></script>
            </body>
          </html>
        "#,
      )
    );
  }

  #[test]
  fn test_rewrite_html_reference_with_a_script_mapping_to_multiple_paths() {
    let html = r#"
      <html>
        <body>
          <script src="dep"></script>
        </body>
      </html>
    "#;

    let dom = parse(html);
    let mut visitor = FindHTMLReferenceNodesVisitor::new();
    walk(dom.document.clone(), &mut visitor);
    let refs = visitor.into_references();
    assert_eq!(refs.len(), 1);
    assert!(matches!(refs[0].reference_type, HTMLReferenceType::Script));
    assert_eq!(refs[0].dependency_id, "dep");

    let mut referenced_paths_by_dependency_id = HashMap::new();
    referenced_paths_by_dependency_id.insert(
      String::from("dep"),
      vec![
        String::from("/static/app1.js"),
        String::from("/static/app2.js"),
      ],
    );

    rewrite_html_reference(
      RewriteHTMLReferenceParams {
        referenced_paths_by_dependency_id: &referenced_paths_by_dependency_id,
        contents_by_dependency_specifier: &HashMap::new(),
      },
      &refs[0],
    );

    let output =
      String::from_utf8(atlaspack_plugin_transformer_html::serialize_html(dom).unwrap()).unwrap();

    assert_eq!(
      &normalize_html(&output),
      &normalize_html(
        r#"
          <html>
            <body>
              <script src="/static/app1.js"></script>
              <script src="/static/app2.js"></script>
            </body>
          </html>
        "#,
      )
    );
  }

  #[test]
  fn test_rewrite_html_reference_with_an_inline_script() {
    let html = r#"
      <html>
        <body>
          <script data-parcel-key="abc123">
            console.log('x');
          </script>
        </body>
      </html>
    "#;

    let dom = parse(html);
    let mut visitor = FindHTMLReferenceNodesVisitor::new();
    walk(dom.document.clone(), &mut visitor);
    let refs = visitor.into_references();
    assert_eq!(refs.len(), 1);
    assert!(matches!(
      refs[0].reference_type,
      HTMLReferenceType::InlineScript
    ));
    assert_eq!(refs[0].dependency_specifier, "abc123");

    let mut contents_by_dependency_specifier = HashMap::new();
    contents_by_dependency_specifier.insert(
      String::from("abc123"),
      String::from("console.log('transformed');"),
    );

    rewrite_html_reference(
      RewriteHTMLReferenceParams {
        referenced_paths_by_dependency_id: &HashMap::new(),
        contents_by_dependency_specifier: &contents_by_dependency_specifier,
      },
      &refs[0],
    );

    let output =
      String::from_utf8(atlaspack_plugin_transformer_html::serialize_html(dom).unwrap()).unwrap();

    assert_eq!(
      &normalize_html(&output),
      &normalize_html(
        r#"
          <html>
            <body>
              <script>console.log('transformed');</script>
            </body>
          </html>
        "#,
      )
    );
  }
}
