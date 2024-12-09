// Get the URL without the filename (last / segment)
export function getBaseURL(url: string) {
  return url.slice(0, url.lastIndexOf('/')) + '/';
}
