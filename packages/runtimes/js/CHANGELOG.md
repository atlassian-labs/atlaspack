# @atlaspack/runtime-js

## 2.18.0

### Minor Changes

- [#732](https://github.com/atlassian-labs/atlaspack/pull/732) [`7f5841c`](https://github.com/atlassian-labs/atlaspack/commit/7f5841c39df049f9546cccbeea2a7337e0337b45) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - add tesseract context

### Patch Changes

- Updated dependencies [[`73dd7ba`](https://github.com/atlassian-labs/atlaspack/commit/73dd7baab69456ef2f6e4a0cc7dbb04f407eb148)]:
  - @atlaspack/feature-flags@2.22.0
  - @atlaspack/utils@2.18.1
  - @atlaspack/plugin@2.14.24

## 2.17.0

### Minor Changes

- [#731](https://github.com/atlassian-labs/atlaspack/pull/731) [`23d561e`](https://github.com/atlassian-labs/atlaspack/commit/23d561e51e68b0c38fd1ff4e4fb173e5e7b01cf2) Thanks [@marcins](https://github.com/marcins)! - Implement "inline isolated" scripts

### Patch Changes

- Updated dependencies [[`23d561e`](https://github.com/atlassian-labs/atlaspack/commit/23d561e51e68b0c38fd1ff4e4fb173e5e7b01cf2)]:
  - @atlaspack/feature-flags@2.21.0
  - @atlaspack/utils@2.18.0
  - @atlaspack/plugin@2.14.23

## 2.16.1

### Patch Changes

- [#720](https://github.com/atlassian-labs/atlaspack/pull/720) [`d2fd849`](https://github.com/atlassian-labs/atlaspack/commit/d2fd849770fe6305e9c694bd97b1bd905abd9d94) Thanks [@alshdavid](https://github.com/alshdavid)! - Migrate to TypeScript

- Updated dependencies [[`d2fd849`](https://github.com/atlassian-labs/atlaspack/commit/d2fd849770fe6305e9c694bd97b1bd905abd9d94), [`12bee0e`](https://github.com/atlassian-labs/atlaspack/commit/12bee0e23f0464d7f6bd3e24fbe0d19c126d587d)]:
  - @atlaspack/domain-sharding@2.14.2
  - @atlaspack/feature-flags@2.20.1
  - @atlaspack/diagnostic@2.14.2
  - @atlaspack/plugin@2.14.22
  - @atlaspack/utils@2.17.4

## 2.16.0

### Minor Changes

- [#721](https://github.com/atlassian-labs/atlaspack/pull/721) [`069de47`](https://github.com/atlassian-labs/atlaspack/commit/069de478e64fb5889f6f2ce023eb510782767fbd) Thanks [@benjervis](https://github.com/benjervis)! - Add support for bundle merging based on `webpackChunkName` comments.

  Adding a `webpackChunkName` comment to an import will allow the bundler to merge multiple imports into a single bundle.

  e.g.:

  ```ts
  import(/* webpackChunkName: "my-chunk" */ './my-module');
  import(/* webpackChunkName: "my-chunk" */ './another-module');
  ```

  This can be enabled with the feature flag `supportWebpackChunkName`.

### Patch Changes

- Updated dependencies [[`069de47`](https://github.com/atlassian-labs/atlaspack/commit/069de478e64fb5889f6f2ce023eb510782767fbd)]:
  - @atlaspack/feature-flags@2.20.0
  - @atlaspack/utils@2.17.3
  - @atlaspack/plugin@2.14.21

## 2.15.2

### Patch Changes

- [#702](https://github.com/atlassian-labs/atlaspack/pull/702) [`daaa768`](https://github.com/atlassian-labs/atlaspack/commit/daaa7688786772d7e3713b71c5bba6b89ec704aa) Thanks [@alshdavid](https://github.com/alshdavid)! - Fixes to Flow types

- Updated dependencies [[`daaa768`](https://github.com/atlassian-labs/atlaspack/commit/daaa7688786772d7e3713b71c5bba6b89ec704aa), [`1c7865a`](https://github.com/atlassian-labs/atlaspack/commit/1c7865a64451116d94015e248302435839d347c0), [`a0b959f`](https://github.com/atlassian-labs/atlaspack/commit/a0b959fbf61fc3f820ff03c7e8988945fe40a91a)]:
  - @atlaspack/plugin@2.14.20
  - @atlaspack/feature-flags@2.19.2
  - @atlaspack/utils@2.17.2

## 2.15.1

### Patch Changes

- [#692](https://github.com/atlassian-labs/atlaspack/pull/692) [`13aef17`](https://github.com/atlassian-labs/atlaspack/commit/13aef177eea289a6e40d2113b5ec1ac9be18a33d) Thanks [@JakeLane](https://github.com/JakeLane)! - Add fallback behaviour when conditional bundle is missing

- Updated dependencies [[`13aef17`](https://github.com/atlassian-labs/atlaspack/commit/13aef177eea289a6e40d2113b5ec1ac9be18a33d)]:
  - @atlaspack/feature-flags@2.19.1
  - @atlaspack/utils@2.17.1
  - @atlaspack/plugin@2.14.19

## 2.15.0

### Minor Changes

- [#640](https://github.com/atlassian-labs/atlaspack/pull/640) [`dbb4072`](https://github.com/atlassian-labs/atlaspack/commit/dbb40721ebeb45990a14ba04e6b44e7f836fb32d) Thanks [@JakeLane](https://github.com/JakeLane)! - Clean up conditional bundling feature flags

### Patch Changes

- Updated dependencies [[`dbb4072`](https://github.com/atlassian-labs/atlaspack/commit/dbb40721ebeb45990a14ba04e6b44e7f836fb32d), [`becf977`](https://github.com/atlassian-labs/atlaspack/commit/becf977f625d5ee46dae3d4c679f173bf5f40cc0), [`c4415a4`](https://github.com/atlassian-labs/atlaspack/commit/c4415a455543d984ca28452c2cb87a794d22497c), [`f0f7c71`](https://github.com/atlassian-labs/atlaspack/commit/f0f7c7168a1d3d18c6f30d2daed611275692b7c5), [`de23e0c`](https://github.com/atlassian-labs/atlaspack/commit/de23e0ce49d5504fe3947ac26640a3d951087da3), [`18a57cf`](https://github.com/atlassian-labs/atlaspack/commit/18a57cf8a4789b2de5ad8e2676f317a26cc91417), [`a5ed1b4`](https://github.com/atlassian-labs/atlaspack/commit/a5ed1b414498560f393ff491af4da25b6e8dde56)]:
  - @atlaspack/feature-flags@2.19.0
  - @atlaspack/utils@2.17.0
  - @atlaspack/plugin@2.14.18

## 2.14.17

### Patch Changes

- Updated dependencies []:
  - @atlaspack/utils@2.16.1
  - @atlaspack/plugin@2.14.17

## 2.14.16

### Patch Changes

- Updated dependencies [[`e8a60ff`](https://github.com/atlassian-labs/atlaspack/commit/e8a60ffbea41caef265786bbf73349771760081c), [`30ee2cf`](https://github.com/atlassian-labs/atlaspack/commit/30ee2cfcd34cf2646ded0eda13fdb80a2a5de529)]:
  - @atlaspack/feature-flags@2.18.4
  - @atlaspack/utils@2.16.0
  - @atlaspack/plugin@2.14.16

## 2.14.15

### Patch Changes

- Updated dependencies [[`5ded263`](https://github.com/atlassian-labs/atlaspack/commit/5ded263c7f11b866e8885b81c73e20dd060b25be)]:
  - @atlaspack/feature-flags@2.18.3
  - @atlaspack/utils@2.15.3
  - @atlaspack/plugin@2.14.15

## 2.14.14

### Patch Changes

- Updated dependencies [[`644b157`](https://github.com/atlassian-labs/atlaspack/commit/644b157dee72a871acc2d0facf0b87b8eea51956)]:
  - @atlaspack/feature-flags@2.18.2
  - @atlaspack/utils@2.15.2
  - @atlaspack/plugin@2.14.14

## 2.14.13

### Patch Changes

- [#633](https://github.com/atlassian-labs/atlaspack/pull/633) [`26aa9c5`](https://github.com/atlassian-labs/atlaspack/commit/26aa9c599d2be45ce1438a74c5fa22f39b9b554b) Thanks [@sbhuiyan-atlassian](https://github.com/sbhuiyan-atlassian)! - Ported various HMR changes from Parcel

- Updated dependencies [[`26aa9c5`](https://github.com/atlassian-labs/atlaspack/commit/26aa9c599d2be45ce1438a74c5fa22f39b9b554b), [`0501255`](https://github.com/atlassian-labs/atlaspack/commit/05012550da35b05ce7d356a8cc29311e7f9afdca)]:
  - @atlaspack/feature-flags@2.18.1
  - @atlaspack/utils@2.15.1
  - @atlaspack/plugin@2.14.13

## 2.14.12

### Patch Changes

- Updated dependencies [[`10fbcfb`](https://github.com/atlassian-labs/atlaspack/commit/10fbcfbfa49c7a83da5d7c40983e36e87f524a75), [`85c52d3`](https://github.com/atlassian-labs/atlaspack/commit/85c52d3f7717b3c84a118d18ab98cfbfd71dcbd2), [`e39c6cf`](https://github.com/atlassian-labs/atlaspack/commit/e39c6cf05f7e95ce5420dbcea66f401b1cbd397c)]:
  - @atlaspack/feature-flags@2.18.0
  - @atlaspack/utils@2.15.0
  - @atlaspack/plugin@2.14.12

## 2.14.11

### Patch Changes

- Updated dependencies [[`73ea3c4`](https://github.com/atlassian-labs/atlaspack/commit/73ea3c4d85d4401fdd15abcbf988237e890e7ad3), [`b1b3693`](https://github.com/atlassian-labs/atlaspack/commit/b1b369317c66f8a431c170df2ebba4fa5b2e38ef)]:
  - @atlaspack/feature-flags@2.17.0
  - @atlaspack/utils@2.14.11
  - @atlaspack/plugin@2.14.11

## 2.14.10

### Patch Changes

- Updated dependencies []:
  - @atlaspack/plugin@2.14.10
  - @atlaspack/utils@2.14.10

## 2.14.9

### Patch Changes

- Updated dependencies [[`209692f`](https://github.com/atlassian-labs/atlaspack/commit/209692ffb11eae103a0d65c5e1118a5aa1625818), [`f4da1e1`](https://github.com/atlassian-labs/atlaspack/commit/f4da1e120e73eeb5e8b8927f05e88f04d6148c7b)]:
  - @atlaspack/feature-flags@2.16.0
  - @atlaspack/utils@2.14.9
  - @atlaspack/plugin@2.14.9

## 2.14.8

### Patch Changes

- Updated dependencies [[`30f6017`](https://github.com/atlassian-labs/atlaspack/commit/30f60175ba4d272c5fc193973c63bc298584775b)]:
  - @atlaspack/feature-flags@2.15.1
  - @atlaspack/utils@2.14.8
  - @atlaspack/plugin@2.14.8

## 2.14.7

### Patch Changes

- Updated dependencies [[`a1773d2`](https://github.com/atlassian-labs/atlaspack/commit/a1773d2a62d0ef7805ac7524621dcabcc1afe929)]:
  - @atlaspack/feature-flags@2.15.0
  - @atlaspack/utils@2.14.7
  - @atlaspack/plugin@2.14.7

## 2.14.6

### Patch Changes

- Updated dependencies [[`e0f5337`](https://github.com/atlassian-labs/atlaspack/commit/e0f533757bd1019dbd108a04952c87da15286e09)]:
  - @atlaspack/feature-flags@2.14.4
  - @atlaspack/utils@2.14.6
  - @atlaspack/plugin@2.14.6

## 2.14.5

### Patch Changes

- Updated dependencies [[`11d6f16`](https://github.com/atlassian-labs/atlaspack/commit/11d6f16b6397dee2f217167e5c98b39edb63f7a7)]:
  - @atlaspack/feature-flags@2.14.3
  - @atlaspack/utils@2.14.5
  - @atlaspack/plugin@2.14.5

## 2.14.4

### Patch Changes

- Updated dependencies []:
  - @atlaspack/utils@2.14.4
  - @atlaspack/plugin@2.14.4

## 2.14.3

### Patch Changes

- Updated dependencies []:
  - @atlaspack/utils@2.14.3
  - @atlaspack/plugin@2.14.3

## 2.14.2

### Patch Changes

- Updated dependencies [[`9b85d3e`](https://github.com/atlassian-labs/atlaspack/commit/9b85d3e645b10bd027eed2304afc970a5ba40062), [`17b9579`](https://github.com/atlassian-labs/atlaspack/commit/17b9579484eced0ed8f23e2aba6d23b3c7238c39), [`8f4e6c1`](https://github.com/atlassian-labs/atlaspack/commit/8f4e6c1b0e7c1fd48624afda48c1dcc599f1460f)]:
  - @atlaspack/feature-flags@2.14.2
  - @atlaspack/utils@2.14.2
  - @atlaspack/plugin@2.14.2

## 2.14.1

### Patch Changes

- [#450](https://github.com/atlassian-labs/atlaspack/pull/450) [`b9d41b1`](https://github.com/atlassian-labs/atlaspack/commit/b9d41b175ad5771651a5b0278a5a0147e669234a) Thanks [@benjervis](https://github.com/benjervis)! - Remove the Atlaspack engines compatibility check

- [#478](https://github.com/atlassian-labs/atlaspack/pull/478) [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b) Thanks [@yamadapc](https://github.com/yamadapc)! - The first attempt at Version Packages didn't include the built artifacts.
  This has hopefully been fixed, so this change will force those packages to re-release.
- Updated dependencies [[`b9d41b1`](https://github.com/atlassian-labs/atlaspack/commit/b9d41b175ad5771651a5b0278a5a0147e669234a), [`ce13d5e`](https://github.com/atlassian-labs/atlaspack/commit/ce13d5e885d55518ee6318e7a72e3a6e4e5126f2), [`4aab060`](https://github.com/atlassian-labs/atlaspack/commit/4aab0605c0d4ee8e0dcc3ffa1162eae5b360b677), [`e1422ad`](https://github.com/atlassian-labs/atlaspack/commit/e1422ad0a801faaa4bc4f1023bed042ffe236e9b), [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b)]:
  - @atlaspack/domain-sharding@2.14.1
  - @atlaspack/utils@2.14.1
  - @atlaspack/feature-flags@2.14.1
  - @atlaspack/diagnostic@2.14.1
  - @atlaspack/plugin@2.14.1

## 2.14.0

### Minor Changes

- [#339](https://github.com/atlassian-labs/atlaspack/pull/339) [`bfe81e5`](https://github.com/atlassian-labs/atlaspack/commit/bfe81e551c4e4bb2cac7fc4745222e66962c1728) Thanks [@yamadapc](https://github.com/yamadapc)! - Update cache invalidation metrics with build type

### Patch Changes

- [#379](https://github.com/atlassian-labs/atlaspack/pull/379) [`8bc3db9`](https://github.com/atlassian-labs/atlaspack/commit/8bc3db94cc7382b22ca8207c92af8f6389c17e2e) Thanks [@JakeLane](https://github.com/JakeLane)! - Support nested conditional imports in runtime for dynamic import edges in graph. Introduces a new feature flag `conditionalBundlingNestedRuntime`

- [#414](https://github.com/atlassian-labs/atlaspack/pull/414) [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53) Thanks [@alshdavid](https://github.com/alshdavid)! - Added type:commonjs to package.json files

- [#412](https://github.com/atlassian-labs/atlaspack/pull/412) [`be63a51`](https://github.com/atlassian-labs/atlaspack/commit/be63a515ad13dd5ec1e241843d9ef6fdae8699d5) Thanks [@yamadapc](https://github.com/yamadapc)! - Update to internal unit tests

- Updated dependencies [[`bfe81e5`](https://github.com/atlassian-labs/atlaspack/commit/bfe81e551c4e4bb2cac7fc4745222e66962c1728), [`1953d1b`](https://github.com/atlassian-labs/atlaspack/commit/1953d1bec266a39dc4bfce5f6c7959e77e63411e), [`ce4ce95`](https://github.com/atlassian-labs/atlaspack/commit/ce4ce953914e08991cf58c70c98f758690e5ee21), [`8bc3db9`](https://github.com/atlassian-labs/atlaspack/commit/8bc3db94cc7382b22ca8207c92af8f6389c17e2e), [`e962cd7`](https://github.com/atlassian-labs/atlaspack/commit/e962cd735877f7f16163e60868d70d9c10054ebe), [`4837b69`](https://github.com/atlassian-labs/atlaspack/commit/4837b6988e56ca842a24797b796160964d3696ce), [`7e21377`](https://github.com/atlassian-labs/atlaspack/commit/7e21377914e8091d484f67cb11052a1efd2227e3), [`43113f8`](https://github.com/atlassian-labs/atlaspack/commit/43113f8f00232c5a52169a3f11f846d6e4d94b0a), [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53)]:
  - @atlaspack/diagnostic@2.14.0
  - @atlaspack/feature-flags@2.14.0
  - @atlaspack/plugin@2.14.0
  - @atlaspack/utils@2.14.0
  - @atlaspack/domain-sharding@2.14.0

## 2.13.1

### Patch Changes

- [`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06) Thanks [@yamadapc](https://github.com/yamadapc)! - Add identifier registry and VCS tracing

- Updated dependencies [[`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06)]:
  - @atlaspack/domain-sharding@2.13.1
  - @atlaspack/feature-flags@2.13.1
  - @atlaspack/diagnostic@2.13.1
  - @atlaspack/plugin@2.13.1
  - @atlaspack/utils@2.13.1

## 2.13.0

### Minor Changes

- [#335](https://github.com/atlassian-labs/atlaspack/pull/335) [`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf) Thanks [@yamadapc](https://github.com/yamadapc)! - Initial changeset release

### Patch Changes

- Updated dependencies [[`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf)]:
  - @atlaspack/domain-sharding@2.13.0
  - @atlaspack/diagnostic@2.13.0
  - @atlaspack/feature-flags@2.13.0
  - @atlaspack/plugin@2.13.0
  - @atlaspack/utils@2.13.0
