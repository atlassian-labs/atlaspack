---
'@atlaspack/reporter-cli': patch
---

Fixes the cliProgressReportingImprovements, which were previously being overwritten by the existing packaging messaging. With this fix, now only the progress messages are shown during packaging.
