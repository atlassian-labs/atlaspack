use crate::postcss::value_parser as vp;
use postcss as pc;

fn gcd(mut a: i64, mut b: i64) -> i64 {
  while b != 0 {
    let t = b;
    b = a % b;
    a = t;
  }
  a.abs()
}

fn aspect_ratio(a: i64, b: i64) -> (i64, i64) {
  let d = gcd(a, b);
  (a / d, b / d)
}

fn split_arg(arg: &[vp::Node]) -> String {
  vp::stringify(arg)
}

pub fn plugin() -> pc::BuiltPlugin {
  // Default: no IE10/11 "all" bug handling (legacy=false). Browserslist gating can be added later.
  let legacy = false;

  pc::plugin("postcss-minify-params")
        // Operate directly on @rules to avoid full-tree once_exit traversal (prevents stalls).
        .at_rule_filter("*", move |at, _| {
            let name = at.name().to_lowercase();
            if name != "media" && name != "supports" { return Ok(()); }

            let params_str = at.params();
            if params_str.is_empty() { return Ok(()); }
            let tracing = std::env::var("COMPILED_CLI_TRACE").is_ok();
            if tracing {
                eprintln!("[minify-params] params @{} {}", name, params_str);
                eprintln!("[minify-params] enter @{} len={}", name, params_str.len());
            }
            // Cheap pre-scan: if no tokens that we care about exist, skip work
            let has_tokens = params_str.contains('(')
                || params_str.contains(':')
                || params_str.contains('/')
                || params_str.contains(',')
                || (name == "media" && params_str.to_ascii_lowercase().contains("all"));
            if !has_tokens {
                if tracing { eprintln!("[minify-params] skip (no-tokens) @{}", name); }
                return Ok(());
            }
            // Optional guard via env to avoid pathological allocations on very large params
            if let Ok(max_str) = std::env::var("COMPILED_MINIFY_PARAMS_MAXLEN") {
                if let Ok(max) = max_str.parse::<usize>() {
                    if params_str.len() > max {
                        if tracing { eprintln!("[minify-params] skip (len>{}) @{}", max, name); }
                        return Ok(());
                    }
                }
            }

            // Parse and normalize parameter AST in-place via manual traversal to avoid walker overhead.
            let mut parsed = vp::parse(&params_str);
            if tracing { eprintln!("[minify-params] parsed @{}", name); }

            fn normalize_nodes(nodes: &mut Vec<vp::Node>) {
                let mut i = 0usize;
                while i < nodes.len() {
                    match nodes.get_mut(i) {
                        Some(vp::Node::Div { before, after, .. }) => {
                            before.clear();
                            after.clear();
                            // Remove standalone whitespace nodes around punctuation to mirror
                            // postcss-value-parser attaching spaces to `before`/`after`.
                            while i > 0 {
                                if matches!(nodes.get(i.saturating_sub(1)), Some(vp::Node::Space { .. })) {
                                    nodes.remove(i - 1);
                                    i -= 1;
                                } else {
                                    break;
                                }
                            }
                            while i + 1 < nodes.len() {
                                if matches!(nodes.get(i + 1), Some(vp::Node::Space { .. })) {
                                    nodes.remove(i + 1);
                                } else {
                                    break;
                                }
                            }
                        }
                        Some(vp::Node::Space { value }) => { *value = " ".to_string(); }
                        Some(vp::Node::Function { nodes: inner, before, after, value: _, .. }) => {
                            before.clear();
                            // Custom properties spacing: keep a single trailing space for single-arg custom props
                            if let Some(first) = inner.get(0) {
                                if let vp::Node::Word { value: v0 } = first {
                                    if v0.starts_with("--") && inner.get(2).is_none() { *after = " ".to_string(); } else { after.clear(); }
                                } else { after.clear(); }
                            } else { after.clear(); }
                            // Aspect-ratio: only when first inner node has "-aspect-ratio" at index 3 and there are numbers at [2] and [4]
                            if inner.len() > 4 {
                                let first_is_aspect = match inner.get(0) {
                                    Some(vp::Node::Word { value: v }) => v.to_ascii_lowercase().find("-aspect-ratio") == Some(3),
                                    _ => false,
                                };
                                if first_is_aspect {
                                    let (left, right) = inner.split_at_mut(4);
                                    let n2 = left.get_mut(2);
                                    let n4 = right.get_mut(0);
                                    if let (Some(vp::Node::Word { value: a_str }), Some(vp::Node::Word { value: b_str })) = (n2, n4) {
                                        if let (Ok(a), Ok(b)) = (a_str.parse::<i64>(), b_str.parse::<i64>()) {
                                            let (ra, rb) = aspect_ratio(a, b);
                                            *a_str = ra.to_string(); *b_str = rb.to_string();
                                        }
                                    }
                                }
                            }
                            normalize_nodes(inner);
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }

            normalize_nodes(&mut parsed.nodes);
            if tracing { eprintln!("[minify-params] normalized nodes @{}", name); }
            if tracing && params_str.to_ascii_lowercase().contains("aspect-ratio") {
                eprintln!(
                    "[minify-params] normalized params {} -> {}",
                    params_str,
                    vp::stringify(&parsed.nodes)
                );
            }

            // Handle @media all removal at top-level exactly like JS plugin
            if name == "media" {
                // find any 'all' at start: pattern [Word(all)] optionally followed by Space, Word(and), Space
                let mut i = 0usize;
                while i < parsed.nodes.len() {
                    let is_all = match parsed.nodes.get(i) {
                        Some(vp::Node::Word { value }) => value.eq_ignore_ascii_case("all"),
                        _ => false,
                    };
                    // prevWord defined if i>=2 and parsed.nodes[i-2] is Word
                    let prev_word_exists = if i >= 2 { matches!(parsed.nodes.get(i-2), Some(vp::Node::Word { .. })) } else { false };
                    if is_all && !prev_word_exists {
                        let next_exists = parsed.nodes.get(i+2).is_some();
                        let next_is_and = match parsed.nodes.get(i+2) {
                            Some(vp::Node::Word { value }) => value.eq_ignore_ascii_case("and"),
                            _ => false,
                        };
                        if !legacy || next_exists {
                            if let Some(vp::Node::Word { value }) = parsed.nodes.get_mut(i) { value.clear(); }
                        }
                        if next_is_and {
                            if let Some(node) = parsed.nodes.get_mut(i+2) { *node = vp::Node::Word { value: String::new() }; }
                            if let Some(node) = parsed.nodes.get_mut(i+1) { *node = vp::Node::Word { value: String::new() }; }
                            if let Some(node) = parsed.nodes.get_mut(i+3) { *node = vp::Node::Word { value: String::new() }; }
                        }
                        break;
                    }
                    i += 1;
                }
                if tracing { eprintln!("[minify-params] after media all @{}", name); }
            }

            // Split by commas, then sort and dedupe for deterministic output
            let args = crate::postcss::plugins::normalize_css_engine::ordered_values::library::arguments::get_arguments(&parsed);
            if tracing { eprintln!("[minify-params] get_arguments -> {} args @{}", args.len(), name); }
            let mut joined = {
                let splits: Vec<String> = args.into_iter().map(|a| split_arg(&a)).collect();
                let set: std::collections::BTreeSet<String> = splits.into_iter().collect();
                set.into_iter().collect::<Vec<_>>().join(",")
            };
            if tracing && params_str.to_ascii_lowercase().contains("aspect-ratio") {
                eprintln!("[minify-params] joined raw {}", joined);
            }
            // Ensure no spaces after ':' inside parameters to match cssnano
            if joined.contains(": ") {
                joined = joined.replace(": ", ":");
            }
            if tracing && params_str.to_ascii_lowercase().contains("aspect-ratio") {
                eprintln!("[minify-params] joined norm {}", joined);
            }
            if tracing { eprintln!("[minify-params] joined len={} @{}", joined.len(), name); }

            // Write back. The stringifier inserts one space after name when params are non-empty.
            at.set_params(joined.clone());

            if joined.is_empty() {
                // Ensure no stray space after at-rule name for empty params
                let node_ref = at.to_node();
                {
                    let mut n = node_ref.borrow_mut();
                    n.raws.set_text("afterName", "");
                }
            }
            if tracing { eprintln!("[minify-params] exit   @{}", name); }

            Ok(())
        })
        .build()
}
