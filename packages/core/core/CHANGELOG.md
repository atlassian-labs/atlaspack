# @atlaspack/core

## 2.18.5

### Patch Changes

- [#652](https://github.com/atlassian-labs/atlaspack/pull/652) [`644b157`](https://github.com/atlassian-labs/atlaspack/commit/644b157dee72a871acc2d0facf0b87b8eea51956) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix bugs related to build aborts. Builds and cache writes will no longer be aborted.

- Updated dependencies [[`644b157`](https://github.com/atlassian-labs/atlaspack/commit/644b157dee72a871acc2d0facf0b87b8eea51956)]:
  - @atlaspack/feature-flags@2.18.2
  - @atlaspack/cache@3.2.9
  - @atlaspack/fs@2.15.9
  - @atlaspack/graph@3.5.4
  - @atlaspack/utils@2.15.2
  - @atlaspack/package-manager@2.14.14
  - @atlaspack/profiler@2.14.12
  - @atlaspack/types@2.15.4
  - @atlaspack/workers@2.14.14
  - @atlaspack/plugin@2.14.14

## 2.18.4

### Patch Changes

- [#650](https://github.com/atlassian-labs/atlaspack/pull/650) [`ef3d622`](https://github.com/atlassian-labs/atlaspack/commit/ef3d6228f4e006702198a19c61e051d194d325cb) Thanks [@alshdavid](https://github.com/alshdavid)! - Remove package.json#exports

- [#646](https://github.com/atlassian-labs/atlaspack/pull/646) [`6b1f5ff`](https://github.com/atlassian-labs/atlaspack/commit/6b1f5fff68d7131fae075e14f4d2c02606dc6058) Thanks [@alshdavid](https://github.com/alshdavid)! - Export WORKER_PATH from @atlaspack/core

- [#633](https://github.com/atlassian-labs/atlaspack/pull/633) [`26aa9c5`](https://github.com/atlassian-labs/atlaspack/commit/26aa9c599d2be45ce1438a74c5fa22f39b9b554b) Thanks [@sbhuiyan-atlassian](https://github.com/sbhuiyan-atlassian)! - Ported various HMR changes from Parcel

- [#648](https://github.com/atlassian-labs/atlaspack/pull/648) [`c8f7df4`](https://github.com/atlassian-labs/atlaspack/commit/c8f7df4eadfc4718040fceb065dae6e96a4051e7) Thanks [@alshdavid](https://github.com/alshdavid)! - Export ATLASPACK_VERSION and other internals

- [#626](https://github.com/atlassian-labs/atlaspack/pull/626) [`0501255`](https://github.com/atlassian-labs/atlaspack/commit/05012550da35b05ce7d356a8cc29311e7f9afdca) Thanks [@yamadapc](https://github.com/yamadapc)! - Clean-up tsconfig invalidation improvements feature-flag

- Updated dependencies [[`ef3d622`](https://github.com/atlassian-labs/atlaspack/commit/ef3d6228f4e006702198a19c61e051d194d325cb), [`26aa9c5`](https://github.com/atlassian-labs/atlaspack/commit/26aa9c599d2be45ce1438a74c5fa22f39b9b554b), [`0501255`](https://github.com/atlassian-labs/atlaspack/commit/05012550da35b05ce7d356a8cc29311e7f9afdca)]:
  - @atlaspack/workers@2.14.13
  - @atlaspack/logger@2.14.11
  - @atlaspack/feature-flags@2.18.1
  - @atlaspack/fs@2.15.8
  - @atlaspack/package-manager@2.14.13
  - @atlaspack/types@2.15.3
  - @atlaspack/cache@3.2.8
  - @atlaspack/utils@2.15.1
  - @atlaspack/graph@3.5.3
  - @atlaspack/plugin@2.14.13
  - @atlaspack/profiler@2.14.11

## 2.18.3

### Patch Changes

- Updated dependencies [[`10fbcfb`](https://github.com/atlassian-labs/atlaspack/commit/10fbcfbfa49c7a83da5d7c40983e36e87f524a75), [`85c52d3`](https://github.com/atlassian-labs/atlaspack/commit/85c52d3f7717b3c84a118d18ab98cfbfd71dcbd2), [`e39c6cf`](https://github.com/atlassian-labs/atlaspack/commit/e39c6cf05f7e95ce5420dbcea66f401b1cbd397c)]:
  - @atlaspack/feature-flags@2.18.0
  - @atlaspack/utils@2.15.0
  - @atlaspack/cache@3.2.7
  - @atlaspack/fs@2.15.7
  - @atlaspack/graph@3.5.2
  - @atlaspack/package-manager@2.14.12
  - @atlaspack/workers@2.14.12
  - @atlaspack/profiler@2.14.10
  - @atlaspack/types@2.15.2
  - @atlaspack/plugin@2.14.12

## 2.18.2

### Patch Changes

- Updated dependencies [[`73ea3c4`](https://github.com/atlassian-labs/atlaspack/commit/73ea3c4d85d4401fdd15abcbf988237e890e7ad3), [`b1b3693`](https://github.com/atlassian-labs/atlaspack/commit/b1b369317c66f8a431c170df2ebba4fa5b2e38ef)]:
  - @atlaspack/feature-flags@2.17.0
  - @atlaspack/cache@3.2.6
  - @atlaspack/fs@2.15.6
  - @atlaspack/graph@3.5.1
  - @atlaspack/utils@2.14.11
  - @atlaspack/package-manager@2.14.11
  - @atlaspack/profiler@2.14.9
  - @atlaspack/types@2.15.1
  - @atlaspack/workers@2.14.11
  - @atlaspack/plugin@2.14.11

## 2.18.1

### Patch Changes

- Updated dependencies [[`1b52b99`](https://github.com/atlassian-labs/atlaspack/commit/1b52b99db4298b04c1a6eb0f97994d75a2d436f9)]:
  - @atlaspack/graph@3.5.0

## 2.18.0

### Minor Changes

- [#601](https://github.com/atlassian-labs/atlaspack/pull/601) [`1e32d4e`](https://github.com/atlassian-labs/atlaspack/commit/1e32d4eae6b3af3968e8a0ef97d35b4347fd4196) Thanks [@yamadapc](https://github.com/yamadapc)! - Improve granular configuration file invalidations

- [#599](https://github.com/atlassian-labs/atlaspack/pull/599) [`0b2f6f5`](https://github.com/atlassian-labs/atlaspack/commit/0b2f6f55794d3ff6e2f5a41f963e7e5dd8ad9f8d) Thanks [@pancaspe87](https://github.com/pancaspe87)! - load and write env to cache - change is feature flagged

### Patch Changes

- [#595](https://github.com/atlassian-labs/atlaspack/pull/595) [`51aba5f`](https://github.com/atlassian-labs/atlaspack/commit/51aba5fc0e49235ee06bbc3c376f48c3e7da5c4b) Thanks [@yamadapc](https://github.com/yamadapc)! - Add bundleId to write bundle request results

- [#572](https://github.com/atlassian-labs/atlaspack/pull/572) [`6dd4ccb`](https://github.com/atlassian-labs/atlaspack/commit/6dd4ccb753541de32322d881f973d571dd57e4ca) Thanks [@yamadapc](https://github.com/yamadapc)! - Add feature-flagged change which removes all environment duplication around objects

- Updated dependencies [[`0999fb7`](https://github.com/atlassian-labs/atlaspack/commit/0999fb78da519a6c7582d212883e515fcf6c1252), [`51aba5f`](https://github.com/atlassian-labs/atlaspack/commit/51aba5fc0e49235ee06bbc3c376f48c3e7da5c4b), [`1e32d4e`](https://github.com/atlassian-labs/atlaspack/commit/1e32d4eae6b3af3968e8a0ef97d35b4347fd4196), [`35fdd4b`](https://github.com/atlassian-labs/atlaspack/commit/35fdd4b52da0af20f74667f7b8adfb2f90279b7c), [`6dd4ccb`](https://github.com/atlassian-labs/atlaspack/commit/6dd4ccb753541de32322d881f973d571dd57e4ca)]:
  - @atlaspack/fs@2.15.5
  - @atlaspack/types@2.15.0
  - @atlaspack/rust@3.3.5
  - @atlaspack/cache@3.2.5
  - @atlaspack/package-manager@2.14.10
  - @atlaspack/profiler@2.14.8
  - @atlaspack/workers@2.14.10
  - @atlaspack/plugin@2.14.10
  - @atlaspack/logger@2.14.10
  - @atlaspack/utils@2.14.10

## 2.17.4

### Patch Changes

- [#588](https://github.com/atlassian-labs/atlaspack/pull/588) [`1940859`](https://github.com/atlassian-labs/atlaspack/commit/194085942f0e86532e9d039fc3f8039badce4594) Thanks [@yamadapc](https://github.com/yamadapc)! - Do not invalidate all javascript files when tsconfig files change

- [#592](https://github.com/atlassian-labs/atlaspack/pull/592) [`15b6155`](https://github.com/atlassian-labs/atlaspack/commit/15b61556e9114203ebbc9de94b864118ca764598) Thanks [@yamadapc](https://github.com/yamadapc)! - Report large file invalidations

- [#503](https://github.com/atlassian-labs/atlaspack/pull/503) [`209692f`](https://github.com/atlassian-labs/atlaspack/commit/209692ffb11eae103a0d65c5e1118a5aa1625818) Thanks [@JakeLane](https://github.com/JakeLane)! - Fix conditional bundling reporter when condition is reused

- [#562](https://github.com/atlassian-labs/atlaspack/pull/562) [`d04de26`](https://github.com/atlassian-labs/atlaspack/commit/d04de26af684d7abfba5091fbe3df16a12cd0ebc) Thanks [@yamadapc](https://github.com/yamadapc)! - Update with feature-flagged change to write packages into files rather than LMDB

- Updated dependencies [[`124b7ff`](https://github.com/atlassian-labs/atlaspack/commit/124b7fff44f71aac9fbad289a9a9509b3dfc9aaa), [`e052521`](https://github.com/atlassian-labs/atlaspack/commit/e0525210850ed1606146eb86991049cf567c5dec), [`15c6d70`](https://github.com/atlassian-labs/atlaspack/commit/15c6d7000bd89da876bc590aa75b17a619a41896), [`e4d966c`](https://github.com/atlassian-labs/atlaspack/commit/e4d966c3c9c4292c5013372ae65b10d19d4bacc6), [`209692f`](https://github.com/atlassian-labs/atlaspack/commit/209692ffb11eae103a0d65c5e1118a5aa1625818), [`42a775d`](https://github.com/atlassian-labs/atlaspack/commit/42a775de8eec638ad188f3271964170d8c04d84b), [`29c2f10`](https://github.com/atlassian-labs/atlaspack/commit/29c2f106de9679adfb5afa04e1910471dc65a427), [`f4da1e1`](https://github.com/atlassian-labs/atlaspack/commit/f4da1e120e73eeb5e8b8927f05e88f04d6148c7b), [`1ef91fc`](https://github.com/atlassian-labs/atlaspack/commit/1ef91fcc863fdd2831511937083dbbc1263b3d9d)]:
  - @atlaspack/cache@3.2.4
  - @atlaspack/rust@3.3.4
  - @atlaspack/fs@2.15.4
  - @atlaspack/feature-flags@2.16.0
  - @atlaspack/logger@2.14.9
  - @atlaspack/utils@2.14.9
  - @atlaspack/package-manager@2.14.9
  - @atlaspack/graph@3.4.7
  - @atlaspack/workers@2.14.9
  - @atlaspack/profiler@2.14.7
  - @atlaspack/types@2.14.9
  - @atlaspack/plugin@2.14.9

## 2.17.3

### Patch Changes

- [#551](https://github.com/atlassian-labs/atlaspack/pull/551) [`30f6017`](https://github.com/atlassian-labs/atlaspack/commit/30f60175ba4d272c5fc193973c63bc298584775b) Thanks [@yamadapc](https://github.com/yamadapc)! - Log request tracker invalidation counts on start-up

- [#550](https://github.com/atlassian-labs/atlaspack/pull/550) [`3a3e8e7`](https://github.com/atlassian-labs/atlaspack/commit/3a3e8e7be9e2dffd7304436d792f0f595d59665a) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix typescript declaration files

- [#555](https://github.com/atlassian-labs/atlaspack/pull/555) [`15c1e3c`](https://github.com/atlassian-labs/atlaspack/commit/15c1e3c0628bae4c768d76cf3afc53d6d0d7ce7c) Thanks [@alshdavid](https://github.com/alshdavid)! - Added ATLASPACK_NATIVE_THREADS env variable to control the number of threads used by the native thread schedular

- Updated dependencies [[`30f6017`](https://github.com/atlassian-labs/atlaspack/commit/30f60175ba4d272c5fc193973c63bc298584775b), [`3a3e8e7`](https://github.com/atlassian-labs/atlaspack/commit/3a3e8e7be9e2dffd7304436d792f0f595d59665a), [`1ab0a27`](https://github.com/atlassian-labs/atlaspack/commit/1ab0a275aeca40350415e2b03e7440d1dddc6228), [`b8a4ae8`](https://github.com/atlassian-labs/atlaspack/commit/b8a4ae8f83dc0a83d8b145c5f729936ce52080a3)]:
  - @atlaspack/feature-flags@2.15.1
  - @atlaspack/fs@2.15.3
  - @atlaspack/rust@3.3.3
  - @atlaspack/cache@3.2.3
  - @atlaspack/graph@3.4.6
  - @atlaspack/utils@2.14.8
  - @atlaspack/package-manager@2.14.8
  - @atlaspack/logger@2.14.8
  - @atlaspack/profiler@2.14.6
  - @atlaspack/types@2.14.8
  - @atlaspack/workers@2.14.8
  - @atlaspack/plugin@2.14.8

## 2.17.2

### Patch Changes

- Updated dependencies [[`a1773d2`](https://github.com/atlassian-labs/atlaspack/commit/a1773d2a62d0ef7805ac7524621dcabcc1afe929), [`556d6ab`](https://github.com/atlassian-labs/atlaspack/commit/556d6ab8ede759fa7f37fcd3f4da336ef1c55e8f)]:
  - @atlaspack/feature-flags@2.15.0
  - @atlaspack/logger@2.14.7
  - @atlaspack/rust@3.3.2
  - @atlaspack/cache@3.2.2
  - @atlaspack/fs@2.15.2
  - @atlaspack/graph@3.4.5
  - @atlaspack/utils@2.14.7
  - @atlaspack/package-manager@2.14.7
  - @atlaspack/workers@2.14.7
  - @atlaspack/profiler@2.14.5
  - @atlaspack/types@2.14.7
  - @atlaspack/plugin@2.14.7

## 2.17.1

### Patch Changes

- Updated dependencies [[`e0f5337`](https://github.com/atlassian-labs/atlaspack/commit/e0f533757bd1019dbd108a04952c87da15286e09)]:
  - @atlaspack/feature-flags@2.14.4
  - @atlaspack/rust@3.3.1
  - @atlaspack/cache@3.2.1
  - @atlaspack/fs@2.15.1
  - @atlaspack/graph@3.4.4
  - @atlaspack/utils@2.14.6
  - @atlaspack/logger@2.14.6
  - @atlaspack/package-manager@2.14.6
  - @atlaspack/profiler@2.14.4
  - @atlaspack/types@2.14.6
  - @atlaspack/workers@2.14.6
  - @atlaspack/plugin@2.14.6

## 2.17.0

### Minor Changes

- [#541](https://github.com/atlassian-labs/atlaspack/pull/541) [`e2ba0f6`](https://github.com/atlassian-labs/atlaspack/commit/e2ba0f69702656f3d1ce95ab1454e35062b13b39) Thanks [@yamadapc](https://github.com/yamadapc)! - Add database compaction debug command

### Patch Changes

- [#530](https://github.com/atlassian-labs/atlaspack/pull/530) [`2e90c9b`](https://github.com/atlassian-labs/atlaspack/commit/2e90c9bd07d7eb52645f9d84ccbb7f82685cbc8c) Thanks [@yamadapc](https://github.com/yamadapc)! - Write metadata about the cache in a new entry

- [#511](https://github.com/atlassian-labs/atlaspack/pull/511) [`11d6f16`](https://github.com/atlassian-labs/atlaspack/commit/11d6f16b6397dee2f217167e5c98b39edb63f7a7) Thanks [@yamadapc](https://github.com/yamadapc)! - Clean-up dylib worker threads segmentation fault bug fix feature-flag

- Updated dependencies [[`11d6f16`](https://github.com/atlassian-labs/atlaspack/commit/11d6f16b6397dee2f217167e5c98b39edb63f7a7), [`e2ba0f6`](https://github.com/atlassian-labs/atlaspack/commit/e2ba0f69702656f3d1ce95ab1454e35062b13b39), [`d2c50c2`](https://github.com/atlassian-labs/atlaspack/commit/d2c50c2c020888b33bb25b8690d9320c2b69e2a6), [`46a90dc`](https://github.com/atlassian-labs/atlaspack/commit/46a90dccd019a26b222c878a92d23acc75dc67c5), [`4c17141`](https://github.com/atlassian-labs/atlaspack/commit/4c1714103dab2aa9039c488f381551d2b65d1d01)]:
  - @atlaspack/feature-flags@2.14.3
  - @atlaspack/rust@3.3.0
  - @atlaspack/cache@3.2.0
  - @atlaspack/fs@2.15.0
  - @atlaspack/graph@3.4.3
  - @atlaspack/utils@2.14.5
  - @atlaspack/logger@2.14.5
  - @atlaspack/package-manager@2.14.5
  - @atlaspack/profiler@2.14.3
  - @atlaspack/types@2.14.5
  - @atlaspack/workers@2.14.5
  - @atlaspack/plugin@2.14.5

## 2.16.1

### Patch Changes

- [#525](https://github.com/atlassian-labs/atlaspack/pull/525) [`cb9da16`](https://github.com/atlassian-labs/atlaspack/commit/cb9da16fb2648e7f53c64df0313f60d5fb8970cc) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix issues with large blob cache writes, run cache writes in a write transaction

- Updated dependencies [[`1a2c14c`](https://github.com/atlassian-labs/atlaspack/commit/1a2c14c3cd4587551cc12e94d0680c8b71ea12bf), [`cb9da16`](https://github.com/atlassian-labs/atlaspack/commit/cb9da16fb2648e7f53c64df0313f60d5fb8970cc)]:
  - @atlaspack/rust@3.2.0
  - @atlaspack/cache@3.1.0
  - @atlaspack/fs@2.14.4
  - @atlaspack/logger@2.14.4
  - @atlaspack/utils@2.14.4
  - @atlaspack/package-manager@2.14.4
  - @atlaspack/workers@2.14.4
  - @atlaspack/types@2.14.4
  - @atlaspack/plugin@2.14.4

## 2.16.0

### Minor Changes

- [#520](https://github.com/atlassian-labs/atlaspack/pull/520) [`90150df`](https://github.com/atlassian-labs/atlaspack/commit/90150dfb68236e1d1c11813108ecabd92cff9366) Thanks [@pancaspe87](https://github.com/pancaspe87)! - Modify "large blobs" to be written to the LMDB cache

### Patch Changes

- Updated dependencies [[`f27d39e`](https://github.com/atlassian-labs/atlaspack/commit/f27d39e767b06def059944b3bc5fd50797eaea96)]:
  - @atlaspack/rust@3.1.1
  - @atlaspack/cache@3.0.1
  - @atlaspack/fs@2.14.3
  - @atlaspack/logger@2.14.3
  - @atlaspack/utils@2.14.3
  - @atlaspack/package-manager@2.14.3
  - @atlaspack/workers@2.14.3
  - @atlaspack/types@2.14.3
  - @atlaspack/plugin@2.14.3

## 2.15.1

### Patch Changes

- [#512](https://github.com/atlassian-labs/atlaspack/pull/512) [`8f4e6c1`](https://github.com/atlassian-labs/atlaspack/commit/8f4e6c1b0e7c1fd48624afda48c1dcc599f1460f) Thanks [@yamadapc](https://github.com/yamadapc)! - Remove LMDB cache back-end

- Updated dependencies [[`9b85d3e`](https://github.com/atlassian-labs/atlaspack/commit/9b85d3e645b10bd027eed2304afc970a5ba40062), [`a891d65`](https://github.com/atlassian-labs/atlaspack/commit/a891d652bc4eb3d757d381adf65c5083f706effc), [`d02eab9`](https://github.com/atlassian-labs/atlaspack/commit/d02eab95eb60bf7457e0869af0b773608592c0e6), [`fb87a90`](https://github.com/atlassian-labs/atlaspack/commit/fb87a901973776b33ca4ce530e9d71669a9bd36d), [`7b9e8cf`](https://github.com/atlassian-labs/atlaspack/commit/7b9e8cf29e01a98e72e46b2b2fb74ccc514f4463), [`17b9579`](https://github.com/atlassian-labs/atlaspack/commit/17b9579484eced0ed8f23e2aba6d23b3c7238c39), [`8f4e6c1`](https://github.com/atlassian-labs/atlaspack/commit/8f4e6c1b0e7c1fd48624afda48c1dcc599f1460f)]:
  - @atlaspack/feature-flags@2.14.2
  - @atlaspack/rust@3.1.0
  - @atlaspack/cache@3.0.0
  - @atlaspack/fs@2.14.2
  - @atlaspack/graph@3.4.2
  - @atlaspack/utils@2.14.2
  - @atlaspack/logger@2.14.2
  - @atlaspack/package-manager@2.14.2
  - @atlaspack/profiler@2.14.2
  - @atlaspack/types@2.14.2
  - @atlaspack/workers@2.14.2
  - @atlaspack/plugin@2.14.2

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
