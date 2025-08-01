// Module override when testing
// [] TODO: consider replacing this with RANDOM_GATES and ALL_ENABLED environment variables
if (process.env.NODE_ENV === 'test') {
  // eslint-disable-next-line no-console
  console.log('🛠️ Setting up feature flag override via --require...');

  const mockModule = require('../feature-flag-mock.js');

  // Override the original functions at the source
  const Module = require('module');
  const originalLoad = Module._load;

  Module._load = function (request, parent, isMain) {
    const result = originalLoad.call(this, request, parent, isMain);

    if (
      request === '@atlaspack/feature-flags' ||
      request.includes('feature-flags/src/index')
    ) {
      if (result) {
        // Replace the functions with our mock versions
        result.setFeatureFlags = mockModule.setFeatureFlags;
        result.getFeatureFlag = mockModule.getFeatureFlag;
        result.getFeatureFlagValue = mockModule.getFeatureFlagValue;
        result.resetFlags = mockModule.resetFlags;
      }
    }

    return result;
  };
}
