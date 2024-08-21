const fs = require('fs');
const path = require('path');
const exec = require('child_process').execSync;

let packages = JSON.parse(
  exec(
    `${path.join(__dirname, '..', 'node_modules', '.bin', 'lerna')} ls  --json`,
  ),
);
let packageVersions = new Map(
  packages.map(pkg => [
    pkg.name,
    {version: pkg.version, location: pkg.location},
  ]),
);
let coreVersion = packageVersions.get('@atlaspack/core').version;
let coreRange =
  coreVersion.includes('canary') || process.argv.includes('--exact')
    ? coreVersion
    : `^${coreVersion}`;

console.log('in engines peer deps');
for (let [, {location}] of packageVersions) {
  let pkgPath = path.join(location, 'package.json');
  let pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
  if (pkg.engines?.atlaspack != null) {
    pkg.engines.atlaspack = coreRange;
    console.log('updating atlaspack engine', pkg.name, coreRange);
    fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
  }
  if (pkg.peerDependencies?.['@atlaspack/core'] != null) {
    console.log('updating atlaspack peer dep', pkg.name, coreRange);
    pkg.peerDependencies['@atlaspack/core'] = coreRange;
    fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
  }
}
