use crate::postcss::value_parser as vp;
use once_cell::sync::Lazy;
use postcss as pc;
use regex::Regex;
use std::collections::HashMap;
use std::str::FromStr;

static COLOR_FUNCTION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(?i)(rgb|hsl)a?$").unwrap());
static SKIP_PROPERTY_REGEX: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"(?i)^(composes|font|src$|filter|-webkit-tap-highlight-color)").unwrap()
});

// ===== Names plugin mapping (reverse: hex -> name), built to match JS order (last wins) =====
static HEX_TO_NAME: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
  // Pairs: (name, hex)
  let pairs: &[(&str, &str)] = &[
    ("white", "#ffffff"),
    ("bisque", "#ffe4c4"),
    ("blue", "#0000ff"),
    ("cadetblue", "#5f9ea0"),
    ("chartreuse", "#7fff00"),
    ("chocolate", "#d2691e"),
    ("coral", "#ff7f50"),
    ("antiquewhite", "#faebd7"),
    ("aqua", "#00ffff"),
    ("azure", "#f0ffff"),
    ("whitesmoke", "#f5f5f5"),
    ("papayawhip", "#ffefd5"),
    ("plum", "#dda0dd"),
    ("blanchedalmond", "#ffebcd"),
    ("black", "#000000"),
    ("gold", "#ffd700"),
    ("goldenrod", "#daa520"),
    ("gainsboro", "#dcdcdc"),
    ("cornsilk", "#fff8dc"),
    ("cornflowerblue", "#6495ed"),
    ("burlywood", "#deb887"),
    ("aquamarine", "#7fffd4"),
    ("beige", "#f5f5dc"),
    ("crimson", "#dc143c"),
    ("cyan", "#00ffff"),
    ("darkblue", "#00008b"),
    ("darkcyan", "#008b8b"),
    ("darkgoldenrod", "#b8860b"),
    ("darkkhaki", "#bdb76b"),
    ("darkgray", "#a9a9a9"),
    ("darkgreen", "#006400"),
    ("darkgrey", "#a9a9a9"),
    ("peachpuff", "#ffdab9"),
    ("darkmagenta", "#8b008b"),
    ("darkred", "#8b0000"),
    ("darkorchid", "#9932cc"),
    ("darkorange", "#ff8c00"),
    ("darkslateblue", "#483d8b"),
    ("gray", "#808080"),
    ("darkslategray", "#2f4f4f"),
    ("darkslategrey", "#2f4f4f"),
    ("deeppink", "#ff1493"),
    ("deepskyblue", "#00bfff"),
    ("wheat", "#f5deb3"),
    ("firebrick", "#b22222"),
    ("floralwhite", "#fffaf0"),
    ("ghostwhite", "#f8f8ff"),
    ("darkviolet", "#9400d3"),
    ("magenta", "#ff00ff"),
    ("green", "#008000"),
    ("dodgerblue", "#1e90ff"),
    ("grey", "#808080"),
    ("honeydew", "#f0fff0"),
    ("hotpink", "#ff69b4"),
    ("blueviolet", "#8a2be2"),
    ("forestgreen", "#228b22"),
    ("lawngreen", "#7cfc00"),
    ("indianred", "#cd5c5c"),
    ("indigo", "#4b0082"),
    ("fuchsia", "#ff00ff"),
    ("brown", "#a52a2a"),
    ("maroon", "#800000"),
    ("mediumblue", "#0000cd"),
    ("lightcoral", "#f08080"),
    ("darkturquoise", "#00ced1"),
    ("lightcyan", "#e0ffff"),
    ("ivory", "#fffff0"),
    ("lightyellow", "#ffffe0"),
    ("lightsalmon", "#ffa07a"),
    ("lightseagreen", "#20b2aa"),
    ("linen", "#faf0e6"),
    ("mediumaquamarine", "#66cdaa"),
    ("lemonchiffon", "#fffacd"),
    ("lime", "#00ff00"),
    ("khaki", "#f0e68c"),
    ("mediumseagreen", "#3cb371"),
    ("limegreen", "#32cd32"),
    ("mediumspringgreen", "#00fa9a"),
    ("lightskyblue", "#87cefa"),
    ("lightblue", "#add8e6"),
    ("midnightblue", "#191970"),
    ("lightpink", "#ffb6c1"),
    ("mistyrose", "#ffe4e1"),
    ("moccasin", "#ffe4b5"),
    ("mintcream", "#f5fffa"),
    ("lightslategray", "#778899"),
    ("lightslategrey", "#778899"),
    ("navajowhite", "#ffdead"),
    ("navy", "#000080"),
    ("mediumvioletred", "#c71585"),
    ("powderblue", "#b0e0e6"),
    ("palegoldenrod", "#eee8aa"),
    ("oldlace", "#fdf5e6"),
    ("paleturquoise", "#afeeee"),
    ("mediumturquoise", "#48d1cc"),
    ("mediumorchid", "#ba55d3"),
    ("rebeccapurple", "#663399"),
    ("lightsteelblue", "#b0c4de"),
    ("mediumslateblue", "#7b68ee"),
    ("thistle", "#d8bfd8"),
    ("tan", "#d2b48c"),
    ("orchid", "#da70d6"),
    ("mediumpurple", "#9370db"),
    ("purple", "#800080"),
    ("pink", "#ffc0cb"),
    ("skyblue", "#87ceeb"),
    ("springgreen", "#00ff7f"),
    ("palegreen", "#98fb98"),
    ("red", "#ff0000"),
    ("yellow", "#ffff00"),
    ("slateblue", "#6a5acd"),
    ("lavenderblush", "#fff0f5"),
    ("peru", "#cd853f"),
    ("palevioletred", "#db7093"),
    ("violet", "#ee82ee"),
    ("teal", "#008080"),
    ("slategray", "#708090"),
    ("slategrey", "#708090"),
    ("aliceblue", "#f0f8ff"),
    ("darkseagreen", "#8fbc8f"),
    ("darkolivegreen", "#556b2f"),
    ("greenyellow", "#adff2f"),
    ("seagreen", "#2e8b57"),
    ("seashell", "#fff5ee"),
    ("tomato", "#ff6347"),
    ("silver", "#c0c0c0"),
    ("sienna", "#a0522d"),
    ("lavender", "#e6e6fa"),
    ("lightgreen", "#90ee90"),
    ("orange", "#ffa500"),
    ("orangered", "#ff4500"),
    ("steelblue", "#4682b4"),
    ("royalblue", "#4169e1"),
    ("turquoise", "#40e0d0"),
    ("yellowgreen", "#9acd32"),
    ("salmon", "#fa8072"),
    ("saddlebrown", "#8b4513"),
    ("sandybrown", "#f4a460"),
    ("rosybrown", "#bc8f8f"),
    ("darksalmon", "#e9967a"),
    ("lightgoldenrodyellow", "#fafad2"),
    ("snow", "#fffafa"),
    ("lightgrey", "#d3d3d3"),
    ("lightgray", "#d3d3d3"),
    ("dimgray", "#696969"),
    ("dimgrey", "#696969"),
    ("olivedrab", "#6b8e23"),
    ("olive", "#808000"),
  ];
  let mut m = HashMap::new();
  for (name, hex) in pairs.iter() {
    m.insert(*hex, *name);
  }
  m
});

