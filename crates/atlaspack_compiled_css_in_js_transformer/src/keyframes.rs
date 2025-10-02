use std::cmp::Ordering;
use std::hash::Hasher;

use swc_ecma_ast::*;

use crate::evaluate_expr::{ValueOrNumber, eval_value_expr};
use crate::utils::to_kebab_case;
use crate::{AtomicCssCollector, base_encode36};

type KeyframesRuntimeEntry = (String, Expr, bool);

impl AtomicCssCollector {
  // Public entry for compiling keyframes from an object literal.
  pub fn compile_keyframes_from_object(&mut self, obj: &ObjectLit) -> String {
    let body = self.build_keyframes_body_from_object(obj);
    self.ensure_keyframes_rule(&body)
  }

  // Public entry for compiling keyframes from a CSS text body (template literal after interpolation).
  pub fn compile_keyframes_from_css_text(&mut self, css_body_text: &str) -> String {
    // Normalize whitespace (collapse all runs, including newlines, to a single space)
    let body = normalize_whitespace(css_body_text);
    self.ensure_keyframes_rule(&body)
  }

  // Ensure a hoisted const is emitted for the @keyframes body, and return the generated animation name.
  pub(crate) fn ensure_keyframes_rule(&mut self, body: &str) -> String {
    // Hash the body for a stable animation name.
    let mut name_hasher = xxhash_rust::xxh3::Xxh3Builder::new().build();
    name_hasher.write(body.as_bytes());
    let name_hash = name_hasher.finish();
    let mut name = base_encode36(name_hash);
    // Ensure starts with a letter for CSS validity if base36 begins with a digit.
    if name.as_bytes().first().is_some_and(|b| b.is_ascii_digit()) {
      let mut prefixed = String::with_capacity(name.len() + 1);
      prefixed.push('k');
      prefixed.push_str(&name);
      name = prefixed;
    }

    // Build final @keyframes content.
    let mut full = String::with_capacity(11 + name.len() + body.len() + 2);
    full.push_str("@keyframes ");
    full.push_str(&name);
    full.push('{');
    full.push_str(body);
    full.push('}');

    // Use ensure_const_for_rule for dedupe/hoist. Rule key includes the body to ensure exact match.
    let mut rule_hasher = xxhash_rust::xxh3::Xxh3Builder::new().build();
    rule_hasher.write(b"@keyframes");
    rule_hasher.write(name.as_bytes());
    rule_hasher.write(&[0]);
    rule_hasher.write(body.as_bytes());
    let rule_key = rule_hasher.finish();
    let full_owned = full.clone();
    let _ident = self.ensure_const_for_rule(rule_key, move || full_owned);

    name
  }

  // Build the inside of the @keyframes block from an object literal.
  fn build_keyframes_body_from_object(&mut self, obj: &ObjectLit) -> String {
    // Collect steps into a vector of (step_label, declarations Ordered Vec<(prop, value)>)
    // We will sort steps for determinism: numeric ascending, then 'to' after numbers.
    #[derive(Default)]
    struct Step {
      props: Vec<(String, String)>,
    }
    let mut steps: Vec<(String, Step)> = Vec::new();

    for p in &obj.props {
      if let PropOrSpread::Prop(pp) = p
        && let Prop::KeyValue(kv) = &**pp
      {
        // Step key can be ident/str/num; treat numbers as percentages when appropriate.
        let step_key = match &kv.key {
          PropName::Ident(i) => i.sym.to_string(),
          PropName::Str(s) => s.value.to_string(),
          PropName::Num(n) => format!("{}%", num_to_int_string(n.value)),
          PropName::BigInt(b) => b.value.to_string(),
          PropName::Computed(_) => continue,
        };
        let step_label = normalize_step_label(&step_key);
        if let Expr::Object(style_obj) = &*kv.value {
          let mut decls: Vec<(String, String)> = Vec::new();
          for sp in &style_obj.props {
            if let PropOrSpread::Prop(pp2) = sp
              && let Prop::KeyValue(kv2) = &**pp2
            {
              let original_prop_name = match &kv2.key {
                PropName::Ident(i) => i.sym.to_string(),
                PropName::Str(s) => s.value.to_string(),
                PropName::Num(n) => n.value.to_string(),
                PropName::BigInt(b) => b.value.to_string(),
                PropName::Computed(_) => continue,
              };
              // Resolve value via const-env where possible for correctness.
              let value_text = match eval_value_expr(&kv2.value, &self.const_env) {
                Some(ValueOrNumber::Str(s)) => s,
                Some(ValueOrNumber::Num(n)) => {
                  let mut v = num_to_string(n);
                  if !Self::is_unitless_property_generic(&original_prop_name) {
                    v.push_str("px");
                  }
                  v
                }
                None => continue,
              };
              let prop_kebab = to_kebab_case(&original_prop_name);
              decls.push((prop_kebab, value_text));
            }
          }
          // Sort declarations by property name for stable output.
          decls.sort_by(|a, b| a.0.cmp(&b.0));
          steps.push((step_label, Step { props: decls }));
        }
      }
    }

    // Sort steps: numeric percent ascending, then 'from' as 0%, 'to' last.
    steps.sort_by(|(a, _), (b, _)| compare_step_labels(a, b));

    // Build body string.
    let mut total_len = 0usize;
    for (_, step) in &steps {
      // label + '{' + props joined + '}'
      total_len += 2; // braces
      // assume ~8 chars per prop rough; compute exact below
      for (k, v) in &step.props {
        total_len += k.len() + 1 + v.len() + 1;
      }
    }
    let mut out = String::with_capacity(total_len + steps.len() * 4);
    for (label, step) in steps {
      out.push_str(&label);
      out.push('{');
      let plen = step.props.len();
      for (idx, (k, v)) in step.props.into_iter().enumerate() {
        out.push_str(&k);
        out.push(':');
        out.push_str(&v);
        if idx + 1 < plen {
          out.push(';');
        }
      }
      out.push('}');
    }
    out
  }
}

