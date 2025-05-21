# @atlaspack/cli

## 2.13.5

### Patch Changes

- Updated dependencies [[`90150df`](https://github.com/atlassian-labs/atlaspack/commit/90150dfb68236e1d1c11813108ecabd92cff9366)]:
  - @atlaspack/core@2.16.0
  - @atlaspack/fs@2.14.3
  - @atlaspack/logger@2.14.3
  - @atlaspack/utils@2.14.3
  - @atlaspack/config-default@3.1.3
  - @atlaspack/package-manager@2.14.3
  - @atlaspack/reporter-cli@2.14.3
  - @atlaspack/reporter-dev-server@2.14.3
  - @atlaspack/reporter-tracer@2.14.3

## 2.13.4

### Patch Changes

- [#516](https://github.com/atlassian-labs/atlaspack/pull/516) [`e1c9f47`](https://github.com/atlassian-labs/atlaspack/commit/e1c9f47f884a86d51eb3edff4135707c60f3f3fb) Thanks [@yamadapc](https://github.com/yamadapc)! - Make sure atlaspack debug build-asset-graph stores a no-lazy cache

- Updated dependencies [[`9b85d3e`](https://github.com/atlassian-labs/atlaspack/commit/9b85d3e645b10bd027eed2304afc970a5ba40062), [`17b9579`](https://github.com/atlassian-labs/atlaspack/commit/17b9579484eced0ed8f23e2aba6d23b3c7238c39), [`8f4e6c1`](https://github.com/atlassian-labs/atlaspack/commit/8f4e6c1b0e7c1fd48624afda48c1dcc599f1460f)]:
  - @atlaspack/feature-flags@2.14.2
  - @atlaspack/core@2.15.1
  - @atlaspack/fs@2.14.2
  - @atlaspack/utils@2.14.2
  - @atlaspack/reporter-cli@2.14.2
  - @atlaspack/config-default@3.1.2
  - @atlaspack/logger@2.14.2
  - @atlaspack/package-manager@2.14.2
  - @atlaspack/reporter-dev-server@2.14.2
  - @atlaspack/reporter-tracer@2.14.2

## 2.13.3

### Patch Changes

- [#478](https://github.com/atlassian-labs/atlaspack/pull/478) [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b) Thanks [@yamadapc](https://github.com/yamadapc)! - The first attempt at Version Packages didn't include the built artifacts.
  This has hopefully been fixed, so this change will force those packages to re-release.
- Updated dependencies [[`b9d41b1`](https://github.com/atlassian-labs/atlaspack/commit/b9d41b175ad5771651a5b0278a5a0147e669234a), [`80bd57b`](https://github.com/atlassian-labs/atlaspack/commit/80bd57b9f9e966563957dee0780d956a682eb2d4), [`ce13d5e`](https://github.com/atlassian-labs/atlaspack/commit/ce13d5e885d55518ee6318e7a72e3a6e4e5126f2), [`4aab060`](https://github.com/atlassian-labs/atlaspack/commit/4aab0605c0d4ee8e0dcc3ffa1162eae5b360b677), [`87087f4`](https://github.com/atlassian-labs/atlaspack/commit/87087f44f348ac583a27ea0819122e191ba80f8d), [`e1422ad`](https://github.com/atlassian-labs/atlaspack/commit/e1422ad0a801faaa4bc4f1023bed042ffe236e9b), [`7e357fb`](https://github.com/atlassian-labs/atlaspack/commit/7e357fb173e7958da330e3721667fa5749420952), [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b)]:
  - @atlaspack/reporter-dev-server@2.14.1
  - @atlaspack/reporter-tracer@2.14.1
  - @atlaspack/reporter-cli@2.14.1
  - @atlaspack/core@2.15.0
  - @atlaspack/fs@2.14.1
  - @atlaspack/utils@2.14.1
  - @atlaspack/feature-flags@2.14.1
  - @atlaspack/config-default@3.1.1
  - @atlaspack/diagnostic@2.14.1
  - @atlaspack/logger@2.14.1
  - @atlaspack/package-manager@2.14.1
  - @atlaspack/events@2.14.1

## 2.13.2

### Patch Changes

- [#414](https://github.com/atlassian-labs/atlaspack/pull/414) [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53) Thanks [@alshdavid](https://github.com/alshdavid)! - Added type:commonjs to package.json files

- [#341](https://github.com/atlassian-labs/atlaspack/pull/341) [`cd1b0d9`](https://github.com/atlassian-labs/atlaspack/commit/cd1b0d9353fa362a07582d045fb6f1eb0faee7ff) Thanks [@yamadapc](https://github.com/yamadapc)! - Minor internal refactor

- Updated dependencies [[`bfe81e5`](https://github.com/atlassian-labs/atlaspack/commit/bfe81e551c4e4bb2cac7fc4745222e66962c1728), [`cd964ee`](https://github.com/atlassian-labs/atlaspack/commit/cd964eed5a330ae63733656ded691d1ea3afe4e3), [`1953d1b`](https://github.com/atlassian-labs/atlaspack/commit/1953d1bec266a39dc4bfce5f6c7959e77e63411e), [`ce4ce95`](https://github.com/atlassian-labs/atlaspack/commit/ce4ce953914e08991cf58c70c98f758690e5ee21), [`1de1c81`](https://github.com/atlassian-labs/atlaspack/commit/1de1c8138fbe4d38a64aa1f3c22a70aad59fb5bb), [`8bc3db9`](https://github.com/atlassian-labs/atlaspack/commit/8bc3db94cc7382b22ca8207c92af8f6389c17e2e), [`8386ca4`](https://github.com/atlassian-labs/atlaspack/commit/8386ca4dc318688fbed1af3bbebf2af3e7d24552), [`e962cd7`](https://github.com/atlassian-labs/atlaspack/commit/e962cd735877f7f16163e60868d70d9c10054ebe), [`726b0b0`](https://github.com/atlassian-labs/atlaspack/commit/726b0b02f4ba47426dd38d809036517477b8b1cd), [`8386ca4`](https://github.com/atlassian-labs/atlaspack/commit/8386ca4dc318688fbed1af3bbebf2af3e7d24552), [`7e21377`](https://github.com/atlassian-labs/atlaspack/commit/7e21377914e8091d484f67cb11052a1efd2227e3), [`be88bd9`](https://github.com/atlassian-labs/atlaspack/commit/be88bd9fc4cbc1c579685bf2e5d834b4136a6c7c), [`4837b69`](https://github.com/atlassian-labs/atlaspack/commit/4837b6988e56ca842a24797b796160964d3696ce), [`e5fa92d`](https://github.com/atlassian-labs/atlaspack/commit/e5fa92de26c87fb5d4d681af1931451749ba970a), [`f6dbdff`](https://github.com/atlassian-labs/atlaspack/commit/f6dbdff59d843e2a832d206205343178b33bf1f5), [`7e21377`](https://github.com/atlassian-labs/atlaspack/commit/7e21377914e8091d484f67cb11052a1efd2227e3), [`43113f8`](https://github.com/atlassian-labs/atlaspack/commit/43113f8f00232c5a52169a3f11f846d6e4d94b0a), [`e962cd7`](https://github.com/atlassian-labs/atlaspack/commit/e962cd735877f7f16163e60868d70d9c10054ebe), [`3650f7c`](https://github.com/atlassian-labs/atlaspack/commit/3650f7c9ab803b5ae20b223e82b2268a1b614e43), [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53), [`a4990f6`](https://github.com/atlassian-labs/atlaspack/commit/a4990f6f32045b95d0e6da97f692269a38e13533), [`c2ef915`](https://github.com/atlassian-labs/atlaspack/commit/c2ef915dc54784ce4b8180025ac1b2e13b375002), [`4812d0f`](https://github.com/atlassian-labs/atlaspack/commit/4812d0f7400af0f8416f1b7175ecb87700860a68), [`1b1ef6e`](https://github.com/atlassian-labs/atlaspack/commit/1b1ef6e64fdfcf1c1c744e90e8c6568b0fd0e072), [`3005307`](https://github.com/atlassian-labs/atlaspack/commit/30053076dfd20ca62ddbc682f58adb994029ac55), [`cc66aaa`](https://github.com/atlassian-labs/atlaspack/commit/cc66aaa66d67dd0cb89e083f387a278e74aad3f0), [`67df3f1`](https://github.com/atlassian-labs/atlaspack/commit/67df3f1af1432d77ee6b8850010d976d3313693a), [`0c3ad7a`](https://github.com/atlassian-labs/atlaspack/commit/0c3ad7a302330da1d5e3c025963cc583eb5c28ed), [`a1e3c87`](https://github.com/atlassian-labs/atlaspack/commit/a1e3c87f25c8d108807fb8ea0e91e8effb2c71a7)]:
  - @atlaspack/core@2.14.0
  - @atlaspack/diagnostic@2.14.0
  - @atlaspack/feature-flags@2.14.0
  - @atlaspack/fs@2.14.0
  - @atlaspack/logger@2.14.0
  - @atlaspack/package-manager@2.14.0
  - @atlaspack/utils@2.14.0
  - @atlaspack/reporter-cli@2.14.0
  - @atlaspack/reporter-dev-server@2.14.0
  - @atlaspack/reporter-tracer@2.14.0
  - @atlaspack/events@2.14.0
  - @atlaspack/config-default@3.1.0

## 2.13.1

### Patch Changes

- [`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06) Thanks [@yamadapc](https://github.com/yamadapc)! - Add identifier registry and VCS tracing

- Updated dependencies [[`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06)]:
  - @atlaspack/package-manager@2.13.1
  - @atlaspack/reporter-dev-server@2.13.1
  - @atlaspack/feature-flags@2.13.1
  - @atlaspack/reporter-tracer@2.13.1
  - @atlaspack/config-default@3.0.1
  - @atlaspack/diagnostic@2.13.1
  - @atlaspack/reporter-cli@2.13.1
  - @atlaspack/events@2.13.1
  - @atlaspack/logger@2.13.1
  - @atlaspack/utils@2.13.1
  - @atlaspack/core@2.13.1
  - @atlaspack/fs@2.13.1

## 2.13.0

### Minor Changes

- [#335](https://github.com/atlassian-labs/atlaspack/pull/335) [`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf) Thanks [@yamadapc](https://github.com/yamadapc)! - Initial changeset release

### Patch Changes

- Updated dependencies [[`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf)]:
  - @atlaspack/core@2.13.0
  - @atlaspack/fs@2.13.0
  - @atlaspack/config-default@3.0.0
  - @atlaspack/diagnostic@2.13.0
  - @atlaspack/feature-flags@2.13.0
  - @atlaspack/logger@2.13.0
  - @atlaspack/package-manager@2.13.0
  - @atlaspack/utils@2.13.0
  - @atlaspack/reporter-cli@2.13.0
  - @atlaspack/reporter-dev-server@2.13.0
  - @atlaspack/reporter-tracer@2.13.0
  - @atlaspack/events@2.13.0
