#!/bin/bash

cd ~/atlassian/atlaspack
yarn build-native &> /dev/null && yarn build &> /dev/null

cd ~/atlassian/atlassian-frontend-monorepo/jira
node transform-file.js "$1" --cwd ~/atlassian/atlassian-frontend-monorepo/jira --config ~/atlassian/atlassian-frontend-monorepo/jira/.parcelrc-v3 --feature-flag compiledCssInJsTransformer=false --feature-flag enableTokensTransformer=true --no-cache --output-file=babel
node transform-file.js "$1" --cwd ~/atlassian/atlassian-frontend-monorepo/jira --config ~/atlassian/atlassian-frontend-monorepo/jira/.parcelrc-v3 --feature-flag compiledCssInJsTransformer=true --feature-flag enableTokensTransformer=true --no-cache --output-file=swc

echo "Diffing $1.babel and $1.swc"
diff "$1.babel" "$1.swc"