// Debug: Log environment variables to see if ATLASPACK_V3 is set
console.log('Worker starting with ATLASPACK_V3:', process.env.ATLASPACK_V3);
console.log('Worker starting with ATLASPACK_SOURCES:', process.env.ATLASPACK_SOURCES);
console.log('Worker starting with ATLASPACK_BUILD_ENV:', process.env.ATLASPACK_BUILD_ENV);
console.log('Worker starting with ATLASPACK_SELF_BUILD:', process.env.ATLASPACK_SELF_BUILD);

if (
  process.env.ATLASPACK_SOURCES === 'true' ||
  process.env.ATLASPACK_BUILD_ENV === 'test' ||
  process.env.ATLASPACK_SELF_BUILD ||
  process.env.ATLASPACK_V3 === 'true'
) {
  console.log('Loading babel-register in worker');

  // For v3, we need to ensure ES modules are converted to CommonJS
  if (process.env.ATLASPACK_V3 === 'true') {
    // Use a custom babel configuration for v3 that properly converts ES modules
    require('@babel/register')({
      presets: [
        ['@babel/preset-env', {
          targets: { node: 16 },
          modules: 'commonjs' // Force CommonJS conversion
        }],
        '@babel/preset-typescript'
      ],
      extensions: ['.js', '.jsx', '.ts', '.tsx'],
      ignore: [
        (filepath) => filepath.includes('/node_modules/'),
      ],
    });
  } else {
    require('@atlaspack/babel-register');
  }
} else {
  console.log('NOT loading babel-register in worker');
}

require('./worker');
