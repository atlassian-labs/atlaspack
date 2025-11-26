---
'@atlaspack/rust': patch
---

Fix issue where aliased files that depend on the aliases file itself would cause invalid self dependencies.

This solves an issue when aliases are used to create wrapper modules that re-export items from the original module, causing a self-dependency cycle.
Aliases will now not be resolved if they cause the dependency to point back to the resolve from location.
