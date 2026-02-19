# @atlaspack/build-cache

## 2.13.13

### Patch Changes

- Updated dependencies [[`936209f`](https://github.com/atlassian-labs/atlaspack/commit/936209f3c6c201288720dd62b61e1f538368268c)]:
  - @atlaspack/feature-flags@2.30.1

## 2.13.12

### Patch Changes

- Updated dependencies [[`d8e984b`](https://github.com/atlassian-labs/atlaspack/commit/d8e984b7f6d04fa03707fa299bfd8b8bf9f58280), [`45a0dc5`](https://github.com/atlassian-labs/atlaspack/commit/45a0dc530fd9472dbfdebcbb05f1aad812ab3b23)]:
  - @atlaspack/feature-flags@2.30.0

## 2.13.11

### Patch Changes

- Updated dependencies [[`1815c2c`](https://github.com/atlassian-labs/atlaspack/commit/1815c2ce48e32f4df97ccdd668fd650fc79d1051)]:
  - @atlaspack/feature-flags@2.29.1

## 2.13.10

### Patch Changes

- Updated dependencies [[`349b19c`](https://github.com/atlassian-labs/atlaspack/commit/349b19c3aca2ccb1ffb5cdcdc74890f4b62228be)]:
  - @atlaspack/feature-flags@2.29.0

## 2.13.9

### Patch Changes

- Updated dependencies [[`8826fd0`](https://github.com/atlassian-labs/atlaspack/commit/8826fd02c29c9c67cf0c80da41f424257fbdef93)]:
  - @atlaspack/feature-flags@2.28.0

## 2.13.8

### Patch Changes

- Updated dependencies [[`f33f9c4`](https://github.com/atlassian-labs/atlaspack/commit/f33f9c48dd24b319df352d197e4a83cbb1b053bc), [`e15fb6c`](https://github.com/atlassian-labs/atlaspack/commit/e15fb6c885c6354c6c02283de35ce18abc8c9e18)]:
  - @atlaspack/feature-flags@2.27.7

## 2.13.7

### Patch Changes

- [#960](https://github.com/atlassian-labs/atlaspack/pull/960) [`565bab3`](https://github.com/atlassian-labs/atlaspack/commit/565bab3771cc334659d873cabff4cdfac0860cc7) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add LargeMap to work around Node 24's Map size limit in build cache serializer.

  This change is behind the `useLargeMapInBuildCache` feature flag.

- Updated dependencies [[`c31090c`](https://github.com/atlassian-labs/atlaspack/commit/c31090c9025f35d3fa8561b42dca170853a32e6f), [`565bab3`](https://github.com/atlassian-labs/atlaspack/commit/565bab3771cc334659d873cabff4cdfac0860cc7)]:
  - @atlaspack/feature-flags@2.27.6

## 2.13.6

### Patch Changes

- [#785](https://github.com/atlassian-labs/atlaspack/pull/785) [`0e7dd5e`](https://github.com/atlassian-labs/atlaspack/commit/0e7dd5ec6fbe05aa9e0bb5775a9d0975f206a922) Thanks [@matt-koko](https://github.com/matt-koko)! - We need to re-publish every package in Atlaspack with the corrected types field.

## 2.13.5

### Patch Changes

- [#742](https://github.com/atlassian-labs/atlaspack/pull/742) [`ee040bb`](https://github.com/atlassian-labs/atlaspack/commit/ee040bb6428f29b57d892ddd8107e29077d08ffd) Thanks [@yamadapc](https://github.com/yamadapc)! - Internal changes and bug fixes to environmentDeduplication flag

## 2.13.4

### Patch Changes

- [#720](https://github.com/atlassian-labs/atlaspack/pull/720) [`d2fd849`](https://github.com/atlassian-labs/atlaspack/commit/d2fd849770fe6305e9c694bd97b1bd905abd9d94) Thanks [@alshdavid](https://github.com/alshdavid)! - Migrate to TypeScript

## 2.13.3

### Patch Changes

- [#478](https://github.com/atlassian-labs/atlaspack/pull/478) [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b) Thanks [@yamadapc](https://github.com/yamadapc)! - The first attempt at Version Packages didn't include the built artifacts.
  This has hopefully been fixed, so this change will force those packages to re-release.

## 2.13.2

### Patch Changes

- [#414](https://github.com/atlassian-labs/atlaspack/pull/414) [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53) Thanks [@alshdavid](https://github.com/alshdavid)! - Added type:commonjs to package.json files

## 2.13.1

### Patch Changes

- [`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06) Thanks [@yamadapc](https://github.com/yamadapc)! - Add identifier registry and VCS tracing

## 2.13.0

### Minor Changes

- [#335](https://github.com/atlassian-labs/atlaspack/pull/335) [`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf) Thanks [@yamadapc](https://github.com/yamadapc)! - Initial changeset release
