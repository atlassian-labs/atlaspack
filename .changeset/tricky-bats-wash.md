---
'@atlaspack/feature-flags': minor
---

Export DEFAULT_FEATURE_FLAGS so it will be included in the associate type declaration file and able to be imported elsewhere.

This will enable patterns like:

```
import type { FeatureFlags } from '@atlaspack/feature-flags';
import { DEFAULT_FEATURE_FLAGS } from '@atlaspack/feature-flags';
```
