if (
  process.env.ATLASPACK_BUILD_ENV !== 'production' ||
  process.env.ATLASPACK_SELF_BUILD
) {
  const fs = require('fs');

  require('@atlaspack/babel-register');

  // eslint-disable-next-line import/no-extraneous-dependencies
  require('esbuild-register/dist/node').register({
    // For development mode, skip flow files when compiling typescript plugins
    hookMatcher: (f) => {
      const contents = fs.readFileSync(f, 'utf-8');
      if (contents.trim().split()[0].includes('@flow')) {
        return false;
      }
      return true;
    },
  });
} else {
  // eslint-disable-next-line import/no-extraneous-dependencies
  require('esbuild-register/dist/node').register({});
}

require('./worker');
