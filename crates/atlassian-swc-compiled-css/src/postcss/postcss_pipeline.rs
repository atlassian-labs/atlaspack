#[cfg(feature = "postcss_engine")]
use postcss as pc;
#[cfg(feature = "postcss_engine")]
#[allow(unused_imports)]
use postcss::ast::NodeAccess;
use postcss::ast::nodes::{
  Declaration as PcDeclaration, Rule as PcRule, as_at_rule, as_declaration, as_rule,
};

use super::transform::{
  CssTransformError, TransformCssOptions, TransformCssResult, transform_css_via_swc_pipeline,
};
#[cfg(feature = "postcss_engine")]
use crate::postcss::plugins::vendor_autoprefixer::{AutoprefixerData, PrefixedDecl};
#[cfg(feature = "postcss_engine")]
use crate::postcss::utils::value_minifier::minify_value_whitespace;
#[cfg(feature = "postcss_engine")]
use once_cell::sync::Lazy;
#[cfg(feature = "postcss_engine")]
use regex::{Captures, Regex};
use std::sync::{Arc, Mutex};

fn collapse_adjacent_ampersands(selector: &str) -> String {
  let mut out = String::with_capacity(selector.len());
  let mut chars = selector.chars().peekable();

  while let Some(ch) = chars.next() {
    if ch == '&' {
      out.push('&');

      loop {
        let mut consumed_ws = false;
        while let Some(&next) = chars.peek() {
          if next.is_whitespace() {
            consumed_ws = true;
            chars.next();
          } else {
            break;
          }
        }

        match chars.peek() {
          Some('&') => {
            // Collapse any chain of ampersands separated by whitespace.
            chars.next();
            out.push('&');
            continue;
          }
          Some(_) => {
            if consumed_ws {
              // Preserve a single space when whitespace wasn't between two ampersands.
              out.push(' ');
            }
          }
          None => {
            // Do not emit trailing whitespace at end of selector.
          }
        }
        break;
      }

      continue;
    }

    out.push(ch);
  }

  out
}

#[cfg(feature = "postcss_engine")]
fn collapse_repeated_class_descendants(selector: &str) -> String {
  static RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\.([A-Za-z0-9_-]+)\s+\.([A-Za-z0-9_-]+)").expect("class regex"));
  let mut current = selector.to_string();
  loop {
    let next = RE
      .replace_all(&current, |caps: &Captures| {
        let first = caps.get(1).expect("first class capture").as_str();
        let second = caps.get(2).expect("second class capture").as_str();
        if first == second {
          format!(".{first}.{second}")
        } else {
          caps.get(0).expect("full class match").as_str().to_string()
        }
      })
      .to_string();
    if next == current {
      return current;
    }
    current = next;
  }
}

#[cfg(feature = "postcss_engine")]
#[derive(Clone, Default)]
struct AtomicCollector {
  sheets: Arc<Mutex<Vec<CollectedSheet>>>,
  class_names: Arc<Mutex<Vec<String>>>,
}

#[derive(Clone, Default)]
struct CollectedSheet {
  path: Vec<(String, String, usize)>,
  css: String,
}

#[cfg(feature = "postcss_engine")]
impl AtomicCollector {
  fn push_sheet(&self, path: Vec<(String, String, usize)>, css: String) {
    self
      .sheets
      .lock()
      .unwrap()
      .push(CollectedSheet { path, css });
  }

  fn push_class(&self, class: String) {
    self.class_names.lock().unwrap().push(class);
  }

  fn take(self) -> (Vec<CollectedSheet>, Vec<String>) {
    // Do not rely on Arc::try_unwrap since plugin closures may still
    // hold references while the processor struct is alive. Instead,
    // extract contents under the mutex and leave the Arc in place.
    let sheets = {
      let mut guard = self.sheets.lock().unwrap();
      std::mem::take(&mut *guard)
    };
    let classes = {
      let mut guard = self.class_names.lock().unwrap();
      std::mem::take(&mut *guard)
    };
    (sheets, classes)
  }
}

#[cfg(feature = "postcss_engine")]
fn prefixed_decl_entries(
  autoprefixer: Option<&AutoprefixerData>,
  prop: &str,
  normalized_value: &str,
  important: bool,
) -> Vec<(String, String)> {
  let mut entries: Vec<(String, String)> = Vec::new();
  if let Some(engine) = autoprefixer {
    for PrefixedDecl {
      property,
      mut value,
    } in engine.prefixed_decls(prop, normalized_value)
    {
      if important {
        value.push_str("!important");
      }
      entries.push((property, value));
    }
  }
  let mut base = normalized_value.to_string();
  if important {
    base.push_str("!important");
  }
  entries.push((prop.to_string(), base));
  entries
}

#[cfg(feature = "postcss_engine")]
fn selector_variants_with_autoprefixer(
  autoprefixer: Option<&AutoprefixerData>,
  selector: &str,
) -> Vec<String> {
  let mut variants: Vec<String> = Vec::new();
  variants.push(selector.to_string());
  if let Some(engine) = autoprefixer {
    for variant in engine.placeholder_selector_variants(selector) {
      if !variants.iter().any(|existing| existing == &variant) {
        variants.push(variant);
      }
    }
  }
  variants
}

#[cfg(feature = "postcss_engine")]
fn serialize_decl_entries(entries: &[(String, String)]) -> String {
  let mut out = String::new();
  for (idx, (prop, value)) in entries.iter().enumerate() {
    out.push_str(prop);
    out.push(':');
    out.push_str(value);
    if idx + 1 != entries.len() {
      out.push(';');
    }
  }
  out
}

#[cfg(feature = "postcss_engine")]
fn is_empty_value(value: &str) -> bool {
  let trimmed = value.trim();
  trimmed.is_empty() || trimmed == "undefined" || trimmed == "null"
}

#[cfg(feature = "postcss_engine")]
fn discard_empty_in_container(container: &pc::RootLike) {
  // Remove empty-valued declarations within every rule/at-rule, then
  // remove empty rules/at-rules recursively.
  match container {
    pc::RootLike::Root(root) => {
      // Clean declarations in all rules under root.
      root.walk_rules(|rule_ref, _| {
        let rule: PcRule = match as_rule(&rule_ref) {
          Some(r) => r,
          None => return true,
        };
        let children = rule.nodes();
        for child in children {
          if let Some(decl) = as_declaration(&child) {
            let decl: PcDeclaration = decl;
            let value = decl.value();
            if is_empty_value(&value) {
              rule.remove_child(child);
            }
          }
        }
        true
      });

      // Remove rules that no longer have any children.
      let mut remove_rules: Vec<pc::ast::NodeRef> = Vec::new();
      root.walk_rules(|rule_ref, _| {
        if let Some(rule) = as_rule(&rule_ref) {
          if rule.nodes().is_empty() {
            remove_rules.push(rule_ref.clone());
          }
        }
        true
      });
      for r in remove_rules {
        root.remove_child(r);
      }

      // Note: removing empty rules requires parent access; for now we
      // only prune empty-valued declarations across the full tree.
    }
    pc::RootLike::Document(document) => {
      // Clean under document similarly
      document.walk_rules(|rule_ref, _| {
        let rule: PcRule = match as_rule(&rule_ref) {
          Some(r) => r,
          None => return true,
        };
        let children = rule.nodes();
        for child in children {
          if let Some(decl) = as_declaration(&child) {
            let decl: PcDeclaration = decl;
            let value = decl.value();
            if is_empty_value(&value) {
              rule.remove_child(child);
            }
          }
        }
        true
      });

      let mut remove_rules: Vec<pc::ast::NodeRef> = Vec::new();
      document.walk_rules(|rule_ref, _| {
        if let Some(rule) = as_rule(&rule_ref) {
          if rule.nodes().is_empty() {
            remove_rules.push(rule_ref.clone());
          }
        }
        true
      });
      for r in remove_rules {
        document.remove_child(r);
      }

      // Note: at-rule empty removal will be added once public helpers are exposed.
    }
  }
}

#[cfg(feature = "postcss_engine")]
fn discard_empty_rules_plugin() -> pc::BuiltPlugin {
  pc::plugin("discard-empty-rules")
    .once_exit(|root, _result| {
      discard_empty_in_container(root);
      Ok(())
    })
    .build()
}

#[cfg(feature = "postcss_engine")]
fn build_processor(options: &TransformCssOptions, collector: &AtomicCollector) -> pc::Processor {
  let optimize_css = options.optimize_css.unwrap_or(true);
  if std::env::var("COMPILED_CSS_TRACE").is_ok() {
    eprintln!("[postcss-engine] optimize_css={}", optimize_css);
  }
  let flatten_enabled = options.flatten_multiple_selectors.unwrap_or(true);
  let mut plugins: Vec<pc::BuiltPlugin> = Vec::new();
  let autoprefixer_enabled = std::env::var("AUTOPREFIXER")
    .map(|v| v != "off")
    .unwrap_or(true);
  let autoprefixer_data = if autoprefixer_enabled {
    crate::postcss::plugins::vendor_autoprefixer::AutoprefixerData::load()
  } else {
    None
  };

  // Match Babel ordering: run duplicate-declaration removal before wrapping
  // bare declarations into a rule. This ensures last-wins semantics align.
  plugins.push(discard_duplicates_plugin());
  plugins.push(wrap_bare_declarations_plugin(options.clone()));
  plugins.push(discard_empty_rules_plugin());
  plugins.push(parent_orphaned_pseudos_plugin());
  plugins.push(pc::plugin("postcss-nested").build());
  plugins.push(super::plugins::normalize_css_engine::minify_selectors::plugin());
  plugins.push(super::plugins::normalize_css_engine::minify_params::plugin());
  if optimize_css {
    // Plugin order matches cssnano-preset-default for compatibility with Babel/cssnano.
    // Key ordering constraints:
    // - colormin must run before ordered_values (colormin adds spaces after color functions)
    // - ordered_values must run after colormin to clean up those spaces
    // - convert_values must run after ordered_values
    use super::plugins::normalize_css_engine as nce;
    plugins.push(nce::reduce_initial::plugin(
      options.browserslist_config_path.clone(),
    ));
    plugins.push(nce::minify_gradients::plugin());
    plugins.push(nce::colormin::plugin());
    plugins.push(nce::normalize_string::plugin());
    plugins.push(nce::normalize_unicode::plugin());
    plugins.push(nce::normalize_url::plugin());
    plugins.push(nce::normalize_positions::plugin());
    // ordered_values must run AFTER colormin to clean up spaces inserted by colormin
    plugins.push(nce::ordered_values::plugin());
    // convert_values must run AFTER ordered_values
    plugins.push(nce::convert_values::plugin());
    plugins.push(nce::discard_comments_plugin());
    plugins.push(nce::calc::plugin());
    plugins.push(nce::normalize_timing_functions::plugin());
    plugins.push(nce::normalize_current_color_plugin());
  }
  // Match Babel ordering: expand shorthands after normalization.
  plugins.push(super::plugins::expand_shorthands_engine::plugin());
  // Start emitting atomic rules.
  plugins.push(atomicify_rules_plugin(
    options.clone(),
    collector.clone(),
    autoprefixer_data.clone(),
  ));
  if flatten_enabled {
    plugins.push(flatten_multiple_selectors_plugin());
    plugins.push(pc::plugin("discard-duplicates-2").build());
  }
  plugins.push(pc::plugin("increase-specificity").build());
  plugins.push(sort_atomic_style_sheet_plugin());
  plugins.push(normalize_whitespace_plugin());
  // Collect keyframes as sheets to match Babel output
  plugins.push(extract_stylesheets_plugin(
    collector.clone(),
    options.clone(),
    autoprefixer_data.clone(),
  ));
  pc::postcss_with_plugins(plugins)
}

