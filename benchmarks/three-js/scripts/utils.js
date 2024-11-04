const fs = require('node:fs');
const path = require('node:path');

function rm(target) {
  fs.rmSync(target, {force: true, recursive: true});
}

function cp(source, dest) {
  fs.cpSync(source, dest, {recursive: true});
}

function readJson(target) {
  return JSON.parse(fs.readFileSync(target, 'utf8'));
}
function writeJson(target, data) {
  fs.writeFileSync(target, JSON.stringify(data, null, 2));
}

function append(target, data) {
  fs.appendFileSync(target, data, 'utf8');
}

const paths = {
  '~': (...segments) => path.join(__dirname, '..', ...segments),
};

module.exports = {
  rm,
  cp,
  readJson,
  writeJson,
  append,
  paths,
};
