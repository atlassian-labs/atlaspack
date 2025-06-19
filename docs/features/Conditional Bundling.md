# Conditional imports

This is a deep dive into how conditional imports works in Atlaspack. This should cover all the information for platform developers to integrate it into their own products, in order to allow product developers to use it for their features.

If you wish to use conditional imports in Jira, you should use our adoption guide instead. Search "Conditional imports in Jira - adoption guide for devs" on Confluence, or check out the adoption guide in the pinned messages in `#atlaspack-contextual-imports`.

## What are conditional imports?

### The problem

Take the following example code:

```tsx
// my-component.js

import TrueComponent from './true-component.js';
import FalseComponent from './false-component.js';

const MyComponent = fg('my-feature-gate-name') ? TrueComponent : FalseComponent;

// We would then use MyComponent as part of another component, or perhaps export it for other files to use.
```

This is an example of **conditional code**: if `fg('my-feature-gate-name')` is true, we want to run one block of code, and if `fg('my-feature-gate-name')` is false, then we want to run another, non-overlapping block of code.

`fg('my-feature-gate-name')` evaluates a Statsig feature gate (equivalent to `FeatureGates.checkGate`), but potentially in the future, it might also include something like a Statsig experiment value (or something else!).

If:

- `TrueComponent` and `FalseComponent` are quite large, or
- `TrueComponent` has dependencies (e.g. it imports code from other places) that `FalseComponent` doesn't have (or vice versa)

then the user will end up downloading code that they never need to execute. For example, if `fg('my-feature-gate-name')` is true, then they will never need `FalseComponent`, but the user is still forced to download it.

### The solution

Atlaspack's conditional imports API solve this problem by providing a way to notify the Atlaspack bundler that there is conditional code. At build time, Atlaspack will then ensure that `TrueComponent` and `FalseComponent` are separate bundles in the build output, and it will generate a **conditional manifest** which is a JSON file that details which bundle(s) should be loaded for each value of `fg('my-feature-gate-name')`. The web server (e.g. Bifrost) would then check the conditional manifest to determine which bundles to serve to any given user, depending on the value of `fg('my-feature-gate-name')`.

### Syntax

`importCond` is the unique global identifier that we use to identify conditional imports. Example:

```tsx
// index.tsx
const MyComponent = importCond<
  typeof import('./true-component.tsx'),
  typeof import('./false-component.tsx')
>('my-feature-gate-name', './true-component.tsx', './false-component.tsx');

// true-component.tsx
const TrueComponent = ...;
export default TrueComponent;

// false-component.tsx
const FalseComponent = ...;
export default FalseComponent;
```

Caveats:

- The modules you pass into the `typeof import(...)` must match the arguments to `importCond`
  - The need for this is due to TypeScript limitations. Note that there is an ESLint rule in `@atlaspack/eslint-plugin` that adds this boilerplate for you.
- The modules you pass to `importCond` MUST use a **default export**

> **Why do they need to be default exports?**
>
> `importCond` only supports default exports by design. There are two major reasons:
>
> **Type safety:** If you have two files, you'd have to consider the types of each export from the files. That would cause issues if one file exported things the other didn't. If we only consider the default export, that means you only have to match one export and handle one export in the callsite.
>
> **Bundling complexity:** Simplifying to default export means we can simplify the runtime code. If we don't, that means we have to handle a bunch of bundling edge cases to make sure we get optimised code (e.g. dead code removal, side-effects etc.)

At runtime, the `importCond` function call will be converted to something like this:

```js
// index.js
const MyComponent = globalThis.__MCOND('my-feature-gate-name')
  ? require('./true-component.tsx').default
  : require('./false-component.tsx').default;
```

`globalThis.__MCOND` is the feature gate function that the user evaluates at runtime. More about this later...

## How are conditional imports better?

Before conditional imports, there were two main approaches used in Jira for feature gating different modules:

- Conventional imports, e.g. `componentWithFG`
- Dynamic imports

The `componentWithFG` function call has the same impact to bundle size and load time as typing `fg('...') ? TrueComponent : FalseComponent` (with a bit of magic to make it work). It uses synchronous imports so there will be no impact to the idle time, but there will be a bundle size impact as the user will always load both versions of a component synchronously:

```tsx
import TrueComponent from './true-component.tsx';
import FalseComponent from './false-component.tsx';

const MyComponent = componentWithFG(
  'my-feature-gate-name',
  TrueComponent,
  FalseComponent,
);
// has the same bundle size and load time impact as if we did this...
const MyComponent = fg('my-feature-gate-name') ? TrueComponent : FalseComponent;
```

Meanwhile, dynamic imports have no impact to bundle size, as the user will only load the bundle that matches their feature gate value. However, the asynchronous imports will increase idle time due to the user needing to download the bundle at runtime without being able to take advantage of preloading:

```tsx
const MyComponent = fg('my-feature-gate-name')
  ? import('./true-component.tsx')
  : import('./false-component.tsx');
```

