# @atlaspack/rust

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