impl AtomicCssCollector {
  // Try to compile a keyframes expression used inline and return the generated name.
  pub fn maybe_compile_keyframes_to_name(&mut self, expr: &Expr) -> Option<String> {
    match expr {
      Expr::Call(call) => {
        if let Callee::Expr(callee_expr) = &call.callee
          && let Expr::Ident(i) = &**callee_expr
          && self.keyframes_local.as_ref().is_some_and(|w| i.sym == *w)
          && let Some(arg0) = call.args.first()
        {
          return match &*arg0.expr {
            Expr::Object(o) => Some(self.compile_keyframes_from_object(o)),
            Expr::Tpl(t) => {
              if let Some(s) = self.build_css_text_from_tpl(t) {
                Some(self.compile_keyframes_from_css_text(&s))
              } else if let Some((body, entries)) = self.build_keyframes_body_from_tpl_with_vars(t)
              {
                let name = self.ensure_keyframes_rule(&body);
                if !entries.is_empty() {
                  self.keyframes_name_to_runtime.insert(name.clone(), entries);
                }
                Some(name)
              } else {
                None
              }
            }
            _ => None,
          };
        }
        None
      }
      Expr::TaggedTpl(tt) => {
        if let Expr::Ident(tag_ident) = &*tt.tag
          && self
            .keyframes_local
            .as_ref()
            .is_some_and(|w| tag_ident.sym == *w)
        {
          if let Some(s) = self.build_css_text_from_tpl(&tt.tpl) {
            return Some(self.compile_keyframes_from_css_text(&s));
          }
          if let Some((body, entries)) = self.build_keyframes_body_from_tpl_with_vars(&tt.tpl) {
            let name = self.ensure_keyframes_rule(&body);
            if !entries.is_empty() {
              self.keyframes_name_to_runtime.insert(name.clone(), entries);
            }
            return Some(name);
          }
          return None;
        }
        None
      }
      _ => None,
    }
  }
}

fn normalize_step_label(label: &str) -> String {
  let l = label.trim();
  if l.eq_ignore_ascii_case("from") {
    return "0%".to_string();
  }
  if l.eq_ignore_ascii_case("to") {
    return "to".to_string();
  }
  // Ensure percentage suffix if it's a number without %.
  if let Ok(n) = l.parse::<i64>() {
    return format!("{}%", n);
  }
  if l.ends_with('%') {
    return l.to_string();
  }
  l.to_string()
}

fn compare_step_labels(a: &str, b: &str) -> Ordering {
  // from (0%) first, numeric ascending, then 'to' last; other strings lexicographically.
  if a == b {
    return Ordering::Equal;
  }
  if a == "0%" && b != "0%" {
    return Ordering::Less;
  }
  if b == "0%" && a != "0%" {
    return Ordering::Greater;
  }
  if a == "to" {
    return Ordering::Greater;
  }
  if b == "to" {
    return Ordering::Less;
  }
  let an = parse_percent(a);
  let bn = parse_percent(b);
  match (an, bn) {
    (Some(x), Some(y)) => x.cmp(&y),
    (Some(_), None) => Ordering::Less,
    (None, Some(_)) => Ordering::Greater,
    (None, None) => a.cmp(b),
  }
}

