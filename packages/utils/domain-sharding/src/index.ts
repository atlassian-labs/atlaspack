function getFilenameFromUrlPath(pathname: string) {
  const lastSlashIdx = pathname.lastIndexOf('/');

  if (lastSlashIdx === -1 || lastSlashIdx === pathname.length - 1) {
    throw new Error(
      `Expected an absolute URL with a file name, unable to apply sharding.`,
    );
  }

  // Add 1 to skip over the / itself
  return pathname.slice(lastSlashIdx + 1);
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

const trailingShardRegex = /-\d+$/;

function removeTrailingShard(subdomain: string) {
  if (!trailingShardRegex.test(subdomain)) {
    return subdomain;
  }

  const shardIdx = subdomain.lastIndexOf('-');
  return subdomain.slice(0, shardIdx);
}

function applyShardToDomain(domain: string, shard: number) {
  let i = domain.indexOf('.');

  // Domains like localhost have no . separators
  if (i === -1) {
    return `${removeTrailingShard(domain)}-${shard}`;
  }

  // If this domain already has a shard number in it, strip it
  // out before adding the new one
  const firstSubdomain = removeTrailingShard(domain.slice(0, i));

  return `${firstSubdomain}-${shard}${domain.slice(i)}`;
}

/*
 * Takes an absolute URL and applies a shard to the top level subdomain.
 * The shard number is based on a hash of the file name, which is
 * the content after the last / in the URL.
 *
 * Unlike `shardUrl`, this function will always apply sharding, without any
 * conditional logic.
 */

function shardUrlUnchecked(url: string, maxShards: number) {
  const parsedUrl = new URL(url);

  const fileName = getFilenameFromUrlPath(parsedUrl.pathname);
  const shardNumber = getDomainShardIndex(fileName, maxShards);

  parsedUrl.hostname = applyShardToDomain(parsedUrl.hostname, shardNumber);

  return parsedUrl.toString();
}

/*
 * Takes an absolute URL and applies a shard to the top level subdomain.
 * The shard number is based on a hash of the file name, which is
 * the content after the last / in the URL.
 *
 * This function only applies the sharding if the
 * __ATLASPACK_ENABLE_DOMAIN_SHARDS global variable has been set to true
 */
function shardUrl(url: string, maxShards: number) {
  // Global variable is set by SSR servers when HTTP1.1 traffic is detected
  if (!Boolean(globalThis.__ATLASPACK_ENABLE_DOMAIN_SHARDS)) {
    return url;
  }

  return shardUrlUnchecked(url, maxShards);
}

// TODO: convert this file to ESM once HMR issues are resolved
exports.shardUrl = shardUrl;
exports.shardUrlUnchecked = shardUrlUnchecked;