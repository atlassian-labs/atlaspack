use super::{
  CssnanoPlaceholder, discard_comments::discard_comments, minify_params::minify_params,
  minify_selectors::minify_selectors, normalize_current_color::normalize_current_color,
  reduce_initial::reduce_initial,
};
use crate::postcss::transform::Plugin;
use crate::postcss::transform::TransformCssOptions;

/// Return the list of cssnano-aligned plugins that should be executed for normalization.
pub fn normalize_css(options: &TransformCssOptions) -> Vec<Box<dyn Plugin>> {
  let mut plugins: Vec<Box<dyn Plugin>> = Vec::new();

  // Base plugins that always run regardless of `optimize_css`.
  plugins.push(Box::new(minify_selectors(true)));
  plugins.push(Box::new(minify_params()));

  if options.optimize_css.unwrap_or(true) {
    // Production-only plugins that cssnano would include, in preset order.
    plugins.push(Box::new(CssnanoPlaceholder::new("postcss-ordered-values")));
    plugins.push(Box::new(reduce_initial()));
    plugins.push(Box::new(super::convert_values::convert_values()));
    plugins.push(Box::new(CssnanoPlaceholder::new("postcss-colormin")));
    plugins.push(Box::new(CssnanoPlaceholder::new("postcss-normalize-url")));
    plugins.push(Box::new(CssnanoPlaceholder::new(
      "postcss-normalize-unicode",
    )));
    plugins.push(Box::new(CssnanoPlaceholder::new(
      "postcss-normalize-string",
    )));
    plugins.push(Box::new(CssnanoPlaceholder::new(
      "postcss-normalize-positions",
    )));
    plugins.push(Box::new(CssnanoPlaceholder::new(
      "postcss-normalize-timing-functions",
    )));
    plugins.push(Box::new(CssnanoPlaceholder::new(
      "postcss-minify-gradients",
    )));
    plugins.push(Box::new(discard_comments()));
    plugins.push(Box::new(CssnanoPlaceholder::new("postcss-calc")));

    // Custom plugin beyond cssnano preset default.
    plugins.push(Box::new(normalize_current_color()));
  }

  plugins
}
