use caniuse_serde::FeatureName;
use once_cell::sync::Lazy;
use postcss as pc;
use std::{collections::HashMap, path::PathBuf};

use super::browserslist_support::feature_supported_for_config_path;

#[cfg(test)]
use super::browserslist_support::browserslist_cache;

static TO_INITIAL: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
  // Strict port of src/data/toInitial.json
  let mut m = HashMap::new();

  m.insert("background-clip", "border-box");
  m.insert("background-color", "transparent");
  m.insert("background-origin", "padding-box");
  m.insert("background-size", "auto auto");
  m.insert("border-block-color", "currentcolor");
  m.insert("border-block-end-color", "currentcolor");
  m.insert("border-block-start-color", "currentcolor");
  m.insert("border-bottom-color", "currentcolor");
  m.insert("border-collapse", "separate");
  m.insert("border-inline-color", "currentcolor");
  m.insert("border-inline-end-color", "currentcolor");
  m.insert("border-inline-start-color", "currentcolor");
  m.insert("border-left-color", "currentcolor");
  m.insert("border-right-color", "currentcolor");
  m.insert("border-top-color", "currentcolor");
  m.insert("box-sizing", "content-box");
  m.insert("color", "canvastext");
  m.insert("column-rule-color", "currentcolor");
  m.insert("font-synthesis", "weight style small-caps position");
  m.insert("image-orientation", "from-image");
  m.insert("mask-clip", "border-box");
  m.insert("mask-mode", "match-source");
  m.insert("mask-origin", "border-box");
  m.insert("mask-type", "luminance");
  m.insert("ruby-align", "space-around");
  m.insert("ruby-merge", "separate");
  m.insert("ruby-position", "alternate");
  m.insert("text-decoration-color", "currentcolor");
  m.insert("text-emphasis-color", "currentcolor");
  m.insert("text-emphasis-position", "over right");
  m.insert("transform-box", "view-box");
  m.insert("transform-origin", "50% 50% 0");
  m.insert("vertical-align", "baseline");
  m.insert("white-space-collapse", "collapse");
  m.insert("writing-mode", "horizontal-tb");
  m
});

