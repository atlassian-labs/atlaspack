const {getBaseURL, stackTraceUrlRegexp} = require('./bundle-url-common');

const bundleURL = {};

/**
 * Retrieves the cached bundle URL for a given identifier.
 * If the URL is not cached, it computes and stores it in the cache.
 *
 * @param {string} id - The identifier for the bundle.
 * @returns {string} The URL of the bundle, without file name.
 */
function getBundleURLCached(id) {
  let value = bundleURL[id];

  if (!value) {
    value = getBundleURL();
    bundleURL[id] = value;
  }

  return value;
}

function getBundleURL() {
  try {
    throw new Error();
  } catch (err) {
    var matches = ('' + err.stack).match(stackTraceUrlRegexp);
    if (matches) {
      // The first two stack frames will be this function and getBundleURLCached.
      // Use the 3rd one, which will be a runtime in the original bundle.
      return getBaseURL(matches[2]);
    }
  }

  return '/';
}

/**
 * @param {string} url
 * @returns {string}
 */
function getOrigin(url) {
  return new URL(url).origin;
}

exports.getOrigin = getOrigin;
exports.getBundleURL = getBundleURLCached;
