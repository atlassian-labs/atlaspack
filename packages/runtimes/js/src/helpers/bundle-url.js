const stackTraceUrlRegexp =
  /(https?|file|ftp|(chrome|moz|safari-web)-extension):\/\/[^)\n]+/g;

const bundleURL = {};

/**
 * Retrieves the cached bundle URL for a given identifier.
 * If the URL is not cached, it computes and stores it in the cache.
 *
 * @param {string} id - The identifier for the bundle.
 * @param {Error?} inputError - An error object to extract the stack trace from
 * (for testing purposes).
 * @returns {string} The URL of the bundle, without file name.
 */
function getBundleURLCached(id, inputError) {
  let value = bundleURL[id];

  if (!value) {
    value = getBundleURL(inputError);
    bundleURL[id] = value;
  }

  return value;
}

/** Get the URL without the filename (last / segment)
 *
 * @param {string} url
 * @returns {string} The URL with the file name removed
 */
function getBaseURL(url) {
  return url.slice(0, url.lastIndexOf('/')) + '/';
}

function getBundleURL(inputError) {
  try {
    throw inputError ?? new Error();
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

// TODO: convert this file to ESM once HMR issues are resolved
exports.getOrigin = getOrigin;
exports.getBundleURL = getBundleURLCached;
exports.getBaseURL = getBaseURL;