// Inverse mapping for name -> hex to support inputs using named colors.
static NAME_TO_HEX: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
  let pairs: &[(&str, &str)] = &[
    ("white", "#ffffff"),
    ("bisque", "#ffe4c4"),
    ("blue", "#0000ff"),
    ("cadetblue", "#5f9ea0"),
    ("chartreuse", "#7fff00"),
    ("chocolate", "#d2691e"),
    ("coral", "#ff7f50"),
    ("antiquewhite", "#faebd7"),
    ("aqua", "#00ffff"),
    ("azure", "#f0ffff"),
    ("whitesmoke", "#f5f5f5"),
    ("papayawhip", "#ffefd5"),
    ("plum", "#dda0dd"),
    ("blanchedalmond", "#ffebcd"),
    ("black", "#000000"),
    ("gold", "#ffd700"),
    ("goldenrod", "#daa520"),
    ("gainsboro", "#dcdcdc"),
    ("cornsilk", "#fff8dc"),
    ("cornflowerblue", "#6495ed"),
    ("burlywood", "#deb887"),
    ("aquamarine", "#7fffd4"),
    ("beige", "#f5f5dc"),
    ("crimson", "#dc143c"),
    ("cyan", "#00ffff"),
    ("darkblue", "#00008b"),
    ("darkcyan", "#008b8b"),
    ("darkgoldenrod", "#b8860b"),
    ("darkkhaki", "#bdb76b"),
    ("darkgray", "#a9a9a9"),
    ("darkgreen", "#006400"),
    ("darkgrey", "#a9a9a9"),
    ("peachpuff", "#ffdab9"),
    ("darkmagenta", "#8b008b"),
    ("darkred", "#8b0000"),
    ("darkorchid", "#9932cc"),
    ("darkorange", "#ff8c00"),
    ("darkslateblue", "#483d8b"),
    ("gray", "#808080"),
    ("darkslategray", "#2f4f4f"),
    ("darkslategrey", "#2f4f4f"),
    ("deeppink", "#ff1493"),
    ("deepskyblue", "#00bfff"),
    ("wheat", "#f5deb3"),
    ("firebrick", "#b22222"),
    ("floralwhite", "#fffaf0"),
    ("ghostwhite", "#f8f8ff"),
    ("darkviolet", "#9400d3"),
    ("magenta", "#ff00ff"),
    ("green", "#008000"),
    ("dodgerblue", "#1e90ff"),
    ("grey", "#808080"),
    ("honeydew", "#f0fff0"),
    ("hotpink", "#ff69b4"),
    ("blueviolet", "#8a2be2"),
    ("forestgreen", "#228b22"),
    ("lawngreen", "#7cfc00"),
    ("indianred", "#cd5c5c"),
    ("indigo", "#4b0082"),
    ("fuchsia", "#ff00ff"),
    ("brown", "#a52a2a"),
    ("maroon", "#800000"),
    ("mediumblue", "#0000cd"),
    ("lightcoral", "#f08080"),
    ("darkturquoise", "#00ced1"),
    ("lightcyan", "#e0ffff"),
    ("ivory", "#fffff0"),
    ("lightyellow", "#ffffe0"),
    ("lightsalmon", "#ffa07a"),
    ("lightseagreen", "#20b2aa"),
    ("linen", "#faf0e6"),
    ("mediumaquamarine", "#66cdaa"),
    ("lemonchiffon", "#fffacd"),
    ("lime", "#00ff00"),
    ("khaki", "#f0e68c"),
    ("mediumseagreen", "#3cb371"),
    ("limegreen", "#32cd32"),
    ("mediumspringgreen", "#00fa9a"),
    ("lightskyblue", "#87cefa"),
    ("lightblue", "#add8e6"),
    ("midnightblue", "#191970"),
    ("lightpink", "#ffb6c1"),
    ("mistyrose", "#ffe4e1"),
    ("moccasin", "#ffe4b5"),
    ("mintcream", "#f5fffa"),
    ("lightslategray", "#778899"),
    ("lightslategrey", "#778899"),
    ("navajowhite", "#ffdead"),
    ("navy", "#000080"),
    ("mediumvioletred", "#c71585"),
    ("powderblue", "#b0e0e6"),
    ("palegoldenrod", "#eee8aa"),
    ("oldlace", "#fdf5e6"),
    ("paleturquoise", "#afeeee"),
    ("mediumturquoise", "#48d1cc"),
    ("mediumorchid", "#ba55d3"),
    ("rebeccapurple", "#663399"),
    ("lightsteelblue", "#b0c4de"),
    ("mediumslateblue", "#7b68ee"),
    ("thistle", "#d8bfd8"),
    ("tan", "#d2b48c"),
    ("orchid", "#da70d6"),
    ("mediumpurple", "#9370db"),
    ("purple", "#800080"),
    ("pink", "#ffc0cb"),
    ("skyblue", "#87ceeb"),
    ("springgreen", "#00ff7f"),
    ("palegreen", "#98fb98"),
    ("red", "#ff0000"),
    ("yellow", "#ffff00"),
    ("slateblue", "#6a5acd"),
    ("lavenderblush", "#fff0f5"),
    ("peru", "#cd853f"),
    ("palevioletred", "#db7093"),
    ("violet", "#ee82ee"),
    ("teal", "#008080"),
    ("slategray", "#708090"),
    ("slategrey", "#708090"),
    ("aliceblue", "#f0f8ff"),
    ("darkseagreen", "#8fbc8f"),
    ("darkolivegreen", "#556b2f"),
    ("greenyellow", "#adff2f"),
    ("seagreen", "#2e8b57"),
    ("seashell", "#fff5ee"),
    ("tomato", "#ff6347"),
    ("silver", "#c0c0c0"),
    ("sienna", "#a0522d"),
    ("lavender", "#e6e6fa"),
    ("lightgreen", "#90ee90"),
    ("orange", "#ffa500"),
    ("orangered", "#ff4500"),
    ("steelblue", "#4682b4"),
    ("royalblue", "#4169e1"),
    ("turquoise", "#40e0d0"),
    ("yellowgreen", "#9acd32"),
    ("salmon", "#fa8072"),
    ("saddlebrown", "#8b4513"),
    ("sandybrown", "#f4a460"),
    ("rosybrown", "#bc8f8f"),
    ("darksalmon", "#e9967a"),
    ("lightgoldenrodyellow", "#fafad2"),
    ("snow", "#fffafa"),
    ("lightgrey", "#d3d3d3"),
    ("lightgray", "#d3d3d3"),
    ("dimgray", "#696969"),
    ("dimgrey", "#696969"),
    ("olivedrab", "#6b8e23"),
    ("olive", "#808000"),
  ];
  let mut m = HashMap::new();
  for (name, hex) in pairs.iter() {
    m.insert(*name, *hex);
  }
  m
});

