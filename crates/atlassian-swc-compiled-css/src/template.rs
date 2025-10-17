use std::borrow::Cow;
use std::hash::Hasher;

use swc_core::ecma::ast::*;

use crate::evaluate_expr::{
  ConstValue, ValueOrNumber, eval_const_expr, eval_expr_to_const, eval_value_expr,
};
use crate::shorthand;
use crate::utils::{normalize_at_query, normalize_nested_selector, to_kebab_case};
use crate::{AtomicCssCollector, emit::RuleInput};

impl AtomicCssCollector {
  fn normalize_selector(sel: &str) -> String {
    // Collapse whitespace and remove spaces around combinators '>', '+', '~'
    let bytes = sel.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0usize;
    let mut pending_space = false;
    let mut in_str: u8 = 0;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    while i < n {
      let b = bytes[i];
      if in_str != 0 {
        out.push(b as char);
        if b == in_str {
          in_str = 0;
        } else if b == b'\\' && i + 1 < n {
          out.push(bytes[i + 1] as char);
          i += 1;
        }
        i += 1;
        continue;
      }
      if b == b'\'' || b == b'\"' {
        in_str = b;
        out.push(b as char);
        i += 1;
        continue;
      }
      if b.is_ascii_whitespace() {
        pending_space = true;
        i += 1;
        continue;
      }
      if b == b'(' {
        paren += 1;
      }
      if b == b')' && paren > 0 {
        paren -= 1;
      }
      if b == b'[' {
        bracket += 1;
      }
      if b == b']' && bracket > 0 {
        bracket -= 1;
      }
      let is_combinator = (b == b'>' || b == b'+' || b == b'~') && paren == 0 && bracket == 0;
      if is_combinator {
        // remove any pending space before combinator
        if out.ends_with(' ') {
          out.pop();
        }
        out.push(b as char);
        // skip any following whitespace
        i += 1;
        while i < n && bytes[i].is_ascii_whitespace() {
          i += 1;
        }
        pending_space = false;
        continue;
      }
      if pending_space {
        out.push(' ');
        pending_space = false;
      }
      out.push(b as char);
      i += 1;
    }
    out.trim().to_string()
  }
  pub(crate) fn is_unitless_property_generic(name: &str) -> bool {
    if AtomicCssCollector::is_unitless_property(name) {
      return true;
    }
    if name.contains('-') {
      // Build camelCase (line-height -> lineHeight)
      let mut camel_lower = String::new();
      let mut camel_upper = String::new();
      let mut parts = name.split('-').filter(|p| !p.is_empty());
      if let Some(first) = parts.next() {
        let first_lower = first
          .chars()
          .flat_map(|c| c.to_lowercase())
          .collect::<String>();
        camel_lower.push_str(&first_lower);
        let mut first_upper = first_lower.clone();
        if let Some(fc) = first_upper.get_mut(0..1) {
          fc.make_ascii_uppercase();
        }
        camel_upper.push_str(&first_upper);
      }
      for part in parts {
        let lower = part
          .chars()
          .flat_map(|c| c.to_lowercase())
          .collect::<String>();
        let mut chars = lower.chars();
        if let Some(f) = chars.next() {
          camel_lower.push(f.to_ascii_uppercase());
          camel_lower.extend(chars);
        }
        let mut chars2 = lower.chars();
        if let Some(f2) = chars2.next() {
          camel_upper.push(f2.to_ascii_uppercase());
          camel_upper.extend(chars2);
        }
      }
      if AtomicCssCollector::is_unitless_property(&camel_lower)
        || AtomicCssCollector::is_unitless_property(&camel_upper)
      {
        return true;
      }
    }
    false
  }
  pub fn eval_expr_to_string(&mut self, expr: &Expr) -> Option<String> {
    if let Some(cv) = eval_const_expr(expr) {
      return Some(match cv {
        ConstValue::Str(s) => s,
        ConstValue::Num(n) => {
          if (n - (n as i64 as f64)).abs() < f64::EPSILON {
            (n as i64).to_string()
          } else {
            n.to_string()
          }
        }
        _ => return None,
      });
    }
    if let Some(cv) = eval_expr_to_const(expr, &self.const_env) {
      return Some(match cv {
        ConstValue::Str(s) => s,
        ConstValue::Num(n) => {
          if (n - (n as i64 as f64)).abs() < f64::EPSILON {
            (n as i64).to_string()
          } else {
            n.to_string()
          }
        }
        _ => return None,
      });
    }
    // Fallback: support inline keyframes() in template interpolations
    if let Some(name) = self.maybe_compile_keyframes_to_name(expr) {
      return Some(name);
    }
    None
  }

