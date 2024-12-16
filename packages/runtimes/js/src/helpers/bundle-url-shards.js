const {getBaseURL, stackTraceUrlRegexp} = require('./bundle-url-common');

const bundleURL = {};

function getShardedBundleURL(bundleName, maxShards, inputError) {
  let value = bundleURL[bundleName];

  if (value) {
    return value;
  }

  try {
    throw inputError ?? new Error();
  } catch (err) {
    var matches = ('' + err.stack).match(stackTraceUrlRegexp);

    if (!matches) {
      return '/';
    }

    // The first stack frame will be this function.
    // Use the 2nd one, which will be a runtime in the original bundle.
    const stackUrl = matches[1];
    const baseUrl = getBaseURL(stackUrl);

    // Global variable is set by SSR servers when HTTP1.1 traffic is detected
    if (!Boolean(globalThis.__ATLASPACK_ENABLE_DOMAIN_SHARDS)) {
      bundleURL[bundleName] = baseUrl;
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

function getDomainShardIndex(str, maxShards) {
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

function getShardedDomain(domain, shard) {
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

function removeTrailingShard(subdomain) {
  if (!trailingShardRegex.test(subdomain)) {
    return subdomain;
  }

  const shardIdx = subdomain.lastIndexOf('-');
  return subdomain.slice(0, shardIdx);
}

exports.getShardedBundleURL = getShardedBundleURL;
