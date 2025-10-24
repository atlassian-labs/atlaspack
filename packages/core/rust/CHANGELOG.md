# @atlaspack/rust

## 3.9.0

### Minor Changes

- [#837](https://github.com/atlassian-labs/atlaspack/pull/837) [`02f6aa1`](https://github.com/atlassian-labs/atlaspack/commit/02f6aa1906953fc184d2b49c905c2ef815cb878d) Thanks [@OscarCookeAbbott](https://github.com/OscarCookeAbbott)! - Add new `global_this_aliaser` transformer

- [#846](https://github.com/atlassian-labs/atlaspack/pull/846) [`58527ee`](https://github.com/atlassian-labs/atlaspack/commit/58527eec15c1aebaaadbeb63586f3082c16beae3) Thanks [@JakeLane](https://github.com/JakeLane)! - Creating a new temp package @atlaspack/transformer-compiled-css-in-js to aid in native migration from babel

- [#842](https://github.com/atlassian-labs/atlaspack/pull/842) [`525898e`](https://github.com/atlassian-labs/atlaspack/commit/525898e33cc229a4bc938ae853732be36d20c01a) Thanks [@OscarCookeAbbott](https://github.com/OscarCookeAbbott)! - Add new `lazy_loading` transformer

- [#777](https://github.com/atlassian-labs/atlaspack/pull/777) [`cfb39a0`](https://github.com/atlassian-labs/atlaspack/commit/cfb39a0d729eb620cf2ca1611750a2bf7a080d08) Thanks [@matt-koko](https://github.com/matt-koko)! - Added logic to rust JS transformer to account for JSX transformations in scenarios when the file extension of the asset is NOT .jsx or .tsx. The logic to determine whether the file should be treated as JSX syntax now mirrors that of the existing v2 behaviour. Several unit tests and integration tests have been introduced to ensure this parity is maintained.

- [#847](https://github.com/atlassian-labs/atlaspack/pull/847) [`9cd9521`](https://github.com/atlassian-labs/atlaspack/commit/9cd9521978f783046e2ae4ce78f2de7aeb07d073) Thanks [@marcins](https://github.com/marcins)! - Implement a new transformer for Atlaskit Tokens, re-using infrastructure from the existing JS transformer.

### Patch Changes

- [#844](https://github.com/atlassian-labs/atlaspack/pull/844) [`d0c7bea`](https://github.com/atlassian-labs/atlaspack/commit/d0c7bea04458255b4c5d4299600e734b5f919fe1) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - deduplicate reporters from config

- [#851](https://github.com/atlassian-labs/atlaspack/pull/851) [`1086c6a`](https://github.com/atlassian-labs/atlaspack/commit/1086c6a2c59271b63b3fb78e6acdb9d3a0dbf354) Thanks [@benjervis](https://github.com/benjervis)! - Fix missing edges in the asset graph when working with discovered assets.

  When we are transforming an asset that returns "discovered" assets (e.g. CSS modules), we add an edge from the dependency to the asset, and then from the dependency to the discovered asset/s.

  However, we also have some logic to skip some steps if the asset that is being referenced has already been transformed, instead just creating an edge from the new dependency to the existing asset node.

  Unfortunately this reuse logic did not account for the discovered assets when adding edges, only connecting the dependency with the main asset. This has been addressed in this PR.

## 3.8.3

### Patch Changes

- [#816](https://github.com/atlassian-labs/atlaspack/pull/816) [`79a7e22`](https://github.com/atlassian-labs/atlaspack/commit/79a7e22ef119891a2f4180a9d3cc66b38dd52092) Thanks [@yamadapc](https://github.com/yamadapc)! - Clean-up resolver optimisation feature-flag

- [#840](https://github.com/atlassian-labs/atlaspack/pull/840) [`17dfea9`](https://github.com/atlassian-labs/atlaspack/commit/17dfea9e9f3169807808520df9c09ebf70dc8b10) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - cleanup inlineStringReplacmentPerf flag

- [#831](https://github.com/atlassian-labs/atlaspack/pull/831) [`27fd73f`](https://github.com/atlassian-labs/atlaspack/commit/27fd73fa07d358653f1be6f2d15d72dc5d8fc2fd) Thanks [@benjervis](https://github.com/benjervis)! - Update some Rust internals to use Arcs instead of passing references. This won't make any difference in the immediate term, but is required setup for the next set of changes

- [#834](https://github.com/atlassian-labs/atlaspack/pull/834) [`3091fa3`](https://github.com/atlassian-labs/atlaspack/commit/3091fa36425ecffe3938e55247a035b08cda37a2) Thanks [@benjervis](https://github.com/benjervis)! - We no longer want to use `meta` fields internally in Atlaspack, so they have been removed from the Rust side of the codebase, and are mapped back to the legacy format on the JS side.

## 3.8.2

### Patch Changes

- [#790](https://github.com/atlassian-labs/atlaspack/pull/790) [`0207171`](https://github.com/atlassian-labs/atlaspack/commit/0207171e59e985c51e105aec2e0a99de174374bd) Thanks [@benjervis](https://github.com/benjervis)! - Clean up the `unusedComputedPropertyFix` feature flag

- [#786](https://github.com/atlassian-labs/atlaspack/pull/786) [`bbe4e28`](https://github.com/atlassian-labs/atlaspack/commit/bbe4e28c89e5e9284bd730075d4d6f2a3cb37b21) Thanks [@benjervis](https://github.com/benjervis)! - Add `hmr_options` to the native transformer to prevent applying react refresh when it's not in use.

## 3.8.1

### Patch Changes

- [#785](https://github.com/atlassian-labs/atlaspack/pull/785) [`0e7dd5e`](https://github.com/atlassian-labs/atlaspack/commit/0e7dd5ec6fbe05aa9e0bb5775a9d0975f206a922) Thanks [@matt-koko](https://github.com/matt-koko)! - We need to re-publish every package in Atlaspack with the corrected types field.

- [#421](https://github.com/atlassian-labs/atlaspack/pull/421) [`686a398`](https://github.com/atlassian-labs/atlaspack/commit/686a398faa1f9e19cbf9274add2c46e246bafba8) Thanks [@mattcompiles](https://github.com/mattcompiles)! - - Fix native respondToFsEvents API
  - Fix native handling of failed requests, allowing them to rebuild when necessary
  - Fix native environment id type

## 3.8.0

### Minor Changes

- [#767](https://github.com/atlassian-labs/atlaspack/pull/767) [`038e87a`](https://github.com/atlassian-labs/atlaspack/commit/038e87a7858d39556d59d3a2d17db534d45f62c6) Thanks [@marcins](https://github.com/marcins)! - Implement a new feature `allowExplicitTargetEntries` where entries provided as entries can be filtered by the source property in targets to allow different targets to be built for different subsets of entries.

- [#769](https://github.com/atlassian-labs/atlaspack/pull/769) [`f6532d7`](https://github.com/atlassian-labs/atlaspack/commit/f6532d7a4f7f007bd4e5e36af04dd466f0b9f572) Thanks [@marcins](https://github.com/marcins)! - Adds a new feature to support adding custom `env` properties in a target, that are available in the Environment during the build. A new feature flag `customEnvInTargets` has been added to gate this feature.

- [#774](https://github.com/atlassian-labs/atlaspack/pull/774) [`602f8ed`](https://github.com/atlassian-labs/atlaspack/commit/602f8ed9d8381301df8b2cc82c1d5cf6f2f94fec) Thanks [@JakeLane](https://github.com/JakeLane)! - Add feature flag exportsRebindingOptimisation which detects cases where we don't need the rebinding helper for esm exports

### Patch Changes

- [#780](https://github.com/atlassian-labs/atlaspack/pull/780) [`eedcbc4`](https://github.com/atlassian-labs/atlaspack/commit/eedcbc408fc1e86a2a8e25f1a41c57146d8529e1) Thanks [@JakeLane](https://github.com/JakeLane)! - Fix isStaticBindingSafe in v3

- [#776](https://github.com/atlassian-labs/atlaspack/pull/776) [`5a0363f`](https://github.com/atlassian-labs/atlaspack/commit/5a0363f1086e81068959b87e92064bed5a11a9b4) Thanks [@benjervis](https://github.com/benjervis)! - Guard against empty buffers being returned from JS workers, when using the V3 asset graph build

- [#778](https://github.com/atlassian-labs/atlaspack/pull/778) [`0684997`](https://github.com/atlassian-labs/atlaspack/commit/0684997c127a6204af1376b36e556fddd1ba5557) Thanks [@alshdavid](https://github.com/alshdavid)! - Updated internal dependencies

## 3.7.0

### Minor Changes

- [#766](https://github.com/atlassian-labs/atlaspack/pull/766) [`53dd47b`](https://github.com/atlassian-labs/atlaspack/commit/53dd47bd6d23cd47f87297347f03a609ab38a03d) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add support for resolving `module:es2019` main fields

## 3.6.2

### Patch Changes

- [#757](https://github.com/atlassian-labs/atlaspack/pull/757) [`3cfb7cf`](https://github.com/atlassian-labs/atlaspack/commit/3cfb7cfd90b78a5c48ce717e779ff789769825a3) Thanks [@marcins](https://github.com/marcins)! - Clean up the `inlineIsolatedScripts` feature flag

## 3.6.1

### Patch Changes

- [#742](https://github.com/atlassian-labs/atlaspack/pull/742) [`ee040bb`](https://github.com/atlassian-labs/atlaspack/commit/ee040bb6428f29b57d892ddd8107e29077d08ffd) Thanks [@yamadapc](https://github.com/yamadapc)! - Internal changes and bug fixes to environmentDeduplication flag

## 3.6.0

### Minor Changes

- [#733](https://github.com/atlassian-labs/atlaspack/pull/733) [`ad26146`](https://github.com/atlassian-labs/atlaspack/commit/ad26146f13b4c1cc65d4a0f9c67060b90ef14ff3) Thanks [@yamadapc](https://github.com/yamadapc)! - Add support for adding react displayName to components

- [#732](https://github.com/atlassian-labs/atlaspack/pull/732) [`7f5841c`](https://github.com/atlassian-labs/atlaspack/commit/7f5841c39df049f9546cccbeea2a7337e0337b45) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - add tesseract context

### Patch Changes

- [#730](https://github.com/atlassian-labs/atlaspack/pull/730) [`f1b48e7`](https://github.com/atlassian-labs/atlaspack/commit/f1b48e7a04e005cef0f36a3e692087a9ecdb6f7a) Thanks [@alshdavid](https://github.com/alshdavid)! - Fixing types

## 3.5.0

### Minor Changes

- [#731](https://github.com/atlassian-labs/atlaspack/pull/731) [`23d561e`](https://github.com/atlassian-labs/atlaspack/commit/23d561e51e68b0c38fd1ff4e4fb173e5e7b01cf2) Thanks [@marcins](https://github.com/marcins)! - Implement "inline isolated" scripts

## 3.4.2

### Patch Changes

- [#720](https://github.com/atlassian-labs/atlaspack/pull/720) [`d2fd849`](https://github.com/atlassian-labs/atlaspack/commit/d2fd849770fe6305e9c694bd97b1bd905abd9d94) Thanks [@alshdavid](https://github.com/alshdavid)! - Migrate to TypeScript

## 3.4.1

### Patch Changes

- [#686](https://github.com/atlassian-labs/atlaspack/pull/686) [`becf977`](https://github.com/atlassian-labs/atlaspack/commit/becf977f625d5ee46dae3d4c679f173bf5f40cc0) Thanks [@benjervis](https://github.com/benjervis)! - Fix (behind a feature flag) the bug where non-static property access of an imported object was not being considered used by the collector.

- [#685](https://github.com/atlassian-labs/atlaspack/pull/685) [`f0f7c71`](https://github.com/atlassian-labs/atlaspack/commit/f0f7c7168a1d3d18c6f30d2daed611275692b7c5) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Fixes an issue where star re-exports of empty files (usually occurring in compiled typescript libraries) could cause exports to undefined at runtime.
  Fix is behind the feature-flag `emptyFileStarRexportFix`.

## 3.4.0

### Minor Changes

- [#671](https://github.com/atlassian-labs/atlaspack/pull/671) [`c75bf55`](https://github.com/atlassian-labs/atlaspack/commit/c75bf553fff4decc285b5fd499a275853b18f8f2) Thanks [@matt-koko](https://github.com/matt-koko)! - The @atlaspack/rust package should have been bumped in [pull request 633](https://github.com/atlassian-labs/atlaspack/pull/633). This has resulted in the JS half of those changes being released, but not the Rust half.

  Rectifying by creating a new changeset now.

## 3.3.5

### Patch Changes

- [#594](https://github.com/atlassian-labs/atlaspack/pull/594) [`35fdd4b`](https://github.com/atlassian-labs/atlaspack/commit/35fdd4b52da0af20f74667f7b8adfb2f90279b7c) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix issue where cache database could become invalid due to stale readers

- [#572](https://github.com/atlassian-labs/atlaspack/pull/572) [`6dd4ccb`](https://github.com/atlassian-labs/atlaspack/commit/6dd4ccb753541de32322d881f973d571dd57e4ca) Thanks [@yamadapc](https://github.com/yamadapc)! - Add feature-flagged change which removes all environment duplication around objects

## 3.3.4

### Patch Changes

- [#583](https://github.com/atlassian-labs/atlaspack/pull/583) [`124b7ff`](https://github.com/atlassian-labs/atlaspack/commit/124b7fff44f71aac9fbad289a9a9509b3dfc9aaa) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix problem where cache writes could start to fail during a V3 build

- [#568](https://github.com/atlassian-labs/atlaspack/pull/568) [`e052521`](https://github.com/atlassian-labs/atlaspack/commit/e0525210850ed1606146eb86991049cf567c5dec) Thanks [@yamadapc](https://github.com/yamadapc)! - Migrate to parking_lot locks to prevent crashes

- [#564](https://github.com/atlassian-labs/atlaspack/pull/564) [`15c6d70`](https://github.com/atlassian-labs/atlaspack/commit/15c6d7000bd89da876bc590aa75b17a619a41896) Thanks [@benjervis](https://github.com/benjervis)! - The `SourceField` enum in package.json parsing is now marked as "untagged", allowing it to be parsed properly.

- [#591](https://github.com/atlassian-labs/atlaspack/pull/591) [`e4d966c`](https://github.com/atlassian-labs/atlaspack/commit/e4d966c3c9c4292c5013372ae65b10d19d4bacc6) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix bug where renames would not get handled correctly

- [#569](https://github.com/atlassian-labs/atlaspack/pull/569) [`42a775d`](https://github.com/atlassian-labs/atlaspack/commit/42a775de8eec638ad188f3271964170d8c04d84b) Thanks [@benjervis](https://github.com/benjervis)! - There are three types of results that a resolver can return:
  - A successful resolution
  - "Unresolved" when the resolver could not find a match
  - "Excluded" when the result should not be included in the bundle

  This last case wasn't being handle in the NAPI conversion layer, and so was falling through as a successful resolution with no details.

- [#589](https://github.com/atlassian-labs/atlaspack/pull/589) [`29c2f10`](https://github.com/atlassian-labs/atlaspack/commit/29c2f106de9679adfb5afa04e1910471dc65a427) Thanks [@yamadapc](https://github.com/yamadapc)! - Do not use libgit

- [#586](https://github.com/atlassian-labs/atlaspack/pull/586) [`1ef91fc`](https://github.com/atlassian-labs/atlaspack/commit/1ef91fcc863fdd2831511937083dbbc1263b3d9d) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix issue where LMDB database handle could become invalid

## 3.3.3

### Patch Changes

- [#558](https://github.com/atlassian-labs/atlaspack/pull/558) [`1ab0a27`](https://github.com/atlassian-labs/atlaspack/commit/1ab0a275aeca40350415e2b03e7440d1dddc6228) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix bug handling dirty deleted git files

- [#559](https://github.com/atlassian-labs/atlaspack/pull/559) [`b8a4ae8`](https://github.com/atlassian-labs/atlaspack/commit/b8a4ae8f83dc0a83d8b145c5f729936ce52080a3) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix bugs in VCS cache invalidation

## 3.3.2

### Patch Changes

- [#549](https://github.com/atlassian-labs/atlaspack/pull/549) [`556d6ab`](https://github.com/atlassian-labs/atlaspack/commit/556d6ab8ede759fa7f37fcd3f4da336ef1c55e8f) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix log file RUST_LOG levels

## 3.3.1

### Patch Changes

- [#542](https://github.com/atlassian-labs/atlaspack/pull/542) [`e0f5337`](https://github.com/atlassian-labs/atlaspack/commit/e0f533757bd1019dbd108a04952c87da15286e09) Thanks [@yamadapc](https://github.com/yamadapc)! - Add feature-flagged option to use rayon thread-pool to optimize inline requires

## 3.3.0

### Minor Changes

- [#541](https://github.com/atlassian-labs/atlaspack/pull/541) [`e2ba0f6`](https://github.com/atlassian-labs/atlaspack/commit/e2ba0f69702656f3d1ce95ab1454e35062b13b39) Thanks [@yamadapc](https://github.com/yamadapc)! - Add database compaction debug command

- [#531](https://github.com/atlassian-labs/atlaspack/pull/531) [`d2c50c2`](https://github.com/atlassian-labs/atlaspack/commit/d2c50c2c020888b33bb25b8690d9320c2b69e2a6) Thanks [@yamadapc](https://github.com/yamadapc)! - Add way to iterate LMDB cache keys

### Patch Changes

- [#540](https://github.com/atlassian-labs/atlaspack/pull/540) [`46a90dc`](https://github.com/atlassian-labs/atlaspack/commit/46a90dccd019a26b222c878a92d23acc75dc67c5) Thanks [@yamadapc](https://github.com/yamadapc)! - Log verbose errors when failing to read VCS files

## 3.2.0

### Minor Changes

- [#525](https://github.com/atlassian-labs/atlaspack/pull/525) [`cb9da16`](https://github.com/atlassian-labs/atlaspack/commit/cb9da16fb2648e7f53c64df0313f60d5fb8970cc) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix issues with large blob cache writes, run cache writes in a write transaction

### Patch Changes

- [#527](https://github.com/atlassian-labs/atlaspack/pull/527) [`1a2c14c`](https://github.com/atlassian-labs/atlaspack/commit/1a2c14c3cd4587551cc12e94d0680c8b71ea12bf) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix warnings when creating multiple instances on a process

## 3.1.1

### Patch Changes

- [#458](https://github.com/atlassian-labs/atlaspack/pull/458) [`f27d39e`](https://github.com/atlassian-labs/atlaspack/commit/f27d39e767b06def059944b3bc5fd50797eaea96) Thanks [@yamadapc](https://github.com/yamadapc)! - Migrate to LazyLock from lazy_static

## 3.1.0

### Minor Changes

- [#491](https://github.com/atlassian-labs/atlaspack/pull/491) [`a891d65`](https://github.com/atlassian-labs/atlaspack/commit/a891d652bc4eb3d757d381adf65c5083f706effc) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Support ignore comments for node replacements

  Adding `#__ATLASPACK_IGNORE__` before `__filename` and `__dirname` will now disable the default node replacement behaviour of these variables. This is useful when you want your compiled output to be aware of it's runtime directory rather than it's pre-compiled source directory.

  ```js
  const dirname = /*#__ATLASPACK_IGNORE__*/ __dirname;
  ```

### Patch Changes

- [#495](https://github.com/atlassian-labs/atlaspack/pull/495) [`d02eab9`](https://github.com/atlassian-labs/atlaspack/commit/d02eab95eb60bf7457e0869af0b773608592c0e6) Thanks [@yamadapc](https://github.com/yamadapc)! - Update with sentry tracing support

- [#514](https://github.com/atlassian-labs/atlaspack/pull/514) [`fb87a90`](https://github.com/atlassian-labs/atlaspack/commit/fb87a901973776b33ca4ce530e9d71669a9bd36d) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix reading file contents on certain sparse checkouts

- [#498](https://github.com/atlassian-labs/atlaspack/pull/498) [`7b9e8cf`](https://github.com/atlassian-labs/atlaspack/commit/7b9e8cf29e01a98e72e46b2b2fb74ccc514f4463) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix compile time flags (sentry integration, crash reporting, memory allocator)

## 3.0.1

### Patch Changes

- [#444](https://github.com/atlassian-labs/atlaspack/pull/444) [`80bd57b`](https://github.com/atlassian-labs/atlaspack/commit/80bd57b9f9e966563957dee0780d956a682eb2d4) Thanks [@yamadapc](https://github.com/yamadapc)! - Allow missing .yarn-state.yml files without throwing on VCS file change reads

- [#448](https://github.com/atlassian-labs/atlaspack/pull/448) [`ae70b81`](https://github.com/atlassian-labs/atlaspack/commit/ae70b810384cf58f9c57d341ab4c925c7bb2060c) Thanks [@yamadapc](https://github.com/yamadapc)! - Upgrade GLIBC to 2.35

- [#487](https://github.com/atlassian-labs/atlaspack/pull/487) [`c0a61a9`](https://github.com/atlassian-labs/atlaspack/commit/c0a61a92405b6830fe39cc17622cc2e97bf02dd7) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix listing diff for VCS filesystem on sparse checkouts

- [#488](https://github.com/atlassian-labs/atlaspack/pull/488) [`cb35e7d`](https://github.com/atlassian-labs/atlaspack/commit/cb35e7d2b90b372de8401792915f12f410508d24) Thanks [@yamadapc](https://github.com/yamadapc)! - Binaries are now built on debian bullseye

- [#459](https://github.com/atlassian-labs/atlaspack/pull/459) [`6ec11f1`](https://github.com/atlassian-labs/atlaspack/commit/6ec11f10a9366fb8a9fc0475c7678235056bd80e) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix segmentation faults on exit

- [#478](https://github.com/atlassian-labs/atlaspack/pull/478) [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b) Thanks [@yamadapc](https://github.com/yamadapc)! - The first attempt at Version Packages didn't include the built artifacts.
  This has hopefully been fixed, so this change will force those packages to re-release.

## 3.0.0

### Major Changes

- [#402](https://github.com/atlassian-labs/atlaspack/pull/402) [`cc66aaa`](https://github.com/atlassian-labs/atlaspack/commit/cc66aaa66d67dd0cb89e083f387a278e74aad3f0) Thanks [@alshdavid](https://github.com/alshdavid)! - Initialize AtlaspackNative async

### Minor Changes

- [#339](https://github.com/atlassian-labs/atlaspack/pull/339) [`bfe81e5`](https://github.com/atlassian-labs/atlaspack/commit/bfe81e551c4e4bb2cac7fc4745222e66962c1728) Thanks [@yamadapc](https://github.com/yamadapc)! - Update cache invalidation metrics with build type

- [#346](https://github.com/atlassian-labs/atlaspack/pull/346) [`9572aca`](https://github.com/atlassian-labs/atlaspack/commit/9572aca2a2313a3c05551f73e556128e77a37732) Thanks [@pancaspe87](https://github.com/pancaspe87)! - Add tracing logs around yarn state scanning

- [#344](https://github.com/atlassian-labs/atlaspack/pull/344) [`34b740d`](https://github.com/atlassian-labs/atlaspack/commit/34b740d4e2449fba7b50cb9708c56d8033dca5b9) Thanks [@pancaspe87](https://github.com/pancaspe87)! - Add tracing logs for dirty files listing

- [#366](https://github.com/atlassian-labs/atlaspack/pull/366) [`4812d0f`](https://github.com/atlassian-labs/atlaspack/commit/4812d0f7400af0f8416f1b7175ecb87700860a68) Thanks [@alshdavid](https://github.com/alshdavid)! - Added NapiWorkerPool

- [#357](https://github.com/atlassian-labs/atlaspack/pull/357) [`80d963e`](https://github.com/atlassian-labs/atlaspack/commit/80d963ed950f5d742ebd78014cf74f3c65cd4474) Thanks [@alshdavid](https://github.com/alshdavid)! - Added AtlaspackV3Options.featureFlags

### Patch Changes

- [#438](https://github.com/atlassian-labs/atlaspack/pull/438) [`fa4fcf6`](https://github.com/atlassian-labs/atlaspack/commit/fa4fcf69a82b0a3727066ada6e93a149b259936e) Thanks [@yamadapc](https://github.com/yamadapc)! - Strip comments on optimizer

- [#401](https://github.com/atlassian-labs/atlaspack/pull/401) [`cd964ee`](https://github.com/atlassian-labs/atlaspack/commit/cd964eed5a330ae63733656ded691d1ea3afe4e3) Thanks [@MonicaOlejniczak](https://github.com/MonicaOlejniczak)! - Fix respondToFsEvents return type

- [#378](https://github.com/atlassian-labs/atlaspack/pull/378) [`ce4ce95`](https://github.com/atlassian-labs/atlaspack/commit/ce4ce953914e08991cf58c70c98f758690e5ee21) Thanks [@yamadapc](https://github.com/yamadapc)! - Add feature-flagged optimization to resolver specifier handling

- [#392](https://github.com/atlassian-labs/atlaspack/pull/392) [`28dee1d`](https://github.com/atlassian-labs/atlaspack/commit/28dee1db7d9a995161b45f76c1a03b80ccaeab4b) Thanks [@alshdavid](https://github.com/alshdavid)! - Added win32 target to lmdblite

- [#427](https://github.com/atlassian-labs/atlaspack/pull/427) [`a317453`](https://github.com/atlassian-labs/atlaspack/commit/a317453432b7f30e98f2a4cbcafdaa5601bcde63) Thanks [@alshdavid](https://github.com/alshdavid)! - Enabled Rust rust_2018_idioms lints and updated files to match linting rules

- [#349](https://github.com/atlassian-labs/atlaspack/pull/349) [`2055adb`](https://github.com/atlassian-labs/atlaspack/commit/2055adbe31de792e2a2a591b94d2f33f50735879) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix bug in inline requires where it'd produce invalid const statements

- [#350](https://github.com/atlassian-labs/atlaspack/pull/350) [`cfa1c63`](https://github.com/atlassian-labs/atlaspack/commit/cfa1c63d710c5f9c9abc55f34220b70fb517c3b8) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix VCS watcher handling of new yarn.lock files between revisions

- [#387](https://github.com/atlassian-labs/atlaspack/pull/387) [`17427a2`](https://github.com/atlassian-labs/atlaspack/commit/17427a2b2fc9c34ef0b941907c2868edef6d1507) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Fix handling of distDir from target descriptors

- [#365](https://github.com/atlassian-labs/atlaspack/pull/365) [`104a46a`](https://github.com/atlassian-labs/atlaspack/commit/104a46a5ee1fae176d29fcc6420d6bd9c01b35b1) Thanks [@benjervis](https://github.com/benjervis)! - Fix bug that caused variables preceding their require to be missed (see [pull request] for more information).

  [pull request]: https://github.com/atlassian-labs/atlaspack/pull/365

- [#429](https://github.com/atlassian-labs/atlaspack/pull/429) [`4837b69`](https://github.com/atlassian-labs/atlaspack/commit/4837b6988e56ca842a24797b796160964d3696ce) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Increase performance of inline bundle packaging (enabled via `featureFlags.inlineStringReplacementPerf`)

- [#418](https://github.com/atlassian-labs/atlaspack/pull/418) [`e5fa92d`](https://github.com/atlassian-labs/atlaspack/commit/e5fa92de26c87fb5d4d681af1931451749ba970a) Thanks [@yamadapc](https://github.com/yamadapc)! - Do not run VCS queries on the main thread

- [#367](https://github.com/atlassian-labs/atlaspack/pull/367) [`3650f7c`](https://github.com/atlassian-labs/atlaspack/commit/3650f7c9ab803b5ae20b223e82b2268a1b614e43) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add basic incremental build support to V3

- [#414](https://github.com/atlassian-labs/atlaspack/pull/414) [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53) Thanks [@alshdavid](https://github.com/alshdavid)! - Added type:commonjs to package.json files

- [#359](https://github.com/atlassian-labs/atlaspack/pull/359) [`c2ef915`](https://github.com/atlassian-labs/atlaspack/commit/c2ef915dc54784ce4b8180025ac1b2e13b375002) Thanks [@alshdavid](https://github.com/alshdavid)! - Added support for string featureflags to native

- [#368](https://github.com/atlassian-labs/atlaspack/pull/368) [`f635123`](https://github.com/atlassian-labs/atlaspack/commit/f635123f9a06961bc5e053e237f1023f10800ea3) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix finding project root path based on .git / .hg directories

- [#372](https://github.com/atlassian-labs/atlaspack/pull/372) [`8fae5f3`](https://github.com/atlassian-labs/atlaspack/commit/8fae5f3005bd7c806b175b4df1754abf58922591) Thanks [@yamadapc](https://github.com/yamadapc)! - Reduce allocations in the resolver

- [#410](https://github.com/atlassian-labs/atlaspack/pull/410) [`67df3f1`](https://github.com/atlassian-labs/atlaspack/commit/67df3f1af1432d77ee6b8850010d976d3313693a) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix sparse checkout support for VCS watcher

- [#345](https://github.com/atlassian-labs/atlaspack/pull/345) [`0c3ad7a`](https://github.com/atlassian-labs/atlaspack/commit/0c3ad7a302330da1d5e3c025963cc583eb5c28ed) Thanks [@yamadapc](https://github.com/yamadapc)! - Modify how VCS watcher change events are forwarded

## 2.13.1

### Patch Changes

- [`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06) Thanks [@yamadapc](https://github.com/yamadapc)! - Add identifier registry and VCS tracing

## 2.13.0

### Minor Changes

- [#335](https://github.com/atlassian-labs/atlaspack/pull/335) [`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf) Thanks [@yamadapc](https://github.com/yamadapc)! - Initial changeset release
