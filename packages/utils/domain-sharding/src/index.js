let globalKeyName = '__ATLASPACK_ENABLE_DOMAIN_SHARDS';

/**
 * Extracts the file name from a static asset path.
 * Will throw if the path doesn't have segments or ends in a trailing slash.
 *
 * @param {string} pathname
 * @returns {string}
 */
function getFilenameFromUrlPath(pathname) {
  let lastSlashIdx = pathname.lastIndexOf('/');

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
 *
 * @param {string} str
 * @param {number} maxShards
 * @returns {number}
 */
function getDomainShardIndex(str, maxShards) {
  let shard = str.split('').reduce((a, b) => {
    let n = (a << maxShards) - a + b.charCodeAt(0);

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

let trailingShardRegex = /-\d+$/;

/**
 * @param {string} subdomain
 */
function removeTrailingShard(subdomain) {
  if (!trailingShardRegex.test(subdomain)) {
    return subdomain;
  }

  let shardIdx = subdomain.lastIndexOf('-');
  return subdomain.slice(0, shardIdx);
}

/**
 * Given a shard number, inserts that shard in the expected pattern within the
 * domain
 *
 * @param {string} domain
 * @param {number} shard
 */
function applyShardToDomain(domain, shard) {
  let i = domain.indexOf('.');

  // Domains like localhost have no . separators
  if (i === -1) {
    return `${removeTrailingShard(domain)}-${shard}`;
  }

  // If this domain already has a shard number in it, strip it
  // out before adding the new one
  let firstSubdomain = removeTrailingShard(domain.slice(0, i));

  return `${firstSubdomain}-${shard}${domain.slice(i)}`;
}

/**
 * Takes an absolute URL and applies a shard to the top level subdomain.
 * The shard number is based on a hash of the file name, which is
 * the content after the last / in the URL.
 *
 * Unlike `shardUrl`, this function will always apply sharding, without any
 * conditional logic.
 *
 * @param {string} url
 * @param {number} maxShards
 * @returns {string}
 */

function shardUrlUnchecked(url, maxShards) {
  let parsedUrl = new URL(url);

  let fileName = getFilenameFromUrlPath(parsedUrl.pathname);
  let shardNumber = getDomainShardIndex(fileName, maxShards);

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
 *
 * @param {string} url
 * @param {number} maxShards
 * @returns {string}
 */
function shardUrl(url, maxShards) {
  // Global variable is set by SSR servers when HTTP1.1 traffic is detected
  if (!globalThis[globalKeyName]) {
    return url;
  }

  return shardUrlUnchecked(url, maxShards);
}

// TODO: convert this file to ESM once HMR issues are resolved
exports.shardUrl = shardUrl;
exports.shardUrlUnchecked = shardUrlUnchecked;
exports.getDomainShardIndex = getDomainShardIndex;
exports.applyShardToDomain = applyShardToDomain;
exports.domainShardingKey = globalKeyName;
