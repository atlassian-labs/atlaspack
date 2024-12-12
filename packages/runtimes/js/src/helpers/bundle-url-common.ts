// Get the URL without the filename (last / segment)
function getBaseURL(url: string) {
  return url.slice(0, url.lastIndexOf('/')) + '/';
}

const stackTraceUrlRegexp =
  /(https?|file|ftp|(chrome|moz|safari-web)-extension):\/\/[^)\n]+/g;

exports.getBaseURL = getBaseURL;
exports.stackTraceUrlRegexp = stackTraceUrlRegexp;