fn is_math_function(node: &vp::Node) -> bool {
  match node {
    vp::Node::Function { value, .. } => matches!(
      value.to_lowercase().as_str(),
      "calc" | "min" | "max" | "clamp"
    ),
    _ => false,
  }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ColorminOptions {
  transparent: bool,
  alpha_hex: bool,
  name: bool,
}

pub(crate) fn add_plugin_defaults() -> ColorminOptions {
  // Defaults per plugin when no caniuse data is provided via browserslist env:
  // - transparent: true (unless IE 8/9 detected)
  // - alphaHex: false (conservative without caniuse-api to avoid rrggbbaa)
  // - name: true
  ColorminOptions {
    transparent: true,
    alpha_hex: false,
    name: true,
  }
}

fn number_short(n: f64) -> String {
  // If between 0 and 1, emit like .5 instead of 0.5
  if n > 0.0 && n < 1.0 {
    let mut buf = ryu_js::Buffer::new();
    let s = buf.format_finite(n);
    return s.replacen("0.", ".", 1);
  }
  let mut buf = ryu_js::Buffer::new();
  buf.format_finite(n).to_string()
}

fn to_hex_rgba(r: u8, g: u8, b: u8, a: u8) -> String {
  format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
}

fn short_hex_candidate(base_hex: &str, alpha: f32) -> Option<String> {
  // base_hex is #rrggbb or #rrggbbaa (lowercase)
  let chars: Vec<char> = base_hex.chars().collect();
  let (s, o, u, l, p, f, g, v) = (
    chars[1],
    chars[2],
    chars[3],
    chars[4],
    chars[5],
    chars[6],
    *chars.get(7).unwrap_or(&'f'),
    *chars.get(8).unwrap_or(&'f'),
  );
  if alpha > 0.0 && alpha < 1.0 {
    // JS plugin rejects alpha hex for fractional alpha (n=0)
    return None;
  }
  if s == o && u == l && p == f {
    if alpha == 1.0 {
      return Some(format!("#{}{}{}", s, u, p));
    }
    if alpha == 0.0 && g == v {
      return Some(format!("#{}{}{}{}", s, u, p, g));
    }
  }
  Some(base_hex.to_string())
}

fn hex_string_from_rgba(r: u8, g: u8, b: u8, alpha: f32) -> String {
  if alpha >= 1.0 {
    format!("#{:02x}{:02x}{:02x}", r, g, b)
  } else if alpha <= 0.0 {
    // represent as 4-digit short when possible (#0000), else 8-digit
    let base = to_hex_rgba(r, g, b, 0);
    short_hex_candidate(&base, 0.0).unwrap_or(base)
  } else {
    // fractional alpha; produce 8-digit but short_hex_candidate will reject (None), so fallback to 8-digit
    let a = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    to_hex_rgba(r, g, b, a)
  }
}

fn rgba_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
  let r = r as f32 / 255.0;
  let g = g as f32 / 255.0;
  let b = b as f32 / 255.0;
  let max = r.max(g).max(b);
  let min = r.min(g).min(b);
  let mut h;
  let l = (max + min) / 2.0;
  let d = max - min;
  let s = if d == 0.0 {
    0.0
  } else {
    d / (1.0 - (2.0 * l - 1.0).abs())
  };
  if d == 0.0 {
    h = 0.0;
  } else if max == r {
    h = ((g - b) / d) % 6.0;
  } else if max == g {
    h = (b - r) / d + 2.0;
  } else {
    h = (r - g) / d + 4.0;
  }
  h *= 60.0;
  if h < 0.0 {
    h += 360.0;
  }
  (h, s * 100.0, l * 100.0)
}

