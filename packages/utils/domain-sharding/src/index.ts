const globalKeyName = '__ATLASPACK_ENABLE_DOMAIN_SHARDS';

/**
 * Extracts the file name from a static asset path.
 * Will throw if the path doesn't have segments or ends in a trailing slash.
 */
function getFilenameFromUrlPath(pathname: string): string {
  const lastSlashIdx = pathname.lastIndexOf('/');

  if (lastSlashIdx === -1 || lastSlashIdx === pathname.length - 1) {
    throw new Error(
      `Expected an absolute URL with a file name, unable to apply sharding.`,
    );
  }

  // Add 1 to skip over the / itself
  return pathname.slice(lastSlashIdx + 1);
}

/**
 * Generates a bounded numeric hash in [0, maxShards)
 */
function getDomainShardIndex(str: string, maxShards: number): number {
  // As we include the base domain as a shard option then we add 1 to maxShards
  // to account for that.
  const totalShards = maxShards + 1;
  let shard = str.split('').reduce((a, b) => {
    const n = (a << totalShards) - a + b.charCodeAt(0);

    // The value returned by << is 64 bit, the & operator coerces to 32,
    // prevents overflow as we iterate.
    return n & n;
  }, 0);

  shard = shard % totalShards;

  // Make number positive
  if (shard < 0) {
    shard += totalShards;
  }

  return shard;
}

const trailingShardRegex = /-\d+$/;

/**
 * Removes trailing shard number from subdomain
 */
function removeTrailingShard(subdomain: string): string {
  if (!trailingShardRegex.test(subdomain)) {
    return subdomain;
  }

  const shardIdx = subdomain.lastIndexOf('-');
  return subdomain.slice(0, shardIdx);
}

/**
 * Given a shard number, inserts that shard in the expected pattern within the
 * domain
 */
function applyShardToDomain(domain: string, shard: number): string {
  const i = domain.indexOf('.');
  // If the shard is 0, then just use the base domain.
  // If the shard is > 0, then remove 1 as the shards domains index from 0
  const shardSuffix = shard === 0 ? '' : `-${shard - 1}`;

  // Domains like localhost have no . separators
  if (i === -1) {
    return `${removeTrailingShard(domain)}${shardSuffix}`;
  }

  // If this domain already has a shard number in it, strip it
  // out before adding the new one
  const firstSubdomain = removeTrailingShard(domain.slice(0, i));

  return `${firstSubdomain}${shardSuffix}${domain.slice(i)}`;
}

/**
 * Takes an absolute URL and applies a shard to the top level subdomain.
 * The shard number is based on a hash of the file name, which is
 * the content after the last / in the URL.
 *
 * Unlike `shardUrl`, this function will always apply sharding, without any
 * conditional logic.
 */
function shardUrlUnchecked(url: string, maxShards: number): string {
  const parsedUrl = new URL(url);

  const fileName = getFilenameFromUrlPath(parsedUrl.pathname);
  const shardNumber = getDomainShardIndex(fileName, maxShards);

  parsedUrl.hostname = applyShardToDomain(parsedUrl.hostname, shardNumber);

  return parsedUrl.toString();
}

/**
 * Takes an absolute URL and applies a shard to the top level subdomain.
 * The shard number is based on a hash of the file name, which is
 * the content after the last / in the URL.
 *
 * This function only applies the sharding if the
 * __ATLASPACK_ENABLE_DOMAIN_SHARDS global variable has been set to true
 */
function shardUrl(url: string, maxShards: number): string {
  // Global variable is set by SSR servers when HTTP1.1 traffic is detected
  if (!(globalThis as any)[globalKeyName]) {
    return url;
  }

  return shardUrlUnchecked(url, maxShards);
}

export {shardUrl, shardUrlUnchecked, getDomainShardIndex, applyShardToDomain};
export const domainShardingKey = globalKeyName;
