# @atlaspack/transformer-js

## 10.2.0

### Minor Changes

- [#838](https://github.com/atlassian-labs/atlaspack/pull/838) [`25aa273`](https://github.com/atlassian-labs/atlaspack/commit/25aa2735f4db920960de3d65d3a427775086b7d5) Thanks [@marcins](https://github.com/marcins)! - Update SWC from v10 -> v44

- [#916](https://github.com/atlassian-labs/atlaspack/pull/916) [`e55c80c`](https://github.com/atlassian-labs/atlaspack/commit/e55c80cd0ae3cc40356fe063cb7d45fbb9095356) Thanks [@OscarCookeAbbott](https://github.com/OscarCookeAbbott)! - Add Atlaspack V3 config parsing for `typeof_replacer`, `global_aliaser`, `lazy_loading_transformer`, `react_hooks_remover`, `react_async_import_lift`, `static_prevaluator`, `dead_returns_remover`, `unused_bindings_remover`

### Patch Changes

- [#924](https://github.com/atlassian-labs/atlaspack/pull/924) [`cbb66a5`](https://github.com/atlassian-labs/atlaspack/commit/cbb66a554f7ad679fd4dfc9e2bf2d5abb859f846) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add new JSX config system using the `@atlaspack/transformer-js` config in package.json

  This feature is enabled when the `newJsxConfig` feature flag is enabled.

  The new config will be fully responsible for providing JSX settings going forward, instead of inferring values from tsconfig.json and local package.json files.

  | Option           | Type                     | Description                                                                                           |
  | ---------------- | ------------------------ | ----------------------------------------------------------------------------------------------------- |
  | pragma           | String                   | Specifies the JSX pragma to use (e.g., "React.createElement").                                        |
  | pragmaFragment   | String                   | Specifies the JSX fragment pragma to use (e.g., "React.Fragment").                                    |
  | importSource     | String                   | Specifies the import source for JSX. Only relevant for automatic runtime                              |
  | automaticRuntime | `Boolean \| Array<Glob>` | Specifies the file that require the automatic runtime. Either Globs or a boolean to set for all files |

- Updated dependencies [[`787290a`](https://github.com/atlassian-labs/atlaspack/commit/787290a63b3f781e7af86097201dfdb5a36c8551), [`cbb66a5`](https://github.com/atlassian-labs/atlaspack/commit/cbb66a554f7ad679fd4dfc9e2bf2d5abb859f846), [`7815a93`](https://github.com/atlassian-labs/atlaspack/commit/7815a931b18983c3f34a39990b2f4b8b86855cc6), [`25aa273`](https://github.com/atlassian-labs/atlaspack/commit/25aa2735f4db920960de3d65d3a427775086b7d5), [`9468e16`](https://github.com/atlassian-labs/atlaspack/commit/9468e164e8ca20a5e8f552f5dc203aa3716cb154), [`e55c80c`](https://github.com/atlassian-labs/atlaspack/commit/e55c80cd0ae3cc40356fe063cb7d45fbb9095356)]:
  - @atlaspack/rust@3.14.0
  - @atlaspack/feature-flags@2.27.3
  - @atlaspack/core@2.29.2
  - @atlaspack/source-map@3.1.3
  - @atlaspack/utils@3.2.3
  - @atlaspack/workers@2.14.42
  - @atlaspack/plugin@2.14.42

## 10.1.0

### Minor Changes

- [#895](https://github.com/atlassian-labs/atlaspack/pull/895) [`96f1d7f`](https://github.com/atlassian-labs/atlaspack/commit/96f1d7f3efef12ce4f5966e34d00b299fcefb4dd) Thanks [@OscarCookeAbbott](https://github.com/OscarCookeAbbott)! - - Implement new dead_returns_remover transformer and hook up via opt-in flag
  - Implement new unused_bindings_remover transformer and hook up via opt-in flag

- [#895](https://github.com/atlassian-labs/atlaspack/pull/895) [`96f1d7f`](https://github.com/atlassian-labs/atlaspack/commit/96f1d7f3efef12ce4f5966e34d00b299fcefb4dd) Thanks [@OscarCookeAbbott](https://github.com/OscarCookeAbbott)! - Add new `react_hooks_remover` transformer
  Add new `static_prevaluator` transformer
  Rename, expand and add config for existing `global_this_aliaser` -> `global_aliaser`
  Refactor and expand existing `typeof_replacer` transformer
  Rename flags for the altered existing transformers

- [#902](https://github.com/atlassian-labs/atlaspack/pull/902) [`308e7ff`](https://github.com/atlassian-labs/atlaspack/commit/308e7ff979c05c1ae97fe6b08a65666b53d86cc7) Thanks [@OscarCookeAbbott](https://github.com/OscarCookeAbbott)! - - Implement new `react_async_import_lift` transformer
  - Hook up the new transformer with correct ordering in `lib.rs`

### Patch Changes

- Updated dependencies [[`96f1d7f`](https://github.com/atlassian-labs/atlaspack/commit/96f1d7f3efef12ce4f5966e34d00b299fcefb4dd), [`f6fc5e2`](https://github.com/atlassian-labs/atlaspack/commit/f6fc5e2acc30720f9a9676dfbc02bf780066653b), [`91738fd`](https://github.com/atlassian-labs/atlaspack/commit/91738fd77cb6b3b7acc93e799327ec283d27f7c0), [`38f3569`](https://github.com/atlassian-labs/atlaspack/commit/38f3569a7024357c43274666312bc6d024bfe5f9), [`3f6a3b8`](https://github.com/atlassian-labs/atlaspack/commit/3f6a3b8df4589fd4d65c072281c7f33bd37341d6), [`96f1d7f`](https://github.com/atlassian-labs/atlaspack/commit/96f1d7f3efef12ce4f5966e34d00b299fcefb4dd), [`d8024ce`](https://github.com/atlassian-labs/atlaspack/commit/d8024ce5c9ced1a9bc39144c979138be3a12b2fb), [`308e7ff`](https://github.com/atlassian-labs/atlaspack/commit/308e7ff979c05c1ae97fe6b08a65666b53d86cc7), [`d98c992`](https://github.com/atlassian-labs/atlaspack/commit/d98c992a56121a4f9f6a011ced5e103f96fe3999)]:
  - @atlaspack/rust@3.13.0
  - @atlaspack/source-map@3.1.2
  - @atlaspack/feature-flags@2.27.2
  - @atlaspack/core@2.29.1
  - @atlaspack/utils@3.2.2
  - @atlaspack/workers@2.14.41
  - @atlaspack/plugin@2.14.41

## 10.0.0

### Minor Changes

- [#888](https://github.com/atlassian-labs/atlaspack/pull/888) [`25fee72`](https://github.com/atlassian-labs/atlaspack/commit/25fee723d2228d0482ec8a8f1b8379ccd2977ce7) Thanks [@OscarCookeAbbott](https://github.com/OscarCookeAbbott)! - - Implement new dead_returns_remover transformer and hook up via opt-in flag
  - Implement new unused_bindings_remover transformer and hook up via opt-in flag

### Patch Changes

- [#881](https://github.com/atlassian-labs/atlaspack/pull/881) [`2f3af93`](https://github.com/atlassian-labs/atlaspack/commit/2f3af9374bcc082e6f4835f6f5a2b98e32ea45b4) Thanks [@marcins](https://github.com/marcins)! - Cleanup feature flag schemaValidationDeferSourceLoading

- [#876](https://github.com/atlassian-labs/atlaspack/pull/876) [`5817226`](https://github.com/atlassian-labs/atlaspack/commit/5817226eef6f194220b994ce71332fc8ff199f84) Thanks [@at-nathan](https://github.com/at-nathan)! - Add native functionality to handle transformations on dynamic imports

- Updated dependencies [[`a884bd2`](https://github.com/atlassian-labs/atlaspack/commit/a884bd2a6bd6f37d088fb886dd9b88e85328501b), [`25fee72`](https://github.com/atlassian-labs/atlaspack/commit/25fee723d2228d0482ec8a8f1b8379ccd2977ce7), [`5710532`](https://github.com/atlassian-labs/atlaspack/commit/57105322b26ce42af2cbfcc0506cff929e50b630), [`6fac01d`](https://github.com/atlassian-labs/atlaspack/commit/6fac01dbb1bfc5b971176c1a1c008297bf4b8f91), [`2f3af93`](https://github.com/atlassian-labs/atlaspack/commit/2f3af9374bcc082e6f4835f6f5a2b98e32ea45b4), [`432d571`](https://github.com/atlassian-labs/atlaspack/commit/432d571ea4e9fc67be12304fcafe8c445e844174), [`042cd95`](https://github.com/atlassian-labs/atlaspack/commit/042cd95a32ad38cd02d57aa3344c36d4e7ab8f9c), [`28f5424`](https://github.com/atlassian-labs/atlaspack/commit/28f5424f7e2080dd4fbbeb1bc5314a530197a23e), [`432d571`](https://github.com/atlassian-labs/atlaspack/commit/432d571ea4e9fc67be12304fcafe8c445e844174), [`a884bd2`](https://github.com/atlassian-labs/atlaspack/commit/a884bd2a6bd6f37d088fb886dd9b88e85328501b), [`5710532`](https://github.com/atlassian-labs/atlaspack/commit/57105322b26ce42af2cbfcc0506cff929e50b630), [`44eb8c3`](https://github.com/atlassian-labs/atlaspack/commit/44eb8c388d3e63f1ef5d835aee7aee5d1bb0620f)]:
  - @atlaspack/core@2.29.0
  - @atlaspack/rust@3.12.0
  - @atlaspack/feature-flags@2.27.1
  - @atlaspack/source-map@3.1.1
  - @atlaspack/utils@3.2.1
  - @atlaspack/workers@2.14.40
  - @atlaspack/plugin@2.14.40

## 9.0.0

### Minor Changes

- [#789](https://github.com/atlassian-labs/atlaspack/pull/789) [`73168c2`](https://github.com/atlassian-labs/atlaspack/commit/73168c275a5d9abff9907bcf536b340bca1ed5f0) Thanks [@marcins](https://github.com/marcins)! - Use an in-sourced version of @parcel/source-map, as @atlaspack/source-map

### Patch Changes

- Updated dependencies [[`7dd5bd8`](https://github.com/atlassian-labs/atlaspack/commit/7dd5bd89ff60c7b324347d772729eb4c3314f01e), [`c8a0e51`](https://github.com/atlassian-labs/atlaspack/commit/c8a0e51b1096d038a64644ad02fe68383f58682a), [`8eb84ee`](https://github.com/atlassian-labs/atlaspack/commit/8eb84ee61a42bfe87c58079b610802b07a6a13e4), [`71900f3`](https://github.com/atlassian-labs/atlaspack/commit/71900f3508d53c3882ca8ea71501ba3069687d4e), [`73168c2`](https://github.com/atlassian-labs/atlaspack/commit/73168c275a5d9abff9907bcf536b340bca1ed5f0), [`617a318`](https://github.com/atlassian-labs/atlaspack/commit/617a318ddc9419b38360257353fec50b9051ee13), [`2c4953e`](https://github.com/atlassian-labs/atlaspack/commit/2c4953e20dc1a6b012792081667aefeb9340505e), [`3fe7d4e`](https://github.com/atlassian-labs/atlaspack/commit/3fe7d4e821bee162fcbbaf9278b107a5eead944f)]:
  - @atlaspack/core@2.28.0
  - @atlaspack/rust@3.11.0
  - @atlaspack/feature-flags@2.27.0
  - @atlaspack/source-map@3.1.0
  - @atlaspack/utils@3.2.0
  - @atlaspack/plugin@2.14.39
  - @atlaspack/workers@2.14.39

## 8.0.0

### Minor Changes

- [#869](https://github.com/atlassian-labs/atlaspack/pull/869) [`cfb4707`](https://github.com/atlassian-labs/atlaspack/commit/cfb4707087498e4fa4dcf10753fe984a248d196b) Thanks [@benjervis](https://github.com/benjervis)! - Adds a feature to the V3 devloop that will minimise the amount of work done if the change is only a content change.

### Patch Changes

- [#868](https://github.com/atlassian-labs/atlaspack/pull/868) [`c7fe3f7`](https://github.com/atlassian-labs/atlaspack/commit/c7fe3f76f247e9e20299e205e2df0a16c418eaf2) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Fix issue where nested Promise.resolve calls mixed with dynamic imports could cause build errors

- [#875](https://github.com/atlassian-labs/atlaspack/pull/875) [`540f253`](https://github.com/atlassian-labs/atlaspack/commit/540f253dfdcd1a5caebbdc0b197319d439404aae) Thanks [@marcins](https://github.com/marcins)! - Allow configuring the `ascii_only` output of the `emit` function

- Updated dependencies [[`ec3abe4`](https://github.com/atlassian-labs/atlaspack/commit/ec3abe4dffc98560a850fd2f71fb566577e6c99c), [`c7fe3f7`](https://github.com/atlassian-labs/atlaspack/commit/c7fe3f76f247e9e20299e205e2df0a16c418eaf2), [`cfb4707`](https://github.com/atlassian-labs/atlaspack/commit/cfb4707087498e4fa4dcf10753fe984a248d196b), [`33d4c26`](https://github.com/atlassian-labs/atlaspack/commit/33d4c261ceb8d585d56b0a446ed6e28cf7f1126d), [`1468695`](https://github.com/atlassian-labs/atlaspack/commit/1468695fc0c9d06f060a6da9e9b0e154f11dff34), [`f31b041`](https://github.com/atlassian-labs/atlaspack/commit/f31b04107e9077c9946aadb99f6f91bb69703bb7), [`8180981`](https://github.com/atlassian-labs/atlaspack/commit/8180981be14c00f9570adb70d3f350bd91d6ec0a), [`ae77e74`](https://github.com/atlassian-labs/atlaspack/commit/ae77e7452a466b43b3fa5bed24d4ba26345ed765), [`f0a496f`](https://github.com/atlassian-labs/atlaspack/commit/f0a496f70fc8652e090cf1b3f6260e8cfbb796e2), [`119210b`](https://github.com/atlassian-labs/atlaspack/commit/119210b597eb993c50445df87a36b70bd49cd414), [`7d7a55d`](https://github.com/atlassian-labs/atlaspack/commit/7d7a55dd6395ec391a2e4c33b3dec0d1ea477d4c), [`540f253`](https://github.com/atlassian-labs/atlaspack/commit/540f253dfdcd1a5caebbdc0b197319d439404aae)]:
  - @atlaspack/feature-flags@2.26.2
  - @atlaspack/core@2.27.0
  - @atlaspack/rust@3.10.0
  - @atlaspack/utils@3.1.2
  - @atlaspack/workers@2.14.38
  - @atlaspack/plugin@2.14.38

## 7.1.1

### Patch Changes

- Updated dependencies [[`339b600`](https://github.com/atlassian-labs/atlaspack/commit/339b60095d755742cea50f811d08ccc44a4ae521), [`373ee2d`](https://github.com/atlassian-labs/atlaspack/commit/373ee2d3b5cc315df1e90e647619f915892e2b87)]:
  - @atlaspack/rust@3.9.1
  - @atlaspack/feature-flags@2.26.1
  - @atlaspack/core@2.26.2
  - @atlaspack/utils@3.1.1
  - @atlaspack/workers@2.14.37
  - @atlaspack/plugin@2.14.37

## 7.1.0

### Minor Changes

- [#837](https://github.com/atlassian-labs/atlaspack/pull/837) [`02f6aa1`](https://github.com/atlassian-labs/atlaspack/commit/02f6aa1906953fc184d2b49c905c2ef815cb878d) Thanks [@OscarCookeAbbott](https://github.com/OscarCookeAbbott)! - Add new `global_this_aliaser` transformer

- [#842](https://github.com/atlassian-labs/atlaspack/pull/842) [`525898e`](https://github.com/atlassian-labs/atlaspack/commit/525898e33cc229a4bc938ae853732be36d20c01a) Thanks [@OscarCookeAbbott](https://github.com/OscarCookeAbbott)! - Add new `lazy_loading` transformer

### Patch Changes

- [#847](https://github.com/atlassian-labs/atlaspack/pull/847) [`9cd9521`](https://github.com/atlassian-labs/atlaspack/commit/9cd9521978f783046e2ae4ce78f2de7aeb07d073) Thanks [@marcins](https://github.com/marcins)! - Implement a new transformer for Atlaskit Tokens, re-using infrastructure from the existing JS transformer.

- [#852](https://github.com/atlassian-labs/atlaspack/pull/852) [`5776be2`](https://github.com/atlassian-labs/atlaspack/commit/5776be21f70a3f2b9471ba33da3ba1a883f21f1a) Thanks [@marcins](https://github.com/marcins)! - Support passing a function for source property in schema validation to allow for deferred reads of files

- Updated dependencies [[`02f6aa1`](https://github.com/atlassian-labs/atlaspack/commit/02f6aa1906953fc184d2b49c905c2ef815cb878d), [`58527ee`](https://github.com/atlassian-labs/atlaspack/commit/58527eec15c1aebaaadbeb63586f3082c16beae3), [`525898e`](https://github.com/atlassian-labs/atlaspack/commit/525898e33cc229a4bc938ae853732be36d20c01a), [`9cd9521`](https://github.com/atlassian-labs/atlaspack/commit/9cd9521978f783046e2ae4ce78f2de7aeb07d073), [`cfb39a0`](https://github.com/atlassian-labs/atlaspack/commit/cfb39a0d729eb620cf2ca1611750a2bf7a080d08), [`d0c7bea`](https://github.com/atlassian-labs/atlaspack/commit/d0c7bea04458255b4c5d4299600e734b5f919fe1), [`1086c6a`](https://github.com/atlassian-labs/atlaspack/commit/1086c6a2c59271b63b3fb78e6acdb9d3a0dbf354), [`9cd9521`](https://github.com/atlassian-labs/atlaspack/commit/9cd9521978f783046e2ae4ce78f2de7aeb07d073), [`1086c6a`](https://github.com/atlassian-labs/atlaspack/commit/1086c6a2c59271b63b3fb78e6acdb9d3a0dbf354), [`5776be2`](https://github.com/atlassian-labs/atlaspack/commit/5776be21f70a3f2b9471ba33da3ba1a883f21f1a)]:
  - @atlaspack/rust@3.9.0
  - @atlaspack/feature-flags@2.26.0
  - @atlaspack/core@2.26.1
  - @atlaspack/utils@3.1.0
  - @atlaspack/workers@2.14.36
  - @atlaspack/plugin@2.14.36

## 7.0.0

### Patch Changes

- [#834](https://github.com/atlassian-labs/atlaspack/pull/834) [`3091fa3`](https://github.com/atlassian-labs/atlaspack/commit/3091fa36425ecffe3938e55247a035b08cda37a2) Thanks [@benjervis](https://github.com/benjervis)! - We no longer want to use `meta` fields internally in Atlaspack, so they have been removed from the Rust side of the codebase, and are mapped back to the legacy format on the JS side.

- Updated dependencies [[`79a7e22`](https://github.com/atlassian-labs/atlaspack/commit/79a7e22ef119891a2f4180a9d3cc66b38dd52092), [`ff8ac84`](https://github.com/atlassian-labs/atlaspack/commit/ff8ac84bd0be15caa7c7af21bf5b6e47b6d36438), [`3da8e69`](https://github.com/atlassian-labs/atlaspack/commit/3da8e690605472d81c531366b65fa9844f8ca337), [`c372f3f`](https://github.com/atlassian-labs/atlaspack/commit/c372f3fd6fce8200d5cf47f41bc7895c6cbb5558), [`17dfea9`](https://github.com/atlassian-labs/atlaspack/commit/17dfea9e9f3169807808520df9c09ebf70dc8b10), [`f1070ec`](https://github.com/atlassian-labs/atlaspack/commit/f1070ec48e99b4ec111a03179c5ab5b74163fab1), [`27fd73f`](https://github.com/atlassian-labs/atlaspack/commit/27fd73fa07d358653f1be6f2d15d72dc5d8fc2fd), [`3091fa3`](https://github.com/atlassian-labs/atlaspack/commit/3091fa36425ecffe3938e55247a035b08cda37a2)]:
  - @atlaspack/feature-flags@2.25.5
  - @atlaspack/rust@3.8.3
  - @atlaspack/core@2.26.0
  - @atlaspack/utils@3.0.3
  - @atlaspack/plugin@2.14.35
  - @atlaspack/workers@2.14.35

## 6.0.1

### Patch Changes

- Updated dependencies [[`25cbee6`](https://github.com/atlassian-labs/atlaspack/commit/25cbee625fb47ac20423fe34ff37bc818a807245), [`662d3c6`](https://github.com/atlassian-labs/atlaspack/commit/662d3c627888b16bf27df15cfac5a9094509e93d)]:
  - @atlaspack/workers@2.14.34
  - @atlaspack/feature-flags@2.25.4
  - @atlaspack/core@2.25.1
  - @atlaspack/utils@3.0.2
  - @atlaspack/plugin@2.14.34

## 6.0.0

### Patch Changes

- [#803](https://github.com/atlassian-labs/atlaspack/pull/803) [`f07351a`](https://github.com/atlassian-labs/atlaspack/commit/f07351a2398b6a8e5155a2daad77e69b7dcb2b4b) Thanks [@shanshrew](https://github.com/shanshrew)! - feat(core, transformers): add feature-flag to omit sourcesContent from memory; reduce peak memory during builds
  - Introduce `omitSourcesContentInMemory` feature flag to stop retaining full source text in `sourcesContent` throughout transforms. Default OFF; behavior unchanged unless enabled.
  - Guard `asset.sourceContent` initialization and `setSourceContent`/`sourcesContent` copies behind the flag.
  - Mappings and source paths remain correct; packager still inlines or references sources per config.

  Memory (three-js benchmark, V3, 1 run):
  - Baseline OFF: later, larger compactions near end of build (e.g. `~44.2s Mark-Compact 20.4 (50.2) -> 12.5 (53.5) MB`).
  - Flag ON: earlier compactions during transform/packaging, keeping old space ≈10–11 MB (e.g. `~17.7s Mark-Compact 11.5 (28.0) -> 9.6 (27.5) MB`).

  Sourcemaps: unchanged by default; with flag ON, only the in-memory retention is removed.

- Updated dependencies [[`236e546`](https://github.com/atlassian-labs/atlaspack/commit/236e5465863dca6044a7191e05260a5b924c342e), [`f07351a`](https://github.com/atlassian-labs/atlaspack/commit/f07351a2398b6a8e5155a2daad77e69b7dcb2b4b)]:
  - @atlaspack/utils@3.0.1
  - @atlaspack/core@2.25.0
  - @atlaspack/feature-flags@2.25.3
  - @atlaspack/workers@2.14.33
  - @atlaspack/plugin@2.14.33

## 5.0.2

### Patch Changes

- [#790](https://github.com/atlassian-labs/atlaspack/pull/790) [`0207171`](https://github.com/atlassian-labs/atlaspack/commit/0207171e59e985c51e105aec2e0a99de174374bd) Thanks [@benjervis](https://github.com/benjervis)! - Clean up the `unusedComputedPropertyFix` feature flag

- [#786](https://github.com/atlassian-labs/atlaspack/pull/786) [`bbe4e28`](https://github.com/atlassian-labs/atlaspack/commit/bbe4e28c89e5e9284bd730075d4d6f2a3cb37b21) Thanks [@benjervis](https://github.com/benjervis)! - Add `hmr_options` to the native transformer to prevent applying react refresh when it's not in use.

- [#800](https://github.com/atlassian-labs/atlaspack/pull/800) [`1180103`](https://github.com/atlassian-labs/atlaspack/commit/118010351ed444f8178988afb3f77807154dd933) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Fix errors when creating diagnostics with relative file paths

  BREAKING CHANGE: The `remapSourceLocation` API in `@atlaspack/utils` now requires the project root

- Updated dependencies [[`0207171`](https://github.com/atlassian-labs/atlaspack/commit/0207171e59e985c51e105aec2e0a99de174374bd), [`10ee3fa`](https://github.com/atlassian-labs/atlaspack/commit/10ee3fa5b75a92acde8973673d9b3c5b6f3958e5), [`bbe4e28`](https://github.com/atlassian-labs/atlaspack/commit/bbe4e28c89e5e9284bd730075d4d6f2a3cb37b21), [`1180103`](https://github.com/atlassian-labs/atlaspack/commit/118010351ed444f8178988afb3f77807154dd933), [`2bc93b1`](https://github.com/atlassian-labs/atlaspack/commit/2bc93b17cea07fd7cbb68acec84d8471345a22b4)]:
  - @atlaspack/feature-flags@2.25.2
  - @atlaspack/rust@3.8.2
  - @atlaspack/core@2.24.2
  - @atlaspack/utils@3.0.0
  - @atlaspack/workers@2.14.32
  - @atlaspack/plugin@2.14.32

## 5.0.1

### Patch Changes

- [#785](https://github.com/atlassian-labs/atlaspack/pull/785) [`0e7dd5e`](https://github.com/atlassian-labs/atlaspack/commit/0e7dd5ec6fbe05aa9e0bb5775a9d0975f206a922) Thanks [@matt-koko](https://github.com/matt-koko)! - We need to re-publish every package in Atlaspack with the corrected types field.

- Updated dependencies [[`0e7dd5e`](https://github.com/atlassian-labs/atlaspack/commit/0e7dd5ec6fbe05aa9e0bb5775a9d0975f206a922), [`686a398`](https://github.com/atlassian-labs/atlaspack/commit/686a398faa1f9e19cbf9274add2c46e246bafba8)]:
  - @atlaspack/core@2.24.1
  - @atlaspack/diagnostic@2.14.4
  - @atlaspack/feature-flags@2.25.1
  - @atlaspack/plugin@2.14.31
  - @atlaspack/rust@3.8.1
  - @atlaspack/utils@2.19.3
  - @atlaspack/workers@2.14.31

## 5.0.0

### Minor Changes

- [#774](https://github.com/atlassian-labs/atlaspack/pull/774) [`602f8ed`](https://github.com/atlassian-labs/atlaspack/commit/602f8ed9d8381301df8b2cc82c1d5cf6f2f94fec) Thanks [@JakeLane](https://github.com/JakeLane)! - Add feature flag exportsRebindingOptimisation which detects cases where we don't need the rebinding helper for esm exports

### Patch Changes

- [#780](https://github.com/atlassian-labs/atlaspack/pull/780) [`eedcbc4`](https://github.com/atlassian-labs/atlaspack/commit/eedcbc408fc1e86a2a8e25f1a41c57146d8529e1) Thanks [@JakeLane](https://github.com/JakeLane)! - Fix isStaticBindingSafe in v3

- Updated dependencies [[`7a52bab`](https://github.com/atlassian-labs/atlaspack/commit/7a52bab6281e8edbfa66af248eb6f872c6b4b7bd), [`eedcbc4`](https://github.com/atlassian-labs/atlaspack/commit/eedcbc408fc1e86a2a8e25f1a41c57146d8529e1), [`5a0363f`](https://github.com/atlassian-labs/atlaspack/commit/5a0363f1086e81068959b87e92064bed5a11a9b4), [`038e87a`](https://github.com/atlassian-labs/atlaspack/commit/038e87a7858d39556d59d3a2d17db534d45f62c6), [`f6532d7`](https://github.com/atlassian-labs/atlaspack/commit/f6532d7a4f7f007bd4e5e36af04dd466f0b9f572), [`0684997`](https://github.com/atlassian-labs/atlaspack/commit/0684997c127a6204af1376b36e556fddd1ba5557), [`602f8ed`](https://github.com/atlassian-labs/atlaspack/commit/602f8ed9d8381301df8b2cc82c1d5cf6f2f94fec), [`8604d2a`](https://github.com/atlassian-labs/atlaspack/commit/8604d2a49ace5e9194ec4f76e71f06b93610f6b4)]:
  - @atlaspack/feature-flags@2.25.0
  - @atlaspack/core@2.24.0
  - @atlaspack/rust@3.8.0
  - @atlaspack/utils@2.19.2
  - @atlaspack/plugin@2.14.30
  - @atlaspack/workers@2.14.30

## 4.0.5

### Patch Changes

- Updated dependencies [[`830261d`](https://github.com/atlassian-labs/atlaspack/commit/830261dcf4a40293090b61cab52fd75d5683c73f), [`53dd47b`](https://github.com/atlassian-labs/atlaspack/commit/53dd47bd6d23cd47f87297347f03a609ab38a03d)]:
  - @atlaspack/feature-flags@2.24.1
  - @atlaspack/rust@3.7.0
  - @atlaspack/core@2.23.5
  - @atlaspack/utils@2.19.1
  - @atlaspack/plugin@2.14.29
  - @atlaspack/workers@2.14.29

## 4.0.4

### Patch Changes

- Updated dependencies [[`f0349a6`](https://github.com/atlassian-labs/atlaspack/commit/f0349a6b9b04755088f121095ca6301a2ada3767), [`a5081c3`](https://github.com/atlassian-labs/atlaspack/commit/a5081c3f9207d28716ee4edd3841de1830ea4b80), [`f930003`](https://github.com/atlassian-labs/atlaspack/commit/f930003364fdc31e5487520ebe256f1a9389f579), [`f930003`](https://github.com/atlassian-labs/atlaspack/commit/f930003364fdc31e5487520ebe256f1a9389f579), [`8843980`](https://github.com/atlassian-labs/atlaspack/commit/88439807be20025fd3433380204ff1205079729e)]:
  - @atlaspack/feature-flags@2.24.0
  - @atlaspack/utils@2.19.0
  - @atlaspack/core@2.23.4
  - @atlaspack/workers@2.14.28
  - @atlaspack/plugin@2.14.28

## 4.0.3

### Patch Changes

- Updated dependencies [[`3cfb7cf`](https://github.com/atlassian-labs/atlaspack/commit/3cfb7cfd90b78a5c48ce717e779ff789769825a3)]:
  - @atlaspack/feature-flags@2.23.2
  - @atlaspack/rust@3.6.2
  - @atlaspack/core@2.23.3
  - @atlaspack/utils@2.18.4
  - @atlaspack/plugin@2.14.27
  - @atlaspack/workers@2.14.27

## 4.0.2

### Patch Changes

- [#742](https://github.com/atlassian-labs/atlaspack/pull/742) [`ee040bb`](https://github.com/atlassian-labs/atlaspack/commit/ee040bb6428f29b57d892ddd8107e29077d08ffd) Thanks [@yamadapc](https://github.com/yamadapc)! - Internal changes and bug fixes to environmentDeduplication flag

- Updated dependencies [[`ee040bb`](https://github.com/atlassian-labs/atlaspack/commit/ee040bb6428f29b57d892ddd8107e29077d08ffd), [`889c65c`](https://github.com/atlassian-labs/atlaspack/commit/889c65cd25b811045e26a117e7404f694dde77a2), [`b6e72b6`](https://github.com/atlassian-labs/atlaspack/commit/b6e72b6dd2b517cbb0e700fa69e63332d3eacb1e)]:
  - @atlaspack/feature-flags@2.23.1
  - @atlaspack/diagnostic@2.14.3
  - @atlaspack/workers@2.14.26
  - @atlaspack/plugin@2.14.26
  - @atlaspack/utils@2.18.3
  - @atlaspack/core@2.23.2
  - @atlaspack/rust@3.6.1

## 4.0.1

### Patch Changes

- Updated dependencies [[`f6b3f22`](https://github.com/atlassian-labs/atlaspack/commit/f6b3f2276c7e417580b49c4879563aab51f156b1)]:
  - @atlaspack/feature-flags@2.23.0
  - @atlaspack/core@2.23.1
  - @atlaspack/utils@2.18.2
  - @atlaspack/plugin@2.14.25
  - @atlaspack/workers@2.14.25

## 4.0.0

### Minor Changes

- [#733](https://github.com/atlassian-labs/atlaspack/pull/733) [`ad26146`](https://github.com/atlassian-labs/atlaspack/commit/ad26146f13b4c1cc65d4a0f9c67060b90ef14ff3) Thanks [@yamadapc](https://github.com/yamadapc)! - Add support for adding react displayName to components

- [#732](https://github.com/atlassian-labs/atlaspack/pull/732) [`7f5841c`](https://github.com/atlassian-labs/atlaspack/commit/7f5841c39df049f9546cccbeea2a7337e0337b45) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - add tesseract context

### Patch Changes

- Updated dependencies [[`ad26146`](https://github.com/atlassian-labs/atlaspack/commit/ad26146f13b4c1cc65d4a0f9c67060b90ef14ff3), [`f1b48e7`](https://github.com/atlassian-labs/atlaspack/commit/f1b48e7a04e005cef0f36a3e692087a9ecdb6f7a), [`7f5841c`](https://github.com/atlassian-labs/atlaspack/commit/7f5841c39df049f9546cccbeea2a7337e0337b45), [`73dd7ba`](https://github.com/atlassian-labs/atlaspack/commit/73dd7baab69456ef2f6e4a0cc7dbb04f407eb148)]:
  - @atlaspack/rust@3.6.0
  - @atlaspack/core@2.23.0
  - @atlaspack/feature-flags@2.22.0
  - @atlaspack/utils@2.18.1
  - @atlaspack/plugin@2.14.24
  - @atlaspack/workers@2.14.24

## 3.4.2

### Patch Changes

- Updated dependencies [[`23d561e`](https://github.com/atlassian-labs/atlaspack/commit/23d561e51e68b0c38fd1ff4e4fb173e5e7b01cf2)]:
  - @atlaspack/feature-flags@2.21.0
  - @atlaspack/utils@2.18.0
  - @atlaspack/rust@3.5.0
  - @atlaspack/workers@2.14.23
  - @atlaspack/plugin@2.14.23

## 3.4.1

### Patch Changes

- [#720](https://github.com/atlassian-labs/atlaspack/pull/720) [`d2fd849`](https://github.com/atlassian-labs/atlaspack/commit/d2fd849770fe6305e9c694bd97b1bd905abd9d94) Thanks [@alshdavid](https://github.com/alshdavid)! - Migrate to TypeScript

- Updated dependencies [[`d2fd849`](https://github.com/atlassian-labs/atlaspack/commit/d2fd849770fe6305e9c694bd97b1bd905abd9d94), [`12bee0e`](https://github.com/atlassian-labs/atlaspack/commit/12bee0e23f0464d7f6bd3e24fbe0d19c126d587d)]:
  - @atlaspack/feature-flags@2.20.1
  - @atlaspack/diagnostic@2.14.2
  - @atlaspack/workers@2.14.22
  - @atlaspack/plugin@2.14.22
  - @atlaspack/utils@2.17.4
  - @atlaspack/rust@3.4.2

## 3.4.0

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
  - @atlaspack/workers@2.14.21
  - @atlaspack/utils@2.17.3
  - @atlaspack/plugin@2.14.21

## 3.3.10

### Patch Changes

- [#702](https://github.com/atlassian-labs/atlaspack/pull/702) [`daaa768`](https://github.com/atlassian-labs/atlaspack/commit/daaa7688786772d7e3713b71c5bba6b89ec704aa) Thanks [@alshdavid](https://github.com/alshdavid)! - Fixes to Flow types

- Updated dependencies [[`daaa768`](https://github.com/atlassian-labs/atlaspack/commit/daaa7688786772d7e3713b71c5bba6b89ec704aa), [`1c7865a`](https://github.com/atlassian-labs/atlaspack/commit/1c7865a64451116d94015e248302435839d347c0), [`a0b959f`](https://github.com/atlassian-labs/atlaspack/commit/a0b959fbf61fc3f820ff03c7e8988945fe40a91a)]:
  - @atlaspack/plugin@2.14.20
  - @atlaspack/feature-flags@2.19.2
  - @atlaspack/utils@2.17.2
  - @atlaspack/workers@2.14.20

## 3.3.9

### Patch Changes

- Updated dependencies [[`13aef17`](https://github.com/atlassian-labs/atlaspack/commit/13aef177eea289a6e40d2113b5ec1ac9be18a33d)]:
  - @atlaspack/feature-flags@2.19.1
  - @atlaspack/utils@2.17.1
  - @atlaspack/workers@2.14.19
  - @atlaspack/plugin@2.14.19

## 3.3.8

### Patch Changes

- [#686](https://github.com/atlassian-labs/atlaspack/pull/686) [`becf977`](https://github.com/atlassian-labs/atlaspack/commit/becf977f625d5ee46dae3d4c679f173bf5f40cc0) Thanks [@benjervis](https://github.com/benjervis)! - Fix (behind a feature flag) the bug where non-static property access of an imported object was not being considered used by the collector.

- Updated dependencies [[`dbb4072`](https://github.com/atlassian-labs/atlaspack/commit/dbb40721ebeb45990a14ba04e6b44e7f836fb32d), [`becf977`](https://github.com/atlassian-labs/atlaspack/commit/becf977f625d5ee46dae3d4c679f173bf5f40cc0), [`becf977`](https://github.com/atlassian-labs/atlaspack/commit/becf977f625d5ee46dae3d4c679f173bf5f40cc0), [`c4415a4`](https://github.com/atlassian-labs/atlaspack/commit/c4415a455543d984ca28452c2cb87a794d22497c), [`f0f7c71`](https://github.com/atlassian-labs/atlaspack/commit/f0f7c7168a1d3d18c6f30d2daed611275692b7c5), [`de23e0c`](https://github.com/atlassian-labs/atlaspack/commit/de23e0ce49d5504fe3947ac26640a3d951087da3), [`18a57cf`](https://github.com/atlassian-labs/atlaspack/commit/18a57cf8a4789b2de5ad8e2676f317a26cc91417), [`a5ed1b4`](https://github.com/atlassian-labs/atlaspack/commit/a5ed1b414498560f393ff491af4da25b6e8dde56)]:
  - @atlaspack/feature-flags@2.19.0
  - @atlaspack/utils@2.17.0
  - @atlaspack/rust@3.4.1
  - @atlaspack/workers@2.14.18
  - @atlaspack/plugin@2.14.18

## 3.3.7

### Patch Changes

- Updated dependencies [[`c75bf55`](https://github.com/atlassian-labs/atlaspack/commit/c75bf553fff4decc285b5fd499a275853b18f8f2)]:
  - @atlaspack/rust@3.4.0
  - @atlaspack/utils@2.16.1
  - @atlaspack/workers@2.14.17
  - @atlaspack/plugin@2.14.17

## 3.3.6

### Patch Changes

- Updated dependencies [[`e8a60ff`](https://github.com/atlassian-labs/atlaspack/commit/e8a60ffbea41caef265786bbf73349771760081c), [`30ee2cf`](https://github.com/atlassian-labs/atlaspack/commit/30ee2cfcd34cf2646ded0eda13fdb80a2a5de529)]:
  - @atlaspack/feature-flags@2.18.4
  - @atlaspack/utils@2.16.0
  - @atlaspack/workers@2.14.16
  - @atlaspack/plugin@2.14.16

## 3.3.5

### Patch Changes

- Updated dependencies [[`5ded263`](https://github.com/atlassian-labs/atlaspack/commit/5ded263c7f11b866e8885b81c73e20dd060b25be)]:
  - @atlaspack/feature-flags@2.18.3
  - @atlaspack/utils@2.15.3
  - @atlaspack/workers@2.14.15
  - @atlaspack/plugin@2.14.15

## 3.3.4

### Patch Changes

- Updated dependencies [[`644b157`](https://github.com/atlassian-labs/atlaspack/commit/644b157dee72a871acc2d0facf0b87b8eea51956)]:
  - @atlaspack/feature-flags@2.18.2
  - @atlaspack/utils@2.15.2
  - @atlaspack/workers@2.14.14
  - @atlaspack/plugin@2.14.14

## 3.3.3

### Patch Changes

- [#633](https://github.com/atlassian-labs/atlaspack/pull/633) [`26aa9c5`](https://github.com/atlassian-labs/atlaspack/commit/26aa9c599d2be45ce1438a74c5fa22f39b9b554b) Thanks [@sbhuiyan-atlassian](https://github.com/sbhuiyan-atlassian)! - Ported various HMR changes from Parcel

- Updated dependencies [[`ef3d622`](https://github.com/atlassian-labs/atlaspack/commit/ef3d6228f4e006702198a19c61e051d194d325cb), [`26aa9c5`](https://github.com/atlassian-labs/atlaspack/commit/26aa9c599d2be45ce1438a74c5fa22f39b9b554b), [`0501255`](https://github.com/atlassian-labs/atlaspack/commit/05012550da35b05ce7d356a8cc29311e7f9afdca)]:
  - @atlaspack/workers@2.14.13
  - @atlaspack/feature-flags@2.18.1
  - @atlaspack/utils@2.15.1
  - @atlaspack/plugin@2.14.13

## 3.3.2

### Patch Changes

- Updated dependencies [[`10fbcfb`](https://github.com/atlassian-labs/atlaspack/commit/10fbcfbfa49c7a83da5d7c40983e36e87f524a75), [`85c52d3`](https://github.com/atlassian-labs/atlaspack/commit/85c52d3f7717b3c84a118d18ab98cfbfd71dcbd2), [`e39c6cf`](https://github.com/atlassian-labs/atlaspack/commit/e39c6cf05f7e95ce5420dbcea66f401b1cbd397c)]:
  - @atlaspack/feature-flags@2.18.0
  - @atlaspack/utils@2.15.0
  - @atlaspack/workers@2.14.12
  - @atlaspack/plugin@2.14.12

## 3.3.1

### Patch Changes

- Updated dependencies [[`73ea3c4`](https://github.com/atlassian-labs/atlaspack/commit/73ea3c4d85d4401fdd15abcbf988237e890e7ad3), [`b1b3693`](https://github.com/atlassian-labs/atlaspack/commit/b1b369317c66f8a431c170df2ebba4fa5b2e38ef)]:
  - @atlaspack/feature-flags@2.17.0
  - @atlaspack/utils@2.14.11
  - @atlaspack/workers@2.14.11
  - @atlaspack/plugin@2.14.11

## 3.3.0

### Minor Changes

- [#601](https://github.com/atlassian-labs/atlaspack/pull/601) [`1e32d4e`](https://github.com/atlassian-labs/atlaspack/commit/1e32d4eae6b3af3968e8a0ef97d35b4347fd4196) Thanks [@yamadapc](https://github.com/yamadapc)! - Improve granular configuration file invalidations

### Patch Changes

- Updated dependencies [[`35fdd4b`](https://github.com/atlassian-labs/atlaspack/commit/35fdd4b52da0af20f74667f7b8adfb2f90279b7c), [`6dd4ccb`](https://github.com/atlassian-labs/atlaspack/commit/6dd4ccb753541de32322d881f973d571dd57e4ca)]:
  - @atlaspack/rust@3.3.5
  - @atlaspack/workers@2.14.10
  - @atlaspack/plugin@2.14.10
  - @atlaspack/utils@2.14.10

## 3.2.7

### Patch Changes

- Updated dependencies [[`124b7ff`](https://github.com/atlassian-labs/atlaspack/commit/124b7fff44f71aac9fbad289a9a9509b3dfc9aaa), [`e052521`](https://github.com/atlassian-labs/atlaspack/commit/e0525210850ed1606146eb86991049cf567c5dec), [`15c6d70`](https://github.com/atlassian-labs/atlaspack/commit/15c6d7000bd89da876bc590aa75b17a619a41896), [`e4d966c`](https://github.com/atlassian-labs/atlaspack/commit/e4d966c3c9c4292c5013372ae65b10d19d4bacc6), [`209692f`](https://github.com/atlassian-labs/atlaspack/commit/209692ffb11eae103a0d65c5e1118a5aa1625818), [`42a775d`](https://github.com/atlassian-labs/atlaspack/commit/42a775de8eec638ad188f3271964170d8c04d84b), [`29c2f10`](https://github.com/atlassian-labs/atlaspack/commit/29c2f106de9679adfb5afa04e1910471dc65a427), [`f4da1e1`](https://github.com/atlassian-labs/atlaspack/commit/f4da1e120e73eeb5e8b8927f05e88f04d6148c7b), [`1ef91fc`](https://github.com/atlassian-labs/atlaspack/commit/1ef91fcc863fdd2831511937083dbbc1263b3d9d)]:
  - @atlaspack/rust@3.3.4
  - @atlaspack/feature-flags@2.16.0
  - @atlaspack/utils@2.14.9
  - @atlaspack/workers@2.14.9
  - @atlaspack/plugin@2.14.9

## 3.2.6

### Patch Changes

- Updated dependencies [[`30f6017`](https://github.com/atlassian-labs/atlaspack/commit/30f60175ba4d272c5fc193973c63bc298584775b), [`1ab0a27`](https://github.com/atlassian-labs/atlaspack/commit/1ab0a275aeca40350415e2b03e7440d1dddc6228), [`b8a4ae8`](https://github.com/atlassian-labs/atlaspack/commit/b8a4ae8f83dc0a83d8b145c5f729936ce52080a3)]:
  - @atlaspack/feature-flags@2.15.1
  - @atlaspack/rust@3.3.3
  - @atlaspack/utils@2.14.8
  - @atlaspack/workers@2.14.8
  - @atlaspack/plugin@2.14.8

## 3.2.5

### Patch Changes

- Updated dependencies [[`a1773d2`](https://github.com/atlassian-labs/atlaspack/commit/a1773d2a62d0ef7805ac7524621dcabcc1afe929), [`556d6ab`](https://github.com/atlassian-labs/atlaspack/commit/556d6ab8ede759fa7f37fcd3f4da336ef1c55e8f)]:
  - @atlaspack/feature-flags@2.15.0
  - @atlaspack/rust@3.3.2
  - @atlaspack/utils@2.14.7
  - @atlaspack/workers@2.14.7
  - @atlaspack/plugin@2.14.7

## 3.2.4

### Patch Changes

- Updated dependencies [[`e0f5337`](https://github.com/atlassian-labs/atlaspack/commit/e0f533757bd1019dbd108a04952c87da15286e09)]:
  - @atlaspack/feature-flags@2.14.4
  - @atlaspack/rust@3.3.1
  - @atlaspack/utils@2.14.6
  - @atlaspack/workers@2.14.6
  - @atlaspack/plugin@2.14.6

## 3.2.3

### Patch Changes

- Updated dependencies [[`11d6f16`](https://github.com/atlassian-labs/atlaspack/commit/11d6f16b6397dee2f217167e5c98b39edb63f7a7), [`e2ba0f6`](https://github.com/atlassian-labs/atlaspack/commit/e2ba0f69702656f3d1ce95ab1454e35062b13b39), [`d2c50c2`](https://github.com/atlassian-labs/atlaspack/commit/d2c50c2c020888b33bb25b8690d9320c2b69e2a6), [`46a90dc`](https://github.com/atlassian-labs/atlaspack/commit/46a90dccd019a26b222c878a92d23acc75dc67c5)]:
  - @atlaspack/feature-flags@2.14.3
  - @atlaspack/rust@3.3.0
  - @atlaspack/utils@2.14.5
  - @atlaspack/workers@2.14.5
  - @atlaspack/plugin@2.14.5

## 3.2.2

### Patch Changes

- Updated dependencies [[`1a2c14c`](https://github.com/atlassian-labs/atlaspack/commit/1a2c14c3cd4587551cc12e94d0680c8b71ea12bf), [`cb9da16`](https://github.com/atlassian-labs/atlaspack/commit/cb9da16fb2648e7f53c64df0313f60d5fb8970cc)]:
  - @atlaspack/rust@3.2.0
  - @atlaspack/utils@2.14.4
  - @atlaspack/workers@2.14.4
  - @atlaspack/plugin@2.14.4

## 3.2.1

### Patch Changes

- Updated dependencies [[`f27d39e`](https://github.com/atlassian-labs/atlaspack/commit/f27d39e767b06def059944b3bc5fd50797eaea96)]:
  - @atlaspack/rust@3.1.1
  - @atlaspack/utils@2.14.3
  - @atlaspack/workers@2.14.3
  - @atlaspack/plugin@2.14.3

## 3.2.0

### Minor Changes

- [#491](https://github.com/atlassian-labs/atlaspack/pull/491) [`a891d65`](https://github.com/atlassian-labs/atlaspack/commit/a891d652bc4eb3d757d381adf65c5083f706effc) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Support ignore comments for node replacements

  Adding `#__ATLASPACK_IGNORE__` before `__filename` and `__dirname` will now disable the default node replacement behaviour of these variables. This is useful when you want your compiled output to be aware of it's runtime directory rather than it's pre-compiled source directory.

  ```js
  const dirname = /*#__ATLASPACK_IGNORE__*/ __dirname;
  ```

### Patch Changes

- Updated dependencies [[`9b85d3e`](https://github.com/atlassian-labs/atlaspack/commit/9b85d3e645b10bd027eed2304afc970a5ba40062), [`a891d65`](https://github.com/atlassian-labs/atlaspack/commit/a891d652bc4eb3d757d381adf65c5083f706effc), [`d02eab9`](https://github.com/atlassian-labs/atlaspack/commit/d02eab95eb60bf7457e0869af0b773608592c0e6), [`fb87a90`](https://github.com/atlassian-labs/atlaspack/commit/fb87a901973776b33ca4ce530e9d71669a9bd36d), [`7b9e8cf`](https://github.com/atlassian-labs/atlaspack/commit/7b9e8cf29e01a98e72e46b2b2fb74ccc514f4463), [`17b9579`](https://github.com/atlassian-labs/atlaspack/commit/17b9579484eced0ed8f23e2aba6d23b3c7238c39), [`8f4e6c1`](https://github.com/atlassian-labs/atlaspack/commit/8f4e6c1b0e7c1fd48624afda48c1dcc599f1460f)]:
  - @atlaspack/feature-flags@2.14.2
  - @atlaspack/rust@3.1.0
  - @atlaspack/utils@2.14.2
  - @atlaspack/workers@2.14.2
  - @atlaspack/plugin@2.14.2

## 3.1.1

### Patch Changes

- [#450](https://github.com/atlassian-labs/atlaspack/pull/450) [`b9d41b1`](https://github.com/atlassian-labs/atlaspack/commit/b9d41b175ad5771651a5b0278a5a0147e669234a) Thanks [@benjervis](https://github.com/benjervis)! - Remove the Atlaspack engines compatibility check

- [#478](https://github.com/atlassian-labs/atlaspack/pull/478) [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b) Thanks [@yamadapc](https://github.com/yamadapc)! - The first attempt at Version Packages didn't include the built artifacts.
  This has hopefully been fixed, so this change will force those packages to re-release.
- Updated dependencies [[`80bd57b`](https://github.com/atlassian-labs/atlaspack/commit/80bd57b9f9e966563957dee0780d956a682eb2d4), [`ae70b81`](https://github.com/atlassian-labs/atlaspack/commit/ae70b810384cf58f9c57d341ab4c925c7bb2060c), [`ce13d5e`](https://github.com/atlassian-labs/atlaspack/commit/ce13d5e885d55518ee6318e7a72e3a6e4e5126f2), [`4aab060`](https://github.com/atlassian-labs/atlaspack/commit/4aab0605c0d4ee8e0dcc3ffa1162eae5b360b677), [`c0a61a9`](https://github.com/atlassian-labs/atlaspack/commit/c0a61a92405b6830fe39cc17622cc2e97bf02dd7), [`cb35e7d`](https://github.com/atlassian-labs/atlaspack/commit/cb35e7d2b90b372de8401792915f12f410508d24), [`e1422ad`](https://github.com/atlassian-labs/atlaspack/commit/e1422ad0a801faaa4bc4f1023bed042ffe236e9b), [`6ec11f1`](https://github.com/atlassian-labs/atlaspack/commit/6ec11f10a9366fb8a9fc0475c7678235056bd80e), [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b)]:
  - @atlaspack/rust@3.0.1
  - @atlaspack/utils@2.14.1
  - @atlaspack/feature-flags@2.14.1
  - @atlaspack/diagnostic@2.14.1
  - @atlaspack/plugin@2.14.1
  - @atlaspack/workers@2.14.1

## 3.1.0

### Minor Changes

- [#339](https://github.com/atlassian-labs/atlaspack/pull/339) [`bfe81e5`](https://github.com/atlassian-labs/atlaspack/commit/bfe81e551c4e4bb2cac7fc4745222e66962c1728) Thanks [@yamadapc](https://github.com/yamadapc)! - Update cache invalidation metrics with build type

### Patch Changes

- [#382](https://github.com/atlassian-labs/atlaspack/pull/382) [`3460531`](https://github.com/atlassian-labs/atlaspack/commit/3460531d9cb036f2575a99ea69fe2b03cfd6ac06) Thanks [@MonicaOlejniczak](https://github.com/MonicaOlejniczak)! - Replace Fold with VisitMut in InlineFS to improve performance

- [#427](https://github.com/atlassian-labs/atlaspack/pull/427) [`a317453`](https://github.com/atlassian-labs/atlaspack/commit/a317453432b7f30e98f2a4cbcafdaa5601bcde63) Thanks [@alshdavid](https://github.com/alshdavid)! - Enabled Rust rust_2018_idioms lints and updated files to match linting rules

- [#397](https://github.com/atlassian-labs/atlaspack/pull/397) [`f600560`](https://github.com/atlassian-labs/atlaspack/commit/f6005601be5ceacb52350c065070feb5649461e9) Thanks [@MonicaOlejniczak](https://github.com/MonicaOlejniczak)! - Replace Fold with VisitMut in EsmToCjsReplacer to improve performance

- [#394](https://github.com/atlassian-labs/atlaspack/pull/394) [`f13a53f`](https://github.com/atlassian-labs/atlaspack/commit/f13a53fa37def8d4c8b2fc4b596066e7595441dc) Thanks [@MonicaOlejniczak](https://github.com/MonicaOlejniczak)! - Rename ESMFold to EsmToCjsReplacer and lift fold up one level

- [#379](https://github.com/atlassian-labs/atlaspack/pull/379) [`8bc3db9`](https://github.com/atlassian-labs/atlaspack/commit/8bc3db94cc7382b22ca8207c92af8f6389c17e2e) Thanks [@JakeLane](https://github.com/JakeLane)! - Support nested conditional imports in runtime for dynamic import edges in graph. Introduces a new feature flag `conditionalBundlingNestedRuntime`

- [#414](https://github.com/atlassian-labs/atlaspack/pull/414) [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53) Thanks [@alshdavid](https://github.com/alshdavid)! - Added type:commonjs to package.json files

- [#386](https://github.com/atlassian-labs/atlaspack/pull/386) [`3b43acf`](https://github.com/atlassian-labs/atlaspack/commit/3b43acfe15523a2614413b294785e33a6060e41e) Thanks [@MonicaOlejniczak](https://github.com/MonicaOlejniczak)! - Replace Fold with VisitMut in the dependency collector to improve performance

- [#407](https://github.com/atlassian-labs/atlaspack/pull/407) [`6c0f7a7`](https://github.com/atlassian-labs/atlaspack/commit/6c0f7a7378131e8705e2b10af1576cc207271577) Thanks [@MonicaOlejniczak](https://github.com/MonicaOlejniczak)! - Refactor collect and hoist tests

- Updated dependencies [[`bfe81e5`](https://github.com/atlassian-labs/atlaspack/commit/bfe81e551c4e4bb2cac7fc4745222e66962c1728), [`fa4fcf6`](https://github.com/atlassian-labs/atlaspack/commit/fa4fcf69a82b0a3727066ada6e93a149b259936e), [`cd964ee`](https://github.com/atlassian-labs/atlaspack/commit/cd964eed5a330ae63733656ded691d1ea3afe4e3), [`1953d1b`](https://github.com/atlassian-labs/atlaspack/commit/1953d1bec266a39dc4bfce5f6c7959e77e63411e), [`ce4ce95`](https://github.com/atlassian-labs/atlaspack/commit/ce4ce953914e08991cf58c70c98f758690e5ee21), [`1de1c81`](https://github.com/atlassian-labs/atlaspack/commit/1de1c8138fbe4d38a64aa1f3c22a70aad59fb5bb), [`28dee1d`](https://github.com/atlassian-labs/atlaspack/commit/28dee1db7d9a995161b45f76c1a03b80ccaeab4b), [`a317453`](https://github.com/atlassian-labs/atlaspack/commit/a317453432b7f30e98f2a4cbcafdaa5601bcde63), [`2055adb`](https://github.com/atlassian-labs/atlaspack/commit/2055adbe31de792e2a2a591b94d2f33f50735879), [`8bc3db9`](https://github.com/atlassian-labs/atlaspack/commit/8bc3db94cc7382b22ca8207c92af8f6389c17e2e), [`cfa1c63`](https://github.com/atlassian-labs/atlaspack/commit/cfa1c63d710c5f9c9abc55f34220b70fb517c3b8), [`17427a2`](https://github.com/atlassian-labs/atlaspack/commit/17427a2b2fc9c34ef0b941907c2868edef6d1507), [`e962cd7`](https://github.com/atlassian-labs/atlaspack/commit/e962cd735877f7f16163e60868d70d9c10054ebe), [`104a46a`](https://github.com/atlassian-labs/atlaspack/commit/104a46a5ee1fae176d29fcc6420d6bd9c01b35b1), [`9572aca`](https://github.com/atlassian-labs/atlaspack/commit/9572aca2a2313a3c05551f73e556128e77a37732), [`34b740d`](https://github.com/atlassian-labs/atlaspack/commit/34b740d4e2449fba7b50cb9708c56d8033dca5b9), [`4837b69`](https://github.com/atlassian-labs/atlaspack/commit/4837b6988e56ca842a24797b796160964d3696ce), [`e5fa92d`](https://github.com/atlassian-labs/atlaspack/commit/e5fa92de26c87fb5d4d681af1931451749ba970a), [`7e21377`](https://github.com/atlassian-labs/atlaspack/commit/7e21377914e8091d484f67cb11052a1efd2227e3), [`43113f8`](https://github.com/atlassian-labs/atlaspack/commit/43113f8f00232c5a52169a3f11f846d6e4d94b0a), [`3650f7c`](https://github.com/atlassian-labs/atlaspack/commit/3650f7c9ab803b5ae20b223e82b2268a1b614e43), [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53), [`c2ef915`](https://github.com/atlassian-labs/atlaspack/commit/c2ef915dc54784ce4b8180025ac1b2e13b375002), [`f635123`](https://github.com/atlassian-labs/atlaspack/commit/f635123f9a06961bc5e053e237f1023f10800ea3), [`4812d0f`](https://github.com/atlassian-labs/atlaspack/commit/4812d0f7400af0f8416f1b7175ecb87700860a68), [`80d963e`](https://github.com/atlassian-labs/atlaspack/commit/80d963ed950f5d742ebd78014cf74f3c65cd4474), [`8fae5f3`](https://github.com/atlassian-labs/atlaspack/commit/8fae5f3005bd7c806b175b4df1754abf58922591), [`cc66aaa`](https://github.com/atlassian-labs/atlaspack/commit/cc66aaa66d67dd0cb89e083f387a278e74aad3f0), [`67df3f1`](https://github.com/atlassian-labs/atlaspack/commit/67df3f1af1432d77ee6b8850010d976d3313693a), [`0c3ad7a`](https://github.com/atlassian-labs/atlaspack/commit/0c3ad7a302330da1d5e3c025963cc583eb5c28ed)]:
  - @atlaspack/diagnostic@2.14.0
  - @atlaspack/feature-flags@2.14.0
  - @atlaspack/plugin@2.14.0
  - @atlaspack/rust@3.0.0
  - @atlaspack/utils@2.14.0
  - @atlaspack/workers@2.14.0

## 3.0.1

### Patch Changes

- [`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06) Thanks [@yamadapc](https://github.com/yamadapc)! - Add identifier registry and VCS tracing

- Updated dependencies [[`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06)]:
  - @atlaspack/feature-flags@2.13.1
  - @atlaspack/diagnostic@2.13.1
  - @atlaspack/workers@2.13.1
  - @atlaspack/plugin@2.13.1
  - @atlaspack/utils@2.13.1
  - @atlaspack/core@2.13.1
  - @atlaspack/rust@2.13.1

## 3.0.0

### Minor Changes

- [#335](https://github.com/atlassian-labs/atlaspack/pull/335) [`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf) Thanks [@yamadapc](https://github.com/yamadapc)! - Initial changeset release

### Patch Changes

- Updated dependencies [[`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf)]:
  - @atlaspack/core@2.13.0
  - @atlaspack/diagnostic@2.13.0
  - @atlaspack/feature-flags@2.13.0
  - @atlaspack/plugin@2.13.0
  - @atlaspack/rust@2.13.0
  - @atlaspack/utils@2.13.0
  - @atlaspack/workers@2.13.0
