/** Get the URL without the filename (last / segment)
 *
 * @param {string} url
 * @returns {string} The URL with the file name removed
 */
function getBaseURL(url) {
  return url.slice(0, url.lastIndexOf('/')) + '/';
}

const stackTraceUrlRegexp =
  /(https?|file|ftp|(chrome|moz|safari-web)-extension):\/\/[^)\n]+/g;

exports.getBaseURL = getBaseURL;
exports.stackTraceUrlRegexp = stackTraceUrlRegexp;
