use atlassian_swc_compiled_css::{css::normalize_css_value, hash::hash};

fn main() {
  let variants = [
    "calc(100vh - var(--topNavigationHeight,0px) - var(--bannerHeight,0px))",
    "calc(100vh - var(--topNavigationHeight, 0px) - var(--bannerHeight, 0px))",
    "calc(100vh - var(--topNavigationHeight, 0px) - var(--bannerHeight,0px))",
    "calc(100vh - var(--topNavigationHeight,0px) - var(--bannerHeight, 0px))",
    "calc(100vh - (var(--topNavigationHeight,0px) + var(--bannerHeight,0px)))",
    "calc(100vh - (var(--topNavigationHeight, 0px) + var(--bannerHeight, 0px)))",
  ];

  for value in variants {
    let normalized = normalize_css_value(value);
    let hash = hash(&normalized.hash_value, 0);
    println!(
      "{} => hash_value: {:?}, hash: {}, output_value: {:?}",
      value, normalized.hash_value, hash, normalized.output_value
    );
  }
}
