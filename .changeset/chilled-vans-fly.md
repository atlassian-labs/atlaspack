---
'@atlaspack/transformer-js': minor
'@atlaspack/rust': minor
---

Support ignore comments for node replacements

Adding `#__ATLASPACK_IGNORE__` before `__filename` and `__dirname` will now disable the default node replacement behaviour of these variables. This is useful when you want your compiled output to be aware of it's runtime directory rather than it's pre-compiled source directory.

```js
const dirname = /*#__ATLASPACK_IGNORE__*/ __dirname;
```
