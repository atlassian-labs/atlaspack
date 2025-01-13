// @flow
import process from 'process';

// https://nodejs.org/api/packages.html#conditional-exports
export const defaultNodejsConditions: Array<string> = Object.freeze([
  'node-addons',
  'node',
  'import',
  'require',
  'module-sync',
  'default',
]);

let envConditions: void | Array<string> = undefined;

/** @description Gets the export conditions from NODE_OPTIONS and node arguments */
export function getConditionsFromEnv(): Array<string> {
  if (!envConditions) {
    const conditions = [];

    for (const arg of [
      ...process.execArgv,
      ...(process.env.NODE_OPTIONS || '').split(' '),
    ]) {
      if (arg.startsWith('--conditions=')) {
        conditions.push(arg.substring(13));
      }
    }

    envConditions = Object.freeze([...conditions, ...defaultNodejsConditions]);
  }

  return envConditions;
}