static FROM_INITIAL: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
  // Strict port of src/data/fromInitial.json
  let mut m = HashMap::new();
  m.insert("-webkit-line-clamp", "none");
  m.insert("accent-color", "auto");
  m.insert("align-content", "normal");
  m.insert("align-items", "normal");
  m.insert("align-self", "auto");
  m.insert("align-tracks", "normal");
  m.insert("animation-delay", "0s");
  m.insert("animation-direction", "normal");
  m.insert("animation-duration", "0s");
  m.insert("animation-fill-mode", "none");
  m.insert("animation-iteration-count", "1");
  m.insert("animation-name", "none");
  m.insert("animation-timing-function", "ease");
  m.insert("animation-timeline", "auto");
  m.insert("appearance", "none");
  m.insert("aspect-ratio", "auto");
  m.insert("azimuth", "center");
  m.insert("backdrop-filter", "none");
  m.insert("background-attachment", "scroll");
  m.insert("background-blend-mode", "normal");
  m.insert("background-image", "none");
  m.insert("background-position", "0% 0%");
  m.insert("background-position-x", "0%");
  m.insert("background-position-y", "0%");
  m.insert("background-repeat", "repeat");
  m.insert("block-overflow", "clip");
  m.insert("block-size", "auto");
  m.insert("border-block-style", "none");
  m.insert("border-block-width", "medium");
  m.insert("border-block-end-style", "none");
  m.insert("border-block-end-width", "medium");
  m.insert("border-block-start-style", "none");
  m.insert("border-block-start-width", "medium");
  m.insert("border-bottom-left-radius", "0");
  m.insert("border-bottom-right-radius", "0");
  m.insert("border-bottom-style", "none");
  m.insert("border-bottom-width", "medium");
  m.insert("border-end-end-radius", "0");
  m.insert("border-end-start-radius", "0");
  m.insert("border-image-outset", "0");
  m.insert("border-image-slice", "100%");
  m.insert("border-image-source", "none");
  m.insert("border-image-width", "1");
  m.insert("border-inline-style", "none");
  m.insert("border-inline-width", "medium");
  m.insert("border-inline-end-style", "none");
  m.insert("border-inline-end-width", "medium");
  m.insert("border-inline-start-style", "none");
  m.insert("border-inline-start-width", "medium");
  m.insert("border-left-style", "none");
  m.insert("border-left-width", "medium");
  m.insert("border-right-style", "none");
  m.insert("border-right-width", "medium");
  m.insert("border-spacing", "0");
  m.insert("border-start-end-radius", "0");
  m.insert("border-start-start-radius", "0");
  m.insert("border-top-left-radius", "0");
  m.insert("border-top-right-radius", "0");
  m.insert("border-top-style", "none");
  m.insert("border-top-width", "medium");
  m.insert("bottom", "auto");
  m.insert("box-decoration-break", "slice");
  m.insert("box-shadow", "none");
  m.insert("break-after", "auto");
  m.insert("break-before", "auto");
  m.insert("break-inside", "auto");
  m.insert("caption-side", "top");
  m.insert("caret-color", "auto");
  m.insert("caret-shape", "auto");
  m.insert("clear", "none");
  m.insert("clip", "auto");
  m.insert("clip-path", "none");
  m.insert("color-scheme", "normal");
  m.insert("column-count", "auto");
  m.insert("column-gap", "normal");
  m.insert("column-rule-style", "none");
  m.insert("column-rule-width", "medium");
  m.insert("column-span", "none");
  m.insert("column-width", "auto");
  m.insert("contain", "none");
  m.insert("contain-intrinsic-block-size", "none");
  m.insert("contain-intrinsic-height", "none");
  m.insert("contain-intrinsic-inline-size", "none");
  m.insert("contain-intrinsic-width", "none");
  m.insert("content", "normal");
  m.insert("counter-increment", "none");
  m.insert("counter-reset", "none");
  m.insert("counter-set", "none");
  m.insert("cursor", "auto");
  m.insert("direction", "ltr");
  m.insert("empty-cells", "show");
  m.insert("font-variant-ligatures", "normal");
  m.insert("font-variant-numeric", "normal");
  m.insert("font-variant-position", "normal");
  m.insert("font-weight", "normal");
  m.insert("forced-color-adjust", "auto");
  m.insert("grid-auto-columns", "auto");
  m.insert("grid-auto-flow", "row");
  m.insert("grid-auto-rows", "auto");
  m.insert("grid-column-end", "auto");
  m.insert("grid-column-gap", "0");
  m.insert("grid-column-start", "auto");
  m.insert("grid-row-end", "auto");
  m.insert("grid-row-gap", "0");
  m.insert("grid-row-start", "auto");
  m.insert("grid-template-areas", "none");
  m.insert("grid-template-columns", "none");
  m.insert("grid-template-rows", "none");
  m.insert("hanging-punctuation", "none");
  m.insert("height", "auto");
  m.insert("hyphenate-character", "auto");
  m.insert("hyphens", "manual");
  m.insert("image-rendering", "auto");
  m.insert("image-resolution", "1dppx");
  m.insert("ime-mode", "auto");
  m.insert("initial-letter", "normal");
  m.insert("initial-letter-align", "auto");
  m.insert("inline-size", "auto");
  m.insert("input-security", "auto");
  m.insert("inset", "auto");
  m.insert("inset-block", "auto");
  m.insert("inset-block-end", "auto");
  m.insert("inset-block-start", "auto");
  m.insert("inset-inline", "auto");
  m.insert("inset-inline-end", "auto");
  m.insert("inset-inline-start", "auto");
  m.insert("isolation", "auto");
  m.insert("justify-content", "normal");
  m.insert("justify-items", "legacy");
  m.insert("justify-self", "auto");
  m.insert("justify-tracks", "normal");
  m.insert("left", "auto");
  m.insert("letter-spacing", "normal");
  m.insert("line-break", "auto");
  m.insert("line-clamp", "none");
  m.insert("line-height", "normal");
  m.insert("line-height-step", "0");
  m.insert("list-style-image", "none");
  m.insert("list-style-type", "disc");
  m.insert("margin-block", "0");
  m.insert("margin-block-end", "0");
  m.insert("margin-block-start", "0");
  m.insert("margin-bottom", "0");
  m.insert("margin-inline", "0");
  m.insert("margin-inline-end", "0");
  m.insert("margin-inline-start", "0");
  m.insert("margin-left", "0");
  m.insert("margin-right", "0");
  m.insert("margin-top", "0");
  m.insert("margin-trim", "none");
  m.insert("mask-border-mode", "alpha");
  m.insert("mask-border-outset", "0");
  m.insert("mask-border-slice", "0");
  m.insert("mask-border-source", "none");
  m.insert("mask-border-width", "auto");
  m.insert("mask-composite", "add");
  m.insert("mask-image", "none");
  m.insert("mask-position", "center");
  m.insert("mask-repeat", "repeat");
  m.insert("mask-size", "auto");
  m.insert("masonry-auto-flow", "pack");
  m.insert("math-depth", "0");
  m.insert("math-shift", "normal");
  m.insert("math-style", "normal");
  m.insert("max-block-size", "none");
  m.insert("max-height", "none");
  m.insert("max-inline-size", "none");
  m.insert("max-lines", "none");
  m.insert("max-width", "none");
  m.insert("min-block-size", "0");
  m.insert("min-height", "auto");
  m.insert("min-inline-size", "0");
  m.insert("min-width", "auto");
  m.insert("mix-blend-mode", "normal");
  m.insert("object-fit", "fill");
  m.insert("offset-anchor", "auto");
  m.insert("offset-distance", "0");
  m.insert("offset-path", "none");
  m.insert("offset-position", "auto");
  m.insert("offset-rotate", "auto");
  m.insert("opacity", "1");
  m.insert("order", "0");
  m.insert("orphans", "2");
  m.insert("outline-offset", "0");
  m.insert("outline-style", "none");
  m.insert("outline-width", "medium");
  m.insert("overflow-anchor", "auto");
  m.insert("overflow-block", "auto");
  m.insert("overflow-clip-margin", "0px");
  m.insert("overflow-inline", "auto");
  m.insert("overflow-wrap", "normal");
  m.insert("overscroll-behavior", "auto");
  m.insert("overscroll-behavior-block", "auto");
  m.insert("overscroll-behavior-inline", "auto");
  m.insert("overscroll-behavior-x", "auto");
  m.insert("overscroll-behavior-y", "auto");
  m.insert("padding-block", "0");
  m.insert("padding-block-end", "0");
  m.insert("padding-block-start", "0");
  m.insert("padding-bottom", "0");
  m.insert("padding-inline", "0");
  m.insert("padding-inline-end", "0");
  m.insert("padding-inline-start", "0");
  m.insert("padding-left", "0");
  m.insert("padding-right", "0");
  m.insert("padding-top", "0");
  m.insert("page-break-after", "auto");
  m.insert("page-break-before", "auto");
  m.insert("page-break-inside", "auto");
  m.insert("paint-order", "normal");
  m.insert("perspective", "none");
  m.insert("place-content", "normal");
  m.insert("pointer-events", "auto");
  m.insert("position", "static");
  m.insert("resize", "none");
  m.insert("right", "auto");
  m.insert("rotate", "none");
  m.insert("row-gap", "normal");
  m.insert("scale", "none");
  m.insert("scrollbar-color", "auto");
  m.insert("scrollbar-gutter", "auto");
  m.insert("scrollbar-width", "auto");
  m.insert("scroll-behavior", "auto");
  m.insert("scroll-margin", "0");
  m.insert("scroll-margin-block", "0");
  m.insert("scroll-margin-block-start", "0");
  m.insert("scroll-margin-block-end", "0");
  m.insert("scroll-margin-bottom", "0");
  m.insert("scroll-margin-inline", "0");
  m.insert("scroll-margin-inline-start", "0");
  m.insert("scroll-margin-inline-end", "0");
  m.insert("scroll-margin-left", "0");
  m.insert("scroll-margin-right", "0");
  m.insert("scroll-margin-top", "0");
  m.insert("scroll-padding", "auto");
  m.insert("scroll-padding-block", "auto");
  m.insert("scroll-padding-block-start", "auto");
  m.insert("scroll-padding-block-end", "auto");
  m.insert("scroll-padding-bottom", "auto");
  m.insert("scroll-padding-inline", "auto");
  m.insert("scroll-padding-inline-start", "auto");
  m.insert("scroll-padding-inline-end", "auto");
  m.insert("scroll-padding-left", "auto");
  m.insert("scroll-padding-right", "auto");
  m.insert("scroll-padding-top", "auto");
  m.insert("scroll-snap-align", "none");
  m.insert("scroll-snap-coordinate", "none");
  m.insert("scroll-snap-points-x", "none");
  m.insert("scroll-snap-points-y", "none");
  m.insert("scroll-snap-stop", "normal");
  m.insert("scroll-snap-type", "none");
  m.insert("scroll-snap-type-x", "none");
  m.insert("scroll-snap-type-y", "none");
  m.insert("scroll-timeline-axis", "block");
  m.insert("scroll-timeline-name", "none");
  m.insert("shape-image-threshold", "0.0");
  m.insert("shape-margin", "0");
  m.insert("shape-outside", "none");
  m.insert("tab-size", "8");
  m.insert("table-layout", "auto");
  m.insert("text-align-last", "auto");
  m.insert("text-combine-upright", "none");
  m.insert("text-decoration-line", "none");
  m.insert("text-decoration-skip-ink", "auto");
  m.insert("text-decoration-style", "solid");
  m.insert("text-decoration-thickness", "auto");
  m.insert("text-emphasis-style", "none");
  m.insert("text-indent", "0");
  m.insert("text-justify", "auto");
  m.insert("text-orientation", "mixed");
  m.insert("text-overflow", "clip");
  m.insert("text-rendering", "auto");
  m.insert("text-shadow", "none");
  m.insert("text-transform", "none");
  m.insert("text-underline-offset", "auto");
  m.insert("text-underline-position", "auto");
  m.insert("top", "auto");
  m.insert("touch-action", "auto");
  m.insert("transform", "none");
  m.insert("transform-style", "flat");
  m.insert("transition-delay", "0s");
  m.insert("transition-duration", "0s");
  m.insert("transition-property", "all");
  m.insert("transition-timing-function", "ease");
  m.insert("translate", "none");
  m.insert("unicode-bidi", "normal");
  m.insert("user-select", "auto");
  m.insert("white-space", "normal");
  m.insert("widows", "2");
  m.insert("width", "auto");
  m.insert("will-change", "auto");
  m.insert("word-break", "normal");
  m.insert("word-spacing", "normal");
  m.insert("word-wrap", "normal");
  m.insert("z-index", "auto");
  // Plus shorthand examples used by the plugin
  m.insert("outline", "invert none medium");
  m
});