#[cfg(feature = "postcss_engine")]
fn flatten_multiple_selectors_plugin() -> pc::BuiltPlugin {
  fn flatten_rule(rule: &PcRule) {
    let selector = rule.selector();
    let selectors = postcss::list::comma(&selector);
    if selectors.len() <= 1 {
      return;
    }

    let inside_keyframes = is_rule_inside_keyframes(rule);
    let mut iter = selectors
      .into_iter()
      .map(|s| {
        if inside_keyframes {
          normalize_keyframe_selector_text(s)
        } else {
          s
        }
      })
      .filter(|s| !s.is_empty());
    let Some(first) = iter.next() else {
      return;
    };
    rule.set_selector(first);
    for selector in iter {
      let text = selector;
      rule.clone_after_with(move |clone| {
        clone.set_selector(text.clone());
      });
    }
  }

  pc::plugin("flatten-multiple-selectors")
    .once_exit(|root, _| {
      match root {
        pc::RootLike::Root(r) => {
          r.walk_rules(|rule_ref, _| {
            if let Some(rule) = as_rule(&rule_ref) {
              flatten_rule(&rule);
            }
            true
          });
        }
        pc::RootLike::Document(d) => {
          d.walk_rules(|rule_ref, _| {
            if let Some(rule) = as_rule(&rule_ref) {
              flatten_rule(&rule);
            }
            true
          });
        }
      }
      Ok(())
    })
    .build()
}

#[cfg(feature = "postcss_engine")]
fn parent_orphaned_pseudos_plugin() -> pc::BuiltPlugin {
  use postcss::ast::nodes::as_rule;

  fn add_nesting(selector: &str) -> Option<String> {
    if std::env::var("COMPILED_CSS_TRACE").is_ok() {
      eprintln!(
        "[postcss-engine][parent-orphaned] selector_in='{}'",
        selector
      );
    }
    fn insert_nesting(body: &str) -> String {
      // Insert '&' immediately before each pseudo group, keeping pseudos on the same
      // compound (no descendant space). If the selector doesn't start with '&', also
      // insert before the first pseudo.
      let mut out = String::with_capacity(body.len() + 4);
      let mut chars = body.chars().peekable();
      let mut _saw_pseudo = false;
      while let Some(ch) = chars.next() {
        if ch == ':' {
          let mut colons = String::from(":");
          while let Some(':') = chars.peek() {
            colons.push(':');
            chars.next();
          }
          if !body.starts_with('&') {
            out.push('&');
          }
          out.push_str(&colons);
          _saw_pseudo = true;
        } else {
          out.push(ch);
        }
      }
      out
    }

    let parts = postcss::list::comma(selector);
    let mut changed = false;
    let mut updated: Vec<String> = Vec::with_capacity(parts.len());
    for part in parts {
      let trimmed = part.trim_start();
      if std::env::var("COMPILED_CSS_TRACE").is_ok() {
        eprintln!("[postcss-engine][parent-orphaned] raw part='{}'", part);
      }
      if trimmed.starts_with(':') || trimmed.starts_with("&:") {
        let offset = part
          .char_indices()
          .find(|(_, ch)| !ch.is_whitespace())
          .map(|(idx, _)| idx)
          .unwrap_or(0);
        let rebuilt_body = insert_nesting(&part[offset..]);
        // Avoid introducing descendant whitespace; keep pseudos on the same compound.
        let mut rebuilt = String::with_capacity(part.len() + 2);
        rebuilt.push_str(&part[..offset]);
        rebuilt.push_str(&rebuilt_body);
        updated.push(rebuilt);
        changed = true;
        if std::env::var("COMPILED_CSS_TRACE").is_ok() {
          eprintln!(
            "[postcss-engine][parent-orphaned] rebuilt='{}' from part='{}'",
            rebuilt_body, part
          );
        }
      } else {
        updated.push(part);
      }
    }
    if changed {
      Some(updated.join(", "))
    } else {
      None
    }
  }

  pc::plugin("parent-orphaned-pseudos")
    .once_exit(|root, _| {
      match root {
        pc::RootLike::Root(r) => {
          r.walk_rules(|rule_ref, _| {
            if let Some(rule) = as_rule(&rule_ref) {
              if let Some(new_selector) = add_nesting(&rule.selector()) {
                rule.set_selector(new_selector);
              }
            }
            true
          });
        }
        pc::RootLike::Document(d) => {
          d.walk_rules(|rule_ref, _| {
            if let Some(rule) = as_rule(&rule_ref) {
              if let Some(new_selector) = add_nesting(&rule.selector()) {
                rule.set_selector(new_selector);
              }
            }
            true
          });
        }
      }
      Ok(())
    })
    .build()
}

fn is_rule_inside_keyframes(rule: &PcRule) -> bool {
  let mut current = rule.to_node().borrow().parent();
  while let Some(node) = current {
    if let Some(at) = as_at_rule(&node) {
      if at.name().eq_ignore_ascii_case("keyframes") {
        return true;
      }
    }
    current = node.borrow().parent();
  }
  false
}

fn normalize_keyframe_selector_text(selector: String) -> String {
  let trimmed = selector.trim();
  if trimmed.eq_ignore_ascii_case("100%") {
    "to".to_string()
  } else {
    trimmed.to_string()
  }
}

#[cfg(feature = "postcss_engine")]
fn normalize_whitespace_plugin() -> pc::BuiltPlugin {
  use crate::postcss::value_parser as vp;
  use postcss::ast::nodes::{as_at_rule, as_declaration, as_rule};

  fn is_variable_function(name: &str) -> bool {
    matches!(
      name.to_ascii_lowercase().as_str(),
      "var" | "env" | "constant"
    )
  }

  fn reduce_calc_whitespace(node: &mut vp::Node) {
    match node {
      vp::Node::Space { value } => {
        *value = " ".to_string();
      }
      vp::Node::Function {
        value,
        before,
        after,
        nodes,
        ..
      } => {
        if !is_variable_function(value) {
          *before = String::new();
          *after = String::new();
        }
        if value.eq_ignore_ascii_case("calc") {
          for n in nodes.iter_mut() {
            reduce_calc_whitespace(n);
          }
        }
      }
      _ => {}
    }
  }

  fn reduce_whitespace(node: &mut vp::Node) -> bool {
    match node {
      vp::Node::Space { value } => {
        *value = " ".to_string();
      }
      vp::Node::Div { before, after, .. } => {
        *before = String::new();
        *after = String::new();
      }
      vp::Node::Function {
        value,
        before,
        after,
        nodes,
        ..
      } => {
        if !is_variable_function(value) {
          *before = String::new();
          *after = String::new();
        }
        if value.eq_ignore_ascii_case("calc") {
          for n in nodes.iter_mut() {
            reduce_calc_whitespace(n);
          }
          return false; // do not re-walk children
        }
      }
      _ => {}
    }
    true
  }

  pc::plugin("normalize-whitespace")
    .once_exit(|root, _| {
      // Walk the full tree and mirror postcss-normalize-whitespace behaviour
      let container = match root {
        pc::RootLike::Root(r) => r.raw().clone(),
        pc::RootLike::Document(d) => d.to_node(),
      };

      // Helper to strip all whitespace characters from a raws text field if present
      fn strip_raw_before(node: &postcss::ast::NodeRef) {
        let mut borrowed = node.borrow_mut();
        if let Some(before) = borrowed.raws.get_text("before") {
          if before.chars().any(|c| c.is_whitespace()) {
            let mut s = String::with_capacity(before.len());
            for ch in before.chars() {
              if !ch.is_whitespace() {
                s.push(ch);
              }
            }
            borrowed.raws.set_text("before", &s);
          }
        }
      }

      // Compute previous sibling of a node
      fn prev_sibling(node: &postcss::ast::NodeRef) -> Option<postcss::ast::NodeRef> {
        let parent = { node.borrow().parent() }?;
        let borrowed = parent.borrow();
        let mut idx = None;
        for (i, child) in borrowed.nodes.iter().enumerate() {
          if std::ptr::eq(child, node) {
            idx = Some(i);
            break;
          }
        }
        let i = idx?;
        if i == 0 {
          None
        } else {
          Some(borrowed.nodes[i - 1].clone())
        }
      }

      // Depth-first traversal
      fn walk(node: &postcss::ast::NodeRef) {
        // Clone children to avoid borrow conflicts during mutation
        let children = { node.borrow().nodes.clone() };
        for child in children {
          // Common: strip raws.before whitespace for decl/rule/atrule
          strip_raw_before(&child);

          if let Some(decl) = as_declaration(&child) {
            // !important spacing
            if decl.important() {
              child.borrow_mut().raws.set_text("important", "!important");
            }
            // IE9 hack spacing around \9
            let mut v = decl.value();
            if v.contains("\\9") {
              v = v.replace(" \\9", "\\9");
              v = v.replace("\\9 ", "\\9");
            }
            // Reduce spaces inside functions/dividers
            let mut parsed = vp::parse(&v);
            vp::walk(&mut parsed.nodes[..], &mut |n| reduce_whitespace(n), false);
            let mut reduced = vp::stringify(&parsed.nodes);
            // Custom properties empty -> single space
            if decl.prop().starts_with("--") && reduced.is_empty() {
              reduced = " ".to_string();
            }
            decl.set_value(reduced);

            // Remove extra semicolons in raws.before when previous sibling is not a rule
            if let Some(prev) = prev_sibling(&child) {
              if as_rule(&prev).is_none() {
                let mut b = child.borrow_mut();
                if let Some(before) = b.raws.get_text("before") {
                  if before.contains(';') {
                    let no_semis = before.replace(';', "");
                    b.raws.set_text("before", &no_semis);
                  }
                }
              }
            }
            // between ':' and drop own semicolon
            let mut b = child.borrow_mut();
            b.raws.set_text("between", ":");
            b.raws.set_text("ownSemicolon", "");
          } else if let Some(_r) = as_rule(&child) {
            let mut b = child.borrow_mut();
            b.raws.set_text("between", "");
            b.raws.set_text("after", "");
            // Ensure last declaration in this block does not force a semicolon
            b.raws.set_text("semicolon", "false");
          } else if let Some(_a) = as_at_rule(&child) {
            let mut b = child.borrow_mut();
            b.raws.set_text("between", "");
            b.raws.set_text("after", "");
            // Ensure last declaration in this block does not force a semicolon
            b.raws.set_text("semicolon", "false");
          }

          // Recurse
          walk(&child);
        }
      }

      walk(&container);
      // Remove final newline
      match root {
        pc::RootLike::Root(r) => r.raw().borrow_mut().raws.set_text("after", ""),
        pc::RootLike::Document(d) => d.to_node().borrow_mut().raws.set_text("after", ""),
      }
      Ok(())
    })
    .build()
}