  pub fn build_css_text_from_tpl(&mut self, tpl: &Tpl) -> Option<String> {
    // Pre-size using raw quasis length to reduce reallocations
    let approx_cap: usize = tpl
      .quasis
      .iter()
      .map(|q| q.raw.as_ref().len())
      .sum::<usize>()
      + tpl.exprs.len() * 8;
    let mut out = String::with_capacity(approx_cap);
    for (idx, quasi) in tpl.quasis.iter().enumerate() {
      out.push_str(quasi.raw.as_ref());
      if let Some(expr) = tpl.exprs.get(idx) {
        if let Some(s) = self.eval_expr_to_string(expr) {
          out.push_str(&s);
        } else if self.allow_runtime_vars {
          // Not const - replace with CSS var and record runtime entry
          let var_name = self.create_runtime_var_for_expr(expr, false);
          out.push_str("var(");
          out.push_str(&var_name);
          out.push(')');
        } else {
          return None;
        }
      }
    }
    Some(out)
  }

  fn collect_atomic_classes_from_css_text_inner(
    &mut self,
    css_text: &str,
    base_suffix: &str,
    wrappers: &mut Vec<String>,
    out_classes: &mut Vec<String>,
  ) {
    use ValueOrNumber::Str as StrVal;
    fn skip_ws(s: &str, mut i: usize) -> usize {
      while let Some(ch) = s.as_bytes().get(i) {
        if ch.is_ascii_whitespace() {
          i += 1;
        } else {
          break;
        }
      }
      i
    }
    // (removed unused helper find_next)
    fn skip_comment(s: &str, mut i: usize) -> usize {
      let n = s.len();
      if i + 1 < n && s.as_bytes()[i] == b'/' && s.as_bytes()[i + 1] == b'*' {
        i += 2;
        while i + 1 < n {
          if s.as_bytes()[i] == b'*' && s.as_bytes()[i + 1] == b'/' {
            i += 2;
            break;
          }
          i += 1;
        }
      }
      i
    }
    fn skip_string(s: &str, mut i: usize) -> usize {
      let n = s.len();
      let quote = s.as_bytes()[i];
      i += 1;
      while i < n {
        let b = s.as_bytes()[i];
        if b == quote {
          i += 1;
          break;
        }
        if b == b'\\' {
          i += 2;
        } else {
          i += 1;
        }
      }
      i
    }
    fn skip_parens(s: &str, mut i: usize) -> usize {
      let n = s.len();
      if s.as_bytes()[i] == b'(' {
        i += 1;
      }
      let mut depth = 1usize;
      while i < n && depth > 0 {
        let b = s.as_bytes()[i];
        if b == b'\'' || b == b'"' {
          i = skip_string(s, i);
          continue;
        }
        if b == b'/' && i + 1 < n && s.as_bytes()[i + 1] == b'*' {
          i = skip_comment(s, i);
          continue;
        }
        if b == b'(' {
          depth += 1;
          i += 1;
          continue;
        }
        if b == b')' {
          depth -= 1;
          i += 1;
          continue;
        }
        i += 1;
      }
      i
    }
    fn find_matching_brace(s: &str, mut i: usize) -> usize {
      let n = s.len();
      let mut depth = 1usize;
      while i < n && depth > 0 {
        let b = s.as_bytes()[i];
        if b == b'\'' || b == b'"' {
          i = skip_string(s, i);
          continue;
        }
        if b == b'/' && i + 1 < n && s.as_bytes()[i + 1] == b'*' {
          i = skip_comment(s, i);
          continue;
        }
        if b == b'(' {
          i = skip_parens(s, i);
          continue;
        }
        if b == b'{' {
          depth += 1;
          i += 1;
          continue;
        }
        if b == b'}' {
          depth -= 1;
          i += 1;
          if depth == 0 {
            break;
          }
          continue;
        }
        i += 1;
      }
      i
    }
    fn split_selectors_top_level(sel: &str) -> Vec<String> {
      let mut out = Vec::new();
      let mut start = 0usize;
      let bytes = sel.as_bytes();
      let mut i = 0usize;
      let n = bytes.len();
      let mut paren = 0usize;
      let mut bracket = 0usize;
      let mut in_str: u8 = 0;
      while i < n {
        let b = bytes[i];
        if in_str != 0 {
          if b == in_str {
            in_str = 0;
          } else if b == b'\\' {
            i += 1;
          }
          i += 1;
          continue;
        }
        if b == b'\'' || b == b'"' {
          in_str = b;
          i += 1;
          continue;
        }
        if b == b'(' {
          paren += 1;
          i += 1;
          continue;
        }
        if b == b')' && paren > 0 {
          paren -= 1;
          i += 1;
          continue;
        }
        if b == b'[' {
          bracket += 1;
          i += 1;
          continue;
        }
        if b == b']' && bracket > 0 {
          bracket -= 1;
          i += 1;
          continue;
        }
        if b == b',' && paren == 0 && bracket == 0 {
          out.push(sel[start..i].trim().to_string());
          i += 1;
          start = i;
          continue;
        }
        i += 1;
      }
      if start < n {
        out.push(sel[start..n].trim().to_string());
      }
      out.into_iter().filter(|s| !s.is_empty()).collect()
    }

    fn parse_block(
      this: &mut AtomicCssCollector,
      s: &str,
      mut i: usize,
      suffix: &str,
      wrappers: &mut Vec<String>,
      out_classes: &mut Vec<String>,
    ) -> usize {
      let n = s.len();
      while i < n {
        i = skip_ws(s, i);
        if i >= n {
          break;
        }
        let ch = s.as_bytes()[i];
        if ch == b'}' {
          i += 1;
          break;
        }
        if ch == b'/' && i + 1 < n && s.as_bytes()[i + 1] == b'*' {
          i = skip_comment(s, i);
          continue;
        }
        if ch == b'@' {
          let start = i;
          i += 1;
          while i < n {
            let b = s.as_bytes()[i];
            if b == b'\'' || b == b'"' {
              i = skip_string(s, i);
              continue;
            }
            if b == b'/' && i + 1 < n && s.as_bytes()[i + 1] == b'*' {
              i = skip_comment(s, i);
              continue;
            }
            if b == b'(' {
              i = skip_parens(s, i);
              continue;
            }
            if b == b'{' || b == b'}' {
              break;
            }
            i += 1;
          }
          let header = s[start..i].trim();
          let wrapper = if let Some(stripped) = header.strip_prefix("@media") {
            let normalized_q = normalize_at_query(stripped.trim());
            if normalized_q.starts_with('(') {
              format!("@media {}", normalized_q)
            } else if normalized_q.is_empty() {
              "@media".to_string()
            } else {
              format!("@media ({})", normalized_q)
            }
          } else if let Some(stripped) = header.strip_prefix("@supports") {
            let normalized_q = normalize_at_query(stripped.trim());
            if normalized_q.starts_with('(') {
              format!("@supports {}", normalized_q)
            } else if normalized_q.is_empty() {
              "@supports".to_string()
            } else {
              format!("@supports ({})", normalized_q)
            }
          } else {
            if i < n && s.as_bytes()[i] == b'{' {
              let body_start = i + 1;
              let body_end = find_matching_brace(s, body_start);
              i = if body_end < n { body_end } else { n };
              if i < n && s.as_bytes()[i] == b'}' {
                i += 1;
              }
            }
            continue;
          };
          if i < n && s.as_bytes()[i] == b'{' {
            let body_start = i + 1;
            let body_end = find_matching_brace(s, body_start);
            if body_end <= n {
              wrappers.push(wrapper);
              let body = &s[body_start..(body_end - 1)];
              let _ = parse_block(this, body, 0, suffix, wrappers, out_classes);
              wrappers.pop();
              i = body_end;
              continue;
            }
          }
          continue;
        }
        let name_start = i;
        let mut j = i;
        let mut pos_colon = usize::MAX;
        let mut pos_lbrace = usize::MAX;
        while j < n {
          let b = s.as_bytes()[j];
          if b == b'\'' || b == b'"' {
            j = skip_string(s, j);
            continue;
          }
          if b == b'/' && j + 1 < n && s.as_bytes()[j + 1] == b'*' {
            j = skip_comment(s, j);
            continue;
          }
          if b == b'(' {
            j = skip_parens(s, j);
            continue;
          }
          if b == b':' && pos_colon == usize::MAX {
            pos_colon = j;
            j += 1;
            continue;
          }
          if b == b'{' {
            pos_lbrace = j;
            break;
          }
          if b == b';' || b == b'}' {
            break;
          }
          j += 1;
        }
        let first = s.as_bytes()[name_start];
        // Treat leading combinators, attribute/universal selectors, and tag names as selectors
        let force_selector = matches!(
            first,
            b'&' | b':' | b'.' | b'#' | b'>' | b'+' | b'~' | b'[' | b'*' | b'a'..=b'z' | b'A'..=b'Z'
        );
        if pos_lbrace != usize::MAX
          && (force_selector || pos_colon == usize::MAX || pos_lbrace < pos_colon)
        {
          let sel_raw = s[name_start..pos_lbrace].trim();
          let body_start = pos_lbrace + 1;
          let body_end = find_matching_brace(s, body_start);
          let body = if body_end <= n {
            &s[body_start..(body_end - 1)]
          } else {
            ""
          };
          for sel in split_selectors_top_level(sel_raw) {
            let normalized = if sel.starts_with('&') || sel.starts_with(':') {
              normalize_nested_selector(&sel)
            } else {
              AtomicCssCollector::normalize_selector(&sel)
            };
            let mut new_suffix = normalized.clone();
            if let Some(stripped) = normalized.strip_prefix('&') {
              let mut ns = String::with_capacity(suffix.len() + normalized.len());
              ns.push_str(suffix);
              ns.push_str(stripped);
              new_suffix = ns;
            }
            let _ = parse_block(this, body, 0, &new_suffix, wrappers, out_classes);
          }
          i = if body_end <= n { body_end } else { n };
          continue;
        }
        if pos_colon != usize::MAX && (pos_lbrace == usize::MAX || pos_colon < pos_lbrace) {
          // Property name slice and fast kebab decision
          let prop_slice = s[name_start..pos_colon].trim();
          // Avoid allocation if already kebab-case (common in CSS text)
          let mut needs_kebab = false;
          for ch in prop_slice.bytes() {
            if ch.is_ascii_uppercase() {
              needs_kebab = true;
              break;
            }
          }
          let prop_kebab_cow: Cow<str> = if needs_kebab {
            Cow::Owned(to_kebab_case(prop_slice))
          } else {
            Cow::Borrowed(prop_slice)
          };
          let prop_kebab: &str = prop_kebab_cow.as_ref();

          // Parse value range, producing a borrowed trimmed slice when possible
          i = pos_colon + 1;
          let val_start_scan = i;
          while i < n {
            let b = s.as_bytes()[i];
            if b == b'\'' || b == b'"' {
              i = skip_string(s, i);
              continue;
            }
            if b == b'/' && i + 1 < n && s.as_bytes()[i + 1] == b'*' {
              i = skip_comment(s, i);
              continue;
            }
            if b == b'(' {
              i = skip_parens(s, i);
              continue;
            }
            if b == b';' || b == b'}' {
              break;
            }
            i += 1;
          }
          // Trim leading/trailing whitespace and an optional trailing ';' without allocating
          let mut val_lo = val_start_scan;
          let mut val_hi = i;
          while val_lo < val_hi && s.as_bytes()[val_lo].is_ascii_whitespace() {
            val_lo += 1;
          }
          while val_hi > val_lo && s.as_bytes()[val_hi - 1].is_ascii_whitespace() {
            val_hi -= 1;
          }
          if val_hi > val_lo && s.as_bytes()[val_hi - 1] == b';' {
            val_hi -= 1;
          }
          while val_hi > val_lo && s.as_bytes()[val_hi - 1].is_ascii_whitespace() {
            val_hi -= 1;
          }
          let val_slice = &s[val_lo..val_hi];

          if shorthand::is_shorthand_prop(prop_kebab) {
            // Allocate only for the expansion path
            let val_owned = val_slice.to_string();
            if let Some(expanded) = shorthand::try_expand_shorthand(prop_kebab, &StrVal(val_owned))
            {
              for (sub_prop, sub_val) in expanded {
                this.emit_rule_input(
                  RuleInput {
                    prop_kebab: &sub_prop,
                    value_text: Cow::Borrowed(sub_val.as_str()),
                    suffix,
                    wrappers,
                  },
                  out_classes,
                );
              }
            } else {
              // Fallback: emit shorthand as-is (e.g., border: none)
              this.emit_rule_input(
                RuleInput {
                  prop_kebab,
                  value_text: Cow::Borrowed(val_slice),
                  suffix,
                  wrappers,
                },
                out_classes,
              );
            }
          } else {
            // In raw CSS text, attempt to convert value to runtime var if allowed and non-const
            if this.allow_runtime_vars {
              // Heuristic: when value contains interpolation markers, this path is unused (handled earlier).
              this.emit_rule_input(
                RuleInput {
                  prop_kebab,
                  value_text: Cow::Borrowed(val_slice),
                  suffix,
                  wrappers,
                },
                out_classes,
              );
            } else {
              this.emit_rule_input(
                RuleInput {
                  prop_kebab,
                  value_text: Cow::Borrowed(val_slice),
                  suffix,
                  wrappers,
                },
                out_classes,
              );
            }
          }
          if i < n && s.as_bytes()[i] == b';' {
            i += 1;
          }
          continue;
        }
        i += 1;
      }
      i
    }
    let _ = parse_block(self, css_text, 0, base_suffix, wrappers, out_classes);
  }

