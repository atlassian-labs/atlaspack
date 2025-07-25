const {Transform} = require('stream');
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

  // Static packages that don't need to be build
  '!packages/core/atlaspack/**',
];

const paths = {
  packageSrc: [
    'packages/*/*/src/**/*.js',
    'packages/*/*/src/**/*.ts',
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

exports.clean = function clean(cb) {
  rimraf('packages/*/*/lib/**').then(
    () => cb(),
    (err) => cb(err),
  );
};

exports.default = exports.build = gulp.parallel(buildBabel, copyOthers);

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
