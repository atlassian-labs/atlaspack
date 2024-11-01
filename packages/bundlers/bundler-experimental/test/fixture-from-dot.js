// @flow strict-local

import type {BundleGraph, Bundle} from '@atlaspack/types';
import type {FileSystem} from '@atlaspack/fs';

import path from 'path';

type GraphEntry = AssetEntry;

type AssetEntry = {|
  type: 'asset',
  value: {|
    filePath: string,
    dependencies: DependencyEntry[],
  |},
|};

type DependencyEntry = {|
  type: 'dependency',
  value: {|
    from: string,
    to: string,
    type: 'sync' | 'async',
  |},
|};

type DependencySpec = {|
  to: string,
  type: 'sync' | 'async',
|};

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

export async function fixtureFromGraph(
  dirname: string,
  fs: FileSystem,
  entries: GraphEntry[],
): Promise<string> {
  await fs.mkdirp(dirname);

  for (let entry of entries) {
    if (entry.type === 'asset') {
      const dependencies = entry.value.dependencies ?? [];
      const symbols = dependencies.map((_, i) => `d${i}`);
      const contents = [
        ...dependencies.map((dependency, i) => {
          return `import ${symbols[i]} from './${dependency.value.to}';`;
        }),
        `export function run() { return [${symbols.join(', ')}] }`,
      ].join('\n');

      await fs.writeFile(path.join(dirname, entry.value.filePath), contents);
    }
  }

  return dotFromGraph(entries);
}

export function dotFromBundleGraph<B: Bundle>(
  entryDir: string,
  bundleGraph: BundleGraph<B>,
): string {
  const cleanPath = (p) => {
    if (p.includes('esmodule-helpers.js')) {
      return 'esmodule_helpers.js';
    }
    return path.relative(entryDir, p);
  };
  const contents = [];

  const bundles = bundleGraph.getBundles();

  for (let bundle of bundles) {
    const bundleId = bundle.id;
    contents.push(`subgraph cluster_${bundleId} {`);
    contents.push(`  label = "Bundle ${bundleId}";`);

    bundle.traverseAssets((asset) => {
      contents.push(`  "${cleanPath(asset.filePath)}";`);
    });

    contents.push('}');
  }

  return `
digraph bundle_graph {
  labelloc="t";
  label="Bundle graph";

${contents.map((line) => (line.length > 0 ? `  ${line}` : '')).join('\n')}
}
  `;
}