fn minify_color(input: &str, options: &ColorminOptions) -> String {
  // Try parsing with csscolorparser (supports many forms), falling back to our name table.
  let parsed = csscolorparser::Color::from_str(input).or_else(|_e| {
    let lower = input.trim().to_ascii_lowercase();
    if let Some(hex) = NAME_TO_HEX.get(lower.as_str()) {
      // Parse the resolved hex so later logic can share the same path
      csscolorparser::Color::from_str(hex)
    } else {
      // Return any parse error; caller only checks Ok/Err
      Err(csscolorparser::ParseColorError::InvalidHex)
    }
  });
  if let Ok(color) = parsed {
    let (r_f, g_f, b_f, a_f) = (color.r, color.g, color.b, color.a);
    let (r, g, b) = (
      (r_f * 255.0).round().clamp(0.0, 255.0) as u8,
      (g_f * 255.0).round().clamp(0.0, 255.0) as u8,
      (b_f * 255.0).round().clamp(0.0, 255.0) as u8,
    );
    let a = a_f;

    let mut candidates: Vec<String> = Vec::new();

    // hex first when allowed; prefer shortest (#rgb over #rrggbb) for opaque
    if a >= 1.0 || options.alpha_hex {
      let base = if a >= 1.0 {
        format!("#{:02x}{:02x}{:02x}", r, g, b)
      } else {
        hex_string_from_rgba(r, g, b, a as f32)
      };
      let alpha = (a.min(1.0).max(0.0)) as f32;
      if a >= 1.0 {
        // Shorten opaque hex to 3-digit when possible
        let short = short_hex_literal(&base);
        candidates.push(short);
      } else {
        if let Some(short) = short_hex_candidate(&base, alpha) {
          candidates.push(short);
        } else {
          candidates.push(base);
        }
      }
    }

    // rgb/rgba
    let rgb = if a >= 1.0 {
      format!("rgb({},{},{})", r, g, b)
    } else {
      let a_str = number_short(a as f64);
      format!("rgba({},{},{},{})", r, g, b, a_str)
    };
    candidates.push(rgb);

    // hsl/hsla
    let (h, s, l) = rgba_to_hsl(r, g, b);
    let h_str = number_short(h as f64);
    let s_str = number_short(s as f64);
    let l_str = number_short(l as f64);
    if a >= 1.0 {
      candidates.push(format!("hsl({},{}% ,{}%)", h_str, s_str, l_str).replace(" %", "%"));
    } else {
      let a_str = number_short(a as f64);
      candidates
        .push(format!("hsla({},{}% ,{}%,{})", h_str, s_str, l_str, a_str).replace(" %", "%"));
    }

    // transparent
    if options.transparent && r == 0 && g == 0 && b == 0 && a == 0.0 {
      candidates.push("transparent".to_string());
    }

    // name (only if opaque) — prefer only when strictly shorter than hex
    if options.name && a >= 1.0 {
      let hex6 = format!("#{:02x}{:02x}{:02x}", r, g, b);
      if let Some(name) = HEX_TO_NAME.get(hex6.as_str()) {
        candidates.push((*name).to_string());
      }
    }

    // pick shortest; on ties keep earlier (JS behaviour)
    if let Some(mut best) = candidates.get(0).cloned() {
      for s in candidates.into_iter().skip(1) {
        if s.len() < best.len() {
          best = s;
        }
      }
      if best.len() < input.len() {
        return best;
      } else {
        return input.to_ascii_lowercase();
      }
    }
  }
  // Fallback: handle named colors explicitly when parser didn't.
  let lower = input.trim().to_ascii_lowercase();
  if let Some(hex) = NAME_TO_HEX.get(lower.as_str()) {
    // Prefer shortened hex when possible.
    let short = short_hex_literal(hex);
    if short.len() < input.len() {
      return short;
    }
    return hex.to_string();
  }
  input.to_string()
}

