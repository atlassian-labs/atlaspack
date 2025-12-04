#[cfg(feature = "postcss_engine")]
use postcss as pc;

#[cfg(feature = "postcss_engine")]
fn collapse_internal_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            prev_space = false;
            out.push(ch);
        }
    }
    out.trim().to_string()
}

#[cfg(feature = "postcss_engine")]
pub fn minify_selectors_plugin() -> pc::BuiltPlugin {
    // Placeholder: whitespace minimization only. To be replaced with exact port.
    pc::plugin("postcss-minify-selectors")
        .rule(|rule, _| {
            let current = rule.selector();
            let normalized = collapse_internal_whitespace(&current);
            if normalized != current {
                rule.set_selector(normalized);
            }
            Ok(())
        })
        .build()
}

#[cfg(feature = "postcss_engine")]
pub fn minify_params_plugin() -> pc::BuiltPlugin {
    // Placeholder: whitespace minimization only. To be replaced with exact port.
    pc::plugin("postcss-minify-params")
        .at_rule(|at, _| {
            let current = at.params();
            let normalized = collapse_internal_whitespace(&current);
            if normalized != current {
                at.set_params(normalized);
            }
            Ok(())
        })
        .build()
}

