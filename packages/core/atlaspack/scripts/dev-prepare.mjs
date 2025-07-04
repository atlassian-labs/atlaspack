import * as fs from 'node:fs';
import * as url from 'node:url';
import * as path from 'node:path';

const __filename = url.fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Copy config files and rename "@atlaspack/*" to "atlaspack/*"
function copyConfig(name, target) {
  const file = fs.readFileSync(
    url.fileURLToPath(import.meta.resolve(name)),
    'utf8',
  );

  const updated = file
    .replaceAll('@atlaspack/', 'atlaspack/')
    .replaceAll('/bundler-', '/bundler/')
    .replaceAll('/compressor-', '/compressor/')
    .replaceAll('/config-', '/config/')
    .replaceAll('/namer-', '/namer/')
    .replaceAll('/packager-', '/packager/')
    .replaceAll('/reporter-', '/reporter/')
    .replaceAll('/resolver-', '/resolver/')
    .replaceAll('/optimizer-', '/optimizer/')
    .replaceAll('/reporter-', '/reporter/')
    .replaceAll('/runtime-', '/runtime/')
    .replaceAll('/transformer-', '/transformer/')
    .replaceAll('/validator-', '/validator/');

  fs.writeFileSync(
    path.join(__dirname, '..', 'static', 'config', target, 'index.json'),
    updated,
    'utf8',
  );
}

copyConfig('@atlaspack/config-default', 'default');
copyConfig('@atlaspack/config-webextension', 'webextension');
