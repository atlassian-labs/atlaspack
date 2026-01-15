/* eslint-disable no-console */
import {execSync} from 'child_process';

function hasUnstagedChanges() {
  // Check for unstaged changes in main branch
  try {
    const result = execSync('git status --porcelain', {encoding: 'utf8'});
    if (result.trim().length > 0) {
      console.log('Unstaged changes detected in main branch:');
      console.log(result);

      console.log(execSync('git diff', {encoding: 'utf8'}));

      return true;
    } else {
      console.log('No unstaged changes in main branch');
      return false;
    }
  } catch (error) {
    console.error('Error checking git status:', error.message);
    return true;
  }
}

function main() {
  if (hasUnstagedChanges()) {
    process.exitCode = 1;
  }
}

main();
