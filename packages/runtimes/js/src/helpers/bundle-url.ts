const {getBaseURL, stackTraceUrlRegexp} = require('./bundle-url-common');

const bundleURL = {};

function getBundleURLCached(id: string) {
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

function getOrigin(url: string) {
  return new URL(url).origin;
}

exports.getOrigin = getOrigin;
exports.getBundleURL = getBundleURLCached;
