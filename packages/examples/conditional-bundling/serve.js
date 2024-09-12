const express = require('express');
const fs = require('node:fs');
const path = require('node:path');

const FEATURES = {
  'my.feature': true,
  'feature.async.condition': true,
  'feature.ui': true,
};

const app = express();

app.get('/', (req, res, next) => {
  const manifest = JSON.parse(
    fs.readFileSync('dist/conditional-manifest.json', 'utf8'),
  );
  let index = fs.readFileSync('dist/index.html', 'utf-8');
  const scripts = [];
  for (const [feature, state] of Object.entries(FEATURES)) {
    const featureManifest = manifest[feature];
    if (!featureManifest) continue;
    for (const asset of featureManifest[
      state ? 'ifTrueBundles' : 'ifFalseBundles'
    ]) {
      const script = `<script type="module" src="/${path.relative(
        'dist/',
        asset,
      )}"></script>`;
      scripts.push(script);
    }
  }
  const pos = index.indexOf('<script');
  index = `${index.slice(
    0,
    pos,
  )}<script>globalThis.__MOD_COND = ${JSON.stringify(
    FEATURES,
  )}</script>${scripts.join('\n')}${index.slice(pos)}`;
  index.slice(pos);
  res.contentType = 'text/html';
  res.send(index);
});
app.use(express.static('dist'));
app.listen(3000, () => {
  console.log('Server is running on http://localhost:3000');
});