  pub fn collect_atomic_classes_from_css_text(&mut self, css_text: &str) -> Vec<String> {
    if let Some(v) = self.collect_atomic_classes_from_css_text_swc(css_text) {
      return v;
    }
    let mut out = Vec::new();
    let mut wrappers: Vec<String> = Vec::new();
    self.collect_atomic_classes_from_css_text_inner(css_text, "", &mut wrappers, &mut out);
    out
  }

  // Build CSS text from an object literal when all values are statically resolvable.
  // Returns None if any value requires runtime evaluation or unsupported patterns are found.
  pub fn build_css_text_from_object(&mut self, obj: &ObjectLit) -> Option<String> {
    fn num_to_string(n: f64) -> String {
      if (n - (n as i64 as f64)).abs() < f64::EPSILON {
        (n as i64).to_string()
      } else {
        n.to_string()
      }
    }
    fn append_block(
      this: &mut AtomicCssCollector,
      obj: &ObjectLit,
      buf: &mut String,
    ) -> Option<()> {
      for p in &obj.props {
        match p {
          PropOrSpread::Prop(pp) => {
            if let Prop::KeyValue(kv) = &**pp {
              // Key name as string
              let key_str = match &kv.key {
                PropName::Ident(i) => i.sym.to_string(),
                PropName::Str(s) => s.value.to_string(),
                PropName::Num(n) => n.value.to_string(),
                PropName::BigInt(b) => b.value.to_string(),
                PropName::Computed(_) => return None,
              };
              // Nested selector (&, :, contains &)
              if key_str.starts_with('&') || key_str.starts_with(':') || key_str.contains('&') {
                if let Expr::Object(nested) = &*kv.value {
                  let normalized = if key_str.starts_with('&') || key_str.starts_with(':') {
                    normalize_nested_selector(&key_str)
                  } else {
                    key_str.clone()
                  };
                  buf.push_str(&normalized);
                  buf.push('{');
                  append_block(this, nested, buf)?;
                  buf.push('}');
                  continue;
                } else {
                  return None;
                }
              }
              // At-rules we support: @media/@supports
              if key_str.starts_with('@') {
                if key_str.starts_with("@media") || key_str.starts_with("@supports") {
                  if let Expr::Object(nested) = &*kv.value {
                    let (name, rest) = if let Some(stripped) = key_str.strip_prefix("@media") {
                      ("@media", stripped)
                    } else {
                      ("@supports", key_str.strip_prefix("@supports").unwrap_or(""))
                    };
                    let normalized_q = normalize_at_query(rest.trim_start());
                    if normalized_q.is_empty() {
                      buf.push_str(name);
                    } else if normalized_q.starts_with('(') {
                      buf.push_str(&format!("{} {}", name, normalized_q));
                    } else {
                      buf.push_str(&format!("{} ({})", name, normalized_q));
                    }
                    buf.push('{');
                    append_block(this, nested, buf)?;
                    buf.push('}');
                    continue;
                  } else {
                    return None;
                  }
                }
                // Unsupported at-rule in fast path
                return None;
              }
              // @property handled elsewhere; bail out to preserve semantics
              if key_str.starts_with("@property") {
                return None;
              }
              // Regular property
              let original_prop_name = match &kv.key {
                PropName::Ident(i) => i.sym.to_string(),
                PropName::Str(s) => s.value.to_string(),
                PropName::Num(n) => n.value.to_string(),
                PropName::BigInt(b) => b.value.to_string(),
                PropName::Computed(_) => return None,
              };
              // Resolve value (return early on success)
              let val = if let Some(v) = eval_value_expr(&kv.value, &this.const_env) {
                match v {
                  ValueOrNumber::Str(s) => s,
                  ValueOrNumber::Num(n) => {
                    let mut s = num_to_string(n);
                    if !AtomicCssCollector::is_unitless_property_generic(&original_prop_name) {
                      s.push_str("px");
                    }
                    s
                  }
                }
              } else {
                // Try inline keyframes for animationName
                if original_prop_name == "animationName" {
                  this.maybe_compile_keyframes_to_name(&kv.value)?
                } else {
                  return None;
                }
              };
              let prop_kebab = to_kebab_case(&original_prop_name);
              // Write "prop: value;"
              buf.push_str(&prop_kebab);
              buf.push(':');
              buf.push_str(&val);
              buf.push(';');
            }
          }
          PropOrSpread::Spread(sp) => {
            if let Some(inner) = crate::evaluate_expr::eval_object_expr(&sp.expr, &this.const_env) {
              // Append each entry
              for (k, v) in inner {
                let prop_kebab = to_kebab_case(&k);
                match v {
                  ConstValue::Str(s) => {
                    buf.push_str(&prop_kebab);
                    buf.push(':');
                    buf.push_str(&s);
                    buf.push(';');
                  }
                  ConstValue::Num(n) => {
                    let mut s = num_to_string(n);
                    if !AtomicCssCollector::is_unitless_property_generic(&k) {
                      s.push_str("px");
                    }
                    buf.push_str(&prop_kebab);
                    buf.push(':');
                    buf.push_str(&s);
                    buf.push(';');
                  }
                  _ => return None,
                }
              }
            } else {
              return None;
            }
          }
        }
      }
      Some(())
    }
    let mut out = String::new();
    append_block(self, obj, &mut out)?;
    Some(out)
  }

