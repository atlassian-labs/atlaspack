# @atlaspack/feature-flags

## 2.18.4

### Patch Changes

- [#661](https://github.com/atlassian-labs/atlaspack/pull/661) [`e8a60ff`](https://github.com/atlassian-labs/atlaspack/commit/e8a60ffbea41caef265786bbf73349771760081c) Thanks [@marcins](https://github.com/marcins)! - Add new feature flag atlaspackV3CleanShutdown which will dispose of the NAPI worker pool when disposing of the Atlaspack class

## 2.18.3

### Patch Changes

- [#655](https://github.com/atlassian-labs/atlaspack/pull/655) [`5ded263`](https://github.com/atlassian-labs/atlaspack/commit/5ded263c7f11b866e8885b81c73e20dd060b25be) Thanks [@yamadapc](https://github.com/yamadapc)! - Clean-up inline requires multi-threading feature-flag

## 2.18.2

### Patch Changes

- [#652](https://github.com/atlassian-labs/atlaspack/pull/652) [`644b157`](https://github.com/atlassian-labs/atlaspack/commit/644b157dee72a871acc2d0facf0b87b8eea51956) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix bugs related to build aborts. Builds and cache writes will no longer be aborted.

## 2.18.1

### Patch Changes

- [#633](https://github.com/atlassian-labs/atlaspack/pull/633) [`26aa9c5`](https://github.com/atlassian-labs/atlaspack/commit/26aa9c599d2be45ce1438a74c5fa22f39b9b554b) Thanks [@sbhuiyan-atlassian](https://github.com/sbhuiyan-atlassian)! - Ported various HMR changes from Parcel

- [#626](https://github.com/atlassian-labs/atlaspack/pull/626) [`0501255`](https://github.com/atlassian-labs/atlaspack/commit/05012550da35b05ce7d356a8cc29311e7f9afdca) Thanks [@yamadapc](https://github.com/yamadapc)! - Clean-up tsconfig invalidation improvements feature-flag

## 2.18.0

### Minor Changes

- [#627](https://github.com/atlassian-labs/atlaspack/pull/627) [`85c52d3`](https://github.com/atlassian-labs/atlaspack/commit/85c52d3f7717b3c84a118d18ab98cfbfd71dcbd2) Thanks [@benjervis](https://github.com/benjervis)! - Adds a feature flag for `applyScopeHoistingImprovement`

### Patch Changes

- [#632](https://github.com/atlassian-labs/atlaspack/pull/632) [`10fbcfb`](https://github.com/atlassian-labs/atlaspack/commit/10fbcfbfa49c7a83da5d7c40983e36e87f524a75) Thanks [@marcins](https://github.com/marcins)! - Added a new feature flag `inlineConstOptimisationFix` which when enabled changes the behaviour for output of constant modules. This fixes two issues with constant modules:

  - Previously constant modules, if they needed a namespace anywhere, would have a namespace everywhere, with this change they only have a namespace in the bundles where needed.
  - Previously in the case of wrapped assets, a constant module dependnecy of that wrapped asset would be rendered after the module - which meant the minifier would not be able to inline the constants safely. With this flag all constant modules are rendered at the top of the bundle.

## 2.17.0

### Minor Changes

- [#619](https://github.com/atlassian-labs/atlaspack/pull/619) [`73ea3c4`](https://github.com/atlassian-labs/atlaspack/commit/73ea3c4d85d4401fdd15abcbf988237e890e7ad3) Thanks [@matt-koko](https://github.com/matt-koko)! - export `CONSISTENCY_CHECK_VALUES` for consumption in other products

### Patch Changes

- [#623](https://github.com/atlassian-labs/atlaspack/pull/623) [`b1b3693`](https://github.com/atlassian-labs/atlaspack/commit/b1b369317c66f8a431c170df2ebba4fa5b2e38ef) Thanks [@JakeLane](https://github.com/JakeLane)! - Load same conditional bundles as conditional manifest in HTML

## 2.16.0

### Minor Changes

- [#582](https://github.com/atlassian-labs/atlaspack/pull/582) [`f4da1e1`](https://github.com/atlassian-labs/atlaspack/commit/f4da1e120e73eeb5e8b8927f05e88f04d6148c7b) Thanks [@matt-koko](https://github.com/matt-koko)! - Export DEFAULT_FEATURE_FLAGS so it will be included in the associate type declaration file and able to be imported elsewhere.

  This will enable patterns like:

  ```
  import type { FeatureFlags } from '@atlaspack/feature-flags';
  import { DEFAULT_FEATURE_FLAGS } from '@atlaspack/feature-flags';
  ```

### Patch Changes

- [#503](https://github.com/atlassian-labs/atlaspack/pull/503) [`209692f`](https://github.com/atlassian-labs/atlaspack/commit/209692ffb11eae103a0d65c5e1118a5aa1625818) Thanks [@JakeLane](https://github.com/JakeLane)! - Fix conditional bundling reporter when condition is reused

## 2.15.1

### Patch Changes

- [#551](https://github.com/atlassian-labs/atlaspack/pull/551) [`30f6017`](https://github.com/atlassian-labs/atlaspack/commit/30f60175ba4d272c5fc193973c63bc298584775b) Thanks [@yamadapc](https://github.com/yamadapc)! - Log request tracker invalidation counts on start-up

## 2.15.0

### Minor Changes

- [#547](https://github.com/atlassian-labs/atlaspack/pull/547) [`a1773d2`](https://github.com/atlassian-labs/atlaspack/commit/a1773d2a62d0ef7805ac7524621dcabcc1afe929) Thanks [@benjervis](https://github.com/benjervis)! - Add a feature flag for resolving the configuration for `@atlaspack/bundler-default` from CWD, rather than exclusively from the project root.

## 2.14.4

### Patch Changes

- [#542](https://github.com/atlassian-labs/atlaspack/pull/542) [`e0f5337`](https://github.com/atlassian-labs/atlaspack/commit/e0f533757bd1019dbd108a04952c87da15286e09) Thanks [@yamadapc](https://github.com/yamadapc)! - Add feature-flagged option to use rayon thread-pool to optimize inline requires

## 2.14.3

### Patch Changes

- [#511](https://github.com/atlassian-labs/atlaspack/pull/511) [`11d6f16`](https://github.com/atlassian-labs/atlaspack/commit/11d6f16b6397dee2f217167e5c98b39edb63f7a7) Thanks [@yamadapc](https://github.com/yamadapc)! - Clean-up dylib worker threads segmentation fault bug fix feature-flag

## 2.14.2

### Patch Changes

- [#494](https://github.com/atlassian-labs/atlaspack/pull/494) [`9b85d3e`](https://github.com/atlassian-labs/atlaspack/commit/9b85d3e645b10bd027eed2304afc970a5ba40062) Thanks [@JakeLane](https://github.com/JakeLane)! - When conditionalBundlingReporterDuplicateFix is enabled, avoid duplicated writes to the descriptor and logging

- [#510](https://github.com/atlassian-labs/atlaspack/pull/510) [`17b9579`](https://github.com/atlassian-labs/atlaspack/commit/17b9579484eced0ed8f23e2aba6d23b3c7238c39) Thanks [@yamadapc](https://github.com/yamadapc)! - Add unused feature-flag for cache rework changes

- [#512](https://github.com/atlassian-labs/atlaspack/pull/512) [`8f4e6c1`](https://github.com/atlassian-labs/atlaspack/commit/8f4e6c1b0e7c1fd48624afda48c1dcc599f1460f) Thanks [@yamadapc](https://github.com/yamadapc)! - Remove LMDB cache back-end

## 2.14.1

### Patch Changes

- [#388](https://github.com/atlassian-labs/atlaspack/pull/388) [`4aab060`](https://github.com/atlassian-labs/atlaspack/commit/4aab0605c0d4ee8e0dcc3ffa1162eae5b360b677) Thanks [@yamadapc](https://github.com/yamadapc)! - Set LMDB.js Lite as the default cache back-end

- [#420](https://github.com/atlassian-labs/atlaspack/pull/420) [`e1422ad`](https://github.com/atlassian-labs/atlaspack/commit/e1422ad0a801faaa4bc4f1023bed042ffe236e9b) Thanks [@JakeLane](https://github.com/JakeLane)! - Support async script runtime in conditional bundling

- [#478](https://github.com/atlassian-labs/atlaspack/pull/478) [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b) Thanks [@yamadapc](https://github.com/yamadapc)! - The first attempt at Version Packages didn't include the built artifacts.
  This has hopefully been fixed, so this change will force those packages to re-release.

## 2.14.0

### Minor Changes

- [#339](https://github.com/atlassian-labs/atlaspack/pull/339) [`bfe81e5`](https://github.com/atlassian-labs/atlaspack/commit/bfe81e551c4e4bb2cac7fc4745222e66962c1728) Thanks [@yamadapc](https://github.com/yamadapc)! - Update cache invalidation metrics with build type

- [#352](https://github.com/atlassian-labs/atlaspack/pull/352) [`e962cd7`](https://github.com/atlassian-labs/atlaspack/commit/e962cd735877f7f16163e60868d70d9c10054ebe) Thanks [@pancaspe87](https://github.com/pancaspe87)! - Introduced new method to return feature flag value

- [#358](https://github.com/atlassian-labs/atlaspack/pull/358) [`7e21377`](https://github.com/atlassian-labs/atlaspack/commit/7e21377914e8091d484f67cb11052a1efd2227e3) Thanks [@benjervis](https://github.com/benjervis)! - Add a new feature flag to enable the side effect scanning

### Patch Changes

- [#413](https://github.com/atlassian-labs/atlaspack/pull/413) [`1953d1b`](https://github.com/atlassian-labs/atlaspack/commit/1953d1bec266a39dc4bfce5f6c7959e77e63411e) Thanks [@pancaspe87](https://github.com/pancaspe87)! - Patch absolute paths

- [#378](https://github.com/atlassian-labs/atlaspack/pull/378) [`ce4ce95`](https://github.com/atlassian-labs/atlaspack/commit/ce4ce953914e08991cf58c70c98f758690e5ee21) Thanks [@yamadapc](https://github.com/yamadapc)! - Add feature-flagged optimization to resolver specifier handling

- [#379](https://github.com/atlassian-labs/atlaspack/pull/379) [`8bc3db9`](https://github.com/atlassian-labs/atlaspack/commit/8bc3db94cc7382b22ca8207c92af8f6389c17e2e) Thanks [@JakeLane](https://github.com/JakeLane)! - Support nested conditional imports in runtime for dynamic import edges in graph. Introduces a new feature flag `conditionalBundlingNestedRuntime`

- [#429](https://github.com/atlassian-labs/atlaspack/pull/429) [`4837b69`](https://github.com/atlassian-labs/atlaspack/commit/4837b6988e56ca842a24797b796160964d3696ce) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Increase performance of inline bundle packaging (enabled via `featureFlags.inlineStringReplacementPerf`)

- [#415](https://github.com/atlassian-labs/atlaspack/pull/415) [`43113f8`](https://github.com/atlassian-labs/atlaspack/commit/43113f8f00232c5a52169a3f11f846d6e4d94b0a) Thanks [@yamadapc](https://github.com/yamadapc)! - Add feature-flag to fix supporting source-maps for inline bundles

- [#414](https://github.com/atlassian-labs/atlaspack/pull/414) [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53) Thanks [@alshdavid](https://github.com/alshdavid)! - Added type:commonjs to package.json files

## 2.13.1

### Patch Changes

- [`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06) Thanks [@yamadapc](https://github.com/yamadapc)! - Add identifier registry and VCS tracing

## 2.13.0

### Minor Changes

- [#335](https://github.com/atlassian-labs/atlaspack/pull/335) [`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf) Thanks [@yamadapc](https://github.com/yamadapc)! - Initial changeset release
