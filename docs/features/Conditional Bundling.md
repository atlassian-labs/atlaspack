# Conditional imports

This is a deep dive into how conditional imports works in Atlaspack. This should cover all the information for platform developers to integrate it into their own products, in order to allow product developers to use it for their features.

If you wish to use conditional imports in Jira, you should use our adoption guide instead. Search "Conditional imports in Jira - adoption guide for devs" on Confluence, or check out the adoption guide in the pinned messages in `#atlaspack-contextual-imports`.

## What are conditional imports?

### The problem

Take the following example code that uses synchronous imports:

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

Atlaspack's conditional imports API solves this problem by giving a way for the Atlaspack bundler to recognise conditional code. It allows for the web server to then only deliver specific bundles to the user, based on their feature gate values.

> **Note on async bundles**
>
> This document covers the case where conditional imports are used in a synchronous bundle. In this case, the web server needs to ensure the correct bundles are loaded on the page, using the bundles listed in the conditional manifest.
>
> However, when conditional imports are used in an asynchronous bundle (most of Jira's bundles are asynchronous), Atlaspack will load the conditional bundles as a dependency alongside the asynchronous bundle.
>
> Scroll down to see an example of this.

All of the bundles are still loaded _synchronously_, making conditional imports more versatile and performant than dynamic imports. We discuss the benefits of conditional imports compared to dynamic imports in the "How do conditional imports differ from dynamic imports?" section below.

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

At build time, Atlaspack will parse all `importCond` function calls, then ensure that `TrueComponent` and `FalseComponent` are separate bundles in the build output:

```js
// index.js

register("avM1e", function(e, t) {
  e.exports = function(e, ifTrue, ifFalse) {
      // runtime feature gate function
      return globalThis.__MCOND && globalThis.__MCOND(e) ? ifTrue() : ifFalse()
  }
}),
register("4qbUE", function(e, t) {
    e.exports = parcelRequire("avM1e")("my-feature-gate-name", function() {
        // TrueComponent
        return parcelRequire("esWld");
    }, function() {
        // FalseComponent
        return parcelRequire("dqBV0");
    })
}),
// ...
const MyComponent = parcelRequire("4qbUE");
```

It will then generate a **conditional manifest** which is a JSON file that details which bundle(s) should be loaded for each value of `fg('my-feature-gate-name')`. The web server (e.g. Bifrost) would then check the conditional manifest to determine which bundles to serve to any given user, depending on the value of `fg('my-feature-gate-name')`. In the above example, the web server would serve either the bundle containing `esWld`, or the bundle containing `dqBV0`. At runtime, the user would then run the feature gate function `globalThis.__MCOND` ("**m**odule **cond**ition") and execute the correct bundle, which we require to match the bundle the web server sends to the user. (More about `__MCOND` later...)

## How do conditional imports differ from dynamic imports?

Before conditional imports, there were two main approaches used in Jira for feature gating different modules:

- Conventional imports, e.g. `componentWithFG`
- Dynamic imports

```tsx
// Conventional imports
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

In the above **conventional imports** example, the `componentWithFG` function call has the same impact to bundle size and load time as typing `fg('...') ? TrueComponent : FalseComponent` (with a bit of magic to make it work). It uses synchronous imports so there will be no impact to the idle time, but there will be a bundle size impact as the user will always load both versions of a component synchronously:

```tsx
// Dynamic imports

const MyComponent = fg('my-feature-gate-name')
  ? import('./true-component.tsx')
  : import('./false-component.tsx');
```

Meanwhile, **dynamic imports** (or asynchronous imports) have no impact to bundle size, as the user will only load the bundle that matches their feature gate value. However, dynamic imports are not feasible for cases where the feature gated component is part of the initial render of the page, where the component must be available synchronously and cannot be loaded dynamically.

Even setting this aside, the dynamic imports aren't ideal performance-wise either: they increase the client-side load time, due to the user needing to download the bundle at runtime without being able to take advantage of preloading:

**Conditional imports** give the best of both worlds, as it allows us to continue using **synchronous imports** (like `componentWithFG`) but without the impact to bundle size. The user only loads the code that they actually need to run. Below is a diagram comparing the three approaches:

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
// index.js

register("avM1e", function(e, t) {
  e.exports = function(e, ifTrue, ifFalse) {
      return globalThis.__MCOND && globalThis.__MCOND(e) ? ifTrue() : ifFalse()
  }
}),
register("4qbUE", function(e, t) {
    e.exports = parcelRequire("avM1e")("my-feature-gate-name", function() {
        return parcelRequire("esWld");
    }, function() {
        return parcelRequire("dqBV0");
    })
}),
// ...
const MyComponent = parcelRequire("4qbUE");
```

You will need to update the web server to add an implementation for the runtime feature gate function, `__MCOND`. This function **must return the same output** as the server-side feature gate function that the web server uses for parsing the conditional manifest. If your feature gate function is called `fg`, it is as simple as:

```tsx
// somewhere in your client-side code
globalThis.__MCOND = (cond: string) => fg(cond);
```

You will also need to add this for your Jest configuration so that `importCond` works in unit tests.

### Aside: how conditional imports inside asynchronous bundles are handled

As noted earlier, when conditional imports are used in an asynchronous bundle, Atlaspack will add to the loader the conditional bundles as a dependency alongside the asynchronous bundle.

If we imagine our source code looking like this:

```tsx
// index.tsx

const LazyComponent = lazy(() => import('./lazy-component'));

// lazy-component.tsx

import React from 'react';

const Button = importCond<
  typeof import('@atlaskit/button/new'),
  typeof import('@atlaskit/button')
>('my.feature.button', '@atlaskit/button', '@atlaskit/button/new');

export default function LazyComponent() {
  return (
    <p>
      This is a lazy component. It has a button. <Button>Lazy button</Button>
    </p>
  );
}
```

Then the output of `index.tsx` will look something like this:

```js
// index.js

// this is the function that will run MCOND (the feature gate function) at runtime
parcelRegister('59IBW', function (module, exports) {
  'use strict';
  module.exports = function (cond, ifTrue, ifFalse) {
    return globalThis.__MCOND && globalThis.__MCOND(cond)
      ? ifTrue()
      : ifFalse();
  };
});
parcelRegister('e2VvX', function (module, exports) {
  // function that resolves bundles
  // see `packages/runtimes/js/src/helpers/bundle-manifest.js`
  var $dgmOi = parcelRequire('dgmOi');

  module.exports = Promise.all([
    // load the conditional bundles that LazyComponent contained
    parcelRequire('59IBW')(
      'my.feature.button',
      function () {
        // bundles to load when feature gate `my.feature.button` is true
        return Promise.all([$dgmOi('1pPgB'), $dgmOi('3iyWa')]);
      },
      function () {
        // bundles to load when feature gate `my.feature.button` is false
        return Promise.all([$dgmOi('63Gye'), $dgmOi('7ml4E'), $dgmOi('3iyWa')]);
      },
    ),
    $dgmOi('9XYbX'), // this resolves LazyComponent
  ]).then(
    // execute LazyComponent
    () => parcelRequire('1Jo9T'),
  );
});
$parcel$global.rwr('5ahOM', () => {
  // ...
  const $b4004c4361ac9f08$var$LazyComponent = /*#__PURE__*/ (0, $ktSyt.lazy)(
    () => parcelRequire('e2VvX'),
  );
  // ...
});
```

Notable parts of the build output:

- `59IBW`: We evaluate `__MCOND` at runtime to ensure that the user requests the correct bundle at runtime.
- `e2VvX`: Atlaspack adds [special loading logic](https://github.com/atlassian-labs/atlaspack/blob/e39c6cf05f7e95ce5420dbcea66f401b1cbd397c/packages/runtimes/js/src/JSRuntime.js#L516) via `Promise.all` to ensure that `LazyComponent` and the conditional bundles (the bundles that we wish to load through `importCond`) are loaded in parallel.

Also see [`packages/examples/conditional-bundling`](https://github.com/atlassian-labs/atlaspack/tree/main/packages/examples/conditional-bundling) for more examples (refer to the `package.json` for commands you can run).
