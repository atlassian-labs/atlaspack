import URL from 'url';
import path from 'path';

/**
 * Joins a path onto a URL, and normalizes Windows paths
 * e.g. from \path\to\res.js to /path/to/res.js.
 */
export default function urlJoin(publicURL: string, assetPath: string): string {
  const url = URL.parse(publicURL, false, true);
  // Leading / ensures that paths with colons are not parsed as a protocol.
  let p = assetPath.startsWith('/') ? assetPath : '/' + assetPath;
  const assetUrl = URL.parse(p);
  // @ts-expect-error - TS2345 - Argument of type 'string | null' is not assignable to parameter of type 'string'.
  url.pathname = path.posix.join(url.pathname, assetUrl.pathname);
  url.search = assetUrl.search;
  url.hash = assetUrl.hash;
  return URL.format(url);
}