Conditional imports give the best of both worlds, as it allows us to continue using **synchronous imports** (like `componentWithFG`) but without the impact to bundle size. The user only loads the code that they actually need to run. Below is a diagram comparing the three approaches:

![Comparison of conditional bundling approaches. Conventional imports do not increase the idle time but increase the bundle size; dynamic imports do not increase the bundle size but increase the idle size. Conditional imports will not increase either of these.](conditional-bundling-comparison.png)

## How it works under the hood

### Conditional manifest

Atlaspack will generate a JSON file called a **conditional manifest** at the path `conditional-manifest.json`.

```ts
type ConditionalManifest = Record<
  string, // bundle path
  Record<
    string, // feature gate name
    {
      ifTrueBundles: string[]; // bundles to load if the feature gate is true
      ifFalseBundles: string[]; // bundles to load if the feature gate is false
    }
  >
>;
```

Here is an example `conditional-manifest.json` file. Note that a bundle that appears in `ifTrueBundles` and/or `ifFalseBundles` may itself contain more conditional import usages, like `index-new.2dde0b59.js` below.

```json
{
  "async-dev-panel.af26650a.js": {
    "create-branch-dropdown-iv-llc-dev-panel": {
      "ifTrueBundles": ["index-new.2dde0b59.js", "another-bundle.1bff84da.js"],
      "ifFalseBundles": ["create-branch-dropdown.809aaa8c.js"]
    }
  },
  "async-issue-view-entrypoint.64e31b38.js": {
    "details_loader_to_entrypoint_iv_llc_dev_panel": {
      "ifTrueBundles": ["ui.85dab1de.js"],
      "ifFalseBundles": ["async.9cd50096.js"]
    },
    "link_paste_recommendations": {
      "ifTrueBundles": [], // in this case, the `true` argument was an import from React, which doesn't result in any new bundles
      "ifFalseBundles": ["children-renderer.fa26e96c.js"]
    }
  },
  "index-new.2dde0b59.js": {
    // ...
  }
  // ...
}
```

### Integrating the conditional manifest into your web server

We assume that our web server contains an array of bundles called `scripts`, which contain what JavaScript bundles should be sent to the user:

```tsx
const scripts = [
  'shared.xxx.js',
  'some-new-bundle.xxx.js',
  'jira-spa-issue.view.xxx.js',
];
```

and let us assume that the web server is able to convert `scripts` into some HTML like so:

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <title>Example</title>
  </head>
  <body>
    <script src="shared.xxx.js"></script>
    <script src="some-new-bundle.xxx.js"></script>
    <script src="jira-spa-issue-view.xxx.js"></script>
  </body>
</html>
```

With this kind of setup, we can easily update the web server to parse the conditional manifest and ensure that the user loads the correct bundles:

```ts
const scripts: string[] = [];
const conditionalManifest: ConditionalManifest = fetchFile(
  'conditional-manifest.json',
);

for (const script in initialScripts) {
  if (conditionalManifest[script]) {
    for (const condition of conditionalManifest[script]) {
      // where `fg` is your feature gate function
      const conditionalScripts = fg(condition)
        ? conditionalManifest[script][condition].ifTrueBundles
        : conditionalManifest[script][condition].ifFalseBundles;

      // We need this to handle the case where a bundle in ifTrueBundles or ifFalseBundles
      // itself contains another `importCond` function call
      if (conditionalScripts.length > 0) {
        scripts.push(...conditionalScripts);
      }
    }
  }

  scripts.push(script);
}
```

The feature gates, whose names are provided in the first argument to each `importCond` function call, is evaluated in your web server, not in Atlaspack.

### Adding `__MCOND`

Now that we have set up the correct bundles to be loaded from the server side, we need to ensure the user `require`s the correct bundles client-side at runtime as well.

Recall that `importCond` is converted to this in the build output:

```js
const MyComponent = globalThis.__MCOND('my-feature-gate-name')
  ? require('./true-component.tsx').default
  : require('./false-component.tsx').default;
```

> **Sidenote**
>
> Technically, it is converted to this when `server: true`, to prevent `__MCOND` from being evaluated too early:
>
> ```js
> const MyComponent = {
>   ifTrue: require('./true-component.tsx').default,
>   ifFalse: require('./false-component.tsx').default,
> };
>
> Object.defineProperty(MyComponent, 'load', {
>   get: () =>
>     globalThis.__MCOND && globalThis.__MCOND('my-feature-gate-name')
>       ? conditionab.ifTrue
>       : conditionab.ifFalse,
> });
> MyComponent.load;
> ```

You will need to update the web server to add an implementation for `__MCOND`. If your feature gate function is called `fg`, it is as simple as:

```tsx
// somewhere in your client-side code
globalThis.__MCOND = (cond: string) => fg(cond);
```

You will also need to add this for your Jest configuration so that `importCond` works in unit tests.