#[cfg(feature = "postcss_engine")]
fn vendor_prefixing_lite_plugin() -> pc::BuiltPlugin {
  use postcss::ast::nodes::{as_declaration, as_rule};
  pc::plugin("vendor-prefixing-lite")
    .once_exit(|root, _| {
      match root {
        pc::RootLike::Root(r) => {
          r.walk_rules(|rule_ref, _| {
            if let Some(rule) = as_rule(&rule_ref) {
              let mut to_prepend: Vec<(usize, postcss::ast::NodeRef)> = Vec::new();
              let children = rule.nodes();
              for (idx, child) in children.iter().enumerate() {
                if let Some(decl) = as_declaration(child) {
                  let prop = decl.prop().to_lowercase();
                  let value = decl.value();
                  if matches!(prop.as_str(), "width" | "min-width" | "max-width")
                    && value.eq_ignore_ascii_case("fit-content")
                  {
                    let mut raws = postcss::ast::RawData::default();
                    raws.set_text("between", ":");
                    let new_decl = postcss::ast::nodes::declaration_with_raws(
                      prop.clone(),
                      "-moz-fit-content".to_string(),
                      decl.important(),
                      raws,
                    );
                    to_prepend.push((idx, new_decl));
                  }
                }
              }
              // Insert in order before originals
              let mut inserted = 0usize;
              for (idx, new_decl) in to_prepend.into_iter() {
                postcss::ast::Node::insert(&rule.to_node(), idx + inserted, new_decl);
                inserted += 1;
              }
            }
            true
          });
        }
        pc::RootLike::Document(d) => {
          d.walk_rules(|rule_ref, _| {
            if let Some(rule) = as_rule(&rule_ref) {
              let mut to_prepend: Vec<(usize, postcss::ast::NodeRef)> = Vec::new();
              let children = rule.nodes();
              for (idx, child) in children.iter().enumerate() {
                if let Some(decl) = as_declaration(child) {
                  let prop = decl.prop().to_lowercase();
                  let value = decl.value();
                  if matches!(prop.as_str(), "width" | "min-width" | "max-width")
                    && value.eq_ignore_ascii_case("fit-content")
                  {
                    let mut raws = postcss::ast::RawData::default();
                    raws.set_text("between", ":");
                    let new_decl = postcss::ast::nodes::declaration_with_raws(
                      prop.clone(),
                      "-moz-fit-content".to_string(),
                      decl.important(),
                      raws,
                    );
                    to_prepend.push((idx, new_decl));
                  }
                }
              }
              let mut inserted = 0usize;
              for (idx, new_decl) in to_prepend.into_iter() {
                postcss::ast::Node::insert(&rule.to_node(), idx + inserted, new_decl);
                inserted += 1;
              }
            }
            true
          });
        }
      }
      Ok(())
    })
    .build()
}

#[cfg(feature = "postcss_engine")]
fn discard_duplicates_plugin() -> pc::BuiltPlugin {
  use postcss::ast::nodes::as_declaration;
  use std::collections::HashMap;

  // Mirrors packages/css/src/plugins/discard-duplicates.ts: remove earlier
  // duplicate declarations that exist directly under the root.
  pc::plugin("discard-duplicates")
    .once(|root_like, _result| {
      fn process_root(root: &postcss::ast::nodes::Root) {
        let mut by_prop: HashMap<String, Vec<postcss::ast::NodeRef>> = HashMap::new();
        for child in root.nodes() {
          if let Some(decl) = as_declaration(&child) {
            let prop = decl.prop();
            by_prop
              .entry(prop.to_string())
              .or_default()
              .push(child.clone());
          }
        }
        for (_prop, nodes) in by_prop.into_iter() {
          if nodes.len() > 1 {
            for i in 0..nodes.len() - 1 {
              root.remove_child(nodes[i].clone());
            }
          }
        }
      }

      match root_like {
        pc::RootLike::Root(root) => process_root(root),
        pc::RootLike::Document(document) => {
          // Documents can also contain top-level declarations; handle similarly.
          let mut by_prop: HashMap<String, Vec<postcss::ast::NodeRef>> = HashMap::new();
          for child in document.nodes() {
            if let Some(decl) = as_declaration(&child) {
              let prop = decl.prop();
              by_prop
                .entry(prop.to_string())
                .or_default()
                .push(child.clone());
            }
          }
          for (_prop, nodes) in by_prop.into_iter() {
            if nodes.len() > 1 {
              for i in 0..nodes.len() - 1 {
                document.remove_child(nodes[i].clone());
              }
            }
          }
        }
      }

      Ok(())
    })
    .build()
}

// minify_selectors_plugin and minify_params_plugin now live under plugins::normalize_css

#[cfg(feature = "postcss_engine")]
fn sort_atomic_style_sheet_plugin() -> pc::BuiltPlugin {
  use crate::postcss::plugins::at_rules::parse_media_query::parse_media_query;
  use crate::postcss::plugins::at_rules::types::ParsedAtRule;
  use crate::postcss::utils::style_ordering::STYLE_ORDER;
  use postcss::ast::nodes::{AtRule as PcAtRule, Rule as PcRule, as_at_rule, as_rule};

  #[derive(Clone)]
  struct AtInfo {
    name: String,
    query: String,
    parsed: Vec<ParsedAtRule>,
    node: postcss::ast::NodeRef,
  }

  fn pseudo_score_for_selector(selector: &str) -> usize {
    let first = selector.split(',').next().unwrap_or("").trim();
    for (idx, pseudo) in STYLE_ORDER.iter().enumerate() {
      if first.ends_with(pseudo) {
        return idx + 1;
      }
    }
    0
  }

  fn pseudo_score_for_rule(rule: &PcRule) -> usize {
    pseudo_score_for_selector(&rule.selector())
  }

  fn sort_pseudo_selectors_in_at_rule(at: &PcAtRule) {
    // Extract nested rules under this at-rule, sort them by pseudo score, re-append.
    // Build new children by draining existing and collecting nested rules separately.
    let mut extracted_rules: Vec<postcss::ast::NodeRef> = Vec::new();
    let mut new_children: Vec<postcss::ast::NodeRef> = Vec::new();
    let existing = at.nodes();
    for child in existing {
      if let Some(nested_at) = as_at_rule(&child) {
        sort_pseudo_selectors_in_at_rule(&nested_at);
        new_children.push(child);
      } else if let Some(_rule) = as_rule(&child) {
        extracted_rules.push(child.clone());
      } else {
        new_children.push(child);
      }
    }

    // Sort extracted rules by pseudo score
    extracted_rules.sort_by(|a, b| {
      let ra = as_rule(a).unwrap();
      let rb = as_rule(b).unwrap();
      pseudo_score_for_rule(&ra).cmp(&pseudo_score_for_rule(&rb))
    });

    // Append back extracted rules at the end
    for r in extracted_rules {
      new_children.push(r);
    }

    // Replace children
    let node_ref = at.to_node();
    {
      let mut borrowed = node_ref.borrow_mut();
      borrowed.nodes = new_children;
    }
  }

  fn at_rules_cmp(a: &AtInfo, b: &AtInfo) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    // 1. by at-rule name
    let name_cmp = a.name.cmp(&b.name);
    if name_cmp != Ordering::Equal {
      return name_cmp;
    }

    // 2. by parsed components
    let limit = a.parsed.len().min(b.parsed.len());
    for i in 0..limit {
      let pa = &a.parsed[i];
      let pb = &b.parsed[i];
      let key_cmp = pa.sort_key().cmp(&pb.sort_key());
      if key_cmp != Ordering::Equal {
        return key_cmp;
      }
      if (pa.length - pb.length).abs() > f64::EPSILON {
        // If includes '>' then ascending by length, else descending by length
        if pa.comparison_operator.includes_greater() {
          if let Some(ord) = pa.length.partial_cmp(&pb.length) {
            return ord;
          }
        } else {
          if let Some(ord) = pb.length.partial_cmp(&pa.length) {
            return ord;
          }
        }
      }
    }

    // 3. shorter parsed length first if any parsed content exists
    if (a.parsed.len() + b.parsed.len() > 0) && a.parsed.len() != b.parsed.len() {
      return a.parsed.len().cmp(&b.parsed.len());
    }

    // 4. fallback to query string compare
    a.query.cmp(&b.query)
  }

  pc::plugin("sort-atomic-style-sheet")
    .once(|root_like, _| {
      // Collect top-level nodes into buckets
      let container = match root_like {
        pc::RootLike::Root(r) => r.raw().clone(),
        pc::RootLike::Document(d) => d.to_node(),
      };
      let nodes = container.borrow().nodes.clone();
      let mut catch_all: Vec<postcss::ast::NodeRef> = Vec::new();
      let mut rules: Vec<postcss::ast::NodeRef> = Vec::new();
      let mut at_rules: Vec<AtInfo> = Vec::new();

      for node in nodes {
        if let Some(rule) = as_rule(&node) {
          rules.push(rule.to_node());
        } else if let Some(at) = as_at_rule(&node) {
          let name = at.name();
          let query = at.params();
          let parsed = if name == "media" {
            parse_media_query(&query)
          } else {
            Vec::new()
          };
          at_rules.push(AtInfo {
            name,
            query,
            parsed,
            node: at.to_node(),
          });
        } else {
          catch_all.push(node);
        }
      }

      // Sort pseudo selectors for top-level rules
      rules.sort_by(|a, b| {
        let ra = as_rule(a).unwrap();
        let rb = as_rule(b).unwrap();
        pseudo_score_for_rule(&ra).cmp(&pseudo_score_for_rule(&rb))
      });

      // Sort at-rules per comparator
      at_rules.sort_by(|a, b| at_rules_cmp(a, b));

      // Recursively sort pseudo selectors inside each at-rule
      for info in &at_rules {
        if let Some(at) = as_at_rule(&info.node) {
          sort_pseudo_selectors_in_at_rule(&at);
        }
      }

      // Rebuild root nodes: catchAll -> rules -> atRules
      let mut combined: Vec<postcss::ast::NodeRef> = Vec::new();
      combined.extend(catch_all);
      combined.extend(rules);
      combined.extend(at_rules.into_iter().map(|i| i.node));

      let mut borrowed = container.borrow_mut();
      borrowed.nodes = combined;
      Ok(())
    })
    .build()
}

