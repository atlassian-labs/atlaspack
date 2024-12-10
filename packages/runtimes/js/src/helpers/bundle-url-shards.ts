import {getBaseURL} from './bundle-url-common';

const bundleURL: Record<string, string> = {};

export function getShardedBundleURL(
  bundleName: string,
  cookieName: string,
  cookieString: string,
  maxShards: number,
  inputError?: string,
): string {
  let value = bundleURL[bundleName];

  if (value) {
    return value;
  }

  try {
    throw inputError ?? new Error();
  } catch (err) {
    var matches = ('' + err.stack).match(
      /(https?|file|ftp|(chrome|moz|safari-web)-extension):\/\/[^)\n]+/g,
    );

    if (!matches) {
      return '/';
    }

    // The first stack frame will be this function.
    // Use the 2nd one, which will be a runtime in the original bundle.
    const stackUrl = matches[1];
    const baseUrl = getBaseURL(stackUrl);

    // If the cookie doesn't exist then we don't need to shard
    if (cookieString.indexOf(cookieName) === -1) {
      return baseUrl;
    }

    const shardNumber = getDomainShardIndex(bundleName, maxShards);
    const url = new URL(baseUrl);

    const shardedDomain = getShardedDomain(url.hostname, shardNumber);
    url.hostname = shardedDomain;

    value = url.toString();

    bundleURL[bundleName] = value;
    return value;
  }
}

function getDomainShardIndex(str: string, maxShards: number) {
  let shard = str.split('').reduce((a, b) => {
    const n = (a << maxShards) - a + b.charCodeAt(0);

    // The value returned by << is 64 bit, the & operator coerces to 32,
    // prevents overflow as we iterate.
    return n & n;
  }, 0);

  shard = shard % maxShards;

  // Make number positive
  if (shard < 0) {
    shard += maxShards;
  }

  return shard;
}

function getShardedDomain(domain: string, shard: number) {
  let i = domain.indexOf('.');

  // Domains like localhost have no . separators
  if (i === -1) {
    return `${removeTrailingShard(domain)}-${shard}`;
  }

  // If this domain already has a shard number in it, strip it out before adding
  // the new one
  const firstSubdomain = removeTrailingShard(domain.slice(0, i));

  return `${firstSubdomain}-${shard}${domain.slice(i)}`;
}

const trailingShardRegex = /-\d+$/;

function removeTrailingShard(subdomain: string) {
  if (!trailingShardRegex.test(subdomain)) {
    return subdomain;
  }

  const shardIdx = subdomain.lastIndexOf('-');
  return subdomain.slice(0, shardIdx);
}
