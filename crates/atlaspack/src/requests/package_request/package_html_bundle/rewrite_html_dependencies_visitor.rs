use std::collections::{HashMap, HashSet};

use atlaspack_plugin_transformer_html::dom_visitor::{walk, DomTraversalOperation, DomVisitor};
use html5ever::namespace_url;
use markup5ever::{expanded_name, local_name, ns, QualName};
use markup5ever_rcdom::{Handle, NodeData};
use petgraph::visit::EdgeRef;

/// The HTML transformer will rewrite <script src="..."></script> tags to keep the
/// atlaspack `dependency-id` on the SRC attribute.
///
/// This visitor will rewrite the SRC to point at the referenced bundle public URL.
pub struct RewriteHTMLDependenciesVisitor {
  referenced_paths_by_dependency_specifier: HashMap<String, Vec<String>>,
  // Tracks which public paths have been written already to avoid duplicates
  rewritten_public_paths: HashSet<String>,
}

impl RewriteHTMLDependenciesVisitor {
  pub fn new(referenced_paths_by_dependency_specifier: HashMap<String, Vec<String>>) -> Self {
    RewriteHTMLDependenciesVisitor {
      referenced_paths_by_dependency_specifier,
      rewritten_public_paths: HashSet::new(),
    }
  }
}

impl DomVisitor for RewriteHTMLDependenciesVisitor {
  fn visit_node(&mut self, node: Handle) -> DomTraversalOperation {
    match &node.data {
      NodeData::Element { name, attrs, .. } => {
        if name.expanded() == expanded_name!(html "script") {
          let mut attrs = attrs.borrow_mut();

          for attr in attrs.iter_mut() {
            if attr.name.expanded() == expanded_name!("", "src") {
              let src_string = attr.value.to_string();
              let Some(candidates) = self
                .referenced_paths_by_dependency_specifier
                .get(&src_string)
              else {
                break;
              };

              for path in candidates {
                if self.rewritten_public_paths.contains(path.as_str()) {
                  continue;
                }
                attr.value = path.clone().into();
                self.rewritten_public_paths.insert(path.clone());
              }
            }
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

  #[test]
  fn test_rewrites_matching_script_src() {
    let input = r#"
      <html>
        <body>
          <script src="dep"></script>
        </body>
      </html>
    "#;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    map.insert(String::from("dep"), vec![String::from("/static/app.js")]);

    let dom = atlaspack_plugin_transformer_html::parse_html(input.as_bytes()).unwrap();
    walk(
      dom.document.clone(),
      &mut RewriteHTMLDependenciesVisitor::new(map),
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
  fn test_does_not_rewrite_when_mapping_missing() {
    let input = r#"
      <html>
        <body>
          <script src="unknown"></script>
        </body>
      </html>
    "#;

    let map: HashMap<String, Vec<String>> = HashMap::new();

    let dom = atlaspack_plugin_transformer_html::parse_html(input.as_bytes()).unwrap();
    walk(
      dom.document.clone(),
      &mut RewriteHTMLDependenciesVisitor::new(map),
    );

    let output =
      String::from_utf8(atlaspack_plugin_transformer_html::serialize_html(dom).unwrap()).unwrap();

    assert_eq!(&normalize_html(&output), &normalize_html(input));
  }

  #[test]
  fn test_ignores_non_script_tags() {
    let input = r#"
      <html>
        <head>
          <link rel="stylesheet" href="dep" />
        </head>
        <body>
          <script>console.log('no src');</script>
        </body>
      </html>
    "#;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    map.insert(String::from("dep"), vec![String::from("/static/style.css")]);

    let dom = atlaspack_plugin_transformer_html::parse_html(input.as_bytes()).unwrap();
    walk(
      dom.document.clone(),
      &mut RewriteHTMLDependenciesVisitor::new(map),
    );

    let output =
      String::from_utf8(atlaspack_plugin_transformer_html::serialize_html(dom).unwrap()).unwrap();

    // Ensure <link> href is unchanged and inline <script> remains as-is
    assert_eq!(&normalize_html(&output), &normalize_html(input));
  }

  #[test]
  fn test_rewrites_multiple_scripts() {
    let input = r#"
      <html>
        <body>
          <script src="a"></script>
          <script src="b"></script>
        </body>
      </html>
    "#;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    map.insert(String::from("a"), vec![String::from("/bundle/A.js")]);
    map.insert(String::from("b"), vec![String::from("/bundle/B.js")]);

    let dom = atlaspack_plugin_transformer_html::parse_html(input.as_bytes()).unwrap();
    walk(
      dom.document.clone(),
      &mut RewriteHTMLDependenciesVisitor::new(map),
    );

    let output =
      String::from_utf8(atlaspack_plugin_transformer_html::serialize_html(dom).unwrap()).unwrap();

    assert_eq!(
      &normalize_html(&output),
      &normalize_html(
        r#"
          <html>
            <body>
              <script src="/bundle/A.js"></script>
              <script src="/bundle/B.js"></script>
            </body>
          </html>
        "#,
      )
    );
  }

  #[test]
  fn test_rewrites_avoids_duplicate_paths() {
    let input = r#"
      <html>
        <body>
          <script src="dupe"></script>
          <script src="dupe"></script>
        </body>
      </html>
    "#;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    // Provide the same path twice and ensure only the first script is rewritten
    map.insert(
      String::from("dupe"),
      vec![String::from("/shared.js"), String::from("/shared.js")],
    );

    let dom = atlaspack_plugin_transformer_html::parse_html(input.as_bytes()).unwrap();
    walk(
      dom.document.clone(),
      &mut RewriteHTMLDependenciesVisitor::new(map),
    );

    let output =
      String::from_utf8(atlaspack_plugin_transformer_html::serialize_html(dom).unwrap()).unwrap();

    assert_eq!(
      &normalize_html(&output),
      &normalize_html(
        r#"
          <html>
            <body>
              <script src="/shared.js"></script>
              <script src="dupe"></script>
            </body>
          </html>
        "#,
      )
    );
  }
}