#[cfg(feature = "postcss_engine")]
fn extract_stylesheets_plugin(
  collector: AtomicCollector,
  _options: TransformCssOptions,
  _autoprefixer: Option<Arc<AutoprefixerData>>,
) -> pc::BuiltPlugin {
  use postcss::ast::nodes::as_at_rule;
  use postcss::list::comma;

  fn can_atomicify_at_rule(name: &str) -> bool {
    matches!(
      name,
      "container"
        | "-moz-document"
        | "else"
        | "layer"
        | "media"
        | "starting-style"
        | "supports"
        | "when"
    )
  }

  fn normalized_selector(selector: &str) -> String {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
      return "&".to_string();
    }

    let collapsed = collapse_adjacent_ampersands(trimmed);
    if collapsed.contains('&') {
      collapsed
    } else {
      format!("& {}", collapsed)
    }
  }

  fn combine_selectors(parent: &[String], child: &str) -> Vec<String> {
    let child_parts = comma(child);
    let parents = if parent.is_empty() {
      vec!["&".to_string()]
    } else {
      parent.to_vec()
    };
    let mut out = Vec::new();
    for p in parents {
      for c in &child_parts {
        let trimmed = c.trim();
        if std::env::var("COMPILED_CSS_TRACE").is_ok() {
          eprintln!("[engine.combine] parent='{}' child='{}'", p, trimmed);
        }
        if trimmed == "*" && p.trim() == "*" {
          out.push("*".to_string());
          continue;
        }
        if trimmed.contains('&') {
          let replaced = trimmed.replace('&', &p);
          out.push(replaced);
        } else if p == "&" {
          out.push(trimmed.to_string());
        } else if trimmed.is_empty() {
          out.push(p.clone());
        } else {
          out.push(format!("{} {}", p, trimmed));
        }
      }
    }
    out
  }

  fn wrap_in_at_rules(rule_css: &str, at_chain: &[(String, String, usize)]) -> String {
    if at_chain.is_empty() {
      return rule_css.to_string();
    }
    let mut out = String::new();
    for (n, p, _) in at_chain {
      if p.is_empty() {
        out.push_str(&format!("@{}{{", n));
      } else {
        out.push_str(&format!("@{} {}{{", n, p));
      }
    }
    out.push_str(rule_css);
    for _ in at_chain {
      out.push('}');
    }
    out
  }

  #[allow(dead_code)]
  fn minify_color_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
      return value.to_string();
    }
    let opts = super::plugins::normalize_css_engine::colormin::add_plugin_defaults();
    let min = super::plugins::normalize_css_engine::colormin::transform_value(trimmed, &opts);
    if min.len() < trimmed.len() {
      min
    } else {
      trimmed.to_string()
    }
  }

  #[allow(dead_code)]
  fn walk_and_emit(
    node: &postcss::ast::NodeRef,
    selectors: &[String],
    at_chain: &[(String, String, usize)],
    collector: &AtomicCollector,
    opts: &TransformCssOptions,
    autoprefixer: Option<&AutoprefixerData>,
  ) {
    let borrowed = node.borrow();
    let children = borrowed.nodes.clone();
    drop(borrowed);
    for (idx, child) in children.into_iter().enumerate() {
      if let Some(rule) = as_rule(&child) {
        let raw_selector = rule.selector();
        let sels = if let Some(ph) = &opts.declaration_placeholder {
          if raw_selector == *ph {
            selectors.to_vec()
          } else {
            combine_selectors(selectors, &raw_selector)
          }
        } else {
          combine_selectors(selectors, &raw_selector)
        };
        // Emit for each declaration
        for gc in rule.nodes() {
          if let Some(decl) = as_declaration(&gc) {
            let prop = decl.prop();
            let raw_value = decl.value();
            // COMPAT: Hash seed must use the value BEFORE whitespace normalization
            // to match Babel's behavior. In Babel's pipeline, postcss-normalize-whitespace
            // runs AFTER atomicify, so atomicify hashes the value with original spacing
            // (e.g., "var(--ds-space-300, 24px)" with space, not "var(--ds-space-300,24px)").
            // Only apply colormin transformation since that runs before atomicify in Babel.
            let mut hash_seed = minify_color_value(&raw_value);
            if decl.important() {
              hash_seed.push_str("true");
            }
            // For CSS output, apply full normalization
            let mut normalized_value = minify_color_value(&raw_value);
            normalized_value = minify_value_whitespace(&normalized_value);
            let mut base_value = normalized_value.clone();
            if decl.important() {
              base_value.push_str("!important");
            }
            let mut replaced_selectors: Vec<String> = Vec::new();
            for sel in &sels {
              let norm = normalized_selector(sel);
              let mut group_seed = String::new();
              if let Some(prefix) = &opts.class_hash_prefix {
                group_seed.push_str(prefix);
              }
              for (n, p, _) in at_chain {
                group_seed.push_str(n);
                group_seed.push_str(p);
              }
              group_seed.push_str(&norm);
              group_seed.push_str(&prop);
              let group = crate::utils_hash::hash(&group_seed)
                .chars()
                .take(4)
                .collect::<String>();
              let value_hash = crate::utils_hash::hash(&hash_seed)
                .chars()
                .take(4)
                .collect::<String>();
              let full_class = format!("_{}{}", group, value_hash);
              let used_class = if let Some(map) = &opts.class_name_compression_map {
                let key = full_class.trim_start_matches('_');
                if let Some(compressed) = map.get(key) {
                  compressed.clone()
                } else {
                  full_class.clone()
                }
              } else {
                full_class.clone()
              };
              let replaced = norm.replace('&', &format!(".{}", used_class));
              replaced_selectors.push(replaced);
            }
            let mut selector_joined = replaced_selectors.join(", ");
            if let Some(ph) = &opts.declaration_placeholder {
              if !ph.is_empty() {
                let needle = format!(" {}", ph);
                selector_joined = selector_joined.replace(&needle, "");
                selector_joined = selector_joined.replace(ph, "");
                selector_joined = selector_joined.trim().to_string();
              }
            }
            let mut decls_to_emit: Vec<(String, String)> = Vec::new();
            if let Some(engine) = autoprefixer {
              for pref in engine.prefixed_decls(&prop, &normalized_value) {
                let mut v = pref.value.clone();
                if decl.important() {
                  v.push_str("!important");
                }
                decls_to_emit.push((pref.property.clone(), v));
              }
            }
            decls_to_emit.push((prop.clone(), base_value.clone()));

            let mut selector_variants: Vec<String> = Vec::new();
            if let Some(engine) = autoprefixer {
              selector_variants = engine.placeholder_selector_variants(&selector_joined);
            }
            for variant in selector_variants {
              for (emit_prop, emit_value) in &decls_to_emit {
                let css = format!("{}{{{}:{}}}", variant, emit_prop, emit_value);
                collector.push_sheet(at_chain.to_vec(), wrap_in_at_rules(&css, at_chain));
              }
            }
            for (emit_prop, emit_value) in &decls_to_emit {
              let css = format!("{}{{{}:{}}}", selector_joined, emit_prop, emit_value);
              collector.push_sheet(at_chain.to_vec(), wrap_in_at_rules(&css, at_chain));
            }
          } else if let Some(_nested_rule) = as_rule(&gc) {
            // Recurse into nested rules (should be flattened earlier, but handle just in case)
            walk_and_emit(&gc, &sels, at_chain, collector, opts, autoprefixer);
          } else if let Some(nested_at) = as_at_rule(&gc) {
            // Recurse into nested at-rules under this rule, preserving selectors.
            // IMPORTANT: Do not descend into @keyframes — JS atomicify never emits
            // atomic rules for keyframe steps (e.g. `0%`, `to`).
            let name = nested_at.name();
            if name.eq_ignore_ascii_case("keyframes") {
              // Skip atomic emission inside keyframes entirely
              continue;
            }
            // Preserve ignored at-rules (e.g., @property) as full blocks – do not emit atomic rules
            if matches!(
              name.to_ascii_lowercase().as_str(),
              "color-profile"
                | "counter-style"
                | "font-face"
                | "font-palette-values"
                | "page"
                | "property"
            ) {
              continue;
            }
            let mut next_chain = at_chain.to_vec();
            let params = nested_at.params();
            if can_atomicify_at_rule(&name) {
              next_chain.push((name, params, idx));
            }
            walk_and_emit(
              &nested_at.to_node(),
              &sels,
              &next_chain,
              collector,
              opts,
              autoprefixer,
            );
          }
        }
      } else if let Some(at) = as_at_rule(&child) {
        let name = at.name();
        // Do not descend into @keyframes; JS atomicify does not emit
        // atomic class rules inside keyframe steps.
        if name.eq_ignore_ascii_case("keyframes") {
          continue;
        }
        // Preserve ignored at-rules (e.g., @property) as full blocks – do not emit atomic rules
        if matches!(
          name.to_ascii_lowercase().as_str(),
          "color-profile"
            | "counter-style"
            | "font-face"
            | "font-palette-values"
            | "page"
            | "property"
        ) {
          continue;
        }
        let params = at.params();
        let mut next = at_chain.to_vec();
        if can_atomicify_at_rule(&name) {
          next.push((name, params, idx));
        }
        walk_and_emit(
          &at.to_node(),
          selectors,
          &next,
          collector,
          opts,
          autoprefixer,
        );
      } else {
        walk_and_emit(&child, selectors, at_chain, collector, opts, autoprefixer);
      }
    }
  }

  pc::plugin("extract-stylesheets")
    .once_exit(move |root, _| {
      // Only collect @keyframes as standalone sheets to mirror Babel’s extractor.
      // Do not clear or re-emit atomic classes here — those are already pushed
      // by the atomicify_rules plugin earlier in the pipeline.
      match root {
        pc::RootLike::Root(r) => {
          r.walk_at_rules_if(
            |name| {
              let n = name.to_ascii_lowercase();
              matches!(
                n.as_str(),
                "keyframes"
                  | "color-profile"
                  | "counter-style"
                  | "font-face"
                  | "font-palette-values"
                  | "page"
                  | "property"
              )
            },
            |node_ref, _| {
              if let Some(at) = as_at_rule(&node_ref) {
                let tmp = postcss::ast::nodes::Root::new();
                tmp.append(at.to_node());
                if let Ok(mut res) = tmp.to_result() {
                  let css = res.css().to_string();
                  collector.push_sheet(Vec::new(), css);
                }
              }
              true
            },
          );
        }
        pc::RootLike::Document(d) => {
          d.walk_at_rules_if(
            |name| {
              let n = name.to_ascii_lowercase();
              matches!(
                n.as_str(),
                "keyframes"
                  | "color-profile"
                  | "counter-style"
                  | "font-face"
                  | "font-palette-values"
                  | "page"
                  | "property"
              )
            },
            |node_ref, _| {
              if let Some(at) = as_at_rule(&node_ref) {
                let tmp = postcss::ast::nodes::Root::new();
                tmp.append(at.to_node());
                if let Ok(mut res) = tmp.to_result() {
                  let css = res.css().to_string();
                  collector.push_sheet(Vec::new(), css);
                }
              }
              true
            },
          );
        }
      }
      Ok(())
    })
    .build()
}

/// Normalize a value for hashing purposes.
/// This applies the same transformations that run BEFORE atomicify in Babel:
/// - reduce-initial: converts values like `currentColor` to `initial` when supported
/// - colormin: minifies color values
///
/// IMPORTANT: This does NOT apply whitespace normalization, because in Babel
/// postcss-normalize-whitespace runs AFTER atomicify.
#[cfg(feature = "postcss_engine")]
fn normalize_value_for_hash(
  prop: &str,
  value: &str,
  initial_support: bool,
  optimize_css: bool,
) -> String {
  if !optimize_css {
    return value.to_string();
  }
  // First apply reduce-initial transformation
  let after_reduce_initial =
    super::plugins::normalize_css_engine::reduce_initial::transform_value_for_hash(
      prop,
      value,
      initial_support,
    );
  if std::env::var("COMPILED_CSS_TRACE").is_ok()
    && matches!(
      prop,
      "background" | "background-color" | "text-decoration-color"
    )
  {
    eprintln!(
      "[atomicify.hash] prop='{}' raw='{}' reduce_initial='{}'",
      prop, value, after_reduce_initial
    );
  }

  // Then apply colormin transformation
  let trimmed = after_reduce_initial.trim();
  if trimmed.is_empty() {
    return after_reduce_initial;
  }
  let colormin_opts = super::plugins::normalize_css_engine::colormin::add_plugin_defaults();
  let after_colormin =
    super::plugins::normalize_css_engine::colormin::transform_value(trimmed, &colormin_opts);
  if std::env::var("COMPILED_CSS_TRACE").is_ok()
    && matches!(
      prop,
      "background" | "background-color" | "text-decoration-color"
    )
    && after_colormin != value
  {
    eprintln!(
      "[atomicify.hash] prop='{}' raw='{}' normalized='{}'",
      prop, value, after_colormin
    );
  }
  if after_colormin.len() < trimmed.len() {
    after_colormin
  } else {
    trimmed.to_string()
  }
}