  pub fn collect_atomic_classes_from_object(&mut self, obj: &ObjectLit) -> Vec<String> {
    let mut class_names: Vec<String> = Vec::new();
    let mut wrappers: Vec<String> = Vec::new();
    self.collect_with(obj, "", &mut wrappers, &mut class_names);
    class_names
  }

  pub fn collect_with(
    &mut self,
    obj: &ObjectLit,
    suffix: &str,
    wrappers: &mut Vec<String>,
    out_classes: &mut Vec<String>,
  ) {
    for p in &obj.props {
      match p {
        PropOrSpread::Prop(pp) => {
          if let Prop::KeyValue(kv) = &**pp {
            if let PropName::Str(s) = &kv.key {
              let key_str = s.value.to_string();
              if key_str.starts_with('&') || key_str.starts_with(':') || key_str.contains('&') {
                let normalized = if key_str.starts_with('&') || key_str.starts_with(':') {
                  normalize_nested_selector(&key_str)
                } else {
                  key_str.clone()
                };
                if let Expr::Object(nested) = &*kv.value {
                  let new_suffix = if let Some(stripped) = normalized.strip_prefix('&') {
                    let mut ns = String::with_capacity(suffix.len() + normalized.len());
                    ns.push_str(suffix);
                    ns.push_str(stripped);
                    ns
                  } else {
                    normalized.clone()
                  };
                  self.collect_with(nested, &new_suffix, wrappers, out_classes);
                  continue;
                }
              }
              if key_str.starts_with('@') {
                if key_str.starts_with("@media") || key_str.starts_with("@supports") {
                  if let Expr::Object(nested) = &*kv.value {
                    let (name, query) = if let Some(stripped) = key_str.strip_prefix("@media") {
                      ("@media", stripped.trim_start())
                    } else {
                      (
                        "@supports",
                        key_str.strip_prefix("@supports").unwrap_or("").trim_start(),
                      )
                    };
                    let normalized_q = normalize_at_query(query);
                    let wrapper = if normalized_q.starts_with('(') {
                      format!("{} {}", name, normalized_q)
                    } else if normalized_q.is_empty() {
                      name.to_string()
                    } else {
                      format!("{} ({})", name, normalized_q)
                    };
                    wrappers.push(wrapper);
                    self.collect_with(nested, suffix, wrappers, out_classes);
                    wrappers.pop();
                  }
                  continue;
                }
                if let Some(stripped) = key_str.strip_prefix("@property") {
                  if let Expr::Object(defn) = &*kv.value {
                    let name = stripped.trim();
                    let normalized_name = name;
                    let mut entries: Vec<(String, String)> = Vec::new();
                    for dp in &defn.props {
                      if let PropOrSpread::Prop(pp2) = dp
                        && let Prop::KeyValue(kv2) = &**pp2
                      {
                        let key_name = match &kv2.key {
                          PropName::Ident(i) => i.sym.to_string(),
                          PropName::Str(s) => s.value.to_string(),
                          PropName::Num(n) => n.value.to_string(),
                          PropName::BigInt(b) => b.value.to_string(),
                          PropName::Computed(_) => continue,
                        };
                        let key_kebab = to_kebab_case(&key_name);
                        let val = match &*kv2.value {
                          Expr::Lit(Lit::Str(s)) => s.value.to_string(),
                          Expr::Lit(Lit::Num(n)) => {
                            let is_int = (n.value - (n.value as i64 as f64)).abs() < f64::EPSILON;
                            if is_int {
                              (n.value as i64).to_string()
                            } else {
                              n.value.to_string()
                            }
                          }
                          Expr::Lit(Lit::Bool(b)) => {
                            if b.value {
                              "true".to_string()
                            } else {
                              "false".to_string()
                            }
                          }
                          _ => continue,
                        };
                        entries.push((key_kebab, val));
                      }
                    }
                    let mut hasher = xxhash_rust::xxh3::Xxh3Builder::new().build();
                    hasher.write(b"@property");
                    hasher.write(normalized_name.as_bytes());
                    for (k, v) in &entries {
                      hasher.write(k.as_bytes());
                      hasher.write(&[0]);
                      hasher.write(v.as_bytes());
                      hasher.write(&[0]);
                    }
                    let rule_key = hasher.finish();
                    let name_owned = normalized_name.to_string();
                    self.ensure_const_for_rule(rule_key, move || {
                      let mut s = String::with_capacity(
                        16 + name_owned.len()
                          + entries
                            .iter()
                            .map(|(k, v)| k.len() + v.len() + 3)
                            .sum::<usize>()
                          + 2,
                      );
                      s.push_str("@property ");
                      s.push_str(&name_owned);
                      s.push('{');
                      let total = entries.len();
                      for (idx, (k, v)) in entries.iter().enumerate() {
                        s.push_str(k);
                        s.push(':');
                        s.push_str(v);
                        if idx + 1 < total {
                          s.push(';');
                        }
                      }
                      s.push('}');
                      s
                    });
                  }
                  continue;
                }
              }
            }

            let original_prop_name = match &kv.key {
              PropName::Ident(i) => i.sym.to_string(),
              PropName::Str(s) => s.value.to_string(),
              PropName::Num(n) => n.value.to_string(),
              PropName::BigInt(b) => b.value.to_string(),
              PropName::Computed(_) => continue,
            };

            // Resolve value from const env, or compile inline keyframes and use its name
            let mut value_eval = eval_value_expr(&kv.value, &self.const_env);
            if value_eval.is_none() {
              // Try inline keyframes only for animation-name property
              let original_name_is_animation = original_prop_name == "animationName";
              if original_name_is_animation
                && let Some(name) = self.maybe_compile_keyframes_to_name(&kv.value)
              {
                value_eval = Some(ValueOrNumber::Str(name));
              }
            }
            if value_eval.is_none() && self.allow_runtime_vars {
              // Skip function-valued properties here; styled handles them specially
              if matches!(&*kv.value, Expr::Arrow(_) | Expr::Fn(_)) {
                continue;
              }
              // Fallback to runtime variable for non-const values
              let var_name = self.create_runtime_var_for_expr(&kv.value, false);
              let value_text_str = format!("var({})", var_name);
              let prop_kebab = to_kebab_case(&original_prop_name);
              self.emit_rule_input(
                RuleInput {
                  prop_kebab: &prop_kebab,
                  value_text: Cow::Borrowed(value_text_str.as_str()),
                  suffix,
                  wrappers,
                },
                out_classes,
              );
              continue;
            }
            let value_eval = match value_eval {
              Some(v) => v,
              None => continue,
            };
            let prop_kebab = to_kebab_case(&original_prop_name);
            if shorthand::is_shorthand_prop(&prop_kebab) {
              if let Some(expanded) = shorthand::try_expand_shorthand(&prop_kebab, &value_eval) {
                for (sub_prop, sub_val) in expanded {
                  self.emit_rule_input(
                    RuleInput {
                      prop_kebab: &sub_prop,
                      value_text: Cow::Borrowed(sub_val.as_str()),
                      suffix,
                      wrappers,
                    },
                    out_classes,
                  );
                }
                // handled via expansion
                // continue to next prop
              } else {
                // For shorthands that should remain unexpanded (e.g., border: none), emit as-is
                let value_text_str = match value_eval {
                  ValueOrNumber::Str(s) => s,
                  ValueOrNumber::Num(n) => {
                    let mut v = if (n - (n as i64 as f64)).abs() < f64::EPSILON {
                      (n as i64).to_string()
                    } else {
                      n.to_string()
                    };
                    if !AtomicCssCollector::is_unitless_property_generic(&original_prop_name) {
                      v.push_str("px");
                    }
                    v
                  }
                };
                self.emit_rule_input(
                  RuleInput {
                    prop_kebab: &prop_kebab,
                    value_text: Cow::Borrowed(value_text_str.as_str()),
                    suffix,
                    wrappers,
                  },
                  out_classes,
                );
              }
            } else {
              let value_text_str = match value_eval {
                ValueOrNumber::Str(s) => s,
                ValueOrNumber::Num(n) => {
                  let mut v = if (n - (n as i64 as f64)).abs() < f64::EPSILON {
                    (n as i64).to_string()
                  } else {
                    n.to_string()
                  };
                  if !AtomicCssCollector::is_unitless_property_generic(&original_prop_name) {
                    v.push_str("px");
                  }
                  v
                }
              };
              self.emit_rule_input(
                RuleInput {
                  prop_kebab: &prop_kebab,
                  value_text: Cow::Borrowed(value_text_str.as_str()),
                  suffix,
                  wrappers,
                },
                out_classes,
              );
            }
          }
        }
        PropOrSpread::Spread(sp) => {
          if let Some(map) = crate::evaluate_expr::eval_object_expr(&sp.expr, &self.const_env) {
            for (k, v) in map {
              let original_prop_name = k;
              let prop_kebab = to_kebab_case(&original_prop_name);
              let value_text = match v {
                ConstValue::Str(s) => s,
                ConstValue::Num(n) => {
                  let mut v = if (n - (n as i64 as f64)).abs() < f64::EPSILON {
                    (n as i64).to_string()
                  } else {
                    n.to_string()
                  };
                  if !AtomicCssCollector::is_unitless_property_generic(&original_prop_name) {
                    v.push_str("px");
                  }
                  v
                }
                _ => continue,
              };
              let value_eval_for_sh = match &value_text {
                s if !s.is_empty() && s.chars().any(|c| c.is_alphabetic()) => {
                  ValueOrNumber::Str(s.clone())
                }
                s => {
                  if let Ok(n) = s.parse::<f64>() {
                    ValueOrNumber::Num(n)
                  } else {
                    ValueOrNumber::Str(s.clone())
                  }
                }
              };
              if shorthand::is_shorthand_prop(&prop_kebab) {
                if let Some(expanded) =
                  shorthand::try_expand_shorthand(&prop_kebab, &value_eval_for_sh)
                {
                  for (sub_prop, sub_val) in expanded {
                    self.emit_rule_input(
                      RuleInput {
                        prop_kebab: &sub_prop,
                        value_text: Cow::Borrowed(sub_val.as_str()),
                        suffix,
                        wrappers,
                      },
                      out_classes,
                    );
                  }
                } else {
                  self.emit_rule_input(
                    RuleInput {
                      prop_kebab: &prop_kebab,
                      value_text: Cow::Borrowed(value_text.as_str()),
                      suffix,
                      wrappers,
                    },
                    out_classes,
                  );
                }
              } else {
                self.emit_rule_input(
                  RuleInput {
                    prop_kebab: &prop_kebab,
                    value_text: Cow::Borrowed(value_text.as_str()),
                    suffix,
                    wrappers,
                  },
                  out_classes,
                );
              }
            }
          } else if let Some(class_str) = self.eval_expr_to_string(&sp.expr) {
            // Support spreading precompiled css() results: merge their class names
            for token in class_str.split_whitespace() {
              if !token.is_empty() {
                out_classes.push(token.to_string());
              }
            }
          }
        }
      }
    }
  }

