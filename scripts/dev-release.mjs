/* eslint-disable no-console */

import * as path from 'node:path';
import * as url from 'node:url';
import {execFileSync} from 'node:child_process';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);

const {ATLASPACK_RELEASE_TAG = 'dev'} = process.env;
const lerna = path.join(__root, 'node_modules', '.bin', 'lerna');

try {
  const releaseTag = stringToSlug(ATLASPACK_RELEASE_TAG);

  // Get git commit hash (first 7 characters)
  const gitCommitHash = execFileSync('git', ['rev-parse', '--short', 'HEAD'], {
    cwd: __root,
    encoding: 'utf8',
  }).trim();

  const preid = `${releaseTag}-${gitCommitHash}`;
  console.log('Releasing with tag:', releaseTag);
  console.log('Preid:', preid);

  execFileSync(
    'node',
    [
      lerna,
      'publish',
      '--yes',
      `--preid="${preid}"`,
      `--dist-tag="${releaseTag}"`,
      '--exact',
      '--force-publish="*"',
      '--no-git-tag-version',
      '--no-push',
      'prepatch',
    ],
    {
      shell: true,
      stdio: 'inherit',
    },
  );
} catch (error) {
  process.exit(1);
}

/**
 * Converts any string to a valid npm tag/package name format.
 *
 * @param {string}
 * @returns {string}
 *
 * @example
 * stringToSlug("My Feature Branch")     // → "my-feature-branch"
 * stringToSlug("hotfix/urgent-fix")     // → "hotfix-urgent-fix"
 * stringToSlug("")                      // → "dev"
 * stringToSlug("---test---")            // → "test"
 * stringToSlug("multiple   spaces")     // → "multiple-spaces"
 * stringToSlug("café & résumé")         // → "cafe-resume"
 */
function stringToSlug(str) {
  // Handle null, undefined, or non-string inputs
  if (!str || typeof str !== 'string') {
    return 'dev';
  }

  // Normalize: trim whitespace and convert to lowercase
  str = str.trim().toLowerCase();

  // Handle accented characters and common special characters
  var from = 'àáäâèéëêìíïîòóöôùúüûñç·/_,:;';
  var to = 'aaaaeeeeiiiioooouuuunc------';
  for (var i = 0, l = from.length; i < l; i++) {
    str = str.replace(new RegExp(from.charAt(i), 'g'), to.charAt(i));
  }

  // Remove all characters except lowercase letters, numbers, spaces, and hyphens
  str = str.replace(/[^a-z0-9 -]/g, '');

  // Replace multiple spaces with single space
  str = str.replace(/\s+/g, ' ');

  // Replace spaces with hyphens
  str = str.replace(/\s/g, '-');

  // Remove leading and trailing hyphens
  str = str.replace(/^-+|-+$/g, '');

  // Replace multiple consecutive hyphens with single hyphen
  str = str.replace(/-+/g, '-');

  // Ensure the result is not empty and has reasonable length
  if (!str || str.length === 0) {
    return 'dev';
  }

  return str;
}
