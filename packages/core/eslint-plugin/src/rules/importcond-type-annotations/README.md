# Use correct type annotations for `importCond` (atlaspack/importcond-type-annotations)

This rule ensures that all `importCond` function calls have the correct type annotations.

## Rule Details

The `importCond` function is used for conditional imports, which allows developers to switch between
two modules or components based on a feature gate value. It is a more performant alternative to
`componentWithFG` as the user only loads the module or component matching the feature gate value,
instead of loading both regardless of the feature gate value.

Due to TypeScript limitations, we need to declare type annotations on every `importCond` usage in
order for type-checking to continue to work. These are defined between the `<` and `>`.

This ESLint rule ensures two things:

- that the type annotations exist, to make type-checking the component possible
- that the type annotations match the arguments passed to `importCond`, otherwise the type-checking
  will not be accurate.

👍 Examples of **correct** code for this rule:

```ts
const Component = importCond<
  typeof import('./new.tsx'),
  typeof import('./old.tsx')
>('gate_name', './new.tsx', './old.tsx');
```

👎 Examples of **incorrect** code for this rule:

```ts
// Missing type annotation
const Component = importCond('gate_name', './new.tsx', './old.tsx');

// Incorrect type annotation order
const Component = importCond<
  typeof import('./old.tsx'),
  typeof import('./new.tsx')
>('gate_name', './new.tsx', './old.tsx');

// Missing type annotation with extra spaces
const Component = importCond('gate_name', './new.tsx', './old.tsx');
```

## Resources

- Conditional imports - adoption guide for devs
  - Atlassian employees: [go to the #atlaspack-contextual-imports](https://atlassian.enterprise.slack.com/archives/C07SP6N4FD5) channel and check the bookmarks