fn short_hex_literal(hex: &str) -> String {
  let h = hex.trim_start_matches('#');
  let h = h.to_ascii_lowercase();
  if h.len() == 6 {
    let b: Vec<char> = h.chars().collect();
    if b[0] == b[1] && b[2] == b[3] && b[4] == b[5] {
      return format!("#{}{}{}", b[0], b[2], b[4]);
    }
  }
  format!("#{}", h)
}

fn walk(
  parent: &mut vp::ParsedValue,
  cb: &mut dyn FnMut(&mut vp::Node, usize) -> (bool, Option<vp::Node>),
) {
  let mut i = 0usize;
  while i < parent.nodes.len() {
    let (bubble, insert_after) = {
      let node = &mut parent.nodes[i];
      cb(node, i)
    };
    if let Some(to_insert) = insert_after {
      parent.nodes.insert(i + 1, to_insert);
      i += 1;
    }
    if let vp::Node::Function { nodes, .. } = &mut parent.nodes[i] {
      if bubble {
        let mut inner = vp::ParsedValue {
          nodes: nodes.clone(),
        };
        walk(&mut inner, cb);
        *nodes = inner.nodes;
      }
    }
    i += 1;
  }
}

pub(crate) fn transform_value(value: &str, options: &ColorminOptions) -> String {
  let mut parsed = vp::parse(value);
  walk(&mut parsed, &mut |node, _index| {
    match node {
      vp::Node::Function { value, nodes, .. } => {
        if COLOR_FUNCTION_REGEX.is_match(value) {
          let original = value.clone();
          let contents = vp::stringify(nodes);
          let newv = minify_color(&format!("{}({})", original, contents), options);
          *node = vp::Node::Word {
            value: newv.clone(),
          };
          if newv.to_lowercase() != original.to_lowercase() {
            return (
              true,
              Some(vp::Node::Space {
                value: " ".to_string(),
              }),
            );
          }
        } else if is_math_function(node) {
          return (false, None);
        }
      }
      vp::Node::Word { value } => {
        *value = minify_color(value, options);
      }
      _ => {}
    }
    (true, None)
  });
  vp::stringify(&parsed.nodes)
}