#[cfg(feature = "postcss_engine")]
fn atomicify_rules_plugin(
  options: TransformCssOptions,
  collector: AtomicCollector,
  autoprefixer: Option<Arc<AutoprefixerData>>,
) -> pc::BuiltPlugin {
  use crate::utils_hash::hash;
  use postcss::list::comma;

  // Compute initial_support once based on browserslist config
  // This determines whether we convert values like `currentColor` to `initial`
  let optimize_css = options.optimize_css.unwrap_or(true);
  let initial_support = super::plugins::normalize_css_engine::reduce_initial::is_initial_supported(
    options.browserslist_config_path.as_deref(),
  );

  #[derive(Clone)]
  struct Ctx<'a> {
    at_chain: Vec<(String, String, usize)>, // (name, params, occurrence index)
    selectors: Vec<String>,                 // combined selectors at this depth
    opts: &'a TransformCssOptions,
    collector: AtomicCollector,
    autoprefixer: Option<Arc<AutoprefixerData>>,
  }

  fn can_atomicify_at_rule(name: &str) -> bool {
    matches!(
      name,
      "container"
        | "-moz-document"
        | "else"
        | "layer"
        | "media"
        | "starting-style"
        | "supports"
        | "when"
    )
  }

  fn combine_selectors(parent: &[String], child: &str) -> Vec<String> {
    let child_parts = comma(child);
    let parents = if parent.is_empty() {
      vec!["&".to_string()]
    } else {
      parent.to_vec()
    };
    let mut out = Vec::new();
    for p in parents {
      for c in &child_parts {
        let trimmed = c.trim();
        if std::env::var("COMPILED_CSS_TRACE").is_ok() {
          eprintln!("[engine.combine] parent='{}' child='{}'", p, trimmed);
        }
        if trimmed == "*" && p.trim() == "*" {
          out.push("*".to_string());
          continue;
        }
        if trimmed.contains('&') {
          let replaced = trimmed.replace('&', &p);
          out.push(replaced);
        } else if p == "&" {
          out.push(trimmed.to_string());
        } else if trimmed.is_empty() {
          out.push(p.clone());
        } else {
          out.push(format!("{} {}", p, trimmed));
        }
      }
    }
    if std::env::var("COMPILED_CSS_TRACE").is_ok() {
      eprintln!("[engine.combine] result={:?}", out);
    }
    out
  }

  fn normalized_selector(selector: &str) -> String {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
      return "&".to_string();
    }

    let collapsed = collapse_adjacent_ampersands(trimmed);
    if collapsed.contains('&') {
      collapsed
    } else {
      format!("& {}", collapsed)
    }
  }

  fn at_chain_label(at_chain: &[(String, String, usize)]) -> String {
    let mut s = String::new();
    for (n, p, _) in at_chain {
      s.push_str(n);
      s.push_str(p);
    }
    s
  }

  fn wrap_in_at_rules(rule_css: &str, at_chain: &[(String, String, usize)]) -> String {
    if at_chain.is_empty() {
      return rule_css.to_string();
    }
    let mut out = String::new();
    for (n, p, _) in at_chain {
      if p.is_empty() {
        out.push_str(&format!("@{}{{", n));
      } else {
        out.push_str(&format!("@{} {}{{", n, p));
      }
    }
    out.push_str(rule_css);
    for _ in at_chain {
      out.push('}');
    }
    out
  }

  fn clean_placeholder_selector(selector: String, placeholder: Option<&str>) -> String {
    if let Some(ph) = placeholder {
      if !ph.is_empty() {
        let needle = format!(" {}", ph);
        let mut cleaned = selector.replace(&needle, "");
        cleaned = cleaned.replace(ph, "");
        return cleaned.trim().to_string();
      }
    }
    selector
  }

  fn is_inside_keyframes(node: &pc::ast::NodeRef) -> bool {
    // Walk up parents; if any ancestor is an at-rule named 'keyframes', return true
    let mut cur = Some(node.clone());
    while let Some(n) = cur {
      if let Some(at) = as_at_rule(&n) {
        if at.name().eq_ignore_ascii_case("keyframes") {
          return true;
        }
      }
      let borrowed = n.borrow();
      cur = borrowed.parent();
    }
    false
  }

  #[allow(dead_code)]
  fn process_rule(rule: &PcRule, ctx: &mut Ctx, initial_support: bool, optimize_css: bool) {
    if std::env::var("COMPILED_CSS_TRACE").is_ok() {
      eprintln!(
        "[engine.process_rule] selectors={:?} rule.selector()='{}'",
        ctx.selectors,
        rule.selector()
      );
    }
    // Do not emit atomic rules when nested under ignored at-rules like @property.
    if ctx.at_chain.iter().any(|(n, _, _)| {
      matches!(
        n.to_ascii_lowercase().as_str(),
        "color-profile"
          | "counter-style"
          | "font-face"
          | "font-palette-values"
          | "page"
          | "property"
      )
    }) {
      return;
    }
    // Emit atomic rules for each declaration in this rule
    let children = rule.nodes();
    for child in children {
      if let Some(decl) = as_declaration(&child) {
        let prop = decl.prop();
        let orig_value = decl.value();
        let has_important = decl.important();
        // COMPAT: Hash seed uses the value AFTER reduce-initial and colormin transformations
        // but BEFORE whitespace normalization, matching Babel's plugin order.
        let mut hash_seed =
          normalize_value_for_hash(&prop, &orig_value, initial_support, optimize_css);
        if has_important {
          hash_seed.push_str("true");
        }
        let mut value_full =
          normalize_value_for_hash(&prop, &orig_value, initial_support, optimize_css);
        if has_important {
          value_full.push_str("!important");
        }
        value_full = minify_value_whitespace(&value_full);
        if std::env::var("COMPILED_CSS_TRACE").is_ok() && prop == "box-shadow" {
          eprintln!(
            "[atomicify.hash] prop='{}' raw='{}' hash_seed='{}' value='{}'",
            prop, orig_value, hash_seed, value_full
          );
        }

        let mut normalized_list: Vec<String> = ctx
          .selectors
          .iter()
          .map(|s| normalized_selector(s))
          .collect();
        normalized_list.sort();
        let at_label = at_chain_label(&ctx.at_chain);
        let at_seg = if at_label.is_empty() {
          "undefined".to_string()
        } else {
          at_label.clone()
        };

        // Inject vendor-prefixed values into the same atomic rule to match Babel's
        // ordering (prefixed first, then unprefixed) when applicable.
        let mut decls = String::new();
        // Known case: fit-content needs -moz-fit-content for width-like properties
        let lower_val_plain = orig_value.to_ascii_lowercase();
        if matches!(prop.as_str(), "width" | "min-width" | "max-width")
          && lower_val_plain.trim() == "fit-content"
        {
          decls.push_str(&prop);
          decls.push(':');
          decls.push_str("-moz-fit-content");
          if has_important {
            decls.push_str("!important");
          }
          decls.push(';');
        }
        // Future: additional value-level vendor insertions can be added here to match Autoprefixer
        decls.push_str(&prop);
        decls.push(':');
        decls.push_str(&value_full);

        for norm in normalized_list {
          let mut group_seed = String::new();
          if let Some(prefix) = &ctx.opts.class_hash_prefix {
            group_seed.push_str(prefix);
          }
          group_seed.push_str(&at_seg);
          group_seed.push_str(&norm);
          group_seed.push_str(&prop);
          let group = hash(&group_seed).chars().take(4).collect::<String>();
          if std::env::var("COMPILED_CLI_TRACE").is_ok() {
            eprintln!(
              "[atomicify.group] at='{}' sel='{}' prop='{}' seed='{}' -> {}",
              at_label, norm, prop, group_seed, group
            );
          }
          let value_hash = hash(&hash_seed).chars().take(4).collect::<String>();
          let class = format!("_{}{}", group, value_hash);
          ctx.collector.push_class(class.clone());

          let replaced = norm.replace('&', &format!(".{}", class));
          let selector_text =
            clean_placeholder_selector(replaced, ctx.opts.declaration_placeholder.as_deref());
          let rule_css = format!("{}{{{}}}", selector_text, decls);
          let wrapped = wrap_in_at_rules(&rule_css, &ctx.at_chain);
          if std::env::var("COMPILED_CLI_TRACE").is_ok() {
            eprintln!("[engine.atomic] sheet='{}'", wrapped);
          }
          ctx.collector.push_sheet(ctx.at_chain.clone(), wrapped);
        }
      } else if let Some(nested) = as_rule(&child) {
        // Recurse nested rules
        let sels = combine_selectors(&ctx.selectors, &nested.selector());
        let mut next = ctx.clone();
        next.selectors = sels;
        process_rule(&nested, &mut next, initial_support, optimize_css);
      }
    }
  }

  let at_stack = Arc::new(Mutex::new(Vec::<(String, String, usize)>::new()));
  let at_depth_counts = Arc::new(Mutex::new(Vec::<usize>::new()));
  let sheet_count_stack = Arc::new(Mutex::new(Vec::<usize>::new()));
  let sel_stack = Arc::new(Mutex::new(vec![vec!["&".to_string()]]));

  postcss::plugin("atomicify-rules")
    // Handle declarations that appear directly under Root or AtRule trees.
    .decl({
      let sel_stack = sel_stack.clone();
      let at_stack = at_stack.clone();
      let collector = collector.clone();
      let opts = options.clone();
      let autoprefixer = autoprefixer.clone();
      move |decl, _| {
        // Skip if this declaration lives under a normal Rule; the rule_exit
        // hook will handle those to avoid double emission.
        let parent = decl.to_node().borrow().parent();
        if let Some(p) = parent {
          if as_rule(&p).is_some() {
            return Ok(());
          }
          // Skip declarations that live under ignored at-rules (@property, etc.).
          // This matches Babel which does not atomicify inside these blocks.
          let mut cur = Some(p);
          while let Some(node) = cur {
            if let Some(at) = as_at_rule(&node) {
              let name = at.name().to_ascii_lowercase();
              if matches!(
                name.as_str(),
                "color-profile"
                  | "counter-style"
                  | "font-face"
                  | "font-palette-values"
                  | "page"
                  | "property"
              ) {
                return Ok(());
              }
            }
            cur = node.borrow().parent();
          }
        }

        let selectors = {
          let stack = sel_stack.lock().unwrap();
          stack
            .last()
            .cloned()
            .unwrap_or_else(|| vec!["&".to_string()])
        };
        let at_chain = at_stack.lock().unwrap().clone();
        let at_label = at_chain_label(&at_chain);

        let prop = decl.prop();
        let raw_value = decl.value();
        let has_important = decl.important();
        // COMPAT: Hash seed uses the value AFTER reduce-initial and colormin transformations
        // but BEFORE whitespace normalization, matching Babel's plugin order.
        let mut hash_seed =
          normalize_value_for_hash(&prop, &raw_value, initial_support, optimize_css);
        if has_important {
          hash_seed.push_str("true");
        }
        // For CSS output, apply full normalization (including whitespace)
        let mut normalized_value =
          normalize_value_for_hash(&prop, &raw_value, initial_support, optimize_css);
        normalized_value = minify_value_whitespace(&normalized_value);
        if std::env::var("COMPILED_CSS_TRACE").is_ok() && prop == "box-shadow" {
          eprintln!(
            "[atomicify.hash] prop='{}' raw='{}' hash_seed='{}' value='{}'",
            prop, raw_value, hash_seed, normalized_value
          );
        }
        let autoprefixer_ref = autoprefixer.as_ref().map(|arc| arc.as_ref());
        let prefixed_entries =
          prefixed_decl_entries(autoprefixer_ref, &prop, &normalized_value, has_important);
        let decls = serialize_decl_entries(&prefixed_entries);

        let mut normalized_list: indexmap::IndexSet<String> =
          selectors.iter().map(|s| normalized_selector(s)).collect();
        for norm in normalized_list.drain(..) {
          let mut group_seed = String::new();
          if let Some(prefix) = &opts.class_hash_prefix {
            group_seed.push_str(prefix);
          }
          let at_seg = if at_label.is_empty() {
            "undefined"
          } else {
            &at_label
          };
          group_seed.push_str(at_seg);
          group_seed.push_str(&norm);
          group_seed.push_str(&prop);
          let group = hash(&group_seed).chars().take(4).collect::<String>();
          if std::env::var("COMPILED_CLI_TRACE").is_ok() {
            eprintln!(
              "[atomicify.group] at='{}' sel='{}' prop='{}' seed='{}' -> {}",
              at_label, norm, prop, group_seed, group
            );
            if prop == "margin-left" {
              eprintln!("[atomicify.hash.postcss] value='{}'", hash_seed);
            }
          }
          let value_hash = hash(&hash_seed).chars().take(4).collect::<String>();
          let full_class = format!("_{}{}", group, value_hash);
          collector.push_class(full_class.clone());
          let used_class = if let Some(map) = &opts.class_name_compression_map {
            let key = full_class.trim_start_matches('_');
            if let Some(compressed) = map.get(key) {
              compressed.clone()
            } else {
              full_class.clone()
            }
          } else {
            full_class.clone()
          };
          let replaced = norm.replace('&', &format!(".{}", used_class));
          let selector_text =
            clean_placeholder_selector(replaced, opts.declaration_placeholder.as_deref());
          let selector_variants =
            selector_variants_with_autoprefixer(autoprefixer_ref, &selector_text);
          for variant in selector_variants {
            let rule_css = format!("{}{{{}}}", variant, decls);
            let wrapped = wrap_in_at_rules(&rule_css, &at_chain);
            collector.push_sheet(at_chain.clone(), wrapped);
          }
        }
        Ok(())
      }
    })
    .at_rule_filter("*", {
      let at_stack = at_stack.clone();
      let at_depth_counts = at_depth_counts.clone();
      let sheet_count_stack = sheet_count_stack.clone();
      let collector = collector.clone();
      move |at, _| {
        if can_atomicify_at_rule(&at.name()) {
          let mut counts = at_depth_counts.lock().unwrap();
          let depth = at_stack.lock().unwrap().len();
          if counts.len() <= depth {
            counts.resize(depth + 1, 0);
          }
          counts[depth] += 1;
          let idx = counts[depth];
          at_stack.lock().unwrap().push((at.name(), at.params(), idx));
          let current_len = collector.sheets.lock().unwrap().len();
          sheet_count_stack.lock().unwrap().push(current_len);
        }
        Ok(())
      }
    })
    .at_rule_filter_exit("*", {
      let at_stack = at_stack.clone();
      let at_depth_counts = at_depth_counts.clone();
      let sheet_count_stack = sheet_count_stack.clone();
      let collector = collector.clone();
      move |at, _| {
        if can_atomicify_at_rule(&at.name()) {
          if let Some(start_len) = sheet_count_stack.lock().unwrap().pop() {
            let end_len = collector.sheets.lock().unwrap().len();
            if end_len == start_len {
              let at_chain = at_stack.lock().unwrap().clone();
              let empty = wrap_in_at_rules("", &at_chain);
              collector.push_sheet(at_chain, empty);
            }
          }
          {
            let mut counts = at_depth_counts.lock().unwrap();
            let depth = at_stack.lock().unwrap().len();
            if counts.len() > depth {
              counts.truncate(depth);
            }
          }
          let _ = at_stack.lock().unwrap().pop();
        }
        Ok(())
      }
    })
    .rule_filter("*", {
      let sel_stack = sel_stack.clone();
      let opts = options.clone();
      move |rule, _| {
        // Skip rules that live under @keyframes — atomicify does not process keyframe steps
        if is_inside_keyframes(&rule.to_node()) {
          return Ok(());
        }
        let mut stack = sel_stack.lock().unwrap();
        let parent = stack
          .last()
          .cloned()
          .unwrap_or_else(|| vec!["&".to_string()]);
        let mut raw_selector = rule.selector();
        if std::env::var("COMPILED_CSS_TRACE").is_ok() {
          eprintln!(
            "[engine.rule_filter] parent={:?} raw_selector='{}'",
            parent, raw_selector
          );
        }
        if let Some(ph) = &opts.declaration_placeholder {
          if raw_selector.contains(ph) {
            let cleaned = raw_selector.replace(ph, "").trim().to_string();
            raw_selector = cleaned;
          }
        }
        let combined = if raw_selector.is_empty() {
          parent
        } else {
          combine_selectors(&parent, &raw_selector)
        };
        stack.push(combined);
        Ok(())
      }
    })
    .rule_filter_exit("*", {
      let sel_stack = sel_stack.clone();
      // Need the same guard as in rule_filter; if we didn't push on enter, don't pop here.
      let at_stack = at_stack.clone();
      let collector = collector.clone();
      let opts = options.clone();
      let autoprefixer = autoprefixer.clone();
      move |rule, _| {
        if is_inside_keyframes(&rule.to_node()) {
          // We skipped pushing a new selectors frame in rule_filter; keep stack intact.
          return Ok(());
        }
        let selectors = {
          let stack = sel_stack.lock().unwrap();
          stack
            .last()
            .cloned()
            .unwrap_or_else(|| vec!["&".to_string()])
        };

        let at_chain = at_stack.lock().unwrap().clone();
        let at_label = at_chain_label(&at_chain);
        let autoprefixer_ref = autoprefixer.as_ref().map(|arc| arc.as_ref());

        for child in rule.nodes() {
          if let Some(decl) = as_declaration(&child) {
            let prop = decl.prop();
            let raw_value = decl.value();
            let has_important = decl.important();
            // COMPAT: Hash seed uses the value AFTER reduce-initial and colormin transformations
            // but BEFORE whitespace normalization, matching Babel's plugin order.
            let mut hash_seed =
              normalize_value_for_hash(&prop, &raw_value, initial_support, optimize_css);
            if has_important {
              hash_seed.push_str("true");
            }
            // For CSS output, apply full normalization (including whitespace)
            let mut value_full =
              normalize_value_for_hash(&prop, &raw_value, initial_support, optimize_css);
            value_full = minify_value_whitespace(&value_full);
            if std::env::var("COMPILED_CSS_TRACE").is_ok() && prop == "box-shadow" {
              eprintln!(
                "[atomicify.rule_exit] prop='{}' raw='{}' hash_seed='{}'",
                prop, raw_value, hash_seed
              );
            }
            let prefixed_entries =
              prefixed_decl_entries(autoprefixer_ref, &prop, &value_full, has_important);
            let decls = serialize_decl_entries(&prefixed_entries);

            let normalized_list: Vec<String> =
              selectors.iter().map(|s| normalized_selector(s)).collect();
            for norm in normalized_list {
              let mut group_seed = String::new();
              if let Some(prefix) = &opts.class_hash_prefix {
                group_seed.push_str(prefix);
              }
              let at_seg = if at_label.is_empty() {
                "undefined"
              } else {
                &at_label
              };
              group_seed.push_str(at_seg);
              group_seed.push_str(&norm);
              group_seed.push_str(&prop);
              let group = hash(&group_seed).chars().take(4).collect::<String>();
              if std::env::var("COMPILED_CLI_TRACE").is_ok() {
                eprintln!(
                  "[atomicify.group] at='{}' sel='{}' prop='{}' seed='{}' -> {}",
                  at_label, norm, prop, group_seed, group
                );
              }
              let value_hash = hash(&hash_seed).chars().take(4).collect::<String>();
              let full_class = format!("_{}{}", group, value_hash);
              collector.push_class(full_class.clone());
              // Replace using compressed class if map provided.
              let used_class = if let Some(map) = &opts.class_name_compression_map {
                let key = full_class.trim_start_matches('_');
                if let Some(compressed) = map.get(key) {
                  compressed.clone()
                } else {
                  full_class.clone()
                }
              } else {
                full_class.clone()
              };
              let replaced = norm.replace('&', &format!(".{}", used_class));
              let selector_text =
                clean_placeholder_selector(replaced, opts.declaration_placeholder.as_deref());
              let selector_variants =
                selector_variants_with_autoprefixer(autoprefixer_ref, &selector_text);
              for variant in selector_variants {
                let rule_css = format!("{}{{{}}}", variant, decls);
                let wrapped = wrap_in_at_rules(&rule_css, &at_chain);
                collector.push_sheet(at_chain.clone(), wrapped);
              }
            }
          } else if let Some(nested) = as_rule(&child) {
            // Nested rule like &:hover — combine selectors and emit
            let nested_raw = nested.selector();
            let nested_selectors = combine_selectors(&selectors, &nested_raw);
            let normalized_list: Vec<String> = nested_selectors
              .iter()
              .map(|s| normalized_selector(s))
              .collect();

            for gc in nested.nodes() {
              if let Some(nested_decl) = as_declaration(&gc) {
                let prop = nested_decl.prop();
                let raw_value = nested_decl.value();
                let has_important = nested_decl.important();
                // COMPAT: Hash seed uses the value AFTER reduce-initial and colormin transformations
                // but BEFORE whitespace normalization, matching Babel's plugin order.
                let mut hash_seed =
                  normalize_value_for_hash(&prop, &raw_value, initial_support, optimize_css);
                if has_important {
                  hash_seed.push_str("true");
                }
                // For CSS output, apply full normalization (including whitespace)
                let mut normalized_value =
                  normalize_value_for_hash(&prop, &raw_value, initial_support, optimize_css);
                normalized_value = minify_value_whitespace(&normalized_value);
                let prefixed_entries =
                  prefixed_decl_entries(autoprefixer_ref, &prop, &normalized_value, has_important);
                let decls = serialize_decl_entries(&prefixed_entries);

                for norm in &normalized_list {
                  let mut group_seed = String::new();
                  if let Some(prefix) = &opts.class_hash_prefix {
                    group_seed.push_str(prefix);
                  }
                  let at_seg = if at_label.is_empty() {
                    "undefined"
                  } else {
                    &at_label
                  };
                  group_seed.push_str(at_seg);
                  group_seed.push_str(norm);
                  group_seed.push_str(&prop);
                  let group = hash(&group_seed).chars().take(4).collect::<String>();
                  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
                    eprintln!(
                      "[atomicify.group] at='{}' sel='{}' prop='{}' seed='{}' -> {}",
                      at_label, norm, prop, group_seed, group
                    );
                  }
                  let value_hash = hash(&hash_seed).chars().take(4).collect::<String>();
                  let full_class = format!("_{}{}", group, value_hash);
                  collector.push_class(full_class.clone());
                  let used_class = if let Some(map) = &opts.class_name_compression_map {
                    let key = full_class.trim_start_matches('_');
                    if let Some(compressed) = map.get(key) {
                      compressed.clone()
                    } else {
                      full_class.clone()
                    }
                  } else {
                    full_class.clone()
                  };
                  let replaced = norm.replace('&', &format!(".{}", used_class));
                  let selector_text =
                    clean_placeholder_selector(replaced, opts.declaration_placeholder.as_deref());
                  let selector_variants =
                    selector_variants_with_autoprefixer(autoprefixer_ref, &selector_text);
                  for variant in selector_variants {
                    let css = format!("{}{{{}}}", variant, decls);
                    collector.push_sheet(at_chain.clone(), wrap_in_at_rules(&css, &at_chain));
                  }
                }
              }
            }
          }
        }

        let mut stack = sel_stack.lock().unwrap();
        let _ = stack.pop();
        Ok(())
      }
    })
    .build()
}