fn initial_support(config_path: Option<PathBuf>) -> (bool, Vec<String>) {
  feature_supported_for_config_path(config_path, FeatureName::from("css-initial-value"))
}

pub fn plugin(config_path: Option<PathBuf>) -> pc::BuiltPlugin {
  let ignore_default = vec!["writing-mode", "transform-box"];
  // Align with Babel/cssnano: enable `initial` only when the resolved browsers support it.
  let (initial_support, _) = initial_support(config_path);

  pc::plugin("postcss-reduce-initial")
    .once_exit(move |css, _| {
      let process_decl = |decl: postcss::ast::nodes::Declaration| {
        let prop = decl.prop().to_lowercase();
        if ignore_default.contains(&prop.as_str()) {
          return;
        }
        let value_l = decl.value().to_lowercase();
        if initial_support {
          if let Some(&ti) = TO_INITIAL.get(prop.as_str()) {
            if value_l == ti {
              decl.set_value("initial".to_string());
              return;
            }
          }
        }
        if value_l == "initial" {
          if let Some(&from) = FROM_INITIAL.get(prop.as_str()) {
            decl.set_value(from.to_string());
          }
        }
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  #[test]
  fn converts_background_color_transparent_to_initial() {
    let tmp = tempfile::tempdir().expect("tmpdir");
    fs::write(tmp.path().join(".browserslistrc"), "Chrome 100\n")
      .expect("browserslist config write");

    browserslist_cache().lock().unwrap().clear();

    let plugin = plugin(Some(tmp.path().to_path_buf()));
    let processor = pc::postcss_with_plugins(vec![plugin]);
    let mut result = processor
      .process("a{background-color:transparent}")
      .expect("process should succeed");

    assert_eq!(
      result.css().expect("css string").to_string(),
      "a{background-color:initial}"
    );
    browserslist_cache()
      .lock()
      .unwrap()
      .remove(&tmp.path().to_path_buf());
  }
}
