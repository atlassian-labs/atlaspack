// Port of packages/postcss-plugin-sources/postcss-ordered-values/src/lib/vendorUnprefixed.js
pub fn vendor_unprefixed(prop: &str) -> &str {
  prop
    .strip_prefix("-webkit-")
    .or_else(|| prop.strip_prefix("-moz-"))
    .or_else(|| prop.strip_prefix("-ms-"))
    .or_else(|| prop.strip_prefix("-o-"))
    .unwrap_or(prop)
}
