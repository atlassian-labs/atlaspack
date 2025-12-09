---
'@atlaspack/feature-flags': patch
'@atlaspack/transformer-js': patch
'@atlaspack/rust': patch
---

Add new JSX config system using the `@atlaspack/transformer-js` config in package.json

This feature is enabled when the `newJsxConfig` feature flag is enabled.

The new config will be fully responsible for providing JSX settings going forward, instead of inferring values from tsconfig.json and local package.json files.

| Option            | Type                     | Description                                                                                           |
| ----------------- | ------------------------ | ----------------------------------------------------------------------------------------------------- |
| pragma            | String                   | Specifies the JSX pragma to use (e.g., "React.createElement").                                        |
| pragma_fragment   | String                   | Specifies the JSX fragment pragma to use (e.g., "React.Fragment").                                    |
| import_source     | String                   | Specifies the import source for JSX. Only relevant for automatic runtime                              |
| automatic_runtime | `Boolean \| Array<Glob>` | Specifies the file that require the automatic runtime. Either Globs or a boolean to set for all files |
