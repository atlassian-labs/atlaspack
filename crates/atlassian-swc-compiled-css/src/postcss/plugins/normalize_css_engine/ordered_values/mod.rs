use crate::postcss::value_parser as vp;
use postcss as pc;
pub mod data;
pub mod lib;
pub mod rules;

fn vendor_unprefixed(prop: &str) -> &str {
  // Minimal vendor stripping used by the JS plugin
  prop
    .strip_prefix("-webkit-")
    .or_else(|| prop.strip_prefix("-moz-"))
    .or_else(|| prop.strip_prefix("-ms-"))
    .or_else(|| prop.strip_prefix("-o-"))
    .unwrap_or(prop)
}

fn is_variable_function(node: &vp::Node) -> bool {
  match node {
    vp::Node::Function { value, .. } => {
      let name = value.to_lowercase();
      name == "var" || name == "env" || name == "constant"
    }
    _ => false,
  }
}

fn should_abort(parsed: &vp::ParsedValue) -> bool {
  let mut abort = false;
  let mut walk = |node: &mut vp::Node| -> bool {
    match node {
      vp::Node::Comment { .. } => abort = true,
      n if is_variable_function(n) => abort = true,
      vp::Node::Word { value } if value.contains("___CSS_LOADER_IMPORT___") => abort = true,
      _ => {}
    }
    !abort
  };
  let mut nodes = parsed.nodes.clone();
  vp::walk(&mut nodes[..], &mut walk, false);
  abort
}

fn add_space() -> vp::Node {
  vp::Node::Space {
    value: " ".to_string(),
  }
}

fn stringify(nodes: &[vp::Node]) -> String {
  vp::stringify(nodes)
}

fn vp_unit(word: &str) -> Option<(String, String)> {
  vp::unit::unit(word).map(|nu| (nu.number, nu.unit))
}

fn normalize_border(parsed: &vp::ParsedValue) -> String {
  let mut width = String::new();
  let mut style = String::new();
  let mut color = String::new();
  fn is_style(v: &str) -> bool {
    matches!(
      v,
      "none"
        | "auto"
        | "hidden"
        | "dotted"
        | "dashed"
        | "solid"
        | "double"
        | "groove"
        | "ridge"
        | "inset"
        | "outset"
    )
  }
  fn is_width(v: &str) -> bool {
    matches!(v, "thin" | "medium" | "thick") || vp_unit(v).is_some()
  }
  let mut visitor = |node: &mut vp::Node| -> bool {
    match node {
      vp::Node::Word { value } => {
        let low = value.to_lowercase();
        if is_style(&low) {
          style = value.clone();
          return false;
        }
        if is_width(&low) {
          if !width.is_empty() {
            width.push(' ');
            width.push_str(value);
          } else {
            width = value.clone();
          }
          return false;
        }
        color = value.clone();
        return false;
      }
      vp::Node::Function { value, .. } => {
        let low = value.to_lowercase();
        // mathfunctions -> width; else -> color
        if matches!(low.as_str(), "calc" | "min" | "max" | "clamp") {
          // stringify full node
          if let vp::Node::Function { nodes, .. } = node {
            width = stringify(nodes);
          } else {
          }
        } else {
          if let vp::Node::Function { nodes, .. } = node {
            color = stringify(nodes);
          } else {
          }
        }
        return false;
      }
      _ => true,
    }
  };
  let mut nodes = parsed.nodes.clone();
  vp::walk(&mut nodes[..], &mut visitor, false);
  let mut out = String::new();
  if !width.is_empty() {
    out.push_str(&width);
    out.push(' ');
  }
  if !style.is_empty() {
    out.push_str(&style);
    out.push(' ');
  }
  if !color.is_empty() {
    out.push_str(&color);
  }
  out.trim().to_string()
}

