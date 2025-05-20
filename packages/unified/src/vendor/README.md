# Vendor Strategy

Atlaspack is distributed as a single package with no dependencies.

This folder contains dependencies that will be vendored/included in the published package.

During development these packages point to their respective dependencies in `node_modules`.

Running `yarn build` will copy the dependencies (recursively) from `node_modules` to `./lib/vendor/**` 
and rewrite their import statements to relative paths pointing to their respective vendored packages.

This approach ensures that re-exported type signatures from dependencies are included in the output.

## Adding a vendored package

Add a folder on the top level with a `vendor.json`, `index.js` and so on;

```
/src/vendor
  /my-dep
    index.js
    index.d.ts
    index.fs.flow
    vendor.json
```

Import the entry point from Atlaspack sources

```typescript
import { foo } from '../../vendor/my-dep/index.js'
``` 

Running `yarn build` will copy the dependencies (recursively) from `node_modules` into the `lib/vendor` folder.
