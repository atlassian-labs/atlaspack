# caniuse_database

Provides a small wrapper on top of caniuse data to check browser support for a given browser
feature.

The data is taken from the `data.json` file from caniuse. A stripped down version of file is
vendored into the binary and is around 364KB.

Alternatively the full database can be parsed and used to check support at runtime.

## Examples

### Check feature support for a specific version

Check if Chrome 92 supports the WebUSB API using [`check_browser_support`].

```rust
use caniuse_database::{BrowserFeature, BrowserAgent, Version, check_browser_support};

let result: bool = check_browser_support(
   &BrowserFeature::Webusb,
   &BrowserAgent::Chrome,
   &Version::try_from("92.0.0").unwrap(),
);

assert!(result);
```

### Check feature support for a browserlist

Use [`browserslist::resolve`] and [`check_browserslist_support`] to check for feature support
against a certain browserlist query.

```rust
use caniuse_database::{BrowserFeature, check_browserslist_support};

let list = browserslist::resolve(&["last 2 chrome versions"], &Default::default()).unwrap();
let result: bool = check_browserslist_support(&BrowserFeature::ArrowFunctions, &list);

assert!(result);
```

## Version parsing

We add a simplistic version parser to parse `1.2` or `1.0` or `1.2-1.3` into [`Version`] and
[`VersionRange`].

## Caveats

- Technology preview / non-numeric versions are not supported.
