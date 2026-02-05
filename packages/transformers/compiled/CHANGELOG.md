# @atlaspack/transformer-compiled

## 0.3.0

### Minor Changes

- [#993](https://github.com/atlassian-labs/atlaspack/pull/993) [`e058f0e`](https://github.com/atlassian-labs/atlaspack/commit/e058f0e7a0423ba9373e85a7dbd5c1dd43b47916) Thanks [@matt-koko](https://github.com/matt-koko)! - Add disableCache option to TransformerSetup API

### Patch Changes

- Updated dependencies []:
  - @atlaspack/plugin@2.14.52
  - @atlaspack/utils@3.3.4
  - @atlaspack/source-map@3.2.7

## 0.2.5

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.2.6
  - @atlaspack/utils@3.3.3
  - @atlaspack/plugin@2.14.51

## 0.2.4

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.2.5
  - @atlaspack/utils@3.3.2
  - @atlaspack/plugin@2.14.50

## 0.2.3

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.2.4
  - @atlaspack/utils@3.3.1
  - @atlaspack/plugin@2.14.49

## 0.2.2

### Patch Changes

- Updated dependencies [[`47aa84e`](https://github.com/atlassian-labs/atlaspack/commit/47aa84e44c61066072a5bf3d57678565b9d2c5a3)]:
  - @atlaspack/utils@3.3.0
  - @atlaspack/source-map@3.2.3
  - @atlaspack/plugin@2.14.48

## 0.2.1

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.2.2
  - @atlaspack/utils@3.2.8
  - @atlaspack/plugin@2.14.47

## 0.2.0

### Minor Changes

- [#934](https://github.com/atlassian-labs/atlaspack/pull/934) [`02cc8b3`](https://github.com/atlassian-labs/atlaspack/commit/02cc8b32c06ca6b51806b33f6f707ca06e55e957) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add experimental native persistent cache for Atlaspack V3.

- [#934](https://github.com/atlassian-labs/atlaspack/pull/934) [`02cc8b3`](https://github.com/atlassian-labs/atlaspack/commit/02cc8b32c06ca6b51806b33f6f707ca06e55e957) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add new Transformer `setup` method and deprecate `loadConfig`.

  Atlaspack is moving to a pure Transformer model to improve caching performance and consistency.
  The old `loadConfig` method which ran once per Asset goes against this behaviour is now deprecated.
  The new `setup` method runs once per Transformer instance, allowing for better caching and performance optimizations.

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.2.1
  - @atlaspack/utils@3.2.7
  - @atlaspack/plugin@2.14.46

## 0.1.0

### Minor Changes

- [#932](https://github.com/atlassian-labs/atlaspack/pull/932) [`d9c93d9`](https://github.com/atlassian-labs/atlaspack/commit/d9c93d9fc21e4b4b41e8d59079a7fd1f748d4144) Thanks [@matt-koko](https://github.com/matt-koko)! - Insource compiled parcel transformers

### Patch Changes

- Updated dependencies []:
  - @atlaspack/utils@3.2.6
  - @atlaspack/plugin@2.14.45
