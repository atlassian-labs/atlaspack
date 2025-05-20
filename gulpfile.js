const {Transform} = require('stream');
const {execSync} = require('child_process');
const babel = require('gulp-babel');
const gulp = require('gulp');
const path = require('path');
const {rimraf} = require('rimraf');
const babelConfig = require('./babel.config.json');

const IGNORED_PACKAGES = [
  '!packages/examples/**',
  '!packages/core/integration-tests/**',
  '!packages/core/workers/test/integration/**',
  '!packages/core/test-utils/**',
  '!packages/core/types/**',
  '!packages/core/types-internal/**',
  '!packages/shims/**',
];

const paths = {
  packageSrc: [
    'packages/*/*/src/**/*.js',
    '!**/dev-prelude.js',
    ...IGNORED_PACKAGES,
  ],
  packageOther: [
    'packages/*/*/src/**/dev-prelude.js',
    // This has to have some glob syntax so that vinyl.base will be right
    'packages/{runtimes,}/js/src/helpers/*.ts',
  ],
  packages: 'packages/',
};

/*
 * "Taps" into the contents of a flowing stream, yielding chunks to the passed
 * callback. Continues to pass data chunks down the stream.
 */
class TapStream extends Transform {
  constructor(tap, options) {
    super({...options, objectMode: true});
    this._tap = tap;
  }

  _transform(chunk, encoding, callback) {
    try {
      this._tap(chunk);
      callback(null, chunk);
    } catch (err) {
      callback(err);
    }
  }
}

gulp.task('clean', async (cb) => {
  await Promise.all([
    rimraf.sync('packages/*/*/lib/**'),
    rimraf.sync('packages/unified/lib'),
  ]).then(
    () => cb(),
    (err) => cb(err),
  );
});

gulp.task('prepare', (cb) => {
  execSync('yarn lerna run dev:prepare', {
    cwd: __dirname,
    shell: true,
    stdio: 'inherit',
  });
  cb();
});

gulp.task('typescript', (cb) => {
  execSync('yarn lerna run build-ts', {
    cwd: __dirname,
    shell: true,
    stdio: 'inherit',
  });

  execSync('yarn lerna run check-ts', {
    cwd: __dirname,
    shell: true,
    stdio: 'inherit',
  });

  cb();
});

gulp.task('unified:post', (cb) => {
  execSync('node ./scripts/unified-build-vendor.mjs', {
    cwd: __dirname,
    shell: true,
    stdio: 'inherit',
  });

  execSync('node ./scripts/unified-build-ts.mjs', {
    cwd: __dirname,
    shell: true,
    stdio: 'inherit',
  });

  cb();
});

function buildUnified() {
  return gulp
    .src(['packages/unified/src/**/*.js', '!packages/unified/src/vendor/**'])
    .pipe(
      babel({
        ...babelConfig,
        babelrcRoots: [__dirname + '/packages/unified/**'],
      }),
    )
    .pipe(renameStream((relative) => relative.replace('src', 'lib')))
    .pipe(gulp.dest('packages/unified/lib'));
}

function buildBabel() {
  return gulp
    .src(paths.packageSrc)
    .pipe(babel({...babelConfig, babelrcRoots: [__dirname + '/packages/*/*']}))
    .pipe(renameStream((relative) => relative.replace('src', 'lib')))
    .pipe(gulp.dest(paths.packages));
}

function copyOthers() {
  return gulp
    .src(paths.packageOther)
    .pipe(renameStream((relative) => relative.replace('src', 'lib')))
    .pipe(gulp.dest(paths.packages));
}

function renameStream(fn) {
  return new TapStream((vinyl) => {
    let relative = path.relative(vinyl.base, vinyl.path);
    vinyl.path = path.join(vinyl.base, fn(relative));
  });
}

exports.default = exports.build = gulp.series(
  'clean',
  buildUnified,
  'unified:post',
  'prepare',
  buildBabel,
  copyOthers,
  'typescript',
);
