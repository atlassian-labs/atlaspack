---
'@atlaspack/rust': patch
---

Fix missing edges in the asset graph when working with discovered assets.

When we are transforming an asset that returns "discovered" assets (e.g. CSS modules), we add an edge from the dependency to the asset, and then from the dependency to the discovered asset/s.

However, we also have some logic to skip some steps if the asset that is being referenced has already been transformed, instead just creating an edge from the new dependency to the existing asset node.

Unfortunately this reuse logic did not account for the discovered assets when adding edges, only connecting the dependency with the main asset. This has been addressed in this PR.
