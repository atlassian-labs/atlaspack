const express = require('express');
const fs = require('node:fs');
const path = require('node:path');
const FEATURES = require('./features');

const app = express();

app.get('/', (req, res, next) => {
  let manifest = {};
  try {
    manifest = JSON.parse(
      fs.readFileSync('dist/conditional-manifest.json', 'utf8'),
    );
  } catch (err) {
    console.log('Manifest not loaded or found');
  }
  let index = fs.readFileSync('dist/index.html', 'utf-8');
  const assets = new Set();

  for (const [script, condition] of Object.entries(manifest)) {
    if (script.startsWith('index.')) {
      const scriptManifest = manifest[script];
      for (const [feature, state] of Object.entries(FEATURES)) {
        const featureManifest = scriptManifest[feature];

        if (!featureManifest) continue;

        for (const asset of featureManifest[
          state ? 'ifTrueBundles' : 'ifFalseBundles'
        ]) {
          console.log('Sending asset', asset, 'for condition', feature);

          assets.add(asset);
        }
      }
    }
  }

  const scripts = Array.from(assets).map(
    (asset) =>
      `<script type="module" src="/${path.relative('dist/', asset)}"></script>`,
  );

  const pos = index.indexOf('<script');
  index = `${index.slice(0, pos)}<script>const features = ${JSON.stringify(
    FEATURES,
  )};globalThis.__MCOND = (key) => features[key];</script>${scripts.join(
    '\n',
  )}${index.slice(pos)}`;
  index.slice(pos);
  res.contentType = 'text/html';
  res.send(index);
});
app.use(express.static('dist'));
app.listen(3000, () => {
  console.log('Server is running on http://localhost:3000');
});
