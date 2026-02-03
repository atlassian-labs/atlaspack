use crate::postcss::value_parser as vp;

// Port of packages/postcss-plugin-sources/postcss-ordered-values/src/lib/getValue.js
pub fn get_value(lists: Vec<Vec<vp::Node>>) -> String {
  let mut nodes: Vec<vp::Node> = Vec::new();
  let total = lists.len();
  for (index, arg) in lists.into_iter().enumerate() {
    let last = arg.len().saturating_sub(1);
    for (idx, val) in arg.into_iter().enumerate() {
      if index + 1 == total && idx == last {
        if let vp::Node::Space { .. } = val {
          continue;
        }
      }
      nodes.push(val);
    }
    if index + 1 != total {
      if let Some(last_node) = nodes.last_mut() {
        *last_node = vp::Node::Div {
          value: ",".to_string(),
          before: String::new(),
          after: String::new(),
        };
      } else {
        nodes.push(vp::Node::Div {
          value: ",".to_string(),
          before: String::new(),
          after: String::new(),
        });
      }
    }
  }
  vp::stringify(&nodes)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::plugins::normalize_css_engine::ordered_values::library::add_space::add_space;
  use pretty_assertions::assert_eq;

  fn word(s: &str) -> vp::Node {
    vp::Node::Word {
      value: s.to_string(),
    }
  }

  #[test]
  fn get_value_joins_two_shadows_without_space_before_comma() {
    // Simulate what box_shadow.rs produces for two shadows
    let shadow1 = vec![
      word("0"),
      add_space(),
      word("0"),
      add_space(),
      word("1px"),
      add_space(),
      word("0"),
      add_space(),
      word("#1e1f214f"),
      add_space(),
    ];
    let shadow2 = vec![
      word("0"),
      add_space(),
      word("8px"),
      add_space(),
      word("9pt"),
      add_space(),
      word("0"),
      add_space(),
      word("#1e1f2126"),
      add_space(),
    ];
    let lists = vec![shadow1, shadow2];
    let result = get_value(lists);

    // Expected: no space before comma
    assert_eq!(result, "0 0 1px 0 #1e1f214f,0 8px 9pt 0 #1e1f2126");
  }
}
