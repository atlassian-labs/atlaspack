---
'@atlaspack/core': patch
---

Remove asset groups from V3.

Asset groups were initially created to address the problem of a single asset being transformed into multiple assets (e.g. css modules).

This did the job, but means that an asset group is created for every single asset, and most groups only have a single asset inside.
This is a lot of unnecessary nodes, and we don't need to carry that over to the native rewrite.

This PR deletes the step that used to create them, and updates things like symbol prop to handle there potentially being no asset groups.
