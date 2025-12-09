use super::Node;

pub fn stringify(nodes: &[Node]) -> String {
  let mut out = String::new();
  for n in nodes {
    match n {
      Node::Space { value } => out.push_str(value),
      Node::String { value, quote, .. } => {
        out.push(*quote);
        out.push_str(value);
        out.push(*quote);
      }
      Node::Comment { value, .. } => {
        out.push_str("/*");
        out.push_str(value);
        out.push_str("*/");
      }
      Node::Word { value } => out.push_str(value),
      Node::Div {
        value,
        before,
        after,
      } => {
        out.push_str(before);
        out.push_str(value);
        out.push_str(after);
      }
      Node::Function {
        value,
        nodes,
        before,
        after,
        ..
      } => {
        out.push_str(value);
        out.push('(');
        out.push_str(before);
        out.push_str(&stringify(nodes));
        out.push_str(after);
        out.push(')');
      }
    }
  }
  out
}
