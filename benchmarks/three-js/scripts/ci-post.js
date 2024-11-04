/* eslint-disable no-console */

const fs = require('node:fs');
const {readJson, paths} = require('./utils');

void (function main() {
  let total = 0;
  let count = 0;

  for (const report of fs.readdirSync(paths['~']('reports'))) {
    count += 1;
    const {buildTime} = readJson(paths['~']('reports', report));
    total += buildTime;
  }

  console.log(total / count);
})();
