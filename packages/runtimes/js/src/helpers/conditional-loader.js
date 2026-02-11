const {getBundleURL} = require('./bundle-url');
const {resolve} = require('./bundle-manifest');

module.exports = function loadCond(cond, ifTrue, ifFalse, fallback) {
  let result = globalThis.__MCOND(cond);
  try {
    return result ? ifTrue() : ifFalse();
  } catch (err) {
    if (fallback) {
      globalThis.__ATLASPACK_ERRORS = globalThis.__ATLASPACK_ERRORS || [];
      globalThis.__ATLASPACK_ERRORS.push(
        new Error(
          `Sync dependency fallback triggered for condition "${cond}": ${err.message}`,
        ),
      );
      for (const id of fallback.i) {
        fallback.l(new URL(resolve(id), getBundleURL(id)).toString());
      }

      return result ? ifTrue() : ifFalse();
    } else {
      throw err;
    }
  }
};
