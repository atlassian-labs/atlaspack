---
'@atlaspack/transformer-compiled': minor
---

read browserslist from project config only once during transformer setup() rather than reading package.json on every transform() call.
