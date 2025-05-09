# @atlaspack/core

## 2.15.0

### Minor Changes

- [#486](https://github.com/atlassian-labs/atlaspack/pull/486) [`87087f4`](https://github.com/atlassian-labs/atlaspack/commit/87087f44f348ac583a27ea0819122e191ba80f8d) Thanks [@yamadapc](https://github.com/yamadapc)! - Add environment variable to skip cache invalidation

### Patch Changes

- [#450](https://github.com/atlassian-labs/atlaspack/pull/450) [`b9d41b1`](https://github.com/atlassian-labs/atlaspack/commit/b9d41b175ad5771651a5b0278a5a0147e669234a) Thanks [@benjervis](https://github.com/benjervis)! - Remove the Atlaspack engines compatibility check

- [#420](https://github.com/atlassian-labs/atlaspack/pull/420) [`e1422ad`](https://github.com/atlassian-labs/atlaspack/commit/e1422ad0a801faaa4bc4f1023bed042ffe236e9b) Thanks [@JakeLane](https://github.com/JakeLane)! - Support async script runtime in conditional bundling

- [#472](https://github.com/atlassian-labs/atlaspack/pull/472) [`7e357fb`](https://github.com/atlassian-labs/atlaspack/commit/7e357fb173e7958da330e3721667fa5749420952) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix segmentation fault on exit on certain cases

- [#478](https://github.com/atlassian-labs/atlaspack/pull/478) [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b) Thanks [@yamadapc](https://github.com/yamadapc)! - The first attempt at Version Packages didn't include the built artifacts.
  This has hopefully been fixed, so this change will force those packages to re-release.
- Updated dependencies [[`80bd57b`](https://github.com/atlassian-labs/atlaspack/commit/80bd57b9f9e966563957dee0780d956a682eb2d4), [`ae70b81`](https://github.com/atlassian-labs/atlaspack/commit/ae70b810384cf58f9c57d341ab4c925c7bb2060c), [`ce13d5e`](https://github.com/atlassian-labs/atlaspack/commit/ce13d5e885d55518ee6318e7a72e3a6e4e5126f2), [`4aab060`](https://github.com/atlassian-labs/atlaspack/commit/4aab0605c0d4ee8e0dcc3ffa1162eae5b360b677), [`c0a61a9`](https://github.com/atlassian-labs/atlaspack/commit/c0a61a92405b6830fe39cc17622cc2e97bf02dd7), [`cb35e7d`](https://github.com/atlassian-labs/atlaspack/commit/cb35e7d2b90b372de8401792915f12f410508d24), [`e1422ad`](https://github.com/atlassian-labs/atlaspack/commit/e1422ad0a801faaa4bc4f1023bed042ffe236e9b), [`6ec11f1`](https://github.com/atlassian-labs/atlaspack/commit/6ec11f10a9366fb8a9fc0475c7678235056bd80e), [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b)]:
  - @atlaspack/fs@2.14.1
  - @atlaspack/rust@3.0.1
  - @atlaspack/utils@2.14.1
  - @atlaspack/feature-flags@2.14.1
  - @atlaspack/diagnostic@2.14.1
  - @atlaspack/graph@3.4.1
  - @atlaspack/logger@2.14.1
  - @atlaspack/package-manager@2.14.1
  - @atlaspack/plugin@2.14.1
  - @atlaspack/profiler@2.14.1
  - @atlaspack/types@2.14.1
  - @atlaspack/workers@2.14.1
  - @atlaspack/events@2.14.1
  - @atlaspack/build-cache@2.13.3
  - @atlaspack/cache@2.13.3

## 2.14.0

### Minor Changes

- [#339](https://github.com/atlassian-labs/atlaspack/pull/339) [`bfe81e5`](https://github.com/atlassian-labs/atlaspack/commit/bfe81e551c4e4bb2cac7fc4745222e66962c1728) Thanks [@yamadapc](https://github.com/yamadapc)! - Update cache invalidation metrics with build type

- [#383](https://github.com/atlassian-labs/atlaspack/pull/383) [`8386ca4`](https://github.com/atlassian-labs/atlaspack/commit/8386ca4dc318688fbed1af3bbebf2af3e7d24552) Thanks [@benjervis](https://github.com/benjervis)! - `loadPlugin` no longer returns a `range` field. This field was only set by looking up a package's `parcelDependencies`, which no longer exist.

- [#358](https://github.com/atlassian-labs/atlaspack/pull/358) [`7e21377`](https://github.com/atlassian-labs/atlaspack/commit/7e21377914e8091d484f67cb11052a1efd2227e3) Thanks [@benjervis](https://github.com/benjervis)! - Add a step to the BundleGraphRequest that will scan for assets that have a transitive dep on `@confluence/loadable` and marks them as having side effects.
  This allows the inline requires optimizer to be applied to projects that don't necessarily declare side effects correctly.

- [#366](https://github.com/atlassian-labs/atlaspack/pull/366) [`4812d0f`](https://github.com/atlassian-labs/atlaspack/commit/4812d0f7400af0f8416f1b7175ecb87700860a68) Thanks [@alshdavid](https://github.com/alshdavid)! - Added NapiWorkerPool

### Patch Changes

- [#401](https://github.com/atlassian-labs/atlaspack/pull/401) [`cd964ee`](https://github.com/atlassian-labs/atlaspack/commit/cd964eed5a330ae63733656ded691d1ea3afe4e3) Thanks [@MonicaOlejniczak](https://github.com/MonicaOlejniczak)! - Fix respondToFsEvents return type

- [#413](https://github.com/atlassian-labs/atlaspack/pull/413) [`1953d1b`](https://github.com/atlassian-labs/atlaspack/commit/1953d1bec266a39dc4bfce5f6c7959e77e63411e) Thanks [@pancaspe87](https://github.com/pancaspe87)! - Patch absolute paths

- [#416](https://github.com/atlassian-labs/atlaspack/pull/416) [`1de1c81`](https://github.com/atlassian-labs/atlaspack/commit/1de1c8138fbe4d38a64aa1f3c22a70aad59fb5bb) Thanks [@alshdavid](https://github.com/alshdavid)! - Replace require.resolve with path.join

- [#362](https://github.com/atlassian-labs/atlaspack/pull/362) [`726b0b0`](https://github.com/atlassian-labs/atlaspack/commit/726b0b02f4ba47426dd38d809036517477b8b1cd) Thanks [@alshdavid](https://github.com/alshdavid)! - Added conditional bundling config to native js transformer

- [#404](https://github.com/atlassian-labs/atlaspack/pull/404) [`be88bd9`](https://github.com/atlassian-labs/atlaspack/commit/be88bd9fc4cbc1c579685bf2e5d834b4136a6c7c) Thanks [@benjervis](https://github.com/benjervis)! - Removes the dependency check within the config default `package.json`.

  Any dependencies that used to be auto-installed from `parcelDependencies` should
  now be installed in the project root.

- [#415](https://github.com/atlassian-labs/atlaspack/pull/415) [`43113f8`](https://github.com/atlassian-labs/atlaspack/commit/43113f8f00232c5a52169a3f11f846d6e4d94b0a) Thanks [@yamadapc](https://github.com/yamadapc)! - Add feature-flag to fix supporting source-maps for inline bundles

- [#367](https://github.com/atlassian-labs/atlaspack/pull/367) [`3650f7c`](https://github.com/atlassian-labs/atlaspack/commit/3650f7c9ab803b5ae20b223e82b2268a1b614e43) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add basic incremental build support to V3

- [#414](https://github.com/atlassian-labs/atlaspack/pull/414) [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53) Thanks [@alshdavid](https://github.com/alshdavid)! - Added type:commonjs to package.json files

- [#354](https://github.com/atlassian-labs/atlaspack/pull/354) [`a4990f6`](https://github.com/atlassian-labs/atlaspack/commit/a4990f6f32045b95d0e6da97f692269a38e13533) Thanks [@yamadapc](https://github.com/yamadapc)! - Log errors to load the graph including bail-out errors

- [#359](https://github.com/atlassian-labs/atlaspack/pull/359) [`c2ef915`](https://github.com/atlassian-labs/atlaspack/commit/c2ef915dc54784ce4b8180025ac1b2e13b375002) Thanks [@alshdavid](https://github.com/alshdavid)! - Added support for string featureflags to native

- [#340](https://github.com/atlassian-labs/atlaspack/pull/340) [`1b1ef6e`](https://github.com/atlassian-labs/atlaspack/commit/1b1ef6e64fdfcf1c1c744e90e8c6568b0fd0e072) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Ensure bundle graph request errors show up correctly

- [#402](https://github.com/atlassian-labs/atlaspack/pull/402) [`cc66aaa`](https://github.com/atlassian-labs/atlaspack/commit/cc66aaa66d67dd0cb89e083f387a278e74aad3f0) Thanks [@alshdavid](https://github.com/alshdavid)! - Initialize AtlaspackNative async

- [#373](https://github.com/atlassian-labs/atlaspack/pull/373) [`a1e3c87`](https://github.com/atlassian-labs/atlaspack/commit/a1e3c87f25c8d108807fb8ea0e91e8effb2c71a7) Thanks [@yamadapc](https://github.com/yamadapc)! - Add config request ID into the identifier registry

- Updated dependencies [[`bfe81e5`](https://github.com/atlassian-labs/atlaspack/commit/bfe81e551c4e4bb2cac7fc4745222e66962c1728), [`fa4fcf6`](https://github.com/atlassian-labs/atlaspack/commit/fa4fcf69a82b0a3727066ada6e93a149b259936e), [`cd964ee`](https://github.com/atlassian-labs/atlaspack/commit/cd964eed5a330ae63733656ded691d1ea3afe4e3), [`1953d1b`](https://github.com/atlassian-labs/atlaspack/commit/1953d1bec266a39dc4bfce5f6c7959e77e63411e), [`ce4ce95`](https://github.com/atlassian-labs/atlaspack/commit/ce4ce953914e08991cf58c70c98f758690e5ee21), [`1de1c81`](https://github.com/atlassian-labs/atlaspack/commit/1de1c8138fbe4d38a64aa1f3c22a70aad59fb5bb), [`28dee1d`](https://github.com/atlassian-labs/atlaspack/commit/28dee1db7d9a995161b45f76c1a03b80ccaeab4b), [`a317453`](https://github.com/atlassian-labs/atlaspack/commit/a317453432b7f30e98f2a4cbcafdaa5601bcde63), [`2055adb`](https://github.com/atlassian-labs/atlaspack/commit/2055adbe31de792e2a2a591b94d2f33f50735879), [`8bc3db9`](https://github.com/atlassian-labs/atlaspack/commit/8bc3db94cc7382b22ca8207c92af8f6389c17e2e), [`cfa1c63`](https://github.com/atlassian-labs/atlaspack/commit/cfa1c63d710c5f9c9abc55f34220b70fb517c3b8), [`17427a2`](https://github.com/atlassian-labs/atlaspack/commit/17427a2b2fc9c34ef0b941907c2868edef6d1507), [`e962cd7`](https://github.com/atlassian-labs/atlaspack/commit/e962cd735877f7f16163e60868d70d9c10054ebe), [`104a46a`](https://github.com/atlassian-labs/atlaspack/commit/104a46a5ee1fae176d29fcc6420d6bd9c01b35b1), [`9572aca`](https://github.com/atlassian-labs/atlaspack/commit/9572aca2a2313a3c05551f73e556128e77a37732), [`34b740d`](https://github.com/atlassian-labs/atlaspack/commit/34b740d4e2449fba7b50cb9708c56d8033dca5b9), [`4837b69`](https://github.com/atlassian-labs/atlaspack/commit/4837b6988e56ca842a24797b796160964d3696ce), [`e5fa92d`](https://github.com/atlassian-labs/atlaspack/commit/e5fa92de26c87fb5d4d681af1931451749ba970a), [`f6dbdff`](https://github.com/atlassian-labs/atlaspack/commit/f6dbdff59d843e2a832d206205343178b33bf1f5), [`7e21377`](https://github.com/atlassian-labs/atlaspack/commit/7e21377914e8091d484f67cb11052a1efd2227e3), [`43113f8`](https://github.com/atlassian-labs/atlaspack/commit/43113f8f00232c5a52169a3f11f846d6e4d94b0a), [`e962cd7`](https://github.com/atlassian-labs/atlaspack/commit/e962cd735877f7f16163e60868d70d9c10054ebe), [`3650f7c`](https://github.com/atlassian-labs/atlaspack/commit/3650f7c9ab803b5ae20b223e82b2268a1b614e43), [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53), [`c2ef915`](https://github.com/atlassian-labs/atlaspack/commit/c2ef915dc54784ce4b8180025ac1b2e13b375002), [`f635123`](https://github.com/atlassian-labs/atlaspack/commit/f635123f9a06961bc5e053e237f1023f10800ea3), [`4812d0f`](https://github.com/atlassian-labs/atlaspack/commit/4812d0f7400af0f8416f1b7175ecb87700860a68), [`80d963e`](https://github.com/atlassian-labs/atlaspack/commit/80d963ed950f5d742ebd78014cf74f3c65cd4474), [`8fae5f3`](https://github.com/atlassian-labs/atlaspack/commit/8fae5f3005bd7c806b175b4df1754abf58922591), [`3005307`](https://github.com/atlassian-labs/atlaspack/commit/30053076dfd20ca62ddbc682f58adb994029ac55), [`cc66aaa`](https://github.com/atlassian-labs/atlaspack/commit/cc66aaa66d67dd0cb89e083f387a278e74aad3f0), [`67df3f1`](https://github.com/atlassian-labs/atlaspack/commit/67df3f1af1432d77ee6b8850010d976d3313693a), [`0c3ad7a`](https://github.com/atlassian-labs/atlaspack/commit/0c3ad7a302330da1d5e3c025963cc583eb5c28ed)]:
  - @atlaspack/diagnostic@2.14.0
  - @atlaspack/feature-flags@2.14.0
  - @atlaspack/fs@2.14.0
  - @atlaspack/graph@3.4.0
  - @atlaspack/logger@2.14.0
  - @atlaspack/package-manager@2.14.0
  - @atlaspack/plugin@2.14.0
  - @atlaspack/profiler@2.14.0
  - @atlaspack/rust@3.0.0
  - @atlaspack/types@2.14.0
  - @atlaspack/utils@2.14.0
  - @atlaspack/workers@2.14.0
  - @atlaspack/events@2.14.0
  - @atlaspack/build-cache@2.13.2
  - @atlaspack/cache@2.13.2

## 2.13.1

### Patch Changes

- [`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06) Thanks [@yamadapc](https://github.com/yamadapc)! - Add identifier registry and VCS tracing

- Updated dependencies [[`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06)]:
  - @atlaspack/package-manager@2.13.1
  - @atlaspack/feature-flags@2.13.1
  - @atlaspack/build-cache@2.13.1
  - @atlaspack/diagnostic@2.13.1
  - @atlaspack/profiler@2.13.1
  - @atlaspack/workers@2.13.1
  - @atlaspack/events@2.13.1
  - @atlaspack/logger@2.13.1
  - @atlaspack/plugin@2.13.1
  - @atlaspack/cache@2.13.1
  - @atlaspack/graph@3.3.1
  - @atlaspack/types@2.13.1
  - @atlaspack/utils@2.13.1
  - @atlaspack/rust@2.13.1
  - @atlaspack/fs@2.13.1

## 2.13.0

### Minor Changes

- [#335](https://github.com/atlassian-labs/atlaspack/pull/335) [`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf) Thanks [@yamadapc](https://github.com/yamadapc)! - Initial changeset release

### Patch Changes

- Updated dependencies [[`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf)]:
  - @atlaspack/cache@2.13.0
  - @atlaspack/graph@3.3.0
  - @atlaspack/fs@2.13.0
  - @atlaspack/build-cache@2.13.0
  - @atlaspack/diagnostic@2.13.0
  - @atlaspack/feature-flags@2.13.0
  - @atlaspack/logger@2.13.0
  - @atlaspack/package-manager@2.13.0
  - @atlaspack/plugin@2.13.0
  - @atlaspack/profiler@2.13.0
  - @atlaspack/rust@2.13.0
  - @atlaspack/types@2.13.0
  - @atlaspack/utils@2.13.0
  - @atlaspack/workers@2.13.0
  - @atlaspack/events@2.13.0
