# @atlaspack/feature-flags

## 2.27.7

### Patch Changes

- [#921](https://github.com/atlassian-labs/atlaspack/pull/921) [`f33f9c4`](https://github.com/atlassian-labs/atlaspack/commit/f33f9c48dd24b319df352d197e4a83cbb1b053bc) Thanks [@JakeLane](https://github.com/JakeLane)! - Update Compiled CSS-in-JS SWC transformer to be more accurate

- [#962](https://github.com/atlassian-labs/atlaspack/pull/962) [`e15fb6c`](https://github.com/atlassian-labs/atlaspack/commit/e15fb6c885c6354c6c02283de35ce18abc8c9e18) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - adds flag to remove server file check in tesseract resolver

## 2.27.6

### Patch Changes

- [#953](https://github.com/atlassian-labs/atlaspack/pull/953) [`c31090c`](https://github.com/atlassian-labs/atlaspack/commit/c31090c9025f35d3fa8561b42dca170853a32e6f) Thanks [@marcins](https://github.com/marcins)! - Cleanup feature flag precomputeReferencedAssets

- [#960](https://github.com/atlassian-labs/atlaspack/pull/960) [`565bab3`](https://github.com/atlassian-labs/atlaspack/commit/565bab3771cc334659d873cabff4cdfac0860cc7) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add LargeMap to work around Node 24's Map size limit in build cache serializer.

  This change is behind the `useLargeMapInBuildCache` feature flag.

## 2.27.5

### Patch Changes

- [#934](https://github.com/atlassian-labs/atlaspack/pull/934) [`02cc8b3`](https://github.com/atlassian-labs/atlaspack/commit/02cc8b32c06ca6b51806b33f6f707ca06e55e957) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add experimental native persistent cache for Atlaspack V3.

## 2.27.4

### Patch Changes

- [#935](https://github.com/atlassian-labs/atlaspack/pull/935) [`793e78c`](https://github.com/atlassian-labs/atlaspack/commit/793e78c68eed25032e686f70cd77ea3601409b35) Thanks [@benjervis](https://github.com/benjervis)! - Add a flag to skip symbol propagation for the runtime graph

## 2.27.3

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

## 2.27.2

### Patch Changes

- [#908](https://github.com/atlassian-labs/atlaspack/pull/908) [`91738fd`](https://github.com/atlassian-labs/atlaspack/commit/91738fd77cb6b3b7acc93e799327ec283d27f7c0) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Disallows circular package aliases during resolution. This fixes cases where package A aliases to package B, but package B imports package A, causing infinite loops during resolution. Instead, we disable the alias allow package B to import the real package A.

  This is useful in cases where you create wrapper packages that re-export another package under a different name.

  Enabled via the `disallowCircularPackageAliases` feature flag.

## 2.27.1

### Patch Changes

- [#881](https://github.com/atlassian-labs/atlaspack/pull/881) [`2f3af93`](https://github.com/atlassian-labs/atlaspack/commit/2f3af9374bcc082e6f4835f6f5a2b98e32ea45b4) Thanks [@marcins](https://github.com/marcins)! - Cleanup feature flag schemaValidationDeferSourceLoading

## 2.27.0

### Minor Changes

- [#858](https://github.com/atlassian-labs/atlaspack/pull/858) [`8eb84ee`](https://github.com/atlassian-labs/atlaspack/commit/8eb84ee61a42bfe87c58079b610802b07a6a13e4) Thanks [@marcins](https://github.com/marcins)! - Introduce a new `getReferencedAssets(bundle)` method to the BundleGraph to pre-compute referenced assets, this is used by the scope hoisting packager behind a new `precomputeReferencedAssets` feature flag.

- [#789](https://github.com/atlassian-labs/atlaspack/pull/789) [`73168c2`](https://github.com/atlassian-labs/atlaspack/commit/73168c275a5d9abff9907bcf536b340bca1ed5f0) Thanks [@marcins](https://github.com/marcins)! - Use an in-sourced version of @parcel/source-map, as @atlaspack/source-map

## 2.26.2

### Patch Changes

- [#873](https://github.com/atlassian-labs/atlaspack/pull/873) [`ec3abe4`](https://github.com/atlassian-labs/atlaspack/commit/ec3abe4dffc98560a850fd2f71fb566577e6c99c) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - cleanup deduplicateReporters flag

- [#868](https://github.com/atlassian-labs/atlaspack/pull/868) [`c7fe3f7`](https://github.com/atlassian-labs/atlaspack/commit/c7fe3f76f247e9e20299e205e2df0a16c418eaf2) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Fix issue where nested Promise.resolve calls mixed with dynamic imports could cause build errors

## 2.26.1

### Patch Changes

- [#863](https://github.com/atlassian-labs/atlaspack/pull/863) [`373ee2d`](https://github.com/atlassian-labs/atlaspack/commit/373ee2d3b5cc315df1e90e647619f915892e2b87) Thanks [@matt-koko](https://github.com/matt-koko)! - Revert clean up of resolver optimisation feature-flag.
  Reverts PR: 'Clean-up resolver optimisation #816'

## 2.26.0

### Minor Changes

- [#846](https://github.com/atlassian-labs/atlaspack/pull/846) [`58527ee`](https://github.com/atlassian-labs/atlaspack/commit/58527eec15c1aebaaadbeb63586f3082c16beae3) Thanks [@JakeLane](https://github.com/JakeLane)! - Creating a new temp package @atlaspack/transformer-compiled-css-in-js to aid in native migration from babel

- [#777](https://github.com/atlassian-labs/atlaspack/pull/777) [`cfb39a0`](https://github.com/atlassian-labs/atlaspack/commit/cfb39a0d729eb620cf2ca1611750a2bf7a080d08) Thanks [@matt-koko](https://github.com/matt-koko)! - Added logic to rust JS transformer to account for JSX transformations in scenarios when the file extension of the asset is NOT .jsx or .tsx. The logic to determine whether the file should be treated as JSX syntax now mirrors that of the existing v2 behaviour. Several unit tests and integration tests have been introduced to ensure this parity is maintained.

### Patch Changes

- [#844](https://github.com/atlassian-labs/atlaspack/pull/844) [`d0c7bea`](https://github.com/atlassian-labs/atlaspack/commit/d0c7bea04458255b4c5d4299600e734b5f919fe1) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - deduplicate reporters from config

- [#847](https://github.com/atlassian-labs/atlaspack/pull/847) [`9cd9521`](https://github.com/atlassian-labs/atlaspack/commit/9cd9521978f783046e2ae4ce78f2de7aeb07d073) Thanks [@marcins](https://github.com/marcins)! - Implement a new transformer for Atlaskit Tokens, re-using infrastructure from the existing JS transformer.

- [#852](https://github.com/atlassian-labs/atlaspack/pull/852) [`5776be2`](https://github.com/atlassian-labs/atlaspack/commit/5776be21f70a3f2b9471ba33da3ba1a883f21f1a) Thanks [@marcins](https://github.com/marcins)! - Support passing a function for source property in schema validation to allow for deferred reads of files

## 2.25.5

### Patch Changes

- [#816](https://github.com/atlassian-labs/atlaspack/pull/816) [`79a7e22`](https://github.com/atlassian-labs/atlaspack/commit/79a7e22ef119891a2f4180a9d3cc66b38dd52092) Thanks [@yamadapc](https://github.com/yamadapc)! - Clean-up resolver optimisation feature-flag

- [#817](https://github.com/atlassian-labs/atlaspack/pull/817) [`3da8e69`](https://github.com/atlassian-labs/atlaspack/commit/3da8e690605472d81c531366b65fa9844f8ca337) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Fix an issue when creating the BundleGraph and some Dependency.sourceAssetId values have diverged from the actual Asset.id values.

  Fix behind flag `sourceAssetIdBundleGraphFix`

- [#840](https://github.com/atlassian-labs/atlaspack/pull/840) [`17dfea9`](https://github.com/atlassian-labs/atlaspack/commit/17dfea9e9f3169807808520df9c09ebf70dc8b10) Thanks [@vykimnguyen](https://github.com/vykimnguyen)! - cleanup inlineStringReplacmentPerf flag

- [#828](https://github.com/atlassian-labs/atlaspack/pull/828) [`f1070ec`](https://github.com/atlassian-labs/atlaspack/commit/f1070ec48e99b4ec111a03179c5ab5b74163fab1) Thanks [@marcins](https://github.com/marcins)! - Cleanup customEnvInTargets feature flag

## 2.25.4

### Patch Changes

- [#811](https://github.com/atlassian-labs/atlaspack/pull/811) [`662d3c6`](https://github.com/atlassian-labs/atlaspack/commit/662d3c627888b16bf27df15cfac5a9094509e93d) Thanks [@yamadapc](https://github.com/yamadapc)! - Clean-up feature-flag

## 2.25.3

### Patch Changes

- [#803](https://github.com/atlassian-labs/atlaspack/pull/803) [`f07351a`](https://github.com/atlassian-labs/atlaspack/commit/f07351a2398b6a8e5155a2daad77e69b7dcb2b4b) Thanks [@shanshrew](https://github.com/shanshrew)! - feat(core, transformers): add feature-flag to omit sourcesContent from memory; reduce peak memory during builds
  - Introduce `omitSourcesContentInMemory` feature flag to stop retaining full source text in `sourcesContent` throughout transforms. Default OFF; behavior unchanged unless enabled.
  - Guard `asset.sourceContent` initialization and `setSourceContent`/`sourcesContent` copies behind the flag.
  - Mappings and source paths remain correct; packager still inlines or references sources per config.

  Memory (three-js benchmark, V3, 1 run):
  - Baseline OFF: later, larger compactions near end of build (e.g. `~44.2s Mark-Compact 20.4 (50.2) -> 12.5 (53.5) MB`).
  - Flag ON: earlier compactions during transform/packaging, keeping old space ≈10–11 MB (e.g. `~17.7s Mark-Compact 11.5 (28.0) -> 9.6 (27.5) MB`).

  Sourcemaps: unchanged by default; with flag ON, only the in-memory retention is removed.

## 2.25.2

### Patch Changes

- [#790](https://github.com/atlassian-labs/atlaspack/pull/790) [`0207171`](https://github.com/atlassian-labs/atlaspack/commit/0207171e59e985c51e105aec2e0a99de174374bd) Thanks [@benjervis](https://github.com/benjervis)! - Clean up the `unusedComputedPropertyFix` feature flag

- [#791](https://github.com/atlassian-labs/atlaspack/pull/791) [`10ee3fa`](https://github.com/atlassian-labs/atlaspack/commit/10ee3fa5b75a92acde8973673d9b3c5b6f3958e5) Thanks [@benjervis](https://github.com/benjervis)! - Remove emptyFileStarRexportFix and make the flagged behaviour standard.

  See PR [#685][pr] for details on the change.

  [pr]: https://github.com/atlassian-labs/atlaspack/pull/685

- [#800](https://github.com/atlassian-labs/atlaspack/pull/800) [`1180103`](https://github.com/atlassian-labs/atlaspack/commit/118010351ed444f8178988afb3f77807154dd933) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Fix errors when creating diagnostics with relative file paths

  BREAKING CHANGE: The `remapSourceLocation` API in `@atlaspack/utils` now requires the project root

- [#781](https://github.com/atlassian-labs/atlaspack/pull/781) [`2bc93b1`](https://github.com/atlassian-labs/atlaspack/commit/2bc93b17cea07fd7cbb68acec84d8471345a22b4) Thanks [@matt-koko](https://github.com/matt-koko)! - add attribution to feature flags and eslint rule to enforce attribution convention

## 2.25.1

### Patch Changes

- [#785](https://github.com/atlassian-labs/atlaspack/pull/785) [`0e7dd5e`](https://github.com/atlassian-labs/atlaspack/commit/0e7dd5ec6fbe05aa9e0bb5775a9d0975f206a922) Thanks [@matt-koko](https://github.com/matt-koko)! - We need to re-publish every package in Atlaspack with the corrected types field.

## 2.25.0

### Minor Changes

- [#767](https://github.com/atlassian-labs/atlaspack/pull/767) [`038e87a`](https://github.com/atlassian-labs/atlaspack/commit/038e87a7858d39556d59d3a2d17db534d45f62c6) Thanks [@marcins](https://github.com/marcins)! - Implement a new feature `allowExplicitTargetEntries` where entries provided as entries can be filtered by the source property in targets to allow different targets to be built for different subsets of entries.

- [#769](https://github.com/atlassian-labs/atlaspack/pull/769) [`f6532d7`](https://github.com/atlassian-labs/atlaspack/commit/f6532d7a4f7f007bd4e5e36af04dd466f0b9f572) Thanks [@marcins](https://github.com/marcins)! - Adds a new feature to support adding custom `env` properties in a target, that are available in the Environment during the build. A new feature flag `customEnvInTargets` has been added to gate this feature.

- [#774](https://github.com/atlassian-labs/atlaspack/pull/774) [`602f8ed`](https://github.com/atlassian-labs/atlaspack/commit/602f8ed9d8381301df8b2cc82c1d5cf6f2f94fec) Thanks [@JakeLane](https://github.com/JakeLane)! - Add feature flag exportsRebindingOptimisation which detects cases where we don't need the rebinding helper for esm exports

### Patch Changes

- [#779](https://github.com/atlassian-labs/atlaspack/pull/779) [`7a52bab`](https://github.com/atlassian-labs/atlaspack/commit/7a52bab6281e8edbfa66af248eb6f872c6b4b7bd) Thanks [@marcins](https://github.com/marcins)! - Add new feature flag `preserveUnstableSingleFileOutputInCss` which when enabled will ensure the `unstableSingleFileOutput` property on the asset environment is preserved when transforming CSS.

## 2.24.1

### Patch Changes

- [#763](https://github.com/atlassian-labs/atlaspack/pull/763) [`830261d`](https://github.com/atlassian-labs/atlaspack/commit/830261dcf4a40293090b61cab52fd75d5683c73f) Thanks [@benjervis](https://github.com/benjervis)! - Fixes scope hoisting by iterating from the main entry asset of a bundle when packaging. Behind the feature flag `applyScopeHoistingImprovementsV2`.

## 2.24.0

### Minor Changes

- [#753](https://github.com/atlassian-labs/atlaspack/pull/753) [`f0349a6`](https://github.com/atlassian-labs/atlaspack/commit/f0349a6b9b04755088f121095ca6301a2ada3767) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Add configurable async bundle merging and redundant shared bundle removal behind a feature flag.

### Patch Changes

- [#762](https://github.com/atlassian-labs/atlaspack/pull/762) [`a5081c3`](https://github.com/atlassian-labs/atlaspack/commit/a5081c3f9207d28716ee4edd3841de1830ea4b80) Thanks [@JakeLane](https://github.com/JakeLane)! - Add feature flag 'useNewPrelude' which enables a micro-optimisation removing a 'for in' call and replacing with undefined check

- [#760](https://github.com/atlassian-labs/atlaspack/pull/760) [`f930003`](https://github.com/atlassian-labs/atlaspack/commit/f930003364fdc31e5487520ebe256f1a9389f579) Thanks [@marcins](https://github.com/marcins)! - Fix TypeScript errors in various packages

- [#760](https://github.com/atlassian-labs/atlaspack/pull/760) [`f930003`](https://github.com/atlassian-labs/atlaspack/commit/f930003364fdc31e5487520ebe256f1a9389f579) Thanks [@marcins](https://github.com/marcins)! - Fix typescript errors

- [#761](https://github.com/atlassian-labs/atlaspack/pull/761) [`8843980`](https://github.com/atlassian-labs/atlaspack/commit/88439807be20025fd3433380204ff1205079729e) Thanks [@marcins](https://github.com/marcins)! - Adds a new feature flag `singleFileOutputStableName` - when enabled, bundles produced by the experimental single file output bundler will have stable names (i.e. no hash).

## 2.23.2

### Patch Changes

- [#757](https://github.com/atlassian-labs/atlaspack/pull/757) [`3cfb7cf`](https://github.com/atlassian-labs/atlaspack/commit/3cfb7cfd90b78a5c48ce717e779ff789769825a3) Thanks [@marcins](https://github.com/marcins)! - Clean up the `inlineIsolatedScripts` feature flag

## 2.23.1

### Patch Changes

- [#742](https://github.com/atlassian-labs/atlaspack/pull/742) [`ee040bb`](https://github.com/atlassian-labs/atlaspack/commit/ee040bb6428f29b57d892ddd8107e29077d08ffd) Thanks [@yamadapc](https://github.com/yamadapc)! - Internal changes and bug fixes to environmentDeduplication flag

## 2.23.0

### Minor Changes

- [#745](https://github.com/atlassian-labs/atlaspack/pull/745) [`f6b3f22`](https://github.com/atlassian-labs/atlaspack/commit/f6b3f2276c7e417580b49c4879563aab51f156b1) Thanks [@matt-koko](https://github.com/matt-koko)! - Use ATLASPACK_BUILD_ENV instead of NODE_ENV to determine if Atlaspack is being run in the context of Atlaspack tests.

## 2.22.0

### Minor Changes

- [#741](https://github.com/atlassian-labs/atlaspack/pull/741) [`73dd7ba`](https://github.com/atlassian-labs/atlaspack/commit/73dd7baab69456ef2f6e4a0cc7dbb04f407eb148) Thanks [@matt-koko](https://github.com/matt-koko)! - combine FeatureFlags and DEFAULT_FEATURE_FLAGS into one object

## 2.21.0

### Minor Changes

- [#731](https://github.com/atlassian-labs/atlaspack/pull/731) [`23d561e`](https://github.com/atlassian-labs/atlaspack/commit/23d561e51e68b0c38fd1ff4e4fb173e5e7b01cf2) Thanks [@marcins](https://github.com/marcins)! - Implement "inline isolated" scripts

## 2.20.1

### Patch Changes

- [#720](https://github.com/atlassian-labs/atlaspack/pull/720) [`d2fd849`](https://github.com/atlassian-labs/atlaspack/commit/d2fd849770fe6305e9c694bd97b1bd905abd9d94) Thanks [@alshdavid](https://github.com/alshdavid)! - Migrate to TypeScript

- [#725](https://github.com/atlassian-labs/atlaspack/pull/725) [`12bee0e`](https://github.com/atlassian-labs/atlaspack/commit/12bee0e23f0464d7f6bd3e24fbe0d19c126d587d) Thanks [@marcins](https://github.com/marcins)! - Clean up `atlaspackV3CleanShutdown` feature flag.

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

## 2.19.2

### Patch Changes

- [#706](https://github.com/atlassian-labs/atlaspack/pull/706) [`1c7865a`](https://github.com/atlassian-labs/atlaspack/commit/1c7865a64451116d94015e248302435839d347c0) Thanks [@yamadapc](https://github.com/yamadapc)! - Clean-up feature-flag

- [#707](https://github.com/atlassian-labs/atlaspack/pull/707) [`a0b959f`](https://github.com/atlassian-labs/atlaspack/commit/a0b959fbf61fc3f820ff03c7e8988945fe40a91a) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix content key not found exceptions when bundling is aborted after a unsafe to incrementally bundle asset graph request

## 2.19.1

### Patch Changes

- [#692](https://github.com/atlassian-labs/atlaspack/pull/692) [`13aef17`](https://github.com/atlassian-labs/atlaspack/commit/13aef177eea289a6e40d2113b5ec1ac9be18a33d) Thanks [@JakeLane](https://github.com/JakeLane)! - Add fallback behaviour when conditional bundle is missing

## 2.19.0

### Minor Changes

- [#640](https://github.com/atlassian-labs/atlaspack/pull/640) [`dbb4072`](https://github.com/atlassian-labs/atlaspack/commit/dbb40721ebeb45990a14ba04e6b44e7f836fb32d) Thanks [@JakeLane](https://github.com/JakeLane)! - Clean up conditional bundling feature flags

- [#686](https://github.com/atlassian-labs/atlaspack/pull/686) [`becf977`](https://github.com/atlassian-labs/atlaspack/commit/becf977f625d5ee46dae3d4c679f173bf5f40cc0) Thanks [@benjervis](https://github.com/benjervis)! - Adds a feature flag for unusedComputedPropertyFix, to enable the fix for computed keys accessed on imports not being marked as used

- [#693](https://github.com/atlassian-labs/atlaspack/pull/693) [`18a57cf`](https://github.com/atlassian-labs/atlaspack/commit/18a57cf8a4789b2de5ad8e2676f317a26cc91417) Thanks [@mattcompiles](https://github.com/mattcompiles)! - These packages should have been bumped in [pull request 691](https://github.com/atlassian-labs/atlaspack/pull/691).

  Rectifying by creating a new changeset now.

### Patch Changes

- [#690](https://github.com/atlassian-labs/atlaspack/pull/690) [`c4415a4`](https://github.com/atlassian-labs/atlaspack/commit/c4415a455543d984ca28452c2cb87a794d22497c) Thanks [@yamadapc](https://github.com/yamadapc)! - Bug fix for build abort state corruption

- [#685](https://github.com/atlassian-labs/atlaspack/pull/685) [`f0f7c71`](https://github.com/atlassian-labs/atlaspack/commit/f0f7c7168a1d3d18c6f30d2daed611275692b7c5) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Fixes an issue where star re-exports of empty files (usually occurring in compiled typescript libraries) could cause exports to undefined at runtime.
  Fix is behind the feature-flag `emptyFileStarRexportFix`.

## 2.18.4

### Patch Changes

- [#661](https://github.com/atlassian-labs/atlaspack/pull/661) [`e8a60ff`](https://github.com/atlassian-labs/atlaspack/commit/e8a60ffbea41caef265786bbf73349771760081c) Thanks [@marcins](https://github.com/marcins)! - Add new feature flag atlaspackV3CleanShutdown which will dispose of the NAPI worker pool when disposing of the Atlaspack class

## 2.18.3

### Patch Changes

- [#655](https://github.com/atlassian-labs/atlaspack/pull/655) [`5ded263`](https://github.com/atlassian-labs/atlaspack/commit/5ded263c7f11b866e8885b81c73e20dd060b25be) Thanks [@yamadapc](https://github.com/yamadapc)! - Clean-up inline requires multi-threading feature-flag

## 2.18.2

### Patch Changes

- [#652](https://github.com/atlassian-labs/atlaspack/pull/652) [`644b157`](https://github.com/atlassian-labs/atlaspack/commit/644b157dee72a871acc2d0facf0b87b8eea51956) Thanks [@yamadapc](https://github.com/yamadapc)! - Fix bugs related to build aborts. Builds and cache writes will no longer be aborted.

## 2.18.1

### Patch Changes

- [#633](https://github.com/atlassian-labs/atlaspack/pull/633) [`26aa9c5`](https://github.com/atlassian-labs/atlaspack/commit/26aa9c599d2be45ce1438a74c5fa22f39b9b554b) Thanks [@sbhuiyan-atlassian](https://github.com/sbhuiyan-atlassian)! - Ported various HMR changes from Parcel

- [#626](https://github.com/atlassian-labs/atlaspack/pull/626) [`0501255`](https://github.com/atlassian-labs/atlaspack/commit/05012550da35b05ce7d356a8cc29311e7f9afdca) Thanks [@yamadapc](https://github.com/yamadapc)! - Clean-up tsconfig invalidation improvements feature-flag

## 2.18.0

### Minor Changes

- [#627](https://github.com/atlassian-labs/atlaspack/pull/627) [`85c52d3`](https://github.com/atlassian-labs/atlaspack/commit/85c52d3f7717b3c84a118d18ab98cfbfd71dcbd2) Thanks [@benjervis](https://github.com/benjervis)! - Adds a feature flag for `applyScopeHoistingImprovement`

### Patch Changes

- [#632](https://github.com/atlassian-labs/atlaspack/pull/632) [`10fbcfb`](https://github.com/atlassian-labs/atlaspack/commit/10fbcfbfa49c7a83da5d7c40983e36e87f524a75) Thanks [@marcins](https://github.com/marcins)! - Added a new feature flag `inlineConstOptimisationFix` which when enabled changes the behaviour for output of constant modules. This fixes two issues with constant modules:
  - Previously constant modules, if they needed a namespace anywhere, would have a namespace everywhere, with this change they only have a namespace in the bundles where needed.
  - Previously in the case of wrapped assets, a constant module dependnecy of that wrapped asset would be rendered after the module - which meant the minifier would not be able to inline the constants safely. With this flag all constant modules are rendered at the top of the bundle.

## 2.17.0

### Minor Changes

- [#619](https://github.com/atlassian-labs/atlaspack/pull/619) [`73ea3c4`](https://github.com/atlassian-labs/atlaspack/commit/73ea3c4d85d4401fdd15abcbf988237e890e7ad3) Thanks [@matt-koko](https://github.com/matt-koko)! - export `CONSISTENCY_CHECK_VALUES` for consumption in other products

### Patch Changes

- [#623](https://github.com/atlassian-labs/atlaspack/pull/623) [`b1b3693`](https://github.com/atlassian-labs/atlaspack/commit/b1b369317c66f8a431c170df2ebba4fa5b2e38ef) Thanks [@JakeLane](https://github.com/JakeLane)! - Load same conditional bundles as conditional manifest in HTML

## 2.16.0

### Minor Changes

- [#582](https://github.com/atlassian-labs/atlaspack/pull/582) [`f4da1e1`](https://github.com/atlassian-labs/atlaspack/commit/f4da1e120e73eeb5e8b8927f05e88f04d6148c7b) Thanks [@matt-koko](https://github.com/matt-koko)! - Export DEFAULT_FEATURE_FLAGS so it will be included in the associate type declaration file and able to be imported elsewhere.

  This will enable patterns like:

  ```
  import type { FeatureFlags } from '@atlaspack/feature-flags';
  import { DEFAULT_FEATURE_FLAGS } from '@atlaspack/feature-flags';
  ```

### Patch Changes

- [#503](https://github.com/atlassian-labs/atlaspack/pull/503) [`209692f`](https://github.com/atlassian-labs/atlaspack/commit/209692ffb11eae103a0d65c5e1118a5aa1625818) Thanks [@JakeLane](https://github.com/JakeLane)! - Fix conditional bundling reporter when condition is reused

## 2.15.1

### Patch Changes

- [#551](https://github.com/atlassian-labs/atlaspack/pull/551) [`30f6017`](https://github.com/atlassian-labs/atlaspack/commit/30f60175ba4d272c5fc193973c63bc298584775b) Thanks [@yamadapc](https://github.com/yamadapc)! - Log request tracker invalidation counts on start-up

## 2.15.0

### Minor Changes

- [#547](https://github.com/atlassian-labs/atlaspack/pull/547) [`a1773d2`](https://github.com/atlassian-labs/atlaspack/commit/a1773d2a62d0ef7805ac7524621dcabcc1afe929) Thanks [@benjervis](https://github.com/benjervis)! - Add a feature flag for resolving the configuration for `@atlaspack/bundler-default` from CWD, rather than exclusively from the project root.

## 2.14.4

### Patch Changes

- [#542](https://github.com/atlassian-labs/atlaspack/pull/542) [`e0f5337`](https://github.com/atlassian-labs/atlaspack/commit/e0f533757bd1019dbd108a04952c87da15286e09) Thanks [@yamadapc](https://github.com/yamadapc)! - Add feature-flagged option to use rayon thread-pool to optimize inline requires

## 2.14.3

### Patch Changes

- [#511](https://github.com/atlassian-labs/atlaspack/pull/511) [`11d6f16`](https://github.com/atlassian-labs/atlaspack/commit/11d6f16b6397dee2f217167e5c98b39edb63f7a7) Thanks [@yamadapc](https://github.com/yamadapc)! - Clean-up dylib worker threads segmentation fault bug fix feature-flag

## 2.14.2

### Patch Changes

- [#494](https://github.com/atlassian-labs/atlaspack/pull/494) [`9b85d3e`](https://github.com/atlassian-labs/atlaspack/commit/9b85d3e645b10bd027eed2304afc970a5ba40062) Thanks [@JakeLane](https://github.com/JakeLane)! - When conditionalBundlingReporterDuplicateFix is enabled, avoid duplicated writes to the descriptor and logging

- [#510](https://github.com/atlassian-labs/atlaspack/pull/510) [`17b9579`](https://github.com/atlassian-labs/atlaspack/commit/17b9579484eced0ed8f23e2aba6d23b3c7238c39) Thanks [@yamadapc](https://github.com/yamadapc)! - Add unused feature-flag for cache rework changes

- [#512](https://github.com/atlassian-labs/atlaspack/pull/512) [`8f4e6c1`](https://github.com/atlassian-labs/atlaspack/commit/8f4e6c1b0e7c1fd48624afda48c1dcc599f1460f) Thanks [@yamadapc](https://github.com/yamadapc)! - Remove LMDB cache back-end

## 2.14.1

### Patch Changes

- [#388](https://github.com/atlassian-labs/atlaspack/pull/388) [`4aab060`](https://github.com/atlassian-labs/atlaspack/commit/4aab0605c0d4ee8e0dcc3ffa1162eae5b360b677) Thanks [@yamadapc](https://github.com/yamadapc)! - Set LMDB.js Lite as the default cache back-end

- [#420](https://github.com/atlassian-labs/atlaspack/pull/420) [`e1422ad`](https://github.com/atlassian-labs/atlaspack/commit/e1422ad0a801faaa4bc4f1023bed042ffe236e9b) Thanks [@JakeLane](https://github.com/JakeLane)! - Support async script runtime in conditional bundling

- [#478](https://github.com/atlassian-labs/atlaspack/pull/478) [`570493b`](https://github.com/atlassian-labs/atlaspack/commit/570493beaf754e7985aebc7daaaf6dfcfa8fe56b) Thanks [@yamadapc](https://github.com/yamadapc)! - The first attempt at Version Packages didn't include the built artifacts.
  This has hopefully been fixed, so this change will force those packages to re-release.

## 2.14.0

### Minor Changes

- [#339](https://github.com/atlassian-labs/atlaspack/pull/339) [`bfe81e5`](https://github.com/atlassian-labs/atlaspack/commit/bfe81e551c4e4bb2cac7fc4745222e66962c1728) Thanks [@yamadapc](https://github.com/yamadapc)! - Update cache invalidation metrics with build type

- [#352](https://github.com/atlassian-labs/atlaspack/pull/352) [`e962cd7`](https://github.com/atlassian-labs/atlaspack/commit/e962cd735877f7f16163e60868d70d9c10054ebe) Thanks [@pancaspe87](https://github.com/pancaspe87)! - Introduced new method to return feature flag value

- [#358](https://github.com/atlassian-labs/atlaspack/pull/358) [`7e21377`](https://github.com/atlassian-labs/atlaspack/commit/7e21377914e8091d484f67cb11052a1efd2227e3) Thanks [@benjervis](https://github.com/benjervis)! - Add a new feature flag to enable the side effect scanning

### Patch Changes

- [#413](https://github.com/atlassian-labs/atlaspack/pull/413) [`1953d1b`](https://github.com/atlassian-labs/atlaspack/commit/1953d1bec266a39dc4bfce5f6c7959e77e63411e) Thanks [@pancaspe87](https://github.com/pancaspe87)! - Patch absolute paths

- [#378](https://github.com/atlassian-labs/atlaspack/pull/378) [`ce4ce95`](https://github.com/atlassian-labs/atlaspack/commit/ce4ce953914e08991cf58c70c98f758690e5ee21) Thanks [@yamadapc](https://github.com/yamadapc)! - Add feature-flagged optimization to resolver specifier handling

- [#379](https://github.com/atlassian-labs/atlaspack/pull/379) [`8bc3db9`](https://github.com/atlassian-labs/atlaspack/commit/8bc3db94cc7382b22ca8207c92af8f6389c17e2e) Thanks [@JakeLane](https://github.com/JakeLane)! - Support nested conditional imports in runtime for dynamic import edges in graph. Introduces a new feature flag `conditionalBundlingNestedRuntime`

- [#429](https://github.com/atlassian-labs/atlaspack/pull/429) [`4837b69`](https://github.com/atlassian-labs/atlaspack/commit/4837b6988e56ca842a24797b796160964d3696ce) Thanks [@mattcompiles](https://github.com/mattcompiles)! - Increase performance of inline bundle packaging (enabled via `featureFlags.inlineStringReplacementPerf`)

- [#415](https://github.com/atlassian-labs/atlaspack/pull/415) [`43113f8`](https://github.com/atlassian-labs/atlaspack/commit/43113f8f00232c5a52169a3f11f846d6e4d94b0a) Thanks [@yamadapc](https://github.com/yamadapc)! - Add feature-flag to fix supporting source-maps for inline bundles

- [#414](https://github.com/atlassian-labs/atlaspack/pull/414) [`eff9809`](https://github.com/atlassian-labs/atlaspack/commit/eff98093703b9999a511b87a19562f5aaccfcb53) Thanks [@alshdavid](https://github.com/alshdavid)! - Added type:commonjs to package.json files

## 2.13.1

### Patch Changes

- [`3ddd868`](https://github.com/atlassian-labs/atlaspack/commit/3ddd8682a6edb5c6a35357cfa3ade5741aff5f06) Thanks [@yamadapc](https://github.com/yamadapc)! - Add identifier registry and VCS tracing

## 2.13.0

### Minor Changes

- [#335](https://github.com/atlassian-labs/atlaspack/pull/335) [`b4dbd4d`](https://github.com/atlassian-labs/atlaspack/commit/b4dbd4d5b23d1b7aa3fcdf59cc7bc8bedd3a59cf) Thanks [@yamadapc](https://github.com/yamadapc)! - Initial changeset release