fn parse_percent(s: &str) -> Option<i64> {
  if let Some(stripped) = s.strip_suffix('%')
    && let Ok(n) = stripped.parse::<i64>()
  {
    return Some(n);
  }
  None
}

fn num_to_int_string(n: f64) -> String {
  if (n - (n as i64 as f64)).abs() < f64::EPSILON {
    (n as i64).to_string()
  } else {
    n.to_string()
  }
}

fn num_to_string(n: f64) -> String {
  if (n - (n as i64 as f64)).abs() < f64::EPSILON {
    (n as i64).to_string()
  } else {
    n.to_string()
  }
}

fn normalize_whitespace(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  let mut in_ws = false;
  for ch in s.chars() {
    if ch.is_whitespace() {
      if !in_ws {
        out.push(' ');
        in_ws = true;
      }
    } else {
      in_ws = false;
      out.push(ch);
    }
  }
  out.trim().to_string()
}

impl AtomicCssCollector {
  // Build keyframes body from a template literal, inserting CSS variables for non-const
  // interpolations and recording runtime ix entries to set those variables.
  pub fn build_keyframes_body_from_tpl_with_vars(
    &mut self,
    tpl: &Tpl,
  ) -> Option<(String, Vec<KeyframesRuntimeEntry>)> {
    let approx_cap: usize = tpl
      .quasis
      .iter()
      .map(|q| q.raw.as_ref().len())
      .sum::<usize>()
      + tpl.exprs.len() * 12;
    let mut out = String::with_capacity(approx_cap);
    let mut entries: Vec<KeyframesRuntimeEntry> = Vec::new();
    for (idx, quasi) in tpl.quasis.iter().enumerate() {
      out.push_str(quasi.raw.as_ref());
      if let Some(expr) = tpl.exprs.get(idx) {
        // Try const folding first
        if let Some(s) = self.eval_expr_to_string(expr) {
          out.push_str(&s);
          continue;
        }
        // Non-const: emit CSS var and capture runtime ix entry
        let key = expr_key_for_var(expr, idx);
        let var_name = format!("--{}", self.hash_class_name(&["ix", "kf", &key]));
        out.push_str("var(");
        out.push_str(&var_name);
        out.push(')');
        entries.push((var_name, (*expr.as_ref()).clone(), false));
      }
    }
    Some((normalize_whitespace(&out), entries))
  }
}

// Create a stable key for a runtime expression to derive a CSS variable name.
fn expr_key_for_var(expr: &Expr, idx: usize) -> String {
  fn member_path(e: &Expr, out: &mut Vec<String>) -> bool {
    match e {
      Expr::Member(m) => {
        if !member_path(&m.obj, out) {
          return false;
        }
        match &m.prop {
          MemberProp::Ident(i) => {
            out.push(i.sym.to_string());
            true
          }
          _ => false,
        }
      }
      Expr::Ident(i) => {
        out.push(i.sym.to_string());
        true
      }
      _ => false,
    }
  }
  let mut parts: Vec<String> = Vec::new();
  if member_path(expr, &mut parts) {
    return parts.join(".");
  }
  match expr {
    Expr::Ident(i) => i.sym.to_string(),
    Expr::Call(c) => {
      let callee_key = match &c.callee {
        Callee::Expr(e) => expr_key_for_var(e, idx),
        _ => String::new(),
      };
      let mut args_key_parts: Vec<String> = Vec::new();
      for (ai, a) in c.args.iter().enumerate() {
        let part = match &*a.expr {
          Expr::Lit(Lit::Num(n)) => {
            if (n.value - (n.value as i64 as f64)).abs() < f64::EPSILON {
              (n.value as i64).to_string()
            } else {
              n.value.to_string()
            }
          }
          Expr::Lit(Lit::Str(s)) => s.value.to_string(),
          Expr::Lit(Lit::Bool(b)) => {
            if b.value {
              "true".to_string()
            } else {
              "false".to_string()
            }
          }
          Expr::Lit(Lit::Null(_)) => "null".to_string(),
          _ => expr_key_for_var(&a.expr, ai),
        };
        args_key_parts.push(part);
      }
      let args_key = if args_key_parts.is_empty() {
        "()".to_string()
      } else {
        format!("({})", args_key_parts.join(","))
      };
      if callee_key.is_empty() {
        format!("call#{}{}", idx, args_key)
      } else {
        format!("call:{}{}", callee_key, args_key)
      }
    }
    _ => format!("e#{}", idx),
  }
}
