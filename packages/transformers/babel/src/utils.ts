import type {Environment} from '@atlaspack/types';
// @ts-expect-error - TS7016 - Could not find a declaration file for module '@babel/preset-env'. '/home/ubuntu/parcel/node_modules/@babel/preset-env/lib/index.js' implicitly has an 'any' type.
import type {Targets as BabelTargets} from '@babel/preset-env';

import invariant from 'assert';
import semver from 'semver';

// Copied from @babel/helper-compilation-targets/lib/options.js
const TargetNames = {
  node: 'node',
  chrome: 'chrome',
  opera: 'opera',
  edge: 'edge',
  firefox: 'firefox',
  safari: 'safari',
  ie: 'ie',
  ios: 'ios',
  android: 'android',
  electron: 'electron',
  samsung: 'samsung',
  rhino: 'rhino',
} as const;

// List of browsers to exclude when the esmodule target is specified.
// Based on https://caniuse.com/#feat=es6-module
const ESMODULE_BROWSERS = [
  'not ie <= 11',
  'not edge < 16',
  'not firefox < 60',
  'not chrome < 61',
  'not safari < 11',
  'not opera < 48',
  'not ios_saf < 11',
  'not op_mini all',
  'not android < 76',
  'not blackberry > 0',
  'not op_mob > 0',
  'not and_chr < 76',
  'not and_ff < 68',
  'not ie_mob > 0',
  'not and_uc > 0',
  'not samsung < 8.2',
  'not and_qq > 0',
  'not baidu > 0',
  'not kaios > 0',
];

export function enginesToBabelTargets(env: Environment): BabelTargets {
  // "Targets" is the name @babel/preset-env uses for what Parcel calls engines.
  // This should not be confused with Parcel's own targets.
  // Unlike Parcel's engines, @babel/preset-env expects to work with minimum
  // versions, not semver ranges, of its targets.
  let targets: Record<string, any> = {};
  for (let engineName of Object.keys(env.engines)) {
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'string' can't be used to index type 'Engines'.
    let engineValue = env.engines[engineName];

    // if the engineValue is a string, it might be a semver range. Use the minimum
    // possible version instead.
    if (engineName === 'browsers') {
      targets[engineName] = engineValue;
    } else {
      invariant(typeof engineValue === 'string');
      if (!TargetNames.hasOwnProperty(engineName)) continue;
      let minVersion = semver.minVersion(engineValue)?.toString();
      targets[engineName] = minVersion ?? engineValue;
    }
  }

  if (env.outputFormat === 'esmodule' && env.isBrowser()) {
    // If there is already a browsers target, add a blacklist to exclude
    // instead of using babel's esmodules target. This allows specifying
    // a newer set of browsers than the baseline esmodule support list.
    // See https://github.com/babel/babel/issues/8809.
    if (targets.browsers) {
      let browsers = Array.isArray(targets.browsers)
        ? targets.browsers
        : [targets.browsers];
      targets.browsers = [...browsers, ...ESMODULE_BROWSERS];
    } else {
      targets.esmodules = true;
    }
  }

  return targets;
}
