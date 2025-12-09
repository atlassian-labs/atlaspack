use crate::postcss::value_parser as vp;
use postcss as pc;

fn normalize_pair(value: &str) -> String {
  let parsed = vp::parse(value);
  let mut tokens: Vec<String> = Vec::new();
  let mut nodes = parsed.nodes.clone();
  vp::walk(
    &mut nodes[..],
    &mut |n| {
      match n {
        vp::Node::Word { value } => {
          let v = value.replace("0px", "0");
          tokens.push(v);
        }
        vp::Node::Space { .. } => tokens.push(" ".to_string()),
        _ => tokens.push(vp::stringify(&[n.clone()])),
      }
      true
    },
    false,
  );
  let mut s = tokens.join("");
  // Collapse center center -> center
  if s.trim().eq_ignore_ascii_case("center center") {
    s = "center".to_string();
  }
  s
}

pub fn plugin() -> pc::BuiltPlugin {
  pc::plugin("postcss-normalize-positions")
    .decl_filter("background-position", |decl, _| {
      let current = decl.value();
      let next = normalize_pair(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    .build()
}
