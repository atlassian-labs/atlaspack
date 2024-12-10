// Get the URL without the filename (last / segment)
export function getBaseURL(url: string) {
  return url.slice(0, url.lastIndexOf('/')) + '/';
}

export const stackTraceUrlRegexp =
  /(https?|file|ftp|(chrome|moz|safari-web)-extension):\/\/[^)\n]+/g;
