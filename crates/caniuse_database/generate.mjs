import {writeFileSync} from 'node:fs';

import {$} from 'zx';

await $`rm -f data.json`;
try {
  await $`wget --version`
} catch {
  console.error('wget is not installed. Please install it to run the script.');
  process.exit(1);
}
await $`wget https://raw.githubusercontent.com/Fyrd/caniuse/main/data.json`;

// This import is dependent on the caniuse await above
// import data from './data.json' with { type: "json" };

const data = (await import('./data.json', {
  assert: {type: 'json'},
})).default;

const write = console.log;

write('#![allow(clippy::all)]');
write('//! This file was automatically generated and should not be edited manually.');
write('//!');
write('//! Use `yarn workspace caniuse-database generate` to regenerate the contents of this file.');
write('//!');
write('');

const browserAgents = Object.entries(data.agents).map(([key, agent]) => ({
  name: capitalize(key),
  comment: [...agent.browser.split('\n')],
  key,
}));

write('use serde::{Deserialize, Serialize};');

write('');

write('#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]');
write('pub enum BrowserAgent {');
browserAgents.forEach(agent => {
  agent.comment.forEach(comment => {
    write(`  /// ${comment}`);
  });
  write(`  ${agent.name},`);
});
write(`  /// Any other browser`);
write(`  Any(String),`);
write('}');

write('');

write('impl BrowserAgent {');
write('  pub fn key(&self) -> &str {');
write('    match self {');
browserAgents.forEach(agent => {
  write(`      BrowserAgent::${agent.name} => "${agent.key}",`);
});
write(`      BrowserAgent::Any(key) => key,`);
write('    }');
write('  }');
write('');
write('  pub fn from_key(key: &str) -> Self {');
write('    match key {');
browserAgents.forEach(agent => {
  write(`      "${agent.key}" => BrowserAgent::${agent.name},`);
});
write('      key => BrowserAgent::Any(key.to_string()),');
write('    }');
write('  }');
write('}');

write('');

const featuresEnum = Object.entries(data.data).map(([key, feature]) => ({
  name: capitalize(key),
  key,
  comment: [
    ...feature.title.split('\n'),
    '',
    ...feature.description.split('\n'),
    '',
    ...feature.links.map(link => `* [${link.title}](${link.url})`),
  ],
}));

write('#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]');
write('pub enum BrowserFeature {');
featuresEnum.forEach(feature => {
  feature.comment.forEach(comment => {
    write(`  /// ${comment}`);
  });
  write(`  ${feature.name},`);
});
write(`  /// Any other browser feature`);
write(`  Any(String),`);
write('}');

write('');

write('impl BrowserFeature {');
write('  pub fn key(&self) -> &str {');
write('    match self {');
featuresEnum.forEach(feature => {
  write(`      BrowserFeature::${feature.name} => "${feature.key}",`);
});
write(`      BrowserFeature::Any(key) => key,`);
write('    }');
write('  }');
write('');
write('  pub fn from_key(key: &str) -> Self {');
write('    match key {');
featuresEnum.forEach(feature => {
  write(`      "${feature.key}" => BrowserFeature::${feature.name},`);
});
write('      key => BrowserFeature::Any(key.to_string()),');
write('    }');
write('  }');
write('}');

const minimalData = {};
for (const [key, feature] of Object.entries(data.data)) {
  const minimalStats = {};
  for (let [browser, versions] of Object.entries(feature.stats)) {
    const minimalVersions = {};
    const requirements = collapseRequirements(versions);
    for (let range of requirements) {
      minimalVersions[range] = 1;
    }
    minimalStats[browser] = minimalVersions;
  }
  minimalData[key] = minimalStats;
}

writeFileSync('src/data.json', JSON.stringify(minimalData, null, 2));

function capitalize(s) {
  const parts = s.split(/[\W_]/g);
  return parts
    .map(part => {
      return part.charAt(0).toUpperCase() + part.slice(1);
    })
    .join('');
}

function collapseRequirements(versions) {
  let currentMinimum = null;
  let currentMaximum = null;
  const ranges = [];

  for (let [version, supports] of Object.entries(versions)) {
    if (supports === 'y') {
      if (currentMinimum === null) {
        currentMinimum = version;
      }
      currentMaximum = version;
    } else {
      if (currentMinimum !== null) {
        const range = [currentMinimum];
        if (currentMaximum !== currentMinimum) {
          range.push(currentMaximum);
        }
        ranges.push(range.join('-'));
      }
      currentMinimum = null;
      currentMaximum = null;
    }
  }

  if (currentMinimum !== null) {
    const range = [currentMinimum];
    if (currentMaximum !== currentMinimum) {
      range.push(currentMaximum);
    }
    ranges.push(range.join('-'));
  }

  return ranges;
}
