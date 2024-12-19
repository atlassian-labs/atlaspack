let load = (maxShards) => (id) => {
  // eslint-disable-next-line no-undef
  return __parcel__import__(
    require('@atlaspack/domain-sharding').shardUrl(
      require('../bundle-manifest').resolve(id),
      maxShards,
    ),
  );
};

module.exports = load;
