# @atlaspack/core

## 2.34.1

### Patch Changes

- [#1036](https://github.com/atlassian-labs/atlaspack/pull/1036) [`e88b258`](https://github.com/atlassian-labs/atlaspack/commit/e88b25808a181fafbe98e5ca278b35bfc0197caf) Thanks [@benjervis](https://github.com/benjervis)! - Fix Rust symbol tracker to correctly handle renamed exports during symbol propagation through barrel files.

- [#1035](https://github.com/atlassian-labs/atlaspack/pull/1035) [`936209f`](https://github.com/atlassian-labs/atlaspack/commit/936209f3c6c201288720dd62b61e1f538368268c) Thanks [@marcins](https://github.com/marcins)! - Remove `cachePerformanceImprovements` feature flag

- Updated dependencies [[`f216e22`](https://github.com/atlassian-labs/atlaspack/commit/f216e227e0aca8a3944f66267d3f75238c732802), [`e88b258`](https://github.com/atlassian-labs/atlaspack/commit/e88b25808a181fafbe98e5ca278b35bfc0197caf), [`936209f`](https://github.com/atlassian-labs/atlaspack/commit/936209f3c6c201288720dd62b61e1f538368268c)]:
  - @atlaspack/rust@3.24.0
  - @atlaspack/feature-flags@2.30.1
  - @atlaspack/cache@3.2.50
  - @atlaspack/fs@2.15.50
  - @atlaspack/logger@2.14.47
  - @atlaspack/source-map@3.2.10
  - @atlaspack/utils@3.3.7
  - @atlaspack/build-cache@2.13.13
  - @atlaspack/graph@3.6.17
  - @atlaspack/package-manager@2.14.55
  - @atlaspack/profiler@2.15.16
  - @atlaspack/workers@2.14.55
  - @atlaspack/plugin@2.14.55
  - @atlaspack/types@2.15.45

## 2.34.0

### Minor Changes

- [#1010](https://github.com/atlassian-labs/atlaspack/pull/1010) [`45a0dc5`](https://github.com/atlassian-labs/atlaspack/commit/45a0dc530fd9472dbfdebcbb05f1aad812ab3b23) Thanks [@benjervis](https://github.com/benjervis)! - Add initial implementation of native symbol propagation

### Patch Changes

- [#1027](https://github.com/atlassian-labs/atlaspack/pull/1027) [`d8e984b`](https://github.com/atlassian-labs/atlaspack/commit/d8e984b7f6d04fa03707fa299bfd8b8bf9f58280) Thanks [@marcins](https://github.com/marcins)! - Improve source map quality through various accuracy fixes.

- Updated dependencies [[`c4082ba`](https://github.com/atlassian-labs/atlaspack/commit/c4082ba3fc1a9328a2e5f23195d5972fbc5d10c8), [`d8e984b`](https://github.com/atlassian-labs/atlaspack/commit/d8e984b7f6d04fa03707fa299bfd8b8bf9f58280), [`ca78e1d`](https://github.com/atlassian-labs/atlaspack/commit/ca78e1d2007bfecd267b283d66a73f3695da4234), [`94e1a58`](https://github.com/atlassian-labs/atlaspack/commit/94e1a58a1a8ed5a4e329745d8a7e6a9530b5cb11), [`3ad8e88`](https://github.com/atlassian-labs/atlaspack/commit/3ad8e880c4c2b9126cbfb6963a3862a75306a26f), [`4c1f39a`](https://github.com/atlassian-labs/atlaspack/commit/4c1f39a911acaefd630d877af1ae5a039931662f), [`fcf7ec5`](https://github.com/atlassian-labs/atlaspack/commit/fcf7ec56fea644e21f7a67d649e6efd1f03c4a6e), [`45a0dc5`](https://github.com/atlassian-labs/atlaspack/commit/45a0dc530fd9472dbfdebcbb05f1aad812ab3b23)]:
  - @atlaspack/rust@3.23.0
  - @atlaspack/package-manager@2.14.54
  - @atlaspack/feature-flags@2.30.0
  - @atlaspack/utils@3.3.6
  - @atlaspack/cache@3.2.49
  - @atlaspack/fs@2.15.49
  - @atlaspack/logger@2.14.46
  - @atlaspack/source-map@3.2.9
  - @atlaspack/build-cache@2.13.12
  - @atlaspack/graph@3.6.16
  - @atlaspack/profiler@2.15.15
  - @atlaspack/workers@2.14.54
  - @atlaspack/plugin@2.14.54
  - @atlaspack/types@2.15.44

## 2.33.1

### Patch Changes

- Updated dependencies [[`95914a7`](https://github.com/atlassian-labs/atlaspack/commit/95914a7f33f9370cdb0c8a3fd6845c042911af59), [`883c107`](https://github.com/atlassian-labs/atlaspack/commit/883c107e60dab2f459e27342017c54e3a7db8bc5), [`1815c2c`](https://github.com/atlassian-labs/atlaspack/commit/1815c2ce48e32f4df97ccdd668fd650fc79d1051)]:
  - @atlaspack/rust@3.22.1
  - @atlaspack/feature-flags@2.29.1
  - @atlaspack/cache@3.2.48
  - @atlaspack/fs@2.15.48
  - @atlaspack/logger@2.14.45
  - @atlaspack/source-map@3.2.8
  - @atlaspack/utils@3.3.5
  - @atlaspack/build-cache@2.13.11
  - @atlaspack/graph@3.6.15
  - @atlaspack/package-manager@2.14.53
  - @atlaspack/profiler@2.15.14
  - @atlaspack/workers@2.14.53
  - @atlaspack/plugin@2.14.53
  - @atlaspack/types@2.15.43

## 2.33.0

### Minor Changes

- [#993](https://github.com/atlassian-labs/atlaspack/pull/993) [`e058f0e`](https://github.com/atlassian-labs/atlaspack/commit/e058f0e7a0423ba9373e85a7dbd5c1dd43b47916) Thanks [@matt-koko](https://github.com/matt-koko)! - Add disableCache option to TransformerSetup API

### Patch Changes

- [#998](https://github.com/atlassian-labs/atlaspack/pull/998) [`349b19c`](https://github.com/atlassian-labs/atlaspack/commit/349b19c3aca2ccb1ffb5cdcdc74890f4b62228be) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add initial native bundler boilerplate and single file bundler

- Updated dependencies [[`e058f0e`](https://github.com/atlassian-labs/atlaspack/commit/e058f0e7a0423ba9373e85a7dbd5c1dd43b47916), [`053e375`](https://github.com/atlassian-labs/atlaspack/commit/053e3754c08c5b5dc239e9f7267df289cbbf31ab), [`25c976e`](https://github.com/atlassian-labs/atlaspack/commit/25c976e01c30e536fb1027eab5b17594c604efff), [`1a86b65`](https://github.com/atlassian-labs/atlaspack/commit/1a86b657868ee881ca5acc68661cca208ac37779), [`349b19c`](https://github.com/atlassian-labs/atlaspack/commit/349b19c3aca2ccb1ffb5cdcdc74890f4b62228be), [`9959efe`](https://github.com/atlassian-labs/atlaspack/commit/9959efe30699a2de3b69275be7ecb47afe81c0b6)]:
  - @atlaspack/rust@3.22.0
  - @atlaspack/feature-flags@2.29.0
  - @atlaspack/fs@2.15.47
  - @atlaspack/graph@3.6.14
  - @atlaspack/logger@2.14.44
  - @atlaspack/plugin@2.14.52
  - @atlaspack/profiler@2.15.13
  - @atlaspack/types@2.15.42
  - @atlaspack/utils@3.3.4
  - @atlaspack/workers@2.14.52
  - @atlaspack/cache@3.2.47
  - @atlaspack/source-map@3.2.7
  - @atlaspack/build-cache@2.13.10
  - @atlaspack/package-manager@2.14.52

## 2.32.1

### Patch Changes

- [#988](https://github.com/atlassian-labs/atlaspack/pull/988) [`a631dcd`](https://github.com/atlassian-labs/atlaspack/commit/a631dcd961112db072b0f8de0831efd178f355a7) Thanks [@marcins](https://github.com/marcins)! - Implement a basic package() method for the native packager

- [#990](https://github.com/atlassian-labs/atlaspack/pull/990) [`5755a11`](https://github.com/atlassian-labs/atlaspack/commit/5755a114903bbf660e2ada3ae2e7ff6ceac7565b) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - changes conditional bundleGraphEdgeType value

- [#987](https://github.com/atlassian-labs/atlaspack/pull/987) [`fcaf517`](https://github.com/atlassian-labs/atlaspack/commit/fcaf517010d15c9300393bcad3f9b465689d9d16) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - add get_bundle_assets

- Updated dependencies [[`a631dcd`](https://github.com/atlassian-labs/atlaspack/commit/a631dcd961112db072b0f8de0831efd178f355a7), [`e9dce31`](https://github.com/atlassian-labs/atlaspack/commit/e9dce3168a8e6727a994bf2a6ac6041eb29f6027), [`59e1345`](https://github.com/atlassian-labs/atlaspack/commit/59e1345f84f43e0632d434ab42c06bf748241985), [`783118c`](https://github.com/atlassian-labs/atlaspack/commit/783118c772f45a0cf6a3b6b447fb9a0e225b25a6), [`fcaf517`](https://github.com/atlassian-labs/atlaspack/commit/fcaf517010d15c9300393bcad3f9b465689d9d16)]:
  - @atlaspack/rust@3.21.0
  - @atlaspack/cache@3.2.46
  - @atlaspack/fs@2.15.46
  - @atlaspack/logger@2.14.43
  - @atlaspack/source-map@3.2.6
  - @atlaspack/utils@3.3.3
  - @atlaspack/package-manager@2.14.51
  - @atlaspack/profiler@2.15.12
  - @atlaspack/workers@2.14.51
  - @atlaspack/types@2.15.41
  - @atlaspack/graph@3.6.13
  - @atlaspack/plugin@2.14.51

## 2.32.0

### Minor Changes

- [#976](https://github.com/atlassian-labs/atlaspack/pull/976) [`e8ea59b`](https://github.com/atlassian-labs/atlaspack/commit/e8ea59beabb4b4fef647dc9ebea3519b6d56d7b5) Thanks [@marcins](https://github.com/marcins)! - Initial implementation of loadBundleGraph to deserialise JS -> Rust BundleGraph.

- [#970](https://github.com/atlassian-labs/atlaspack/pull/970) [`8826fd0`](https://github.com/atlassian-labs/atlaspack/commit/8826fd02c29c9c67cf0c80da41f424257fbdef93) Thanks [@marcins](https://github.com/marcins)! - Add initial plumbing for native packaging to core and Atlaspack V3 code

### Patch Changes

- [#984](https://github.com/atlassian-labs/atlaspack/pull/984) [`dbcaabb`](https://github.com/atlassian-labs/atlaspack/commit/dbcaabbf15d4fbc8ecd9c0be58cf7b2317eebfc4) Thanks [@marcins](https://github.com/marcins)! - Serialise to JSON on the JS side before sending bundle graph nodes to Rust for performance.

- Updated dependencies [[`e8ea59b`](https://github.com/atlassian-labs/atlaspack/commit/e8ea59beabb4b4fef647dc9ebea3519b6d56d7b5), [`3753cb1`](https://github.com/atlassian-labs/atlaspack/commit/3753cb1bf9155eaf3a1a8f952886864682738647), [`8826fd0`](https://github.com/atlassian-labs/atlaspack/commit/8826fd02c29c9c67cf0c80da41f424257fbdef93), [`225683f`](https://github.com/atlassian-labs/atlaspack/commit/225683f7c59355da53b7004d2b8596701ce3af41), [`43adda0`](https://github.com/atlassian-labs/atlaspack/commit/43adda06bf3b6a404b54f8ba2a3b810d92e61d75), [`dbcaabb`](https://github.com/atlassian-labs/atlaspack/commit/dbcaabbf15d4fbc8ecd9c0be58cf7b2317eebfc4)]:
  - @atlaspack/rust@3.20.0
  - @atlaspack/feature-flags@2.28.0
  - @atlaspack/cache@3.2.45
  - @atlaspack/fs@2.15.45
  - @atlaspack/logger@2.14.42
  - @atlaspack/source-map@3.2.5
  - @atlaspack/utils@3.3.2
  - @atlaspack/build-cache@2.13.9
  - @atlaspack/graph@3.6.12
  - @atlaspack/package-manager@2.14.50
  - @atlaspack/profiler@2.15.11
  - @atlaspack/workers@2.14.50
  - @atlaspack/plugin@2.14.50
  - @atlaspack/types@2.15.40

## 2.31.3

### Patch Changes

- Updated dependencies [[`fc3adc0`](https://github.com/atlassian-labs/atlaspack/commit/fc3adc098f583e40d6d7687412cac6dde7cbb3f3), [`f33f9c4`](https://github.com/atlassian-labs/atlaspack/commit/f33f9c48dd24b319df352d197e4a83cbb1b053bc), [`e15fb6c`](https://github.com/atlassian-labs/atlaspack/commit/e15fb6c885c6354c6c02283de35ce18abc8c9e18), [`53da216`](https://github.com/atlassian-labs/atlaspack/commit/53da216320e19c9b71b001386eb420d05f111db8)]:
  - @atlaspack/rust@3.19.0
  - @atlaspack/feature-flags@2.27.7
  - @atlaspack/cache@3.2.44
  - @atlaspack/fs@2.15.44
  - @atlaspack/logger@2.14.41
  - @atlaspack/source-map@3.2.4
  - @atlaspack/utils@3.3.1
  - @atlaspack/build-cache@2.13.8
  - @atlaspack/graph@3.6.11
  - @atlaspack/package-manager@2.14.49
  - @atlaspack/profiler@2.15.10
  - @atlaspack/workers@2.14.49
  - @atlaspack/plugin@2.14.49
  - @atlaspack/types@2.15.39

## 2.31.2

### Patch Changes

- Updated dependencies [[`47aa84e`](https://github.com/atlassian-labs/atlaspack/commit/47aa84e44c61066072a5bf3d57678565b9d2c5a3), [`564fb4f`](https://github.com/atlassian-labs/atlaspack/commit/564fb4fecc2a20bdeb951863a8fac29b618d91b7), [`9df722c`](https://github.com/atlassian-labs/atlaspack/commit/9df722c38b225054883693ba9731344f701767a7), [`c31090c`](https://github.com/atlassian-labs/atlaspack/commit/c31090c9025f35d3fa8561b42dca170853a32e6f), [`565bab3`](https://github.com/atlassian-labs/atlaspack/commit/565bab3771cc334659d873cabff4cdfac0860cc7), [`34b01ec`](https://github.com/atlassian-labs/atlaspack/commit/34b01ec03e2264bd617be000465821e1ed11eae5), [`6e5fec1`](https://github.com/atlassian-labs/atlaspack/commit/6e5fec1406c53257be7e7fbe06c028beda70fcf1), [`e4e5522`](https://github.com/atlassian-labs/atlaspack/commit/e4e55222422d8c73653d3ea09f1a0d13946345b9)]:
  - @atlaspack/utils@3.3.0
  - @atlaspack/rust@3.18.0
  - @atlaspack/feature-flags@2.27.6
  - @atlaspack/build-cache@2.13.7
  - @atlaspack/cache@3.2.43
  - @atlaspack/package-manager@2.14.48
  - @atlaspack/profiler@2.15.9
  - @atlaspack/workers@2.14.48
  - @atlaspack/fs@2.15.43
  - @atlaspack/logger@2.14.40
  - @atlaspack/source-map@3.2.3
  - @atlaspack/graph@3.6.10
  - @atlaspack/types@2.15.38
  - @atlaspack/plugin@2.14.48

## 2.31.1

### Patch Changes

- [#942](https://github.com/atlassian-labs/atlaspack/pull/942) [`695339e`](https://github.com/atlassian-labs/atlaspack/commit/695339e4fd46df9590013c2ef4de74cb22225927) Thanks [@marcins](https://github.com/marcins)! - Improve error handling in Tokens transform

- Updated dependencies [[`7cb8569`](https://github.com/atlassian-labs/atlaspack/commit/7cb85698143e381fcb887173b2bfd15f0aea7918), [`695339e`](https://github.com/atlassian-labs/atlaspack/commit/695339e4fd46df9590013c2ef4de74cb22225927)]:
  - @atlaspack/rust@3.17.0
  - @atlaspack/cache@3.2.42
  - @atlaspack/fs@2.15.42
  - @atlaspack/logger@2.14.39
  - @atlaspack/source-map@3.2.2
  - @atlaspack/utils@3.2.8
  - @atlaspack/package-manager@2.14.47
  - @atlaspack/profiler@2.15.8
  - @atlaspack/workers@2.14.47
  - @atlaspack/types@2.15.37
  - @atlaspack/graph@3.6.9
  - @atlaspack/plugin@2.14.47

## 2.31.0

### Minor Changes

- [#934](https://github.com/atlassian-labs/atlaspack/pull/934) [`02cc8b3`](https://github.com/atlassian-labs/atlaspack/commit/02cc8b32c06ca6b51806b33f6f707ca06e55e957) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add experimental native persistent cache for Atlaspack V3.

- [#934](https://github.com/atlassian-labs/atlaspack/pull/934) [`02cc8b3`](https://github.com/atlassian-labs/atlaspack/commit/02cc8b32c06ca6b51806b33f6f707ca06e55e957) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add new Transformer `setup` method and deprecate `loadConfig`.

  Atlaspack is moving to a pure Transformer model to improve caching performance and consistency.
  The old `loadConfig` method which ran once per Asset goes against this behaviour is now deprecated.
  The new `setup` method runs once per Transformer instance, allowing for better caching and performance optimizations.

### Patch Changes

- Updated dependencies [[`515149d`](https://github.com/atlassian-labs/atlaspack/commit/515149d0a0767d844af803efdc611646780ad0fe), [`02cc8b3`](https://github.com/atlassian-labs/atlaspack/commit/02cc8b32c06ca6b51806b33f6f707ca06e55e957), [`02cc8b3`](https://github.com/atlassian-labs/atlaspack/commit/02cc8b32c06ca6b51806b33f6f707ca06e55e957)]:
  - @atlaspack/rust@3.16.0
  - @atlaspack/feature-flags@2.27.5
  - @atlaspack/cache@3.2.41
  - @atlaspack/fs@2.15.41
  - @atlaspack/logger@2.14.38
  - @atlaspack/source-map@3.2.1
  - @atlaspack/utils@3.2.7
  - @atlaspack/graph@3.6.8
  - @atlaspack/plugin@2.14.46
  - @atlaspack/profiler@2.15.7
  - @atlaspack/types@2.15.36
  - @atlaspack/workers@2.14.46
  - @atlaspack/package-manager@2.14.46

## 2.30.2

### Patch Changes

- Updated dependencies [[`793e78c`](https://github.com/atlassian-labs/atlaspack/commit/793e78c68eed25032e686f70cd77ea3601409b35)]:
  - @atlaspack/feature-flags@2.27.4
  - @atlaspack/cache@3.2.40
  - @atlaspack/fs@2.15.40
  - @atlaspack/graph@3.6.7
  - @atlaspack/utils@3.2.6
  - @atlaspack/package-manager@2.14.45
  - @atlaspack/logger@2.14.37
  - @atlaspack/plugin@2.14.45
  - @atlaspack/profiler@2.15.6
  - @atlaspack/types@2.15.35
  - @atlaspack/workers@2.14.45

## 2.30.1

### Patch Changes

- Updated dependencies [[`362a85d`](https://github.com/atlassian-labs/atlaspack/commit/362a85deb9722f769eeb5099bae915f8b858a126), [`c81b368`](https://github.com/atlassian-labs/atlaspack/commit/c81b368db20af701d9e019a11272b138f9cfe080)]:
  - @atlaspack/rust@3.15.0
  - @atlaspack/source-map@3.2.0
  - @atlaspack/cache@3.2.39
  - @atlaspack/fs@2.15.39
  - @atlaspack/logger@2.14.36
  - @atlaspack/utils@3.2.5
  - @atlaspack/package-manager@2.14.44
  - @atlaspack/profiler@2.15.5
  - @atlaspack/workers@2.14.44
  - @atlaspack/graph@3.6.6
  - @atlaspack/plugin@2.14.44
  - @atlaspack/types@2.15.34

## 2.30.0

### Minor Changes

- [#925](https://github.com/atlassian-labs/atlaspack/pull/925) [`00fa643`](https://github.com/atlassian-labs/atlaspack/commit/00fa6433202bfd3311479f6314c9ec878a789f2c) Thanks [@benjervis](https://github.com/benjervis)! - Save propagating symbols for runtimes by requiring symbol and dependency data be returned at the time the assets are injected.

### Patch Changes

- Updated dependencies []:
  - @atlaspack/fs@2.15.38
  - @atlaspack/graph@3.6.5
  - @atlaspack/logger@2.14.35
  - @atlaspack/plugin@2.14.43
  - @atlaspack/profiler@2.15.4
  - @atlaspack/types@2.15.33
  - @atlaspack/utils@3.2.4
  - @atlaspack/workers@2.14.43
  - @atlaspack/cache@3.2.38
  - @atlaspack/package-manager@2.14.43

## 2.29.2

### Patch Changes

- Updated dependencies [[`787290a`](https://github.com/atlassian-labs/atlaspack/commit/787290a63b3f781e7af86097201dfdb5a36c8551), [`cbb66a5`](https://github.com/atlassian-labs/atlaspack/commit/cbb66a554f7ad679fd4dfc9e2bf2d5abb859f846), [`7815a93`](https://github.com/atlassian-labs/atlaspack/commit/7815a931b18983c3f34a39990b2f4b8b86855cc6), [`25aa273`](https://github.com/atlassian-labs/atlaspack/commit/25aa2735f4db920960de3d65d3a427775086b7d5), [`9468e16`](https://github.com/atlassian-labs/atlaspack/commit/9468e164e8ca20a5e8f552f5dc203aa3716cb154), [`e55c80c`](https://github.com/atlassian-labs/atlaspack/commit/e55c80cd0ae3cc40356fe063cb7d45fbb9095356)]:
  - @atlaspack/rust@3.14.0
  - @atlaspack/feature-flags@2.27.3
  - @atlaspack/cache@3.2.37
  - @atlaspack/fs@2.15.37
  - @atlaspack/logger@2.14.34
  - @atlaspack/source-map@3.1.3
  - @atlaspack/utils@3.2.3
  - @atlaspack/graph@3.6.4
  - @atlaspack/package-manager@2.14.42
  - @atlaspack/profiler@2.15.3
  - @atlaspack/workers@2.14.42
  - @atlaspack/plugin@2.14.42
  - @atlaspack/types@2.15.32

## 2.29.1

### Patch Changes

- [#911](https://github.com/atlassian-labs/atlaspack/pull/911) [`d98c992`](https://github.com/atlassian-labs/atlaspack/commit/d98c992a56121a4f9f6a011ced5e103f96fe3999) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Fix dist dir when using V3 in dev mode

- Updated dependencies [[`96f1d7f`](https://github.com/atlassian-labs/atlaspack/commit/96f1d7f3efef12ce4f5966e34d00b299fcefb4dd), [`f6fc5e2`](https://github.com/atlassian-labs/atlaspack/commit/f6fc5e2acc30720f9a9676dfbc02bf780066653b), [`91738fd`](https://github.com/atlassian-labs/atlaspack/commit/91738fd77cb6b3b7acc93e799327ec283d27f7c0), [`38f3569`](https://github.com/atlassian-labs/atlaspack/commit/38f3569a7024357c43274666312bc6d024bfe5f9), [`3f6a3b8`](https://github.com/atlassian-labs/atlaspack/commit/3f6a3b8df4589fd4d65c072281c7f33bd37341d6), [`96f1d7f`](https://github.com/atlassian-labs/atlaspack/commit/96f1d7f3efef12ce4f5966e34d00b299fcefb4dd), [`d8024ce`](https://github.com/atlassian-labs/atlaspack/commit/d8024ce5c9ced1a9bc39144c979138be3a12b2fb), [`308e7ff`](https://github.com/atlassian-labs/atlaspack/commit/308e7ff979c05c1ae97fe6b08a65666b53d86cc7), [`d98c992`](https://github.com/atlassian-labs/atlaspack/commit/d98c992a56121a4f9f6a011ced5e103f96fe3999)]:
  - @atlaspack/rust@3.13.0
  - @atlaspack/source-map@3.1.2
  - @atlaspack/feature-flags@2.27.2
  - @atlaspack/cache@3.2.36
  - @atlaspack/fs@2.15.36
  - @atlaspack/logger@2.14.33
  - @atlaspack/utils@3.2.2
  - @atlaspack/graph@3.6.3
  - @atlaspack/package-manager@2.14.41
  - @atlaspack/profiler@2.15.2
  - @atlaspack/workers@2.14.41
  - @atlaspack/plugin@2.14.41
  - @atlaspack/types@2.15.31

## 2.29.0

### Minor Changes

- [#898](https://github.com/atlassian-labs/atlaspack/pull/898) [`a884bd2`](https://github.com/atlassian-labs/atlaspack/commit/a884bd2a6bd6f37d088fb886dd9b88e85328501b) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add `unstable_alias` feature to .parcelrc V3 builds for configuring build specific aliases

### Patch Changes

- [#899](https://github.com/atlassian-labs/atlaspack/pull/899) [`5710532`](https://github.com/atlassian-labs/atlaspack/commit/57105322b26ce42af2cbfcc0506cff929e50b630) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Fix v3 environment ids not being consistent with v2

- [#901](https://github.com/atlassian-labs/atlaspack/pull/901) [`6fac01d`](https://github.com/atlassian-labs/atlaspack/commit/6fac01dbb1bfc5b971176c1a1c008297bf4b8f91) Thanks [@benjervis](https://github.com/benjervis)! - Makes the serialization and LMDB write steps into separate Promises, so that we can return them separately and parallelise some work.

- [#905](https://github.com/atlassian-labs/atlaspack/pull/905) [`432d571`](https://github.com/atlassian-labs/atlaspack/commit/432d571ea4e9fc67be12304fcafe8c445e844174) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Decrease max V3 JS workers to 4

- [#899](https://github.com/atlassian-labs/atlaspack/pull/899) [`5710532`](https://github.com/atlassian-labs/atlaspack/commit/57105322b26ce42af2cbfcc0506cff929e50b630) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add HMR and react-refresh support to v3

- Updated dependencies [[`a884bd2`](https://github.com/atlassian-labs/atlaspack/commit/a884bd2a6bd6f37d088fb886dd9b88e85328501b), [`25fee72`](https://github.com/atlassian-labs/atlaspack/commit/25fee723d2228d0482ec8a8f1b8379ccd2977ce7), [`6fac01d`](https://github.com/atlassian-labs/atlaspack/commit/6fac01dbb1bfc5b971176c1a1c008297bf4b8f91), [`2f3af93`](https://github.com/atlassian-labs/atlaspack/commit/2f3af9374bcc082e6f4835f6f5a2b98e32ea45b4), [`432d571`](https://github.com/atlassian-labs/atlaspack/commit/432d571ea4e9fc67be12304fcafe8c445e844174), [`042cd95`](https://github.com/atlassian-labs/atlaspack/commit/042cd95a32ad38cd02d57aa3344c36d4e7ab8f9c), [`28f5424`](https://github.com/atlassian-labs/atlaspack/commit/28f5424f7e2080dd4fbbeb1bc5314a530197a23e), [`a884bd2`](https://github.com/atlassian-labs/atlaspack/commit/a884bd2a6bd6f37d088fb886dd9b88e85328501b), [`5710532`](https://github.com/atlassian-labs/atlaspack/commit/57105322b26ce42af2cbfcc0506cff929e50b630), [`44eb8c3`](https://github.com/atlassian-labs/atlaspack/commit/44eb8c388d3e63f1ef5d835aee7aee5d1bb0620f)]:
  - @atlaspack/rust@3.12.0
  - @atlaspack/feature-flags@2.27.1
  - @atlaspack/cache@3.2.35
  - @atlaspack/fs@2.15.35
  - @atlaspack/logger@2.14.32
  - @atlaspack/source-map@3.1.1
  - @atlaspack/utils@3.2.1
  - @atlaspack/graph@3.6.2
  - @atlaspack/package-manager@2.14.40
  - @atlaspack/profiler@2.15.1
  - @atlaspack/workers@2.14.40
  - @atlaspack/plugin@2.14.40
  - @atlaspack/types@2.15.30

## 2.28.0

### Minor Changes

- [#858](https://github.com/atlassian-labs/atlaspack/pull/858) [`8eb84ee`](https://github.com/atlassian-labs/atlaspack/commit/8eb84ee61a42bfe87c58079b610802b07a6a13e4) Thanks [@marcins](https://github.com/marcins)! - Introduce a new `getReferencedAssets(bundle)` method to the BundleGraph to pre-compute referenced assets, this is used by the scope hoisting packager behind a new `precomputeReferencedAssets` feature flag.

- [#789](https://github.com/atlassian-labs/atlaspack/pull/789) [`73168c2`](https://github.com/atlassian-labs/atlaspack/commit/73168c275a5d9abff9907bcf536b340bca1ed5f0) Thanks [@marcins](https://github.com/marcins)! - Use an in-sourced version of @parcel/source-map, as @atlaspack/source-map

- [#892](https://github.com/atlassian-labs/atlaspack/pull/892) [`617a318`](https://github.com/atlassian-labs/atlaspack/commit/617a318ddc9419b38360257353fec50b9051ee13) Thanks [@marcins](https://github.com/marcins)! - Added a new `shouldProfileNative` option that provides a way to pause and connect a native profiler to Atlaspack.

### Patch Changes

- [#890](https://github.com/atlassian-labs/atlaspack/pull/890) [`7dd5bd8`](https://github.com/atlassian-labs/atlaspack/commit/7dd5bd89ff60c7b324347d772729eb4c3314f01e) Thanks [@matt-koko](https://github.com/matt-koko)! - fix sideEffects not defaulting to true correctly in v3

- Updated dependencies [[`7dd5bd8`](https://github.com/atlassian-labs/atlaspack/commit/7dd5bd89ff60c7b324347d772729eb4c3314f01e), [`c8a0e51`](https://github.com/atlassian-labs/atlaspack/commit/c8a0e51b1096d038a64644ad02fe68383f58682a), [`8eb84ee`](https://github.com/atlassian-labs/atlaspack/commit/8eb84ee61a42bfe87c58079b610802b07a6a13e4), [`71900f3`](https://github.com/atlassian-labs/atlaspack/commit/71900f3508d53c3882ca8ea71501ba3069687d4e), [`73168c2`](https://github.com/atlassian-labs/atlaspack/commit/73168c275a5d9abff9907bcf536b340bca1ed5f0), [`617a318`](https://github.com/atlassian-labs/atlaspack/commit/617a318ddc9419b38360257353fec50b9051ee13), [`2c4953e`](https://github.com/atlassian-labs/atlaspack/commit/2c4953e20dc1a6b012792081667aefeb9340505e), [`3fe7d4e`](https://github.com/atlassian-labs/atlaspack/commit/3fe7d4e821bee162fcbbaf9278b107a5eead944f)]:
  - @atlaspack/rust@3.11.0
  - @atlaspack/feature-flags@2.27.0
  - @atlaspack/source-map@3.1.0
  - @atlaspack/utils@3.2.0
  - @atlaspack/profiler@2.15.0
  - @atlaspack/cache@3.2.34
  - @atlaspack/fs@2.15.34
  - @atlaspack/logger@2.14.31
  - @atlaspack/graph@3.6.1
  - @atlaspack/plugin@2.14.39
  - @atlaspack/types@2.15.29
  - @atlaspack/workers@2.14.39
  - @atlaspack/package-manager@2.14.39

## 2.27.0

### Minor Changes

- [#869](https://github.com/atlassian-labs/atlaspack/pull/869) [`cfb4707`](https://github.com/atlassian-labs/atlaspack/commit/cfb4707087498e4fa4dcf10753fe984a248d196b) Thanks [@benjervis](https://github.com/benjervis)! - Adds a feature to the V3 devloop that will minimise the amount of work done if the change is only a content change.

### Patch Changes

- [#873](https://github.com/atlassian-labs/atlaspack/pull/873) [`ec3abe4`](https://github.com/atlassian-labs/atlaspack/commit/ec3abe4dffc98560a850fd2f71fb566577e6c99c) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - cleanup deduplicateReporters flag

- [#870](https://github.com/atlassian-labs/atlaspack/pull/870) [`33d4c26`](https://github.com/atlassian-labs/atlaspack/commit/33d4c261ceb8d585d56b0a446ed6e28cf7f1126d) Thanks [@matt-koko](https://github.com/matt-koko)! - Support Loading TypeScript Plugin Files in v3

- [#856](https://github.com/atlassian-labs/atlaspack/pull/856) [`f31b041`](https://github.com/atlassian-labs/atlaspack/commit/f31b04107e9077c9946aadb99f6f91bb69703bb7) Thanks [@matt-koko](https://github.com/matt-koko)! - Handle Directory Entry Points in v3

- [#859](https://github.com/atlassian-labs/atlaspack/pull/859) [`8180981`](https://github.com/atlassian-labs/atlaspack/commit/8180981be14c00f9570adb70d3f350bd91d6ec0a) Thanks [@benjervis](https://github.com/benjervis)! - Re-use asset graphs when building in Native, if we have one left over from a previous incremental build

- [#883](https://github.com/atlassian-labs/atlaspack/pull/883) [`119210b`](https://github.com/atlassian-labs/atlaspack/commit/119210b597eb993c50445df87a36b70bd49cd414) Thanks [@matt-koko](https://github.com/matt-koko)! - Fix bitflags rust/js interop

- Updated dependencies [[`ec3abe4`](https://github.com/atlassian-labs/atlaspack/commit/ec3abe4dffc98560a850fd2f71fb566577e6c99c), [`c7fe3f7`](https://github.com/atlassian-labs/atlaspack/commit/c7fe3f76f247e9e20299e205e2df0a16c418eaf2), [`cfb4707`](https://github.com/atlassian-labs/atlaspack/commit/cfb4707087498e4fa4dcf10753fe984a248d196b), [`1468695`](https://github.com/atlassian-labs/atlaspack/commit/1468695fc0c9d06f060a6da9e9b0e154f11dff34), [`f31b041`](https://github.com/atlassian-labs/atlaspack/commit/f31b04107e9077c9946aadb99f6f91bb69703bb7), [`8180981`](https://github.com/atlassian-labs/atlaspack/commit/8180981be14c00f9570adb70d3f350bd91d6ec0a), [`ae77e74`](https://github.com/atlassian-labs/atlaspack/commit/ae77e7452a466b43b3fa5bed24d4ba26345ed765), [`f0a496f`](https://github.com/atlassian-labs/atlaspack/commit/f0a496f70fc8652e090cf1b3f6260e8cfbb796e2), [`7d7a55d`](https://github.com/atlassian-labs/atlaspack/commit/7d7a55dd6395ec391a2e4c33b3dec0d1ea477d4c), [`540f253`](https://github.com/atlassian-labs/atlaspack/commit/540f253dfdcd1a5caebbdc0b197319d439404aae)]:
  - @atlaspack/feature-flags@2.26.2
  - @atlaspack/rust@3.10.0
  - @atlaspack/graph@3.6.0
  - @atlaspack/utils@3.1.2
  - @atlaspack/cache@3.2.33
  - @atlaspack/fs@2.15.33
  - @atlaspack/logger@2.14.30
  - @atlaspack/package-manager@2.14.38
  - @atlaspack/workers@2.14.38
  - @atlaspack/plugin@2.14.38
  - @atlaspack/profiler@2.14.35
  - @atlaspack/types@2.15.28

## 2.26.2

### Patch Changes

- Updated dependencies [[`339b600`](https://github.com/atlassian-labs/atlaspack/commit/339b60095d755742cea50f811d08ccc44a4ae521), [`373ee2d`](https://github.com/atlassian-labs/atlaspack/commit/373ee2d3b5cc315df1e90e647619f915892e2b87)]:
  - @atlaspack/rust@3.9.1
  - @atlaspack/feature-flags@2.26.1
  - @atlaspack/cache@3.2.32
  - @atlaspack/fs@2.15.32
  - @atlaspack/logger@2.14.29
  - @atlaspack/utils@3.1.1
  - @atlaspack/graph@3.5.26
  - @atlaspack/package-manager@2.14.37
  - @atlaspack/workers@2.14.37
  - @atlaspack/plugin@2.14.37
  - @atlaspack/profiler@2.14.34
  - @atlaspack/types@2.15.27

## 2.26.1

### Patch Changes

- [#847](https://github.com/atlassian-labs/atlaspack/pull/847) [`9cd9521`](https://github.com/atlassian-labs/atlaspack/commit/9cd9521978f783046e2ae4ce78f2de7aeb07d073) Thanks [@marcins](https://github.com/marcins)! - Ensure feature flags are set in V3 workers

- [#844](https://github.com/atlassian-labs/atlaspack/pull/844) [`d0c7bea`](https://github.com/atlassian-labs/atlaspack/commit/d0c7bea04458255b4c5d4299600e734b5f919fe1) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - deduplicate reporters from config

- [#851](https://github.com/atlassian-labs/atlaspack/pull/851) [`1086c6a`](https://github.com/atlassian-labs/atlaspack/commit/1086c6a2c59271b63b3fb78e6acdb9d3a0dbf354) Thanks [@benjervis](https://github.com/benjervis)! - Remove asset groups from V3.

  Asset groups were initially created to address the problem of a single asset being transformed into multiple assets (e.g. css modules).

  This did the job, but means that an asset group is created for every single asset, and most groups only have a single asset inside.
  This is a lot of unnecessary nodes, and we don't need to carry that over to the native rewrite.

  This PR deletes the step that used to create them, and updates things like symbol prop to handle there potentially being no asset groups.

- Updated dependencies [[`02f6aa1`](https://github.com/atlassian-labs/atlaspack/commit/02f6aa1906953fc184d2b49c905c2ef815cb878d), [`58527ee`](https://github.com/atlassian-labs/atlaspack/commit/58527eec15c1aebaaadbeb63586f3082c16beae3), [`525898e`](https://github.com/atlassian-labs/atlaspack/commit/525898e33cc229a4bc938ae853732be36d20c01a), [`cfb39a0`](https://github.com/atlassian-labs/atlaspack/commit/cfb39a0d729eb620cf2ca1611750a2bf7a080d08), [`d0c7bea`](https://github.com/atlassian-labs/atlaspack/commit/d0c7bea04458255b4c5d4299600e734b5f919fe1), [`1086c6a`](https://github.com/atlassian-labs/atlaspack/commit/1086c6a2c59271b63b3fb78e6acdb9d3a0dbf354), [`9cd9521`](https://github.com/atlassian-labs/atlaspack/commit/9cd9521978f783046e2ae4ce78f2de7aeb07d073), [`5776be2`](https://github.com/atlassian-labs/atlaspack/commit/5776be21f70a3f2b9471ba33da3ba1a883f21f1a)]:
  - @atlaspack/rust@3.9.0
  - @atlaspack/feature-flags@2.26.0
  - @atlaspack/utils@3.1.0
  - @atlaspack/cache@3.2.31
  - @atlaspack/fs@2.15.31
  - @atlaspack/logger@2.14.28
  - @atlaspack/graph@3.5.25
  - @atlaspack/package-manager@2.14.36
  - @atlaspack/workers@2.14.36
  - @atlaspack/plugin@2.14.36
  - @atlaspack/profiler@2.14.33
  - @atlaspack/types@2.15.26

## 2.26.0

### Minor Changes

- [#827](https://github.com/atlassian-labs/atlaspack/pull/827) [`ff8ac84`](https://github.com/atlassian-labs/atlaspack/commit/ff8ac84bd0be15caa7c7af21bf5b6e47b6d36438) Thanks [@OscarCookeAbbott](https://github.com/OscarCookeAbbott)! - Fix custom environment properties sometimes being ignored

### Patch Changes

- [#817](https://github.com/atlassian-labs/atlaspack/pull/817) [`3da8e69`](https://github.com/atlassian-labs/atlaspack/commit/3da8e690605472d81c531366b65fa9844f8ca337) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Fix an issue when creating the BundleGraph and some Dependency.sourceAssetId values have diverged from the actual Asset.id values.

  Fix behind flag `sourceAssetIdBundleGraphFix`

- [#810](https://github.com/atlassian-labs/atlaspack/pull/810) [`c372f3f`](https://github.com/atlassian-labs/atlaspack/commit/c372f3fd6fce8200d5cf47f41bc7895c6cbb5558) Thanks [@benjervis](https://github.com/benjervis)! - Add the `scope-hosting-stats` debug tool to report on the percentage of assets that were able to be scope hoisted during the build.

- [#831](https://github.com/atlassian-labs/atlaspack/pull/831) [`27fd73f`](https://github.com/atlassian-labs/atlaspack/commit/27fd73fa07d358653f1be6f2d15d72dc5d8fc2fd) Thanks [@benjervis](https://github.com/benjervis)! - Update some Rust internals to use Arcs instead of passing references. This won't make any difference in the immediate term, but is required setup for the next set of changes

- [#834](https://github.com/atlassian-labs/atlaspack/pull/834) [`3091fa3`](https://github.com/atlassian-labs/atlaspack/commit/3091fa36425ecffe3938e55247a035b08cda37a2) Thanks [@benjervis](https://github.com/benjervis)! - We no longer want to use `meta` fields internally in Atlaspack, so they have been removed from the Rust side of the codebase, and are mapped back to the legacy format on the JS side.

- Updated dependencies [[`79a7e22`](https://github.com/atlassian-labs/atlaspack/commit/79a7e22ef119891a2f4180a9d3cc66b38dd52092), [`3da8e69`](https://github.com/atlassian-labs/atlaspack/commit/3da8e690605472d81c531366b65fa9844f8ca337), [`c372f3f`](https://github.com/atlassian-labs/atlaspack/commit/c372f3fd6fce8200d5cf47f41bc7895c6cbb5558), [`17dfea9`](https://github.com/atlassian-labs/atlaspack/commit/17dfea9e9f3169807808520df9c09ebf70dc8b10), [`f1070ec`](https://github.com/atlassian-labs/atlaspack/commit/f1070ec48e99b4ec111a03179c5ab5b74163fab1), [`27fd73f`](https://github.com/atlassian-labs/atlaspack/commit/27fd73fa07d358653f1be6f2d15d72dc5d8fc2fd), [`3091fa3`](https://github.com/atlassian-labs/atlaspack/commit/3091fa36425ecffe3938e55247a035b08cda37a2)]:
  - @atlaspack/feature-flags@2.25.5
  - @atlaspack/rust@3.8.3
  - @atlaspack/utils@3.0.3
  - @atlaspack/cache@3.2.30
  - @atlaspack/fs@2.15.30
  - @atlaspack/graph@3.5.24
  - @atlaspack/package-manager@2.14.35
  - @atlaspack/logger@2.14.27
  - @atlaspack/plugin@2.14.35
  - @atlaspack/profiler@2.14.32
  - @atlaspack/types@2.15.25
  - @atlaspack/workers@2.14.35

## 2.25.1

### Patch Changes

- Updated dependencies [[`25cbee6`](https://github.com/atlassian-labs/atlaspack/commit/25cbee625fb47ac20423fe34ff37bc818a807245), [`662d3c6`](https://github.com/atlassian-labs/atlaspack/commit/662d3c627888b16bf27df15cfac5a9094509e93d)]:
  - @atlaspack/workers@2.14.34
  - @atlaspack/feature-flags@2.25.4
  - @atlaspack/fs@2.15.29
  - @atlaspack/package-manager@2.14.34
  - @atlaspack/types@2.15.24
  - @atlaspack/cache@3.2.29
  - @atlaspack/graph@3.5.23
  - @atlaspack/utils@3.0.2
  - @atlaspack/logger@2.14.26
  - @atlaspack/plugin@2.14.34
  - @atlaspack/profiler@2.14.31

## 2.25.0

### Minor Changes

- [#803](https://github.com/atlassian-labs/atlaspack/pull/803) [`f07351a`](https://github.com/atlassian-labs/atlaspack/commit/f07351a2398b6a8e5155a2daad77e69b7dcb2b4b) Thanks [@shanshrew](https://github.com/shanshrew)! - feat(core, transformers): add feature-flag to omit sourcesContent from memory; reduce peak memory during builds
  - Introduce `omitSourcesContentInMemory` feature flag to stop retaining full source text in `sourcesContent` throughout transforms. Default OFF; behavior unchanged unless enabled.
  - Guard `asset.sourceContent` initialization and `setSourceContent`/`sourcesContent` copies behind the flag.
  - Mappings and source paths remain correct; packager still inlines or references sources per config.

  Memory (three-js benchmark, V3, 1 run):
  - Baseline OFF: later, larger compactions near end of build (e.g. `~44.2s Mark-Compact 20.4 (50.2) -> 12.5 (53.5) MB`).
  - Flag ON: earlier compactions during transform/packaging, keeping old space ≈10–11 MB (e.g. `~17.7s Mark-Compact 11.5 (28.0) -> 9.6 (27.5) MB`).

  Sourcemaps: unchanged by default; with flag ON, only the in-memory retention is removed.

### Patch Changes

- Updated dependencies [[`236e546`](https://github.com/atlassian-labs/atlaspack/commit/236e5465863dca6044a7191e05260a5b924c342e), [`f07351a`](https://github.com/atlassian-labs/atlaspack/commit/f07351a2398b6a8e5155a2daad77e69b7dcb2b4b)]:
  - @atlaspack/utils@3.0.1
  - @atlaspack/feature-flags@2.25.3
  - @atlaspack/cache@3.2.28
  - @atlaspack/package-manager@2.14.33
  - @atlaspack/workers@2.14.33
  - @atlaspack/fs@2.15.28
  - @atlaspack/graph@3.5.22
  - @atlaspack/types@2.15.23
  - @atlaspack/logger@2.14.25
  - @atlaspack/plugin@2.14.33
  - @atlaspack/profiler@2.14.30

## 2.24.2

### Patch Changes

- [#791](https://github.com/atlassian-labs/atlaspack/pull/791) [`10ee3fa`](https://github.com/atlassian-labs/atlaspack/commit/10ee3fa5b75a92acde8973673d9b3c5b6f3958e5) Thanks [@benjervis](https://github.com/benjervis)! - Remove emptyFileStarRexportFix and make the flagged behaviour standard.

  See PR [#685][pr] for details on the change.

  [pr]: https://github.com/atlassian-labs/atlaspack/pull/685

- Updated dependencies [[`0207171`](https://github.com/atlassian-labs/atlaspack/commit/0207171e59e985c51e105aec2e0a99de174374bd), [`10ee3fa`](https://github.com/atlassian-labs/atlaspack/commit/10ee3fa5b75a92acde8973673d9b3c5b6f3958e5), [`bbe4e28`](https://github.com/atlassian-labs/atlaspack/commit/bbe4e28c89e5e9284bd730075d4d6f2a3cb37b21), [`1180103`](https://github.com/atlassian-labs/atlaspack/commit/118010351ed444f8178988afb3f77807154dd933), [`2bc93b1`](https://github.com/atlassian-labs/atlaspack/commit/2bc93b17cea07fd7cbb68acec84d8471345a22b4)]:
  - @atlaspack/feature-flags@2.25.2
  - @atlaspack/rust@3.8.2
  - @atlaspack/utils@3.0.0
  - @atlaspack/cache@3.2.27
  - @atlaspack/fs@2.15.27
  - @atlaspack/graph@3.5.21
  - @atlaspack/logger@2.14.24
  - @atlaspack/package-manager@2.14.32
  - @atlaspack/workers@2.14.32
  - @atlaspack/plugin@2.14.32
  - @atlaspack/profiler@2.14.29
  - @atlaspack/types@2.15.22

## 2.24.1

### Patch Changes

- [#785](https://github.com/atlassian-labs/atlaspack/pull/785) [`0e7dd5e`](https://github.com/atlassian-labs/atlaspack/commit/0e7dd5ec6fbe05aa9e0bb5775a9d0975f206a922) Thanks [@matt-koko](https://github.com/matt-koko)! - We need to re-publish every package in Atlaspack with the corrected types field.

- [#421](https://github.com/atlassian-labs/atlaspack/pull/421) [`686a398`](https://github.com/atlassian-labs/atlaspack/commit/686a398faa1f9e19cbf9274add2c46e246bafba8) Thanks [@mattcompiles](https://github.com/mattcompiles)! - - Fix native respondToFsEvents API
  - Fix native handling of failed requests, allowing them to rebuild when necessary
  - Fix native environment id type
- Updated dependencies [[`0e7dd5e`](https://github.com/atlassian-labs/atlaspack/commit/0e7dd5ec6fbe05aa9e0bb5775a9d0975f206a922), [`686a398`](https://github.com/atlassian-labs/atlaspack/commit/686a398faa1f9e19cbf9274add2c46e246bafba8)]:
  - @atlaspack/build-cache@2.13.6
  - @atlaspack/cache@3.2.26
  - @atlaspack/diagnostic@2.14.4
  - @atlaspack/feature-flags@2.25.1
  - @atlaspack/fs@2.15.26
  - @atlaspack/graph@3.5.20
  - @atlaspack/logger@2.14.23
  - @atlaspack/package-manager@2.14.31
  - @atlaspack/plugin@2.14.31
  - @atlaspack/profiler@2.14.28
  - @atlaspack/rust@3.8.1
  - @atlaspack/types@2.15.21
  - @atlaspack/utils@2.19.3
  - @atlaspack/workers@2.14.31
  - @atlaspack/events@2.14.4

## 2.24.0

### Minor Changes

- [#767](https://github.com/atlassian-labs/atlaspack/pull/767) [`038e87a`](https://github.com/atlassian-labs/atlaspack/commit/038e87a7858d39556d59d3a2d17db534d45f62c6) Thanks [@marcins](https://github.com/marcins)! - Implement a new feature `allowExplicitTargetEntries` where entries provided as entries can be filtered by the source property in targets to allow different targets to be built for different subsets of entries.

- [#769](https://github.com/atlassian-labs/atlaspack/pull/769) [`f6532d7`](https://github.com/atlassian-labs/atlaspack/commit/f6532d7a4f7f007bd4e5e36af04dd466f0b9f572) Thanks [@marcins](https://github.com/marcins)! - Adds a new feature to support adding custom `env` properties in a target, that are available in the Environment during the build. A new feature flag `customEnvInTargets` has been added to gate this feature.

### Patch Changes

- [#780](https://github.com/atlassian-labs/atlaspack/pull/780) [`eedcbc4`](https://github.com/atlassian-labs/atlaspack/commit/eedcbc408fc1e86a2a8e25f1a41c57146d8529e1) Thanks [@JakeLane](https://github.com/JakeLane)! - Fix isStaticBindingSafe in v3

- [#776](https://github.com/atlassian-labs/atlaspack/pull/776) [`5a0363f`](https://github.com/atlassian-labs/atlaspack/commit/5a0363f1086e81068959b87e92064bed5a11a9b4) Thanks [@benjervis](https://github.com/benjervis)! - Guard against empty buffers being returned from JS workers, when using the V3 asset graph build

- [#771](https://github.com/atlassian-labs/atlaspack/pull/771) [`8604d2a`](https://github.com/atlassian-labs/atlaspack/commit/8604d2a49ace5e9194ec4f76e71f06b93610f6b4) Thanks [@benjervis](https://github.com/benjervis)! - Fixes a small error with inline bundle packaging resulting from an incorrect comparison.

- Updated dependencies [[`7a52bab`](https://github.com/atlassian-labs/atlaspack/commit/7a52bab6281e8edbfa66af248eb6f872c6b4b7bd), [`eedcbc4`](https://github.com/atlassian-labs/atlaspack/commit/eedcbc408fc1e86a2a8e25f1a41c57146d8529e1), [`5a0363f`](https://github.com/atlassian-labs/atlaspack/commit/5a0363f1086e81068959b87e92064bed5a11a9b4), [`038e87a`](https://github.com/atlassian-labs/atlaspack/commit/038e87a7858d39556d59d3a2d17db534d45f62c6), [`f6532d7`](https://github.com/atlassian-labs/atlaspack/commit/f6532d7a4f7f007bd4e5e36af04dd466f0b9f572), [`0684997`](https://github.com/atlassian-labs/atlaspack/commit/0684997c127a6204af1376b36e556fddd1ba5557), [`602f8ed`](https://github.com/atlassian-labs/atlaspack/commit/602f8ed9d8381301df8b2cc82c1d5cf6f2f94fec)]:
  - @atlaspack/feature-flags@2.25.0
  - @atlaspack/rust@3.8.0
  - @atlaspack/cache@3.2.25
  - @atlaspack/fs@2.15.25
  - @atlaspack/graph@3.5.19
  - @atlaspack/utils@2.19.2
  - @atlaspack/logger@2.14.22
  - @atlaspack/plugin@2.14.30
  - @atlaspack/profiler@2.14.27
  - @atlaspack/types@2.15.20
  - @atlaspack/workers@2.14.30
  - @atlaspack/package-manager@2.14.30

## 2.23.5

### Patch Changes

- Updated dependencies [[`830261d`](https://github.com/atlassian-labs/atlaspack/commit/830261dcf4a40293090b61cab52fd75d5683c73f), [`58ddd5d`](https://github.com/atlassian-labs/atlaspack/commit/58ddd5d79adde2ac5dc4c60ca575e4705a91e592), [`53dd47b`](https://github.com/atlassian-labs/atlaspack/commit/53dd47bd6d23cd47f87297347f03a609ab38a03d)]:
  - @atlaspack/feature-flags@2.24.1
  - @atlaspack/package-manager@2.14.29
  - @atlaspack/rust@3.7.0
  - @atlaspack/cache@3.2.24
  - @atlaspack/fs@2.15.24
  - @atlaspack/graph@3.5.18
  - @atlaspack/utils@2.19.1
  - @atlaspack/logger@2.14.21
  - @atlaspack/plugin@2.14.29
  - @atlaspack/profiler@2.14.26
  - @atlaspack/types@2.15.19
  - @atlaspack/workers@2.14.29

## 2.23.4

### Patch Changes

- Updated dependencies [[`f0349a6`](https://github.com/atlassian-labs/atlaspack/commit/f0349a6b9b04755088f121095ca6301a2ada3767), [`a5081c3`](https://github.com/atlassian-labs/atlaspack/commit/a5081c3f9207d28716ee4edd3841de1830ea4b80), [`f930003`](https://github.com/atlassian-labs/atlaspack/commit/f930003364fdc31e5487520ebe256f1a9389f579), [`f930003`](https://github.com/atlassian-labs/atlaspack/commit/f930003364fdc31e5487520ebe256f1a9389f579), [`8843980`](https://github.com/atlassian-labs/atlaspack/commit/88439807be20025fd3433380204ff1205079729e)]:
  - @atlaspack/feature-flags@2.24.0
  - @atlaspack/utils@2.19.0
  - @atlaspack/cache@3.2.23
  - @atlaspack/fs@2.15.23
  - @atlaspack/graph@3.5.17
  - @atlaspack/package-manager@2.14.28
  - @atlaspack/workers@2.14.28
  - @atlaspack/logger@2.14.20
  - @atlaspack/plugin@2.14.28
  - @atlaspack/profiler@2.14.25
  - @atlaspack/types@2.15.18

## 2.23.3

### Patch Changes

- Updated dependencies [[`3cfb7cf`](https://github.com/atlassian-labs/atlaspack/commit/3cfb7cfd90b78a5c48ce717e779ff789769825a3)]:
  - @atlaspack/feature-flags@2.23.2
  - @atlaspack/rust@3.6.2
  - @atlaspack/cache@3.2.22
  - @atlaspack/fs@2.15.22
  - @atlaspack/graph@3.5.16
  - @atlaspack/utils@2.18.4
  - @atlaspack/logger@2.14.19
  - @atlaspack/package-manager@2.14.27
  - @atlaspack/plugin@2.14.27
  - @atlaspack/profiler@2.14.24
  - @atlaspack/types@2.15.17
  - @atlaspack/workers@2.14.27

## 2.23.2

### Patch Changes

- [#742](https://github.com/atlassian-labs/atlaspack/pull/742) [`ee040bb`](https://github.com/atlassian-labs/atlaspack/commit/ee040bb6428f29b57d892ddd8107e29077d08ffd) Thanks [@yamadapc](https://github.com/yamadapc)! - Internal changes and bug fixes to environmentDeduplication flag

- [#755](https://github.com/atlassian-labs/atlaspack/pull/755) [`b6e72b6`](https://github.com/atlassian-labs/atlaspack/commit/b6e72b6dd2b517cbb0e700fa69e63332d3eacb1e) Thanks [@benjervis](https://github.com/benjervis)! - Allow runtime assets to be marked as requiring execution on load

- Updated dependencies [[`ee040bb`](https://github.com/atlassian-labs/atlaspack/commit/ee040bb6428f29b57d892ddd8107e29077d08ffd), [`889c65c`](https://github.com/atlassian-labs/atlaspack/commit/889c65cd25b811045e26a117e7404f694dde77a2)]:
  - @atlaspack/package-manager@2.14.26
  - @atlaspack/feature-flags@2.23.1
  - @atlaspack/build-cache@2.13.5
  - @atlaspack/diagnostic@2.14.3
  - @atlaspack/profiler@2.14.23
  - @atlaspack/workers@2.14.26
  - @atlaspack/events@2.14.3
  - @atlaspack/logger@2.14.18
  - @atlaspack/plugin@2.14.26
  - @atlaspack/cache@3.2.21
  - @atlaspack/graph@3.5.15
  - @atlaspack/types@2.15.16
  - @atlaspack/utils@2.18.3
  - @atlaspack/rust@3.6.1
  - @atlaspack/fs@2.15.21

## 2.23.1

### Patch Changes

- Updated dependencies [[`f6b3f22`](https://github.com/atlassian-labs/atlaspack/commit/f6b3f2276c7e417580b49c4879563aab51f156b1)]:
  - @atlaspack/feature-flags@2.23.0
  - @atlaspack/cache@3.2.20
  - @atlaspack/fs@2.15.20
  - @atlaspack/graph@3.5.14
  - @atlaspack/utils@2.18.2
  - @atlaspack/package-manager@2.14.25
  - @atlaspack/logger@2.14.17
  - @atlaspack/plugin@2.14.25
  - @atlaspack/profiler@2.14.22
  - @atlaspack/types@2.15.15
  - @atlaspack/workers@2.14.25

## 2.23.0

### Minor Changes

- [#732](https://github.com/atlassian-labs/atlaspack/pull/732) [`7f5841c`](https://github.com/atlassian-labs/atlaspack/commit/7f5841c39df049f9546cccbeea2a7337e0337b45) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - add tesseract context

### Patch Changes

- Updated dependencies [[`ad26146`](https://github.com/atlassian-labs/atlaspack/commit/ad26146f13b4c1cc65d4a0f9c67060b90ef14ff3), [`f1b48e7`](https://github.com/atlassian-labs/atlaspack/commit/f1b48e7a04e005cef0f36a3e692087a9ecdb6f7a), [`7f5841c`](https://github.com/atlassian-labs/atlaspack/commit/7f5841c39df049f9546cccbeea2a7337e0337b45), [`73dd7ba`](https://github.com/atlassian-labs/atlaspack/commit/73dd7baab69456ef2f6e4a0cc7dbb04f407eb148)]:
  - @atlaspack/rust@3.6.0
  - @atlaspack/feature-flags@2.22.0
  - @atlaspack/cache@3.2.19
  - @atlaspack/fs@2.15.19
  - @atlaspack/logger@2.14.16
  - @atlaspack/utils@2.18.1
  - @atlaspack/package-manager@2.14.24
  - @atlaspack/graph@3.5.13
  - @atlaspack/plugin@2.14.24
  - @atlaspack/profiler@2.14.21
  - @atlaspack/types@2.15.14
  - @atlaspack/workers@2.14.24

## 2.22.0

### Minor Changes

- [#731](https://github.com/atlassian-labs/atlaspack/pull/731) [`23d561e`](https://github.com/atlassian-labs/atlaspack/commit/23d561e51e68b0c38fd1ff4e4fb173e5e7b01cf2) Thanks [@marcins](https://github.com/marcins)! - Implement "inline isolated" scripts

### Patch Changes

- Updated dependencies [[`23d561e`](https://github.com/atlassian-labs/atlaspack/commit/23d561e51e68b0c38fd1ff4e4fb173e5e7b01cf2)]:
  - @atlaspack/feature-flags@2.21.0
  - @atlaspack/utils@2.18.0
  - @atlaspack/rust@3.5.0
  - @atlaspack/cache@3.2.18
  - @atlaspack/fs@2.15.18
  - @atlaspack/graph@3.5.12
  - @atlaspack/package-manager@2.14.23
  - @atlaspack/workers@2.14.23
  - @atlaspack/logger@2.14.15
  - @atlaspack/profiler@2.14.20
  - @atlaspack/types@2.15.13
  - @atlaspack/plugin@2.14.23

## 2.21.0

### Minor Changes

- [#723](https://github.com/atlassian-labs/atlaspack/pull/723) [`43fdd22`](https://github.com/atlassian-labs/atlaspack/commit/43fdd223860fbc97af17d68c65419b97412cb888) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - adding projectRoot option

### Patch Changes

- [#720](https://github.com/atlassian-labs/atlaspack/pull/720) [`d2fd849`](https://github.com/atlassian-labs/atlaspack/commit/d2fd849770fe6305e9c694bd97b1bd905abd9d94) Thanks [@alshdavid](https://github.com/alshdavid)! - Migrate to TypeScript

- [#725](https://github.com/atlassian-labs/atlaspack/pull/725) [`12bee0e`](https://github.com/atlassian-labs/atlaspack/commit/12bee0e23f0464d7f6bd3e24fbe0d19c126d587d) Thanks [@marcins](https://github.com/marcins)! - Clean up `atlaspackV3CleanShutdown` feature flag.

- Updated dependencies [[`d2fd849`](https://github.com/atlassian-labs/atlaspack/commit/d2fd849770fe6305e9c694bd97b1bd905abd9d94), [`12bee0e`](https://github.com/atlassian-labs/atlaspack/commit/12bee0e23f0464d7f6bd3e24fbe0d19c126d587d)]:
  - @atlaspack/package-manager@2.14.22
  - @atlaspack/feature-flags@2.20.1
  - @atlaspack/build-cache@2.13.4
  - @atlaspack/diagnostic@2.14.2
  - @atlaspack/profiler@2.14.19
  - @atlaspack/workers@2.14.22
  - @atlaspack/events@2.14.2
  - @atlaspack/logger@2.14.14
  - @atlaspack/plugin@2.14.22
  - @atlaspack/cache@3.2.17
  - @atlaspack/graph@3.5.11
  - @atlaspack/types@2.15.12
  - @atlaspack/utils@2.17.4
  - @atlaspack/rust@3.4.2
  - @atlaspack/fs@2.15.17

## 2.20.0

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
  - @atlaspack/fs@2.15.16
  - @atlaspack/profiler@2.14.18
  - @atlaspack/types@2.15.11
  - @atlaspack/workers@2.14.21
  - @atlaspack/cache@3.2.16
  - @atlaspack/graph@3.5.10
  - @atlaspack/utils@2.17.3
  - @atlaspack/package-manager@2.14.21
  - @atlaspack/plugin@2.14.21

## 2.19.2

### Patch Changes

- [#707](https://github.com/atlassian-labs/atlaspack/pull/707) [`a0b959f`](https://github.com/atlassian-labs/atlaspack/commit/a0b959fbf61fc3f820ff03c7e8988945fe40a91a) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix content key not found exceptions when bundling is aborted after a unsafe to incrementally bundle asset graph request

- Updated dependencies [[`daaa768`](https://github.com/atlassian-labs/atlaspack/commit/daaa7688786772d7e3713b71c5bba6b89ec704aa), [`1c7865a`](https://github.com/atlassian-labs/atlaspack/commit/1c7865a64451116d94015e248302435839d347c0), [`a0b959f`](https://github.com/atlassian-labs/atlaspack/commit/a0b959fbf61fc3f820ff03c7e8988945fe40a91a)]:
  - @atlaspack/plugin@2.14.20
  - @atlaspack/feature-flags@2.19.2
  - @atlaspack/cache@3.2.15
  - @atlaspack/fs@2.15.15
  - @atlaspack/graph@3.5.9
  - @atlaspack/utils@2.17.2
  - @atlaspack/package-manager@2.14.20
  - @atlaspack/profiler@2.14.17
  - @atlaspack/types@2.15.10
  - @atlaspack/workers@2.14.20

## 2.19.1

### Patch Changes

- Updated dependencies [[`13aef17`](https://github.com/atlassian-labs/atlaspack/commit/13aef177eea289a6e40d2113b5ec1ac9be18a33d)]:
  - @atlaspack/feature-flags@2.19.1
  - @atlaspack/cache@3.2.14
  - @atlaspack/fs@2.15.14
  - @atlaspack/graph@3.5.8
  - @atlaspack/utils@2.17.1
  - @atlaspack/package-manager@2.14.19
  - @atlaspack/profiler@2.14.16
  - @atlaspack/types@2.15.9
  - @atlaspack/workers@2.14.19
  - @atlaspack/plugin@2.14.19

## 2.19.0

### Minor Changes

- [#640](https://github.com/atlassian-labs/atlaspack/pull/640) [`dbb4072`](https://github.com/atlassian-labs/atlaspack/commit/dbb40721ebeb45990a14ba04e6b44e7f836fb32d) Thanks [@JakeLane](https://github.com/JakeLane)! - Clean up conditional bundling feature flags

- [#693](https://github.com/atlassian-labs/atlaspack/pull/693) [`18a57cf`](https://github.com/atlassian-labs/atlaspack/commit/18a57cf8a4789b2de5ad8e2676f317a26cc91417) Thanks [@mattcompiles](https://github.com/mattcompiles)! - These packages should have been bumped in [pull request 691](https://github.com/atlassian-labs/atlaspack/pull/691).

  Rectifying by creating a new changeset now.

### Patch Changes

- [#690](https://github.com/atlassian-labs/atlaspack/pull/690) [`c4415a4`](https://github.com/atlassian-labs/atlaspack/commit/c4415a455543d984ca28452c2cb87a794d22497c) Thanks [@yamadapc](https://github.com/yamadapc)! - Bug fix for build abort state corruption

- [#685](https://github.com/atlassian-labs/atlaspack/pull/685) [`f0f7c71`](https://github.com/atlassian-labs/atlaspack/commit/f0f7c7168a1d3d18c6f30d2daed611275692b7c5) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Fixes an issue where star re-exports of empty files (usually occurring in compiled typescript libraries) could cause exports to undefined at runtime.
  Fix is behind the feature-flag `emptyFileStarRexportFix`.

- [#678](https://github.com/atlassian-labs/atlaspack/pull/678) [`3ba1aee`](https://github.com/atlassian-labs/atlaspack/commit/3ba1aee6a794a26b2f0255aaf6d003981532d0ae) Thanks [@marcins](https://github.com/marcins)! - Move adding of Atlaspack V3 disposable to be conditional on Atlaspack V3

- Updated dependencies [[`dbb4072`](https://github.com/atlassian-labs/atlaspack/commit/dbb40721ebeb45990a14ba04e6b44e7f836fb32d), [`becf977`](https://github.com/atlassian-labs/atlaspack/commit/becf977f625d5ee46dae3d4c679f173bf5f40cc0), [`becf977`](https://github.com/atlassian-labs/atlaspack/commit/becf977f625d5ee46dae3d4c679f173bf5f40cc0), [`c4415a4`](https://github.com/atlassian-labs/atlaspack/commit/c4415a455543d984ca28452c2cb87a794d22497c), [`f0f7c71`](https://github.com/atlassian-labs/atlaspack/commit/f0f7c7168a1d3d18c6f30d2daed611275692b7c5), [`de23e0c`](https://github.com/atlassian-labs/atlaspack/commit/de23e0ce49d5504fe3947ac26640a3d951087da3), [`c9631af`](https://github.com/atlassian-labs/atlaspack/commit/c9631aff284b2c1c27e8a52f9da392ce65d666e8), [`18a57cf`](https://github.com/atlassian-labs/atlaspack/commit/18a57cf8a4789b2de5ad8e2676f317a26cc91417), [`a5ed1b4`](https://github.com/atlassian-labs/atlaspack/commit/a5ed1b414498560f393ff491af4da25b6e8dde56)]:
  - @atlaspack/feature-flags@2.19.0
  - @atlaspack/utils@2.17.0
  - @atlaspack/rust@3.4.1
  - @atlaspack/package-manager@2.14.18
  - @atlaspack/fs@2.15.13
  - @atlaspack/cache@3.2.13
  - @atlaspack/graph@3.5.7
  - @atlaspack/workers@2.14.18
  - @atlaspack/logger@2.14.13
  - @atlaspack/profiler@2.14.15
  - @atlaspack/types@2.15.8
  - @atlaspack/plugin@2.14.18

## 2.18.8

### Patch Changes

- Updated dependencies [[`c75bf55`](https://github.com/atlassian-labs/atlaspack/commit/c75bf553fff4decc285b5fd499a275853b18f8f2)]:
  - @atlaspack/rust@3.4.0
  - @atlaspack/cache@3.2.12
  - @atlaspack/fs@2.15.12
  - @atlaspack/logger@2.14.12
  - @atlaspack/utils@2.16.1
  - @atlaspack/package-manager@2.14.17
  - @atlaspack/workers@2.14.17
  - @atlaspack/types@2.15.7
  - @atlaspack/plugin@2.14.17

## 2.18.7

### Patch Changes

- [#666](https://github.com/atlassian-labs/atlaspack/pull/666) [`1ff31f1`](https://github.com/atlassian-labs/atlaspack/commit/1ff31f10391c48780c9fcfc243b4e828a1b285e0) Thanks [@marcins](https://github.com/marcins)! - Ensure tracer is disabled if it was enabled on teardown

- [#661](https://github.com/atlassian-labs/atlaspack/pull/661) [`e8a60ff`](https://github.com/atlassian-labs/atlaspack/commit/e8a60ffbea41caef265786bbf73349771760081c) Thanks [@marcins](https://github.com/marcins)! - Add new feature flag atlaspackV3CleanShutdown which will dispose of the NAPI worker pool when disposing of the Atlaspack class

- Updated dependencies [[`e8a60ff`](https://github.com/atlassian-labs/atlaspack/commit/e8a60ffbea41caef265786bbf73349771760081c), [`30ee2cf`](https://github.com/atlassian-labs/atlaspack/commit/30ee2cfcd34cf2646ded0eda13fdb80a2a5de529)]:
  - @atlaspack/feature-flags@2.18.4
  - @atlaspack/utils@2.16.0
  - @atlaspack/cache@3.2.11
  - @atlaspack/fs@2.15.11
  - @atlaspack/graph@3.5.6
  - @atlaspack/package-manager@2.14.16
  - @atlaspack/workers@2.14.16
  - @atlaspack/profiler@2.14.14
  - @atlaspack/types@2.15.6
  - @atlaspack/plugin@2.14.16

## 2.18.6

### Patch Changes

- [#655](https://github.com/atlassian-labs/atlaspack/pull/655) [`5ded263`](https://github.com/atlassian-labs/atlaspack/commit/5ded263c7f11b866e8885b81c73e20dd060b25be) Thanks [@yamadapc](https://github.com/yamadapc)! - Clean-up inline requires multi-threading feature-flag

- [#658](https://github.com/atlassian-labs/atlaspack/pull/658) [`74fd942`](https://github.com/atlassian-labs/atlaspack/commit/74fd94236ac697207082c4b755b079e56f5564fb) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix environment deduplication issues

- Updated dependencies [[`5ded263`](https://github.com/atlassian-labs/atlaspack/commit/5ded263c7f11b866e8885b81c73e20dd060b25be)]:
  - @atlaspack/feature-flags@2.18.3
  - @atlaspack/cache@3.2.10
  - @atlaspack/fs@2.15.10
  - @atlaspack/graph@3.5.5
  - @atlaspack/utils@2.15.3
  - @atlaspack/package-manager@2.14.15
  - @atlaspack/profiler@2.14.13
  - @atlaspack/types@2.15.5
  - @atlaspack/workers@2.14.15
  - @atlaspack/plugin@2.14.15

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
