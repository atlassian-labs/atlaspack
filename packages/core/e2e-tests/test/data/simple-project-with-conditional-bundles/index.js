const conditions = {cond1: true, cond2: true};
globalThis.__MCOND = function (key) {
  return conditions[key];
};

// eslint-disable-next-line no-undef
const imported1 = importCond('cond1', './a', './b');

// eslint-disable-next-line no-undef
document.getElementById('output').innerText = `Hello, ${imported1}!`;
