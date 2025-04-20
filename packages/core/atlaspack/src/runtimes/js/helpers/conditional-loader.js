module.exports = function loadCond(cond, ifTrue, ifFalse) {
  return globalThis.__MCOND && globalThis.__MCOND(cond) ? ifTrue() : ifFalse();
};
