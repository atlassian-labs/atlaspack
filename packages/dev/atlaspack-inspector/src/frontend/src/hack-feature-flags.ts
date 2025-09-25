// @ts-expect-error
window.__PLATFORM_FEATURE_FLAGS__ = {
  booleanResolver() {
    return true;
  },
};