fn resolve_browsers() -> Vec<String> {
  Vec::new()
}

pub fn plugin() -> pc::BuiltPlugin {
  // browserslist resolution (used only for transparent bug). If unavailable, default modern.
  let browsers: Vec<String> = resolve_browsers();
  let has_ie8_9 = browsers.iter().any(|b| b == "ie 8" || b == "ie 9");
  let mut options = add_plugin_defaults();
  if has_ie8_9 {
    options.transparent = false;
  }

  let cache = std::sync::Mutex::new(HashMap::<String, String>::new());
  pc::plugin("postcss-colormin")
    .decl(move |decl, _| {
      // Skip properties cssnano excludes
      if SKIP_PROPERTY_REGEX.is_match(&decl.prop()) {
        return Ok(());
      }

      let value = decl.value();
      if value.is_empty() {
        return Ok(());
      }

      let key = format!(
        "{:?}",
        (
          &value,
          options.transparent,
          options.alpha_hex,
          options.name,
          &browsers
        )
      );
      if let Some(v) = cache.lock().unwrap().get(&key).cloned() {
        decl.set_value(v);
        return Ok(());
      }

      let picked = transform_value(&value, &options);
      if std::env::var("COMPILED_DEBUG_COLORMIN").is_ok() {
        // Build context (parent chain) for debugging
        fn context_for_decl(decl: &postcss::ast::nodes::Declaration) -> String {
          use postcss::ast::nodes::as_rule;
          let node = decl.to_node();
          let mut ctx: Vec<String> = Vec::new();
          let mut current = node.borrow().parent();
          while let Some(p) = current {
            let label = {
              if let Some(rule) = as_rule(&p) {
                format!("rule: {}", rule.selector())
              } else if p.borrow().kind() == postcss::NodeKind::AtRule {
                "AtRule".to_string()
              } else {
                let borrowed = p.borrow();
                format!("{:?}", borrowed.kind())
              }
            };
            ctx.push(label);
            current = p.borrow().parent();
          }
          ctx.join(" <- ")
        }
        eprintln!(
          "[colormin] {}: '{}' -> '{}'  [{}]",
          decl.prop(),
          value,
          picked,
          context_for_decl(decl)
        );
      }
      decl.set_value(picked.clone());
      cache.lock().unwrap().insert(key, picked);
      Ok(())
    })
    .build()
}
