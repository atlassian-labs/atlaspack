module.exports = function loadCond(cond, ifTrue, ifFalse, fallback) {
  if (typeof globalThis.__MCOND !== 'function') {
    throw new TypeError(
      '"globalThis.__MCOND" was not set to an object. Ensure the function is set to return the key condition for conditional bundles to load with.',
    );
  }

  if (typeof globalThis.__MCOND(cond) === 'undefined') {
    console.error(
      `"${cond}" did not match on globalThis.__MCOND. The conditional dependency will be loaded with the false variant.`,
    );
  }

  try {
    return globalThis.__MCOND(cond) ? ifTrue() : ifFalse();
  } catch (err) {
    console.error(
      'Conditional dependency was not registered when executing. Ensure the server sends the correct scripts to the client. Falling back to synchronous bundle loading.',
    );

    if (fallback) {
      globalThis.__ATLASPACK_ERRORS = globalThis.__ATLASPACK_ERRORS || [];
      globalThis.__ATLASPACK_ERRORS.push(
        new Error(
          `Sync dependency fallback triggered for condition "${cond}": ${err.message}`,
        ),
      );
      for (const url of fallback.urls) {
        fallback.l(url);
      }

      return globalThis.__MCOND(cond) ? ifTrue() : ifFalse();
    } else {
      throw new Error('No fallback urls specified, cannot fallback safely');
    }
  }
};