/// Experimental PostCSS-engine-backed pipeline.
/// Currently parses and serializes CSS via the vendored PostCSS crate
/// without plugins, returning a single sheet and no class names.
/// This is a staging point to wire the original plugin chain identically.
pub fn transform_css_via_postcss(
  css: &str,
  mut options: TransformCssOptions,
) -> Result<TransformCssResult, CssTransformError> {
  if std::env::var("STACK_DEBUG").is_ok() {
    eprintln!("[postcss] transform start");
  }
  if std::env::var("COMPILED_DEBUG_COLORMIN").is_ok() {
    eprintln!("[postcss-pipeline] input css: {}", css.replace('\n', "\\n"));
  }
  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    eprintln!("[postcss] via-postcss begin");
  }
  // Do not string-wrap bare declarations here; the engine plugin 'wrap-bare-decls'
  // promotes them into an empty-selector rule before other plugins run.
  let input_css: String = css.to_string();
  // Shared collector for atomic outputs.
  let collector = AtomicCollector::default();
  // Create a processor with the staged plugin chain.
  let mut processor = build_processor(&options, &collector);

  // First attempt to process the CSS directly.
  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    eprintln!("[postcss] process initial");
  }
  use postcss::processor::ProcessOptions;
  let process_opts = ProcessOptions {
    ignore_errors: true,
    ..ProcessOptions::default()
  };

  let mut result = match processor.process_with_options(&input_css, process_opts.clone()) {
    Ok(res) => res,
    Err(err) => {
      // Mirror Babel/JS fallback: wrap declarations in a placeholder rule and retry.
      const PLACEHOLDER: &str = "__compiled_declaration_wrapper__";
      let wrapped = format!(".{PLACEHOLDER} {{{}}}", css);
      options.declaration_placeholder = Some(format!(".{PLACEHOLDER}"));
      // Rebuild the processor to pass updated options through to plugins.
      let collector = AtomicCollector::default();
      processor = build_processor(&options, &collector);
      // Retry with wrapped input; if this fails, surface the original error.
      if std::env::var("COMPILED_CLI_TRACE").is_ok() {
        eprintln!("[postcss] process wrapped");
      }
      match processor.process_with_options(&wrapped, process_opts.clone()) {
        Ok(res) => res,
        Err(_) => {
          return Err(CssTransformError::from_message(format!(
            "postcss error: {err}"
          )));
        }
      }
    }
  };
  if std::env::var("STACK_DEBUG").is_ok() {
    eprintln!("[postcss] processed");
  }
  // Force evaluation so plugin visitors run (PostCSS is lazy),
  // but avoid full stringification for performance.
  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    eprintln!("[postcss] ensure visitors run");
  }
  let _ = result.result();
  if std::env::var("STACK_DEBUG").is_ok() {
    eprintln!("[postcss] result forced");
  }

  // Collect atomic outputs from the plugin.
  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    eprintln!("[postcss] take collector");
  }
  let (collected_sheets, mut class_names) = collector.take();
  if std::env::var("STACK_DEBUG").is_ok() {
    eprintln!(
      "[postcss] collector len sheets={} classes={}",
      collected_sheets.len(),
      class_names.len()
    );
  }
  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    for sheet in &collected_sheets {
      eprintln!("[postcss] sheet {}", sheet.css);
    }
  }
  // eprintln!("[postcss-pipeline] after first pass, sheets={}", sheets.len());
  // If PostCSS parsed the input as declarations (no rules) successfully,
  // the pipeline will emit no sheets. Instead of wrapping in a placeholder
  // rule, fall back to the SWC pipeline to mirror Babel output without
  // introducing placeholder selectors.
  if collected_sheets.is_empty() && options.declaration_placeholder.is_none() {
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!("[postcss] empty sheets; falling back to swc pipeline");
    }
    return transform_css_via_swc_pipeline(css, options);
  }
  // Reorder sheets to match Babel's sort-atomic-style-sheet order
  fn first_selector_text(sheet: &str) -> String {
    // e.g. ".class:hover{...}" or multiple selectors before '{'
    if sheet.find('.').is_some() {
      let before_brace = &sheet[..sheet.find('{').unwrap_or(sheet.len())];
      let comma_split = before_brace.split(',').next().unwrap_or(before_brace);
      return comma_split.trim().to_string();
    }
    String::new()
  }
  fn pseudo_score(selector: &str) -> usize {
    let s = selector.trim();
    for (idx, pseudo) in crate::postcss::utils::style_ordering::STYLE_ORDER
      .iter()
      .enumerate()
    {
      if s.ends_with(pseudo) {
        return idx + 1;
      }
    }
    0
  }
  fn first_property(sheet: &str) -> Option<String> {
    if let Some(open) = sheet.find('{') {
      let after = &sheet[open + 1..];
      if let Some(colon) = after.find(':') {
        return Some(after[..colon].trim().to_string());
      }
    }
    None
  }
  #[derive(Clone)]
  struct SheetInfo {
    idx: usize,
    text: String,
    path: Vec<(String, String, usize)>,
  }
  #[derive(Clone)]
  enum SheetKind {
    CatchAll {
      score: usize,
    },
    AtRule {
      name: String,
      query: String,
      parsed: Vec<crate::postcss::plugins::at_rules::types::ParsedAtRule>,
    },
  }
  fn classify(sheet: &str) -> SheetKind {
    if sheet.starts_with('@') {
      // Parse name and query up to first '{'
      let after_at = &sheet[1..];
      let mut parts = after_at.splitn(2, ' ');
      let name = parts.next().unwrap_or("").to_string();
      let rest = parts.next().unwrap_or("");
      let query = rest.split('{').next().unwrap_or("").trim().to_string();
      let parsed = if name == "media" {
        crate::postcss::plugins::at_rules::parse_media_query::parse_media_query(&query)
      } else {
        Vec::new()
      };
      SheetKind::AtRule {
        name,
        query,
        parsed,
      }
    } else {
      SheetKind::CatchAll {
        score: pseudo_score(&first_selector_text(sheet)),
      }
    }
  }
  fn cmp_at(a: &SheetKind, b: &SheetKind) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
      (SheetKind::CatchAll { .. }, SheetKind::AtRule { .. }) => Ordering::Less,
      (SheetKind::AtRule { .. }, SheetKind::CatchAll { .. }) => Ordering::Greater,
      (SheetKind::CatchAll { score: sa }, SheetKind::CatchAll { score: sb }) => sa.cmp(sb),
      (
        SheetKind::AtRule {
          name: na,
          query: qa,
          parsed: pa,
        },
        SheetKind::AtRule {
          name: nb,
          query: qb,
          parsed: pb,
        },
      ) => {
        // Same comparator as sort_at_rules
        let name_cmp = na.cmp(nb);
        if name_cmp != Ordering::Equal {
          return name_cmp;
        }
        let limit = pa.len().min(pb.len());
        for i in 0..limit {
          let a = &pa[i];
          let b = &pb[i];
          let key_cmp = a.sort_key().cmp(&b.sort_key());
          if key_cmp != Ordering::Equal {
            return key_cmp;
          }
          if (a.length - b.length).abs() > f64::EPSILON {
            if a.comparison_operator.includes_greater() {
              if let Some(ord) = a.length.partial_cmp(&b.length) {
                return ord;
              }
            } else {
              if let Some(ord) = b.length.partial_cmp(&a.length) {
                return ord;
              }
            }
          }
        }
        if (pa.len() + pb.len() > 0) && pa.len() != pb.len() {
          return pa.len().cmp(&pb.len());
        }
        qa.cmp(qb)
      }
    }
  }
  let mut paired: Vec<(SheetKind, SheetInfo)> = collected_sheets
    .iter()
    .cloned()
    .enumerate()
    .map(|(i, s)| {
      (
        classify(&s.css),
        SheetInfo {
          idx: i,
          text: s.css,
          path: s.path,
        },
      )
    })
    .collect();
  paired.sort_by(|(ka, ia), (kb, ib)| {
    use std::cmp::Ordering;
    let mut ord = match (ka, kb) {
      (SheetKind::CatchAll { score: sa }, SheetKind::CatchAll { score: sb }) => {
        let mut o = sa.cmp(sb);
        if o == Ordering::Equal {
          // Fallback to shorthand bucket ordering for identical pseudo score
          let ba = first_property(&ia.text).and_then(|p| {
            use crate::postcss::plugins::sort_shorthand_declarations::{
              parent_shorthand_for, shorthand_bucket,
            };
            shorthand_bucket(&p).or_else(|| parent_shorthand_for(&p).and_then(shorthand_bucket))
          });
          let bb = first_property(&ib.text).and_then(|p| {
            use crate::postcss::plugins::sort_shorthand_declarations::{
              parent_shorthand_for, shorthand_bucket,
            };
            shorthand_bucket(&p).or_else(|| parent_shorthand_for(&p).and_then(shorthand_bucket))
          });
          o = match (ba, bb) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
          };
        }
        o
      }
      _ => cmp_at(ka, kb),
    };
    if ord == Ordering::Equal {
      ord = ia.idx.cmp(&ib.idx);
    }
    ord
  });
  fn path_key(path: &[(String, String, usize)]) -> String {
    path
      .iter()
      .map(|(n, p, idx)| format!("{}|{}|{}", n, p, idx))
      .collect::<Vec<_>>()
      .join(">")
  }
  use std::collections::HashMap;
  use std::collections::HashSet;
  let mut group_map: HashMap<String, (String, Vec<String>)> = HashMap::new();
  let mut group_order: Vec<String> = Vec::new();
  for (kind, info) in &paired {
    if matches!(kind, SheetKind::AtRule { .. }) {
      let brace_pos = info.text.find('{').unwrap_or(info.text.len());
      let header = info.text[..brace_pos].to_string();
      let inner =
        info.text[brace_pos + 1..info.text.rfind('}').unwrap_or(info.text.len())].to_string();
      let key = path_key(&info.path);
      group_map
        .entry(key.clone())
        .or_insert_with(|| (header.clone(), Vec::new()))
        .1
        .push(inner);
      if !group_order.contains(&key) {
        group_order.push(key);
      }
    }
  }
  let mut produced: HashSet<String> = HashSet::new();
  let mut sheets: Vec<String> = Vec::new();
  for (kind, info) in paired {
    match kind {
      SheetKind::CatchAll { .. } => sheets.push(info.text),
      SheetKind::AtRule { .. } => {
        let key = path_key(&info.path);
        if produced.insert(key.clone()) {
          if let Some((header, parts)) = group_map.get(&key) {
            let mut joined = String::new();
            for part in parts {
              joined.push_str(part);
            }
            sheets.push(format!("{}{{{}}}", header, joined));
          } else {
            sheets.push(info.text);
          }
        }
      }
    }
  }

  // eprintln!("[postcss-pipeline] final sheets={}", sheets.len());
  // Deduplicate classes preserving order.
  let mut seen = std::collections::HashSet::new();
  class_names.retain(|c| seen.insert(c.clone()));
  // Order classes by first appearance in sheets to match runtime expectations.
  fn extract_first_class_from_sheet(sheet: &str) -> Option<String> {
    if let Some(dot) = sheet.find('.') {
      let rest = &sheet[dot + 1..];
      let end = rest
        .find(|c: char| c == '{' || c == ' ' || c == ',')
        .unwrap_or(rest.len());
      let name = &rest[..end];
      if !name.is_empty() {
        return Some(name.to_string());
      }
    }
    None
  }
  let mut order: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
  for (i, sheet) in sheets.iter().enumerate() {
    if let Some(class_name) = extract_first_class_from_sheet(sheet) {
      order.entry(class_name).or_insert(i);
    }
  }
  class_names.sort_by_key(|name| order.get(name).copied().unwrap_or(usize::MAX));

  if sheets.is_empty() {
    // Final fallback: run the SWC-backed pipeline to mirror Babel output exactly.
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!("[postcss] fallback to swc");
    }
    return transform_css_via_swc_pipeline(css, options);
  }

  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    eprintln!("[postcss] via-postcss end");
  }
  if std::env::var("STACK_DEBUG").is_ok() {
    eprintln!(
      "[postcss] end sheets={} classes={}",
      sheets.len(),
      class_names.len()
    );
  }
  Ok(TransformCssResult {
    sheets,
    class_names,
  })
}

