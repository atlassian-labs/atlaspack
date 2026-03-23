# @atlaspack/transformer-compiled-external

## 0.2.15

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.3.5
  - @atlaspack/plugin@2.14.61

## 0.2.14

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.3.4
  - @atlaspack/plugin@2.14.60

## 0.2.13

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.3.3
  - @atlaspack/plugin@2.14.59

## 0.2.12

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.3.2
  - @atlaspack/plugin@2.14.58

## 0.2.11

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.3.1
  - @atlaspack/plugin@2.14.57

## 0.2.10

### Patch Changes

- Updated dependencies [[`de388ff`](https://github.com/atlassian-labs/atlaspack/commit/de388ff76d39dece97ad475fcccdb6efb6283bfc)]:
  - @atlaspack/source-map@3.3.0
  - @atlaspack/plugin@2.14.56

## 0.2.9

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.2.10
  - @atlaspack/plugin@2.14.55

## 0.2.8

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.2.9
  - @atlaspack/plugin@2.14.54

## 0.2.7

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.2.8
  - @atlaspack/plugin@2.14.53

## 0.2.6

### Patch Changes

- Updated dependencies []:
  - @atlaspack/plugin@2.14.52
  - @atlaspack/source-map@3.2.7

## 0.2.5

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.2.6
  - @atlaspack/plugin@2.14.51

## 0.2.4

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.2.5
  - @atlaspack/plugin@2.14.50

## 0.2.3

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.2.4
  - @atlaspack/plugin@2.14.49

## 0.2.2

### Patch Changes

- Updated dependencies []:
  - @atlaspack/source-map@3.2.3
  - @atlaspack/plugin@2.14.48

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
