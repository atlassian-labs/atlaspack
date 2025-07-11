module.exports = function loadCond(cond, ifTrue, ifFalse, fallback) {
  let result = globalThis.__MCOND(cond);
  try {
    return result ? ifTrue() : ifFalse();
  } catch (err) {
    if (fallback) {
      console.error(
        'Conditional dependency was not registered when executing. Falling back to synchronous bundle loading.',
      );
      for (const url of fallback.urls) {
        fallback.loader(url);
      }

      return result ? ifTrue() : ifFalse();
    } else {
      throw err;
    }
  }
};
