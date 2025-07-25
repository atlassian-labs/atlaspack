import process from 'process';

// https://nodejs.org/api/packages.html#conditional-exports
// TODO We don't support { "type": "module" }
// @ts-expect-error TS4104
export const defaultNodejsConditions: Array<string> = Object.freeze([
  'node-addons',
  'node',
  // 'import',
  'require',
  'module-sync',
  'default',
]);

let envConditions: undefined | Array<string> = undefined;

/** @description Gets the export conditions from NODE_OPTIONS and node arguments */
export function getConditionsFromEnv(): Array<string> {
  if (!envConditions) {
    const conditions: Array<never> = [];

    for (const arg of [
      ...process.execArgv,
      ...(process.env.NODE_OPTIONS || '').split(' '),
    ]) {
      if (arg.startsWith('--conditions=')) {
        // @ts-expect-error TS2345
        conditions.push(arg.substring(13));
      }
    }

    // @ts-expect-error TS4104
    envConditions = Object.freeze([...conditions, ...defaultNodejsConditions]);
  }

  // @ts-expect-error TS2322
  return envConditions;
}
