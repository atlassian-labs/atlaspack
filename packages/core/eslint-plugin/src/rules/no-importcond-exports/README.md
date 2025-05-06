# Do not export `importCond` (atlaspack/no-importcond-exports)

This rule bans directly exporting conditional import usages (`importCond` function calls) from a
file, which is an unsupported use case for conditional imports.

## Rule Details

The `importCond` function is used for conditional imports, which allows developers to switch between
two modules or components based on a feature gate value. It is a more performant alternative to
`componentWithFG` as the user only loads the module or component matching the feature gate value,
instead of loading both regardless of the feature gate value.

Directly exporting the `importCond` function call will still work in production, however it is
considered bad practice. Additionally, it will be evaluated too early (at module level) in unit
tests, causing feature gate mocks for `importCond` to not work properly.

Instead, developers should import the old and new versions of the components or modules directly in
the test file and test those instead.

👍 Examples of **correct** code for this rule:

```ts
// `importCond` is used within the file but not directly exported
const MyComponent = importCond<
  typeof import('./new.tsx'),
  typeof import('./old.tsx')
>('gate-name', './new.tsx', './old.tsx');

export function getComponent() {
  return MyComponent;
}
```

👎 Examples of **incorrect** code for this rule:

```ts
// Directly exporting `importCond` as a named export
export const MyComponent = importCond<
  typeof import('./new.tsx'),
  typeof import('./old.tsx')
>('gate-name', './new.tsx', './old.tsx');

// Directly exporting `importCond` as the default export
export default importCond<
  typeof import('./new.tsx'),
  typeof import('./old.tsx')
>('gate-name', './new.tsx', './old.tsx');

// Indirectly exporting a variable initialized with `importCond`
const MyComponent = importCond<
  typeof import('./new.tsx'),
  typeof import('./old.tsx')
>('gate-name', './new.tsx', './old.tsx');
export {MyComponent};
```

## Resources

- Conditional imports - adoption guide for devs
  - Atlassian employees: [go to the #atlaspack-contextual-imports](https://atlassian.enterprise.slack.com/archives/C07SP6N4FD5) channel and check the bookmarks
