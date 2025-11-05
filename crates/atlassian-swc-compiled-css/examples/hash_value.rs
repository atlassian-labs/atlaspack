use atlassian_swc_compiled_css::{
  css::normalize_css_value,
  hash::{self, hash},
};

fn main() {
  let candidates = [
    "__cmplp.formatRuleHoverColor",
    "formatRuleHoverColor",
    "var(--ds-surface-hovered,#f1f2f4)",
    "background-color:var(--ds-surface-hovered,#f1f2f4)",
    "__cmplp.formatRuleHoverColor ? __cmplp.formatRuleHoverColor : \"var(--ds-surface-hovered,#f1f2f4)\"",
  ];

  for value in candidates {
    let hash = normalize_css_value(value);
    let hash(value, 0);
    println!("{value} => {hash}");
  }
}
