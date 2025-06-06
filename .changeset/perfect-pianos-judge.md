---
'@atlaspack/bundler-default': major
---

### Breaking change

This new config replaces the previously released `sharedBundleMergeThreshold`.

The following options are available for each merge group.

### Options

#### overlapThreshold

> The same as `sharedBundleMergeThreshold` from #535

Merge bundles share a percentage of source bundles

```json
"@atlaspack/bundler-default": {
  "sharedBundleMerge": [{
    "overlapThreshold": 0.75
  }]
}
```

#### maxBundleSize

Merge bundles that are smaller than a configured amount of bytes.

> Keep in mind these bytes are pre-optimisation

```json
"@atlaspack/bundler-default": {
  "sharedBundleMerge": [{
    "maxBundleSize": 20000
  }]
}
```

#### sourceBundles

Merge bundles that share a set of source bundles. The matching is relative to the project root, like how manual shared bundle roots work.

```json
"@atlaspack/bundler-default": {
  "sharedBundleMerge": [{
    "sourceBundles": ["src/important-route", "src/important-route-2"]
  }]
}
```

#### minBundlesInGroup

Merge bundles that belong to a bundle group that's larger than a set amount. This is useful for targetting bundles that would be deleted by the `maxParallelRequests` option.

```json
"@atlaspack/bundler-default": {
  "maxParallelRequests": 30,
  "sharedBundleMerge": [{
    "minBundlesInGroup": 30
  }]
}
```

## Combining options

When multiple options are provided, all must be true for a merge to be relevant.

For example, merge bundles that are smaller than 20kb and share at least 50% of the same source bundles.

```json
"@atlaspack/bundler-default": {
  "sharedBundleMerge": [{
    "overlapThreshold": 0.5,
    "maxBundleSize": 20000
  }]
}
```

## Multiple merges

You can also have multiple merge configs.

```json
"@atlaspack/bundler-default": {
  "sharedBundleMerge": [
     {
        "overlapThreshold": 0.75,
        "maxBundleSize": 20000
     },
     {
        "minBundlesInGroup": 30
        "sourceBundles": ["src/important-route", "src/important-route-2"]
     }
  ]
}
```