  pub(crate) fn is_unitless_property(property: &str) -> bool {
    matches!(
      property,
      "animationIterationCount"
        | "basePalette"
        | "borderImageOutset"
        | "borderImageSlice"
        | "borderImageWidth"
        | "boxFlex"
        | "boxFlexGroup"
        | "boxOrdinalGroup"
        | "columnCount"
        | "columns"
        | "flex"
        | "flexGrow"
        | "flexPositive"
        | "flexShrink"
        | "flexNegative"
        | "flexOrder"
        | "fontSizeAdjust"
        | "fontWeight"
        | "gridArea"
        | "gridRow"
        | "gridRowEnd"
        | "gridRowSpan"
        | "gridRowStart"
        | "gridColumn"
        | "gridColumnEnd"
        | "gridColumnSpan"
        | "gridColumnStart"
        | "lineClamp"
        | "lineHeight"
        | "opacity"
        | "order"
        | "orphans"
        | "tabSize"
        | "WebkitLineClamp"
        | "widows"
        | "zIndex"
        | "zoom"
        | "fillOpacity"
        | "floodOpacity"
        | "stopOpacity"
        | "strokeDasharray"
        | "strokeDashoffset"
        | "strokeMiterlimit"
        | "strokeOpacity"
        | "strokeWidth"
    )
  }
}

// Lightweight parser for extracting animation-name values from a css`...` text
pub(crate) fn parse_animation_names_from_css_text(css_text: &str) -> Vec<String> {
  let mut out: Vec<String> = Vec::new();
  let bytes = css_text.as_bytes();
  let n = bytes.len();
  let mut i: usize = 0;
  while i < n {
    // find 'animation-name'
    if i + 14 <= n && &css_text[i..].to_lowercase().as_str()[..14] == "animation-name" {
      // advance to ':'
      while i < n && bytes[i] != b':' {
        i += 1;
      }
      if i < n && bytes[i] == b':' {
        i += 1;
      }
      // skip ws
      while i < n && bytes[i].is_ascii_whitespace() {
        i += 1;
      }
      // read until ';' or '}' collecting tokens separated by ','
      let start = i;
      while i < n && bytes[i] != b';' && bytes[i] != b'}' {
        i += 1;
      }
      let slice = css_text[start..i].trim();
      for part in slice.split(',') {
        let name = part.trim();
        if !name.is_empty() {
          // strip quotes if present
          let name = name.trim_matches('"').trim_matches('\'');
          out.push(name.to_string());
        }
      }
    } else {
      i += 1;
    }
  }
  out
}
