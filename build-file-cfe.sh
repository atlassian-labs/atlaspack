#!/bin/bash

set -e
set -x

# Avoid Nx sqlite cache issues in Cursor agent environments.
export NX_DAEMON=false
export NX_SKIP_NX_CACHE=true
export NX_DISABLE_DB=true
export NX_DB_CACHE=false
export NX_CACHE_DIRECTORY="/tmp"
export CI=true
export bamboo_planKey=test
export bamboo_buildNumber=1
export bamboo_shortJobKey=test

cd ~/atlassian/atlaspack-beta
if [[ ! " $@ " =~ " --no-build " ]]; then
  yarn build-native && yarn build
fi

# Ensure 'unsafeReportSafeAssetsForMigration' is set to false in jira/.compiledcssrc
COMPILEDCSSRC=~/atlassian/atlassian-frontend-monorepo/confluence/.compiledcssrc
if [ -f "$COMPILEDCSSRC" ]; then
  TMP_COMPILEDCSSRC="${COMPILEDCSSRC}.tmp"
  jq '.unsafeReportSafeAssetsForMigration = false' "$COMPILEDCSSRC" > "$TMP_COMPILEDCSSRC" && mv "$TMP_COMPILEDCSSRC" "$COMPILEDCSSRC"
else
  echo "File $COMPILEDCSSRC does not exist!"
  exit 1
fi

cd ~/atlassian/atlaspack-beta
node transform-file.js "$1" --cwd ~/atlassian/atlaspack-beta/atlassian-frontend-monorepo/confluence --config ~/atlassian/atlaspack-beta/atlassian-frontend-monorepo/confluence/packages/confluence-frontend-server/.parcelrc --feature-flag compiledCssInJsTransformer=false --feature-flag enableTokensTransformer=true --no-cache --output-file=babel.js
node transform-file.js "$1" --cwd ~/atlassian/atlaspack-beta/atlassian-frontend-monorepo/confluence --config ~/atlassian/atlaspack-beta/atlassian-frontend-monorepo/confluence/packages/confluence-frontend-server/.parcelrc --feature-flag compiledCssInJsTransformer=true --feature-flag enableTokensTransformer=true --no-cache --output-file=swc.js

echo "Created atlassian-frontend-monorepo/confluence/$1.babel.js and atlassian-frontend-monorepo/confluence/$1.swc.js"
