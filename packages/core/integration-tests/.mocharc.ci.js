const mocharc = require('.mocharc.js');

module.exports = {
  ...mocharc,
  retries: 2,
  timeout: 50000,
};
