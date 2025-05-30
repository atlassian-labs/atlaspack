---
'@atlaspack/rust': patch
---

There are three types of results that a resolver can return:

- A successful resolution
- "Unresolved" when the resolver could not find a match
- "Excluded" when the result should not be included in the bundle

This last case wasn't being handle in the NAPI conversion layer, and so was falling through as a successful resolution with no details.
