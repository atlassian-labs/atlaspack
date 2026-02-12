#!/bin/bash

set -e
set -x

# Avoid Nx sqlite cache issues in Cursor agent environments.
export NX_DAEMON=false
export NX_SKIP_NX_CACHE=true
export NX_DISABLE_DB=true
export NX_DB_CACHE=false
export NX_CACHE_DIRECTORY="/tmp"

cd ~/atlassian/atlaspack-beta
if [[ ! " $@ " =~ " --no-build " ]]; then
  yarn build-native-release && yarn build
fi

# Ensure 'unsafeReportSafeAssetsForMigration' is set to true in confluence/.compiledcssrc
COMPILEDCSSRC=~/atlassian/atlassian-frontend-monorepo/confluence/.compiledcssrc
if [ -f "$COMPILEDCSSRC" ]; then
  TMP_COMPILEDCSSRC="${COMPILEDCSSRC}.tmp"
  jq '.unsafeReportSafeAssetsForMigration = true' "$COMPILEDCSSRC" > "$TMP_COMPILEDCSSRC" && mv "$TMP_COMPILEDCSSRC" "$COMPILEDCSSRC"
else
  echo "File $COMPILEDCSSRC does not exist!"
  exit 1
fi

cd ~/atlassian/atlaspack-beta/atlassian-frontend-monorepo/confluence
nvm use
rm -rf compiled-css-migration-map.json .parcel-cache/ build/; unset NODE_OPTIONS && ATLASPACK_COMPILED_CSS_IN_JS_TRANSFORMER=true ATLASPACK_ENABLE_TOKENS_TRANSFORMER=true ATLASPACK_CORE_TOKENS_AND_COMPILED_CSS_IN_JS_TRANSFORM=false BUILD_VARIANT=default afm parcel:build 2>&1 | tee build.log; node compiled-css-diff-report.js > differences-cfe-llm.txt
echo "differences-cfe-llm.txt updated"
