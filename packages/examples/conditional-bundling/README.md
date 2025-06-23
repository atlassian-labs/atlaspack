# Conditional bundling examples

To run:

1. Build Atlaspack - see `CONTRIBUTING.md` in the root folder. (`yarn && yarn build-native-release && yarn build` as of writing)
2. Run `yarn` in this folder
3. Run `yarn serve:on`

`packages/examples/conditional-bundling/features.js` contains the feature flag values. Change this and reload the page to see the loaded components change :)

See `package.json` for all the commands that you can try. Note that `yarn dev:on` is currently broken (and `yarn serve:on` will have non-breaking "Cannot find module" errors), sorry!
