# @atlaspack/bundler-default

## 2.16.1

### Patch Changes

- Updated dependencies [[`30f6017`](https://github.com/atlassian-labs/atlaspack/commit/30f60175ba4d272c5fc193973c63bc298584775b), [`1ab0a27`](https://github.com/atlassian-labs/atlaspack/commit/1ab0a275aeca40350415e2b03e7440d1dddc6228), [`b8a4ae8`](https://github.com/atlassian-labs/atlaspack/commit/b8a4ae8f83dc0a83d8b145c5f729936ce52080a3)]:
  - @atlaspack/feature-flags@2.15.1
  - @atlaspack/rust@3.3.3
  - @atlaspack/graph@3.4.6
  - @atlaspack/utils@2.14.8
  - @atlaspack/plugin@2.14.8

## 2.16.0

### Minor Changes

- [#547](https://github.com/atlassian-labs/atlaspack/pull/547) [`a1773d2`](https://github.com/atlassian-labs/atlaspack/commit/a1773d2a62d0ef7805ac7524621dcabcc1afe929) Thanks [@benjervis](https://github.com/benjervis)! - Add a feature flag for resolving the configuration for `@atlaspack/bundler-default` from CWD, rather than exclusively from the project root.

### Patch Changes

- Updated dependencies [[`a1773d2`](https://github.com/atlassian-labs/atlaspack/commit/a1773d2a62d0ef7805ac7524621dcabcc1afe929), [`556d6ab`](https://github.com/atlassian-labs/atlaspack/commit/556d6ab8ede759fa7f37fcd3f4da336ef1c55e8f)]:
  - @atlaspack/feature-flags@2.15.0
  - @atlaspack/rust@3.3.2
  - @atlaspack/graph@3.4.5
  - @atlaspack/utils@2.14.7
  - @atlaspack/plugin@2.14.7

## 2.15.1

### Patch Changes

- Updated dependencies [[`e0f5337`](https://github.com/atlassian-labs/atlaspack/commit/e0f533757bd1019dbd108a04952c87da15286e09)]:
  - @atlaspack/feature-flags@2.14.4
  - @atlaspack/rust@3.3.1
  - @atlaspack/graph@3.4.4
  - @atlaspack/utils@2.14.6
  - @atlaspack/plugin@2.14.6

## 2.15.0

### Minor Changes

- [#535](https://github.com/atlassian-labs/atlaspack/pull/535) [`a4bc259`](https://github.com/atlassian-labs/atlaspack/commit/a4bc2590196b6c1e743e4edcb0337e8c4c240ab4) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add `sharedBundleMergeThreshold` config option

  In apps with lots of dynamic imports, many shared bundles are often removed
  from the output to prevent an overload in network requests according to the
  `maxParallelRequests` config. In these cases, setting `sharedBundleMergeThreshold` can
  merge shared bundles with a high overlap in their source bundles (bundles that share the bundle).
  This config trades-off potential overfetching to reduce asset duplication.

  The following config would merge shared bundles that have a 75% or higher overlap in source bundles.

  ```json
  {
    "@atlaspack/bundler-default": {
      "sharedBundleMergeThreshold": 0.75
    }
  }
  ```

### Patch Changes

- Updated dependencies [[`11d6f16`](https://github.com/atlassian-labs/atlaspack/commit/11d6f16b6397dee2f217167e5c98b39edb63f7a7), [`e2ba0f6`](https://github.com/atlassian-labs/atlaspack/commit/e2ba0f69702656f3d1ce95ab1454e35062b13b39), [`d2c50c2`](https://github.com/atlassian-labs/atlaspack/commit/d2c50c2c020888b33bb25b8690d9320c2b69e2a6), [`46a90dc`](https://github.com/atlassian-labs/atlaspack/commit/46a90dccd019a26b222c878a92d23acc75dc67c5)]:
  - @atlaspack/feature-flags@2.14.3
  - @atlaspack/rust@3.3.0
  - @atlaspack/graph@3.4.3
  - @atlaspack/utils@2.14.5
  - @atlaspack/plugin@2.14.5

## 2.14.4

### Patch Changes

- Updated dependencies [[`1a2c14c`](https://github.com/atlassian-labs/atlaspack/commit/1a2c14c3cd4587551cc12e94d0680c8b71ea12bf), [`cb9da16`](https://github.com/atlassian-labs/atlaspack/commit/cb9da16fb2648e7f53c64df0313f60d5fb8970cc)]:
  - @atlaspack/rust@3.2.0
  - @atlaspack/utils@2.14.4
  - @atlaspack/plugin@2.14.4

## 2.14.3

### Patch Changes

- Updated dependencies [[`f27d39e`](https://github.com/atlassian-labs/atlaspack/commit/f27d39e767b06def059944b3bc5fd50797eaea96)]:
  - @atlaspack/rust@3.1.1
  - @atlaspack/utils@2.14.3
  - @atlaspack/plugin@2.14.3

## 2.14.2

### Patch Changes

- Updated dependencies [[`9b85d3e`](https://github.com/atlassian-labs/atlaspack/commit/9b85d3e645b10bd027eed2304afc970a5ba40062), [`a891d65`](https://github.com/atlassian-labs/atlaspack/commit/a891d652bc4eb3d757d381adf65c5083f706effc), [`d02eab9`](https://github.com/atlassian-labs/atlaspack/commit/d02eab95eb60bf7457e0869af0b773608592c0e6), [`fb87a90`](https://github.com/atlassian-labs/atlaspack/commit/fb87a901973776b33ca4ce530e9d71669a9bd36d), [`7b9e8cf`](https://github.com/atlassian-labs/atlaspack/commit/7b9e8cf29e01a98e72e46b2b2fb74ccc514f4463), [`17b9579`](https://github.com/atlassian-labs/atlaspack/commit/17b9579484eced0ed8f23e2aba6d23b3c7238c39), [`8f4e6c1`](https://github.com/atlassian-labs/atlaspack/commit/8f4e6c1b0e7c1fd48624afda48c1dcc599f1460f)]:
  - @atlaspack/feature-flags@2.14.2
  - @atlaspack/rust@3.1.0
  - @atlaspack/graph@3.4.2
  - @atlaspack/utils@2.14.2
  - @atlaspack/plugin@2.14.2

## 2.14.1

### Patch Changes

- [#450](https://github.com/atlassian-labs/atlaspack/pull/450) [`b9d41b1`](https://github.com/atlassian-labs/atlaspack/commit/b9d41b175ad5771651a5b0278a5a0147e669234a) Thanks [@benjervis](https://github.com/benjervis)! - Remove the Atlaspack engines compatibility check

- [#478](https://github.com/atlassian-labs/atlaspack/pull/478) [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b) Thanks [@yamadapc](https://github.com/yamadapc)! - The first attempt at Version Packages didn't include the built artifacts.
  This has hopefully been fixed, so this change will force those packages to re-release.
- Updated dependencies [[`80bd57b`](https://github.com/atlassian-labs/atlaspack/commit/80bd57b9f9e966563957dee0780d956a682eb2d4), [`ae70b81`](https://github.com/atlassian-labs/atlaspack/commit/ae70b810384cf58f9c57d341ab4c925c7bb2060c), [`ce13d5e`](https://github.com/atlassian-labs/atlaspack/commit/ce13d5e885d55518ee6318e7a72e3a6e4e5126f2), [`4aab060`](https://github.com/atlassian-labs/atlaspack/commit/4aab0605c0d4ee8e0dcc3ffa1162eae5b360b677), [`c0a61a9`](https://github.com/atlassian-labs/atlaspack/commit/c0a61a92405b6830fe39cc17622cc2e97bf02dd7), [`cb35e7d`](https://github.com/atlassian-labs/atlaspack/commit/cb35e7d2b90b372de8401792915f12f410508d24), [`e1422ad`](https://github.com/atlassian-labs/atlaspack/commit/e1422ad0a801faaa4bc4f1023bed042ffe236e9b), [`6ec11f1`](https://github.com/atlassian-labs/atlaspack/commit/6ec11f10a9366fb8a9fc0475c7678235056bd80e), [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b)]:
  - @atlaspack/rust@3.0.1
  - @atlaspack/utils@2.14.1
  - @atlaspack/feature-flags@2.14.1
  - @atlaspack/diagnostic@2.14.1
  - @atlaspack/graph@3.4.1
  - @atlaspack/plugin@2.14.1

## 2.14.0

### Minor Changes

- [#405](https://github.com/atlassian-labs/atlaspack/pull/405) [`306246e`](https://github.com/atlassian-labs/atlaspack/commit/306246ee5a492583059b028ee5d0d1b49ce42223) Thanks [@benjervis](https://github.com/benjervis)! - Adds additional support for asset types when using the `singleFileOutput` option.

  When the single file output option was first defined, it was a very quick and naive
  implementation that only added JS assets to a single bundle, primarily to support
  SSR runtimes that only allow a single file.

  This falls apart when attempting to server render something like an SVG, because
  the existing implementation would ignore them entirely.

  This is fixed by adding support for "isolated" bundles, of which SVGs are one but
  not the only use case.
  The content itself is not included in the primary bundle, but the references between
  them (like an SVG URL) will now be inserted.

### Patch Changes

- [#379](https://github.com/atlassian-labs/atlaspack/pull/379) [`8bc3db9`](https://github.com/atlassian-labs/atlaspack/commit/8bc3db94cc7382b22ca8207c92af8f6389c17e2e) Thanks [@JakeLane](https://github.com/JakeLane)! - Support nested conditional imports in runtime for dynamic import edges in graph. Introduces a new feature flag `conditionalBundlingNestedRuntime`

- [#414](https://github.com/atlassian-labs/atlaspack/pull/414) [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53) Thanks [@alshdavid](https://github.com/alshdavid)! - Added type:commonjs to package.json files

- [#408](https://github.com/atlassian-labs/atlaspack/pull/408) [`f6afae7`](https://github.com/atlassian-labs/atlaspack/commit/f6afae7a168c85341f9f41aa70c2cd2491a9ff17) Thanks [@benjervis](https://github.com/benjervis)! - In the first attempt to support isolated bundles, there was a check on the number of assets that wasn't really correct.
  That check has been removed, so we can bundle even where there are special cases.

- [#381](https://github.com/atlassian-labs/atlaspack/pull/381) [`91ffa66`](https://github.com/atlassian-labs/atlaspack/commit/91ffa662ea3af48f1ca0c4f0d976db9c48995f4f) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Fix re-used bundles for async imports with circular dependencies

- [#411](https://github.com/atlassian-labs/atlaspack/pull/411) [`50265fd`](https://github.com/atlassian-labs/atlaspack/commit/50265fdf4024ec18439e85b472aa77a7952e2e08) Thanks [@benjervis](https://github.com/benjervis)! - Switches the isolated behaviour in single file output mode to be for inline bundles instead. Also makes isolated bundles an error.

- Updated dependencies [[`bfe81e5`](https://github.com/atlassian-labs/atlaspack/commit/bfe81e551c4e4bb2cac7fc4745222e66962c1728), [`fa4fcf6`](https://github.com/atlassian-labs/atlaspack/commit/fa4fcf69a82b0a3727066ada6e93a149b259936e), [`cd964ee`](https://github.com/atlassian-labs/atlaspack/commit/cd964eed5a330ae63733656ded691d1ea3afe4e3), [`1953d1b`](https://github.com/atlassian-labs/atlaspack/commit/1953d1bec266a39dc4bfce5f6c7959e77e63411e), [`ce4ce95`](https://github.com/atlassian-labs/atlaspack/commit/ce4ce953914e08991cf58c70c98f758690e5ee21), [`28dee1d`](https://github.com/atlassian-labs/atlaspack/commit/28dee1db7d9a995161b45f76c1a03b80ccaeab4b), [`a317453`](https://github.com/atlassian-labs/atlaspack/commit/a317453432b7f30e98f2a4cbcafdaa5601bcde63), [`2055adb`](https://github.com/atlassian-labs/atlaspack/commit/2055adbe31de792e2a2a591b94d2f33f50735879), [`8bc3db9`](https://github.com/atlassian-labs/atlaspack/commit/8bc3db94cc7382b22ca8207c92af8f6389c17e2e), [`cfa1c63`](https://github.com/atlassian-labs/atlaspack/commit/cfa1c63d710c5f9c9abc55f34220b70fb517c3b8), [`17427a2`](https://github.com/atlassian-labs/atlaspack/commit/17427a2b2fc9c34ef0b941907c2868edef6d1507), [`e962cd7`](https://github.com/atlassian-labs/atlaspack/commit/e962cd735877f7f16163e60868d70d9c10054ebe), [`104a46a`](https://github.com/atlassian-labs/atlaspack/commit/104a46a5ee1fae176d29fcc6420d6bd9c01b35b1), [`9572aca`](https://github.com/atlassian-labs/atlaspack/commit/9572aca2a2313a3c05551f73e556128e77a37732), [`34b740d`](https://github.com/atlassian-labs/atlaspack/commit/34b740d4e2449fba7b50cb9708c56d8033dca5b9), [`4837b69`](https://github.com/atlassian-labs/atlaspack/commit/4837b6988e56ca842a24797b796160964d3696ce), [`e5fa92d`](https://github.com/atlassian-labs/atlaspack/commit/e5fa92de26c87fb5d4d681af1931451749ba970a), [`7e21377`](https://github.com/atlassian-labs/atlaspack/commit/7e21377914e8091d484f67cb11052a1efd2227e3), [`43113f8`](https://github.com/atlassian-labs/atlaspack/commit/43113f8f00232c5a52169a3f11f846d6e4d94b0a), [`3650f7c`](https://github.com/atlassian-labs/atlaspack/commit/3650f7c9ab803b5ae20b223e82b2268a1b614e43), [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53), [`c2ef915`](https://github.com/atlassian-labs/atlaspack/commit/c2ef915dc54784ce4b8180025ac1b2e13b375002), [`f635123`](https://github.com/atlassian-labs/atlaspack/commit/f635123f9a06961bc5e053e237f1023f10800ea3), [`4812d0f`](https://github.com/atlassian-labs/atlaspack/commit/4812d0f7400af0f8416f1b7175ecb87700860a68), [`80d963e`](https://github.com/atlassian-labs/atlaspack/commit/80d963ed950f5d742ebd78014cf74f3c65cd4474), [`8fae5f3`](https://github.com/atlassian-labs/atlaspack/commit/8fae5f3005bd7c806b175b4df1754abf58922591), [`cc66aaa`](https://github.com/atlassian-labs/atlaspack/commit/cc66aaa66d67dd0cb89e083f387a278e74aad3f0), [`67df3f1`](https://github.com/atlassian-labs/atlaspack/commit/67df3f1af1432d77ee6b8850010d976d3313693a), [`0c3ad7a`](https://github.com/atlassian-labs/atlaspack/commit/0c3ad7a302330da1d5e3c025963cc583eb5c28ed)]:
  - @atlaspack/diagnostic@2.14.0
  - @atlaspack/feature-flags@2.14.0
  - @atlaspack/graph@3.4.0
  - @atlaspack/plugin@2.14.0
  - @atlaspack/rust@3.0.0
  - @atlaspack/utils@2.14.0

## 2.13.1

### Patch Changes

- [`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06) Thanks [@yamadapc](https://github.com/yamadapc)! - Add identifier registry and VCS tracing

- Updated dependencies [[`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06)]:
  - @atlaspack/feature-flags@2.13.1
  - @atlaspack/diagnostic@2.13.1
  - @atlaspack/plugin@2.13.1
  - @atlaspack/graph@3.3.1
  - @atlaspack/utils@2.13.1
  - @atlaspack/rust@2.13.1

## 2.13.0

### Minor Changes

- [#335](https://github.com/atlassian-labs/atlaspack/pull/335) [`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf) Thanks [@yamadapc](https://github.com/yamadapc)! - Initial changeset release

### Patch Changes

- Updated dependencies [[`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf)]:
  - @atlaspack/graph@3.3.0
  - @atlaspack/diagnostic@2.13.0
  - @atlaspack/feature-flags@2.13.0
  - @atlaspack/plugin@2.13.0
  - @atlaspack/rust@2.13.0
  - @atlaspack/utils@2.13.0
