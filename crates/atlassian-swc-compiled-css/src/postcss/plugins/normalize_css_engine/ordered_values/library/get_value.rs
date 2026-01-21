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