fn get_arguments(parsed: &vp::ParsedValue) -> Vec<Vec<vp::Node>> {
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

fn normalize_transition(args: Vec<Vec<vp::Node>>) -> Vec<Vec<vp::Node>> {
  let mut list: Vec<Vec<vp::Node>> = Vec::new();
  for arg in args.into_iter() {
    let mut property: Vec<vp::Node> = Vec::new();
    let mut time1: Vec<vp::Node> = Vec::new();
    let mut time2: Vec<vp::Node> = Vec::new();
    let mut timing: Vec<vp::Node> = Vec::new();
    for node in arg.into_iter() {
      match &node {
        vp::Node::Space { .. } => {}
        vp::Node::Function { value, .. } => {
          let low = value.to_lowercase();
          if low == "steps" || low == "cubic-bezier" {
            timing.push(node.clone());
            timing.push(add_space());
          } else {
            property.push(node.clone());
            property.push(add_space());
          }
        }
        vp::Node::Word { value } => {
          if vp_unit(value).is_some() {
            if time1.is_empty() {
              time1.push(node.clone());
              time1.push(add_space());
            } else {
              time2.push(node.clone());
              time2.push(add_space());
            }
          } else {
            let low = value.to_lowercase();
            if matches!(
              low.as_str(),
              "ease"
                | "linear"
                | "ease-in"
                | "ease-out"
                | "ease-in-out"
                | "step-start"
                | "step-end"
            ) {
              timing.push(node.clone());
              timing.push(add_space());
            } else {
              property.push(node.clone());
              property.push(add_space());
            }
          }
        }
        _ => {
          property.push(node.clone());
          property.push(add_space());
        }
      }
    }
    let mut combined: Vec<vp::Node> = Vec::new();
    combined.extend(property);
    combined.extend(time1);
    combined.extend(timing);
    combined.extend(time2);
    list.push(combined);
  }
  list
}

fn get_value(lists: Vec<Vec<vp::Node>>) -> String {
  // Flatten lists into a single node array with commas as divs, mirroring getValue.js
  let mut nodes: Vec<vp::Node> = Vec::new();
  let total = lists.len();
  for (idx, arg) in lists.into_iter().enumerate() {
    let last_idx = arg.len().saturating_sub(1);
    for (i, val) in arg.into_iter().enumerate() {
      // Drop trailing space at very end
      if idx + 1 == total && i == last_idx {
        if let vp::Node::Space { .. } = val {
          continue;
        }
      }
      nodes.push(val);
    }
    if idx + 1 != total {
      // Overwrite last node into a div comma if it exists; else push a comma
      if let Some(last) = nodes.last_mut() {
        *last = vp::Node::Div {
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

fn to_string(parsed: &vp::ParsedValue) -> String {
  vp::stringify(&parsed.nodes)
}

fn normalize_list_style(parsed: &vp::ParsedValue) -> String {
  // Port of listStyle.js
  // Build sets
  fn defined_types() -> &'static std::collections::HashSet<&'static str> {
    use once_cell::sync::Lazy;
    static TYPES: Lazy<std::collections::HashSet<&'static str>> = Lazy::new(|| {
      [
        "afar",
        "amharic",
        "amharic-abegede",
        "arabic-indic",
        "armenian",
        "asterisks",
        "bengali",
        "binary",
        "cambodian",
        "circle",
        "cjk-decimal",
        "cjk-earthly-branch",
        "cjk-heavenly-stem",
        "cjk-ideographic",
        "decimal",
        "decimal-leading-zero",
        "devanagari",
        "disc",
        "disclosure-closed",
        "disclosure-open",
        "ethiopic",
        "ethiopic-abegede",
        "ethiopic-abegede-am-et",
        "ethiopic-abegede-gez",
        "ethiopic-abegede-ti-er",
        "ethiopic-abegede-ti-et",
        "ethiopic-halehame",
        "ethiopic-halehame-aa-er",
        "ethiopic-halehame-aa-et",
        "ethiopic-halehame-am",
        "ethiopic-halehame-am-et",
        "ethiopic-halehame-gez",
        "ethiopic-halehame-om-et",
        "ethiopic-halehame-sid-et",
        "ethiopic-halehame-so-et",
        "ethiopic-halehame-ti-er",
        "ethiopic-halehame-ti-et",
        "ethiopic-halehame-tig",
        "ethiopic-numeric",
        "footnotes",
        "georgian",
        "gujarati",
        "gurmukhi",
        "hangul",
        "hangul-consonant",
        "hebrew",
        "hiragana",
        "hiragana-iroha",
        "japanese-formal",
        "japanese-informal",
        "kannada",
        "katakana",
        "katakana-iroha",
        "khmer",
        "korean-hangul-formal",
        "korean-hanja-formal",
        "korean-hanja-informal",
        "lao",
        "lower-alpha",
        "lower-armenian",
        "lower-greek",
        "lower-hexadecimal",
        "lower-latin",
        "lower-norwegian",
        "lower-roman",
        "malayalam",
        "mongolian",
        "myanmar",
        "octal",
        "oriya",
        "oromo",
        "persian",
        "sidama",
        "simp-chinese-formal",
        "simp-chinese-informal",
        "somali",
        "square",
        "string",
        "symbols",
        "tamil",
        "telugu",
        "thai",
        "tibetan",
        "tigre",
        "tigrinya-er",
        "tigrinya-er-abegede",
        "tigrinya-et",
        "tigrinya-et-abegede",
        "trad-chinese-formal",
        "trad-chinese-informal",
        "upper-alpha",
        "upper-armenian",
        "upper-greek",
        "upper-hexadecimal",
        "upper-latin",
        "upper-norwegian",
        "upper-roman",
        "urdu",
      ]
      .into_iter()
      .collect()
    });
    &TYPES
  }
  let positions: std::collections::HashSet<&'static str> =
    ["inside", "outside"].into_iter().collect();
  let mut typ = String::new();
  let mut pos = String::new();
  let mut img = String::new();
  let mut nodes = parsed.nodes.clone();
  vp::walk(
    &mut nodes[..],
    &mut |node| {
      match node {
        vp::Node::Word { value } => {
          let v = value.as_str();
          if defined_types().contains(v) {
            typ.push(' ');
            typ.push_str(v);
          } else if positions.contains(v) {
            pos.push(' ');
            pos.push_str(v);
          } else if v == "none" {
            // If 'none' already in type, treat as image
            if typ.split(' ').any(|e| e == "none") {
              img.push(' ');
              img.push_str(v);
            } else {
              typ.push(' ');
              typ.push_str(v);
            }
          } else {
            typ.push(' ');
            typ.push_str(v);
          }
        }
        vp::Node::Function { .. } => {
          img.push(' ');
          img.push_str(&stringify(&[node.clone()]));
        }
        _ => {}
      }
      true
    },
    false,
  );
  format!("{} {} {}", typ.trim(), pos.trim(), img.trim())
    .trim()
    .to_string()
}

fn normalize_columns(parsed: &vp::ParsedValue) -> String {
  // Port of columns.js
  let mut widths: Vec<String> = Vec::new();
  let mut other: Vec<String> = Vec::new();
  let mut nodes = parsed.nodes.clone();
  vp::walk(
    &mut nodes[..],
    &mut |node| {
      if let vp::Node::Word { value } = node {
        if let Some(u) = vp::unit::unit(value) {
          if !u.unit.is_empty() {
            widths.push(value.clone());
          } else {
            other.push(value.clone());
          }
        } else {
          other.push(value.clone());
        }
      }
      true
    },
    false,
  );
  if other.len() == 1 && widths.len() == 1 {
    return format!("{} {}", widths[0].trim_start(), other[0].trim_start());
  }
  to_string(parsed)
}

fn normalize_flex_flow(parsed: &vp::ParsedValue) -> String {
  let directions: std::collections::HashSet<&'static str> =
    ["row", "row-reverse", "column", "column-reverse"]
      .into_iter()
      .collect();
  let wraps: std::collections::HashSet<&'static str> =
    ["nowrap", "wrap", "wrap-reverse"].into_iter().collect();
  let mut direction = String::new();
  let mut wrap = String::new();
  let mut nodes = parsed.nodes.clone();
  vp::walk(
    &mut nodes[..],
    &mut |node| {
      if let vp::Node::Word { value } = node {
        let low = value.to_lowercase();
        if directions.contains(low.as_str()) {
          direction = value.clone();
        } else if wraps.contains(low.as_str()) {
          wrap = value.clone();
        }
      }
      true
    },
    false,
  );
  format!("{} {}", direction, wrap).trim().to_string()
}

fn math_functions_set() -> &'static std::collections::HashSet<&'static str> {
  use once_cell::sync::Lazy;
  static SET_: Lazy<std::collections::HashSet<&'static str>> =
    Lazy::new(|| ["calc", "clamp", "max", "min"].into_iter().collect());
  &SET_
}

fn normalize_box_shadow(parsed: &vp::ParsedValue) -> Result<String, ()> {
  let args = get_arguments(parsed);
  let mut list: Vec<Vec<vp::Node>> = Vec::new();
  for arg in args.into_iter() {
    let mut val: Vec<vp::Node> = Vec::new();
    let mut inset: Vec<vp::Node> = Vec::new();
    let mut color: Vec<vp::Node> = Vec::new();
    let mut abort = false;
    for node in arg.into_iter() {
      match &node {
        vp::Node::Function { value, .. } => {
          let low = vendor_unprefixed(value.to_lowercase().as_str()).to_string();
          if math_functions_set().contains(low.as_str()) {
            abort = true;
            break;
          }
          color.push(node.clone());
          color.push(add_space());
        }
        vp::Node::Space { .. } => {}
        vp::Node::Word { value } => {
          if vp_unit(value).is_some() {
            val.push(node.clone());
            val.push(add_space());
          } else if value.to_lowercase() == "inset" {
            inset.push(node.clone());
            inset.push(add_space());
          } else {
            color.push(node.clone());
            color.push(add_space());
          }
        }
        _ => {}
      }
    }
    if abort {
      return Err(());
    }
    let mut combined: Vec<vp::Node> = Vec::new();
    combined.extend(inset);
    combined.extend(val);
    combined.extend(color);
    list.push(combined);
  }
  Ok(get_value(list))
}

fn join_grid_value(parts: Vec<String>) -> String {
  parts.join(" / ").trim().to_string()
}

fn normalize_grid_auto_flow(parsed: &vp::ParsedValue) -> String {
  let mut front = String::new();
  let mut back = String::new();
  let mut should = false;
  let mut nodes = parsed.nodes.clone();
  vp::walk(
    &mut nodes[..],
    &mut |node| {
      if let vp::Node::Word { value } = node {
        let v = value.trim().to_lowercase();
        if v == "dense" {
          should = true;
          back = value.clone();
        } else if v == "row" || v == "column" {
          should = true;
          front = value.clone();
        } else {
          should = false;
        }
      }
      true
    },
    false,
  );
  if should {
    format!("{} {}", front.trim(), back.trim())
  } else {
    to_string(parsed)
  }
}

fn normalize_grid_gap(parsed: &vp::ParsedValue) -> String {
  let mut front = String::new();
  let mut back = String::new();
  let mut should = false;
  let mut nodes = parsed.nodes.clone();
  vp::walk(
    &mut nodes[..],
    &mut |node| {
      if let vp::Node::Word { value } = node {
        if value == "normal" {
          should = true;
          front = value.clone();
        } else {
          back.push(' ');
          back.push_str(value);
        }
      }
      true
    },
    false,
  );
  if should {
    format!("{} {}", front.trim(), back.trim())
  } else {
    to_string(parsed)
  }
}

fn normalize_grid_line(parsed: &vp::ParsedValue) -> String {
  // operate on string per JS
  let s = to_string(parsed);
  let grid_value: Vec<String> = s.split('/').map(|p| p.to_string()).collect();
  if grid_value.len() > 1 {
    let mapped: Vec<String> = grid_value
      .into_iter()
      .map(|grid_line| {
        let mut front = String::new();
        let mut back = String::new();
        let gl = grid_line.trim().to_string();
        for token in gl.split(' ') {
          if token == "span" {
            front = token.to_string();
          } else {
            back.push(' ');
            back.push_str(token);
          }
        }
        format!("{} {}", front.trim(), back.trim())
      })
      .collect();
    return join_grid_value(mapped);
  }
  let mut out: Vec<String> = Vec::new();
  for grid_line in s.split('/') {
    let gl = grid_line.trim();
    let mut front = String::new();
    let mut back = String::new();
    for token in gl.split(' ') {
      if token == "span" {
        front = token.to_string();
      } else {
        back.push(' ');
        back.push_str(token);
      }
    }
    out.push(format!("{} {}", front.trim(), back.trim()));
  }
  out.join("")
}

pub fn plugin() -> pc::BuiltPlugin {
  // Map of property -> processor; start with a few implemented processors and fall back to identity.
  let cache = std::sync::Mutex::new(std::collections::HashMap::<String, String>::new());
  pc::plugin("postcss-ordered-values")
    .once_exit(move |css, _| {
      let process_decl = |decl: postcss::ast::nodes::Declaration| {
        let lower_prop = decl.prop().to_lowercase();
        let normalized_prop = vendor_unprefixed(&lower_prop).to_string();
        let supported = matches!(
          normalized_prop.as_str(),
          "animation"
            | "outline"
            | "box-shadow"
            | "flex-flow"
            | "list-style"
            | "transition"
            | "border"
            | "border-top"
            | "border-right"
            | "border-bottom"
            | "border-left"
            | "border-block"
            | "border-inline"
            | "border-block-start"
            | "border-block-end"
            | "border-inline-start"
            | "border-inline-end"
            | "columns"
            | "column-rule"
        );
        if !supported {
          return;
        }
        let value = decl.value();
        if let Some(cached) = cache.lock().unwrap().get(&value).cloned() {
          decl.set_value(cached);
          return;
        }
        let parsed = vp::parse(&value);
        if parsed.nodes.len() < 2 || should_abort(&parsed) {
          cache.lock().unwrap().insert(value.clone(), value.clone());
          return;
        }
        let output = match normalized_prop.as_str() {
          "border"
          | "outline"
          | "border-top"
          | "border-right"
          | "border-bottom"
          | "border-left"
          | "border-block"
          | "border-inline"
          | "border-block-start"
          | "border-block-end"
          | "border-inline-start"
          | "border-inline-end"
          | "column-rule" => rules::border::normalize(&parsed),
          "transition" => rules::transition::normalize(&parsed),
          "list-style" => rules::list_style::normalize(&parsed),
          "columns" => rules::columns::normalize(&parsed),
          "flex-flow" => rules::flex_flow::normalize(&parsed),
          "animation" => lib::get_value::get_value(lib::arguments::get_arguments(&parsed)),
          "box-shadow" => match rules::box_shadow::normalize(&parsed) {
            Ok(s) => s,
            Err(()) => vp::stringify(&parsed.nodes),
          },
          "grid-auto-flow" => rules::grid::normalize_auto_flow(&parsed),
          "grid-column-gap" | "grid-row-gap" => rules::grid::normalize_gap(&parsed),
          "grid-column" | "grid-row" | "grid-row-start" | "grid-row-end" | "grid-column-start"
          | "grid-column-end" => rules::grid::normalize_line(&parsed),
          _ => value.clone(),
        };
        cache.lock().unwrap().insert(value.clone(), output.clone());
        decl.set_value(output);
      };
      match css {
        pc::ast::nodes::RootLike::Root(root) => {
          root.walk_decls(|node, _| {
            if let Some(decl) = postcss::ast::nodes::as_declaration(&node) {
              process_decl(decl);
            }
            true
          });
        }
        pc::ast::nodes::RootLike::Document(doc) => {
          doc.walk_decls(|node, _| {
            if let Some(decl) = postcss::ast::nodes::as_declaration(&node) {
              process_decl(decl);
            }
            true
          });
        }
      }
      Ok(())
    })
    .build()
}
