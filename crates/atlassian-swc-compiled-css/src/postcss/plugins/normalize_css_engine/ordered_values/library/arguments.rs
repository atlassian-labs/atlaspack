use crate::postcss::value_parser as vp;

// Equivalent to cssnano-utils getArguments for simple comma splits
pub fn get_arguments(parsed: &vp::ParsedValue) -> Vec<Vec<vp::Node>> {
  let mut list: Vec<Vec<vp::Node>> = vec![Vec::new()];
  for n in &parsed.nodes {
    if let vp::Node::Div { value, .. } = n {
      if value == "," {
        if let Some(cur) = list.last_mut() {
          while matches!(cur.last(), Some(vp::Node::Space { .. })) {
            cur.pop();
          }
        }
        list.push(Vec::new());
        continue;
      }
    }
    let Some(cur) = list.last_mut() else {
      continue;
    };
    if cur.is_empty() && matches!(n, vp::Node::Space { .. }) {
      continue;
    }
    cur.push(n.clone());
  }

  if let Some(cur) = list.last_mut() {
    while matches!(cur.last(), Some(vp::Node::Space { .. })) {
      cur.pop();
    }
  }
  list
}
