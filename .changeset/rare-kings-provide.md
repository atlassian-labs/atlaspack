---
'@atlaspack/rust': minor
---

Exclude frequently changing env vars from inline environment feature so rust JS transformer can be cacheable. Add cache bailout summary info logging broken down by transformer. Log transformer pipeline cache info immediately after asset graph construction for ergonomic debugging.
