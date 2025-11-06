fn main() {
  let candidates = [
    "__cmplp.formatRuleHoverColor",
    "formatRuleHoverColor",
    "var(--ds-surface-hovered,#f1f2f4)",
    "background-color:var(--ds-surface-hovered,#f1f2f4)",
    "__cmplp.formatRuleHoverColor ? __cmplp.formatRuleHoverColor : \"var(--ds-surface-hovered,#f1f2f4)\"",
  ];

  for value in candidates {
    let hash = compiled_swc_plugin::hash::hash(value, 0);
    println!("{value} => {hash}");
  }
}
