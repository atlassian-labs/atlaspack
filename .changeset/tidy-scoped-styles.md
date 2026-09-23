---
'@atlaspack/transformer-js': patch
'@atlaspack/rust': patch
---

Preserve source order for non-atomic `cssMapScoped` rules so later, equally specific selectors correctly override earlier selectors.
