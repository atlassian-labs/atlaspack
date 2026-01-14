#!/bin/bash

cd ~/atlassian/atlaspack
yarn build-native &> /dev/null && yarn build &> /dev/null

cd ~/atlassian/atlassian-frontend-monorepo/jira
node transform-file.js "$1" --cwd ~/atlassian/atlassian-frontend-monorepo/jira --config ~/atlassian/atlassian-frontend-monorepo/jira/.parcelrc-v3 --feature-flag compiledCssInJsTransformer=false --feature-flag enableTokensTransformer=true --no-cache --output-file=old
node transform-file.js "$1" --cwd ~/atlassian/atlassian-frontend-monorepo/jira --config ~/atlassian/atlassian-frontend-monorepo/jira/.parcelrc-v3 --feature-flag compiledCssInJsTransformer=true --feature-flag enableTokensTransformer=true --no-cache --output-file=new

echo "Diffing $1.old and $1.new"
diff "$1.old" "$1.new"