fn expand_shorthands_plugin() -> pc::BuiltPlugin {
  // Deprecated shim; real expansion is handled by expand_shorthands_engine::plugin()
  pc::plugin("expand-shorthands-disabled").build()
}
fn wrap_bare_declarations_plugin(options: TransformCssOptions) -> pc::BuiltPlugin {
  use postcss::ast::nodes::{Rule as PcRule, as_declaration};

  let _placeholder = options.declaration_placeholder.unwrap_or_default();

  pc::plugin("wrap-bare-decls")
    .once(move |root, _| {
      match root {
        pc::RootLike::Root(r) => {
          let mut decls: Vec<postcss::ast::NodeRef> = Vec::new();
          r.each(|node_ref, _| {
            if as_declaration(&node_ref).is_some() {
              decls.push(node_ref.clone());
            }
            true
          });
          if !decls.is_empty() {
            // Use an empty selector so no placeholder leaks into output
            let wrapper = PcRule::new("");
            for d in decls {
              // Ensure ':' between prop and value
              {
                let mut b = d.borrow_mut();
                b.raws.set_text("between", ":");
              }
              // Move decl into wrapper
              r.remove_child(d.clone());
              wrapper.append(d);
            }
            // Insert wrapper at the beginning
            r.prepend(wrapper.to_node());
          }
        }
        pc::RootLike::Document(d) => {
          let mut decls: Vec<postcss::ast::NodeRef> = Vec::new();
          d.each(|node_ref, _| {
            if as_declaration(&node_ref).is_some() {
              decls.push(node_ref.clone());
            }
            true
          });
          if !decls.is_empty() {
            // Use an empty selector so no placeholder leaks into output
            let wrapper = PcRule::new("");
            for decl in decls {
              {
                let mut b = decl.borrow_mut();
                b.raws.set_text("between", ":");
              }
              // Remove using low-level Node::remove by index
              if let Some(idx) = decl.borrow().parent().and_then(|p| {
                let b = p.borrow();
                b.nodes.iter().position(|n| std::ptr::eq(n, &decl))
              }) {
                postcss::ast::Node::remove(&decl.borrow().parent().unwrap(), idx);
              }
              wrapper.append(decl);
            }
            d.prepend(wrapper.to_node());
          }
        }
      }
      Ok(())
    })
    .build()
}

