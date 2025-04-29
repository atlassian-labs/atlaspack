# @atlaspack/rust

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
