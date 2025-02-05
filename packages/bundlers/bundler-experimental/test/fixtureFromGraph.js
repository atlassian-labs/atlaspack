// @flow strict-local

/*!
 * This module provides a way to write fixtures where we don't care about the
 * code within the assets; only the shape of the asset graphs.
 */
import type {FileSystem} from '@atlaspack/fs';
import path from 'path';

/**
 * A node in the fixture graph
 */
export type GraphEntry = AssetEntry;

/**
 * An asset in the fixture graph. Just a path and dependencies
 */
export type AssetEntry = {|
  type: 'asset',
  value: {|
    filePath: string,
    dependencies: DependencyEntry[],
  |},
|};

/**
 * Sync or async dependency between assets
 */
export type DependencyEntry = {|
  type: 'dependency',
  value: {|
    from: string,
    to: string,
    type: 'sync' | 'async',
  |},
|};

export type DependencySpec = {|
  to: string,
  type: 'sync' | 'async',
|};

/**
 * Create an asset node in the fixture graph
 */
export function asset(
  path: string,
  dependencies?: (string | DependencySpec)[],
): GraphEntry {
  return {
    type: 'asset',
    value: {
      filePath: path,
      dependencies:
        dependencies?.map((dependency) => {
          if (typeof dependency === 'string') {
            return {
              type: 'dependency',
              value: {
                from: path,
                to: dependency,
                type: 'sync',
              },
            };
          } else {
            return {
              type: 'dependency',
              value: {
                from: path,
                to: dependency.to,
                type: dependency.type,
              },
            };
          }
        }) ?? [],
    },
  };
}

/**
 * Create the files for a fixture graph over the `fs` filesystem.
 */
export async function fixtureFromGraph(
  dirname: string,
  fs: FileSystem,
  entries: GraphEntry[],
): Promise<string> {
  await fs.mkdirp(dirname);

  for (let entry of entries) {
    if (entry.type === 'asset') {
      const dependencies = entry.value.dependencies ?? [];
      const symbols = dependencies
        .filter((d) => d.value.type === 'sync')
        .map((_, i) => `d${i}`);
      const asyncDependencies = dependencies
        .filter((d) => d.value.type === 'async')
        .map((d) => `import('./${d.value.to}')`);
      const contents = [
        ...dependencies
          .filter((d) => {
            return d.value.type === 'sync';
          })
          .map((dependency, i) => {
            return `import ${symbols[i]} from './${dependency.value.to}';`;
          }),
        `export default function run() { return [${[
          ...symbols,
          ...asyncDependencies,
        ].join(', ')}] }`,
      ].join('\n');

      await fs.writeFile(path.join(dirname, entry.value.filePath), contents);
    }
  }

  return dotFromGraph(entries);
}

/**
 * Create a graphviz dot string from a fixture graph
 *
 * The contents of the GraphViz file will contain (in this order):
 *
 * * the list of all asset nodes
 * * the dependencies between assets
 *
 * Given a graph build with:
 *
 * ```
 * fixtureFromGraph('dir', fs, [
 *   asset('a.js', ['b.js', 'c.js']),
 *   asset('b.js', ['d.js']),
 *   asset('d.js'),
 * ])
 * ```
 *
 * The output will be:
 * ```
 * digraph assets {
 *   labelloc="t";
 *   label="Assets";
 *
 *   "a.js";
 *   "b.js";
 *   "c.js";
 *   "d.js";
 *
 *   "a.js" -> "b.js";
 *   "a.js" -> "c.js";
 *   "b.js" -> "d.js";
 * }
 * ```
 *
 * That is:
 *
 * * iterate all nodes and declare them
 * * iterate all dependencies between nodes and declare them
 *
 */
export function dotFromGraph(entries: GraphEntry[]): string {
  const contents = [];

  for (let entry of entries) {
    if (entry.type === 'asset') {
      const asset = entry.value;
      contents.push(`"${asset.filePath}";`);
    }
  }

  contents.push('');

  for (let entry of entries) {
    if (entry.type === 'asset') {
      const asset = entry.value;
      for (let dependency of entry.value.dependencies) {
        contents.push(`"${asset.filePath}" -> "${dependency.value.to}";`);
      }
    }
  }

  return `
digraph assets {
  labelloc="t";
  label="Assets";

${contents.map((line) => (line.length > 0 ? `  ${line}` : '')).join('\n')}
}
  `.trim();
}