#[cfg(all(test, feature = "postcss_engine"))]
mod tests {
  use super::collapse_repeated_class_descendants;
  use crate::postcss::transform::{TransformCssOptions, transform_css};
  use crate::utils_hash::hash;
  use pretty_assertions::assert_eq;

  #[test]
  fn collapses_repeated_class_descendants() {
    assert_eq!(collapse_repeated_class_descendants(".foo .foo"), ".foo.foo");
  }

  #[test]
  fn keeps_distinct_class_descendants() {
    assert_eq!(
      collapse_repeated_class_descendants(".foo .bar"),
      ".foo .bar"
    );
  }

  #[test]
  fn collapses_multiple_repetitions() {
    assert_eq!(
      collapse_repeated_class_descendants(".foo .foo .foo"),
      ".foo.foo.foo"
    );
  }

  #[test]
  fn handles_hyphenated_class_names() {
    assert_eq!(
      collapse_repeated_class_descendants(".foo-bar .foo-bar"),
      ".foo-bar.foo-bar"
    );
  }

  #[test]
  fn hashes_box_shadow_with_minified_whitespace() {
    let css = ".foo { box-shadow: 0px 0px 1px 0px rgba(30, 31, 33, 0.31), 0px 8px 12px 0px rgba(30, 31, 33, 0.15); }";
    let mut options = TransformCssOptions::default();
    options.optimize_css = Some(true);
    let result = transform_css(css, options).expect("transform should succeed");

    assert_eq!(result.class_names.len(), 1);
    let group_seed = "undefined& .foobox-shadow";
    let value_seed = "0 0 1px 0 #1e1f214f,0 8px 9pt 0 #1e1f2126";
    let expected = format!(
      "_{}{}",
      hash(group_seed).chars().take(4).collect::<String>(),
      hash(value_seed).chars().take(4).collect::<String>()
    );

    assert_eq!(result.class_names[0], expected);
  }

  /// Regression test: box-shadow with rgba colors must produce exact class name _16qs5pg2
  /// to match Babel's @compiled/babel-plugin output.
  #[test]
  fn box_shadow_rgba_produces_exact_classname_16qs5pg2() {
    // This is the exact CSS pattern from csm-widget-ui-components/widget-container
    let css = "& { box-shadow: 0px 0px 1px 0px rgba(30, 31, 33, 0.31), 0px 8px 12px 0px rgba(30, 31, 33, 0.15); }";
    let mut options = TransformCssOptions::default();
    options.optimize_css = Some(true);
    let result = transform_css(css, options).expect("transform should succeed");

    assert_eq!(result.class_names.len(), 1);
    // The exact class name must be _16qs5pg2 to match Babel output
    assert_eq!(result.class_names[0], "_16qs5pg2");

    // Verify the CSS output contains the normalized value
    assert!(result.sheets[0].contains("box-shadow:0 0 1px 0 #1e1f214f,0 8px 9pt 0 #1e1f2126"));
  }

  /// Regression test: padding-top with var() fallback must produce exact class name
  /// to match Babel's @compiled/babel-plugin output.
  #[test]
  fn padding_top_var_produces_exact_classname() {
    // This is the exact CSS pattern with design token fallback
    let css = "& { padding-top: var(--ds-space-300, 24px); }";
    let mut options = TransformCssOptions::default();
    options.optimize_css = Some(true);
    let result = transform_css(css, options).expect("transform should succeed");

    assert_eq!(result.class_names.len(), 1);
    // The class name must match Babel output: _ca0q1ejb
    // Group hash: ca0q (from "undefined&padding-top")
    // Value hash: 1ejb (from "var(--ds-space-300, 24px)" WITH space - Babel hashes before whitespace normalization)
    assert_eq!(result.class_names[0], "_ca0q1ejb");

    // Verify the CSS output contains the normalized value (no space after comma in var())
    // Note: The OUTPUT is normalized, but the HASH uses the original value with space
    assert!(result.sheets[0].contains("padding-top:var(--ds-space-300,24px)"));
  }

  /// Regression test: background-color var() with rgba fallback should not
  /// introduce a trailing space before ')', matching Babel's hash.
  #[test]
  fn background_color_var_rgba_fallback_produces_exact_classname() {
    let css = "& { background-color: var(--ds-surface, rgba(255, 255, 255, 1)); }";
    let mut options = TransformCssOptions::default();
    options.optimize_css = Some(true);
    let result = transform_css(css, options).expect("transform should succeed");

    assert_eq!(result.class_names.len(), 1);
    // The class name must match Babel output: _bfhkvuon
    assert_eq!(result.class_names[0], "_bfhkvuon");
    // Verify the CSS output contains the normalized fallback color
    assert!(result.sheets[0].contains("background-color:var(--ds-surface,#fff)"));
  }

  /// Regression test: linear-gradient background must match Babel's hash.
  /// Babel outputs: ._11q7taqa{background:linear-gradient(90deg,#4d8ced,#cfe1fd)}
  #[test]
  fn linear_gradient_background_produces_exact_classname() {
    let css = "& { background: linear-gradient(90deg, #4d8ced, #cfe1fd); }";
    let mut options = TransformCssOptions::default();
    options.optimize_css = Some(true);
    let result = transform_css(css, options).expect("transform should succeed");

    assert_eq!(result.class_names.len(), 1);
    // The class name must match Babel output: _11q7taqa
    // Group hash: 11q7 (from "undefined&background")
    // Value hash: taqa (from "linear-gradient(90deg, #4d8ced, #cfe1fd)" WITH spaces)
    assert_eq!(result.class_names[0], "_11q7taqa");

    // Verify the CSS output contains the minified gradient (no spaces after commas)
    assert!(result.sheets[0].contains("background:linear-gradient(90deg,#4d8ced,#cfe1fd)"));
  }

  /// Regression test: text-decoration-color should use 'initial' when browsers support it,
  /// matching Babel's output: ._4bfu18uv{text-decoration-color:initial}
  #[test]
  fn text_decoration_color_uses_initial() {
    use crate::postcss::plugins::normalize_css_engine::browserslist_support::browserslist_cache;
    use std::fs;

    // Create a temp browserslist config that targets modern browsers
    let tmp = tempfile::tempdir().expect("tmpdir");
    fs::write(tmp.path().join(".browserslistrc"), "Chrome >= 80\n")
      .expect("browserslist config write");
    browserslist_cache().lock().unwrap().clear();

    let css = "& { text-decoration-color: currentColor; }";
    let mut options = TransformCssOptions::default();
    options.optimize_css = Some(true);
    options.browserslist_config_path = Some(tmp.path().to_path_buf());
    let result = transform_css(css, options).expect("transform should succeed");

    assert_eq!(result.class_names.len(), 1);
    // The class name must match Babel output: _4bfu18uv
    // Group hash: 4bfu (from "undefined&text-decoration-color")
    // Value hash: 18uv (from "initial")
    assert_eq!(result.class_names[0], "_4bfu18uv");

    // Verify the CSS output contains 'initial'
    assert!(result.sheets[0].contains("text-decoration-color:initial"));

    // Clean up cache
    browserslist_cache()
      .lock()
      .unwrap()
      .remove(&tmp.path().to_path_buf());
  }

  /// Regression test: background: transparent should expand to background-color:initial
  /// matching Babel output: ._bfhk18uv{background-color:initial}
  #[test]
  fn background_transparent_expands_to_initial() {
    use crate::postcss::plugins::normalize_css_engine::browserslist_support::browserslist_cache;
    use std::fs;

    let tmp = tempfile::tempdir().expect("tmpdir");
    fs::write(tmp.path().join(".browserslistrc"), "Chrome >= 80\n")
      .expect("browserslist config write");
    browserslist_cache().lock().unwrap().clear();

    let css = "& { background: transparent; }";
    let mut options = TransformCssOptions::default();
    options.optimize_css = Some(true);
    options.browserslist_config_path = Some(tmp.path().to_path_buf());
    let result = transform_css(css, options).expect("transform should succeed");

    assert_eq!(result.class_names.len(), 1);
    assert_eq!(result.class_names[0], "_bfhk18uv");
    assert!(result.sheets[0].contains("background-color:initial"));

    browserslist_cache()
      .lock()
      .unwrap()
      .remove(&tmp.path().to_path_buf());
  }
}
