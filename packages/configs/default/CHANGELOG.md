# @atlaspack/config-default

## 3.1.1

### Patch Changes

- [#478](https://github.com/atlassian-labs/atlaspack/pull/478) [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b) Thanks [@yamadapc](https://github.com/yamadapc)! - The first attempt at Version Packages didn't include the built artifacts.
  This has hopefully been fixed, so this change will force those packages to re-release.
- Updated dependencies [[`b9d41b1`](https://github.com/atlassian-labs/atlaspack/commit/b9d41b175ad5771651a5b0278a5a0147e669234a), [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b)]:
  - @atlaspack/transformer-react-refresh-wrap@2.14.1
  - @atlaspack/runtime-service-worker@2.14.1
  - @atlaspack/runtime-react-refresh@2.14.1
  - @atlaspack/transformer-posthtml@2.14.1
  - @atlaspack/reporter-dev-server@2.14.1
  - @atlaspack/transformer-postcss@2.14.1
  - @atlaspack/optimizer-htmlnano@2.14.1
  - @atlaspack/transformer-image@3.1.1
  - @atlaspack/resolver-default@2.14.1
  - @atlaspack/transformer-html@2.14.1
  - @atlaspack/transformer-json@2.14.1
  - @atlaspack/bundler-default@2.14.1
  - @atlaspack/optimizer-image@3.1.1
  - @atlaspack/transformer-css@2.14.1
  - @atlaspack/transformer-raw@2.14.1
  - @atlaspack/transformer-svg@2.14.1
  - @atlaspack/compressor-raw@2.13.3
  - @atlaspack/optimizer-svgo@2.14.1
  - @atlaspack/transformer-js@3.1.1
  - @atlaspack/namer-default@2.14.1
  - @atlaspack/optimizer-css@2.14.1
  - @atlaspack/optimizer-swc@2.14.1
  - @atlaspack/packager-html@2.14.1
  - @atlaspack/packager-wasm@2.14.1
  - @atlaspack/packager-css@2.14.1
  - @atlaspack/packager-raw@2.14.1
  - @atlaspack/packager-svg@2.14.1
  - @atlaspack/packager-js@2.14.1
  - @atlaspack/runtime-browser-hmr@2.14.1
  - @atlaspack/runtime-js@2.14.1

## 3.1.0

### Minor Changes

- [#383](https://github.com/atlassian-labs/atlaspack/pull/383) [`8386ca4`](https://github.com/atlassian-labs/atlaspack/commit/8386ca4dc318688fbed1af3bbebf2af3e7d24552) Thanks [@benjervis](https://github.com/benjervis)! - The default config no longer includes a list of auto-installable `parcelDependencies`. Consumers can install the required dependencies in their project like normal.

### Patch Changes

- [#414](https://github.com/atlassian-labs/atlaspack/pull/414) [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53) Thanks [@alshdavid](https://github.com/alshdavid)! - Added type:commonjs to package.json files

- Updated dependencies [[`bfe81e5`](https://github.com/atlassian-labs/atlaspack/commit/bfe81e551c4e4bb2cac7fc4745222e66962c1728), [`3460531`](https://github.com/atlassian-labs/atlaspack/commit/3460531d9cb036f2575a99ea69fe2b03cfd6ac06), [`a317453`](https://github.com/atlassian-labs/atlaspack/commit/a317453432b7f30e98f2a4cbcafdaa5601bcde63), [`f600560`](https://github.com/atlassian-labs/atlaspack/commit/f6005601be5ceacb52350c065070feb5649461e9), [`f13a53f`](https://github.com/atlassian-labs/atlaspack/commit/f13a53fa37def8d4c8b2fc4b596066e7595441dc), [`8bc3db9`](https://github.com/atlassian-labs/atlaspack/commit/8bc3db94cc7382b22ca8207c92af8f6389c17e2e), [`306246e`](https://github.com/atlassian-labs/atlaspack/commit/306246ee5a492583059b028ee5d0d1b49ce42223), [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53), [`3b43acf`](https://github.com/atlassian-labs/atlaspack/commit/3b43acfe15523a2614413b294785e33a6060e41e), [`f6afae7`](https://github.com/atlassian-labs/atlaspack/commit/f6afae7a168c85341f9f41aa70c2cd2491a9ff17), [`6c0f7a7`](https://github.com/atlassian-labs/atlaspack/commit/6c0f7a7378131e8705e2b10af1576cc207271577), [`be63a51`](https://github.com/atlassian-labs/atlaspack/commit/be63a515ad13dd5ec1e241843d9ef6fdae8699d5), [`91ffa66`](https://github.com/atlassian-labs/atlaspack/commit/91ffa662ea3af48f1ca0c4f0d976db9c48995f4f), [`50265fd`](https://github.com/atlassian-labs/atlaspack/commit/50265fdf4024ec18439e85b472aa77a7952e2e08)]:
  - @atlaspack/namer-default@2.14.0
  - @atlaspack/optimizer-css@2.14.0
  - @atlaspack/optimizer-htmlnano@2.14.0
  - @atlaspack/optimizer-image@3.1.0
  - @atlaspack/optimizer-svgo@2.14.0
  - @atlaspack/optimizer-swc@2.14.0
  - @atlaspack/packager-css@2.14.0
  - @atlaspack/packager-html@2.14.0
  - @atlaspack/packager-js@2.14.0
  - @atlaspack/packager-raw@2.14.0
  - @atlaspack/packager-svg@2.14.0
  - @atlaspack/packager-wasm@2.14.0
  - @atlaspack/reporter-dev-server@2.14.0
  - @atlaspack/resolver-default@2.14.0
  - @atlaspack/runtime-browser-hmr@2.14.0
  - @atlaspack/runtime-js@2.14.0
  - @atlaspack/runtime-react-refresh@2.14.0
  - @atlaspack/runtime-service-worker@2.14.0
  - @atlaspack/transformer-css@2.14.0
  - @atlaspack/transformer-html@2.14.0
  - @atlaspack/transformer-image@3.1.0
  - @atlaspack/transformer-js@3.1.0
  - @atlaspack/transformer-json@2.14.0
  - @atlaspack/transformer-postcss@2.14.0
  - @atlaspack/transformer-posthtml@2.14.0
  - @atlaspack/transformer-raw@2.14.0
  - @atlaspack/transformer-react-refresh-wrap@2.14.0
  - @atlaspack/transformer-svg@2.14.0
  - @atlaspack/bundler-default@2.14.0
  - @atlaspack/compressor-raw@2.13.2

## 3.0.1

### Patch Changes

- [`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06) Thanks [@yamadapc](https://github.com/yamadapc)! - Add identifier registry and VCS tracing

- Updated dependencies [[`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06)]:
  - @atlaspack/transformer-react-refresh-wrap@2.13.1
  - @atlaspack/runtime-service-worker@2.13.1
  - @atlaspack/runtime-react-refresh@2.13.1
  - @atlaspack/transformer-posthtml@2.13.1
  - @atlaspack/reporter-dev-server@2.13.1
  - @atlaspack/transformer-postcss@2.13.1
  - @atlaspack/optimizer-htmlnano@2.13.1
  - @atlaspack/transformer-image@3.0.1
  - @atlaspack/resolver-default@2.13.1
  - @atlaspack/transformer-html@2.13.1
  - @atlaspack/transformer-json@2.13.1
  - @atlaspack/bundler-default@2.13.1
  - @atlaspack/optimizer-image@3.0.1
  - @atlaspack/transformer-css@2.13.1
  - @atlaspack/transformer-raw@2.13.1
  - @atlaspack/transformer-svg@2.13.1
  - @atlaspack/compressor-raw@2.13.1
  - @atlaspack/optimizer-svgo@2.13.1
  - @atlaspack/transformer-js@3.0.1
  - @atlaspack/namer-default@2.13.1
  - @atlaspack/optimizer-css@2.13.1
  - @atlaspack/optimizer-swc@2.13.1
  - @atlaspack/packager-html@2.13.1
  - @atlaspack/packager-wasm@2.13.1
  - @atlaspack/packager-css@2.13.1
  - @atlaspack/packager-raw@2.13.1
  - @atlaspack/packager-svg@2.13.1
  - @atlaspack/packager-js@2.13.1
  - @atlaspack/runtime-browser-hmr@2.13.1
  - @atlaspack/runtime-js@2.13.1
  - @atlaspack/core@2.13.1

## 3.0.0

### Minor Changes

- [#335](https://github.com/atlassian-labs/atlaspack/pull/335) [`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf) Thanks [@yamadapc](https://github.com/yamadapc)! - Initial changeset release

### Patch Changes

- Updated dependencies [[`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf)]:
  - @atlaspack/runtime-js@2.13.0
  - @atlaspack/core@2.13.0
  - @atlaspack/bundler-default@2.13.0
  - @atlaspack/compressor-raw@2.13.0
  - @atlaspack/namer-default@2.13.0
  - @atlaspack/optimizer-css@2.13.0
  - @atlaspack/optimizer-htmlnano@2.13.0
  - @atlaspack/optimizer-image@3.0.0
  - @atlaspack/optimizer-svgo@2.13.0
  - @atlaspack/optimizer-swc@2.13.0
  - @atlaspack/packager-css@2.13.0
  - @atlaspack/packager-html@2.13.0
  - @atlaspack/packager-js@2.13.0
  - @atlaspack/packager-raw@2.13.0
  - @atlaspack/packager-svg@2.13.0
  - @atlaspack/packager-wasm@2.13.0
  - @atlaspack/reporter-dev-server@2.13.0
  - @atlaspack/resolver-default@2.13.0
  - @atlaspack/runtime-browser-hmr@2.13.0
  - @atlaspack/runtime-react-refresh@2.13.0
  - @atlaspack/runtime-service-worker@2.13.0
  - @atlaspack/transformer-css@2.13.0
  - @atlaspack/transformer-html@2.13.0
  - @atlaspack/transformer-image@3.0.0
  - @atlaspack/transformer-js@3.0.0
  - @atlaspack/transformer-json@2.13.0
  - @atlaspack/transformer-postcss@2.13.0
  - @atlaspack/transformer-posthtml@2.13.0
  - @atlaspack/transformer-raw@2.13.0
  - @atlaspack/transformer-react-refresh-wrap@2.13.0
  - @atlaspack/transformer-svg@2.13.0
