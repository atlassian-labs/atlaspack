module.exports = function loadCond(cond, ifTrue, ifFalse) {
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
      'Conditional dependency was missing. Ensure the server sends the correct scripts to the client ("conditional-manifest.json").',
    );

    throw err;
  }
};
