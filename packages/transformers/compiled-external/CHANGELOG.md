# @atlaspack/transformer-compiled-external

## 0.2.1

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.2.2
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
  - @atlaspack/plugin@2.14.46

## 0.1.0

### Minor Changes

- [#932](https://github.com/atlassian-labs/atlaspack/pull/932) [`d9c93d9`](https://github.com/atlassian-labs/atlaspack/commit/d9c93d9fc21e4b4b41e8d59079a7fd1f748d4144) Thanks [@matt-koko](https://github.com/matt-koko)! - Insource compiled parcel transformers

### Patch Changes

- Updated dependencies []:
  - @atlaspack/plugin@2.14.45
