/* eslint-disable no-console, monorepo/no-internal-import */
import v8 from 'v8';
import path from 'path';
import fs from 'fs';
import invariant from 'assert';

const {
  InternalBundleGraph: {default: InternalBundleGraph},
  LMDBLiteCache,
} = require('./deep-imports');

/**
 * Load the BundleGraph from an Atlaspack cache directory.
 * Supports both new format (BundleGraph/ keys in LMDB) and
 * old format ({hash}-BundleGraph files in cache directory).
 */
export async function loadBundleGraphFromCache(cacheDir: string): Promise<{
  bundleGraph: InstanceType<typeof InternalBundleGraph>;
  cache: InstanceType<typeof LMDBLiteCache>;
}> {
  const cache = new LMDBLiteCache(cacheDir);

  // First, try the new format: BundleGraph/ keys in LMDB
  let bundleGraphBlob: string | null = null;
  for (const key of cache.keys()) {
    if (key.startsWith('BundleGraph/')) {
      bundleGraphBlob = key;
      break;
    }
  }

  if (bundleGraphBlob != null) {
    const file = await cache.getBlob(bundleGraphBlob);
    const obj = v8.deserialize(file);
    invariant(obj.bundleGraph != null, 'BundleGraph data is null');
    const bundleGraph = InternalBundleGraph.deserialize(obj.bundleGraph.value);
    return {bundleGraph, cache};
  }

  // Fall back to old format: {hash}-BundleGraph-{chunk} files in cache directory
  const files = await fs.promises.readdir(cacheDir);
  const bundleGraphFiles = files
    .filter((f) => f.endsWith('-BundleGraph-0'))
    .map((f) => ({
      name: f,
      key: f.replace(/-0$/, ''),
    }));

  if (bundleGraphFiles.length === 0) {
    throw new Error('BundleGraph not found in cache. Run a build first.');
  }

  const withStats = await Promise.all(
    bundleGraphFiles.map(async (f) => {
      const stat = await fs.promises.stat(path.join(cacheDir, f.name));
      return {...f, mtime: stat.mtime};
    }),
  );
  withStats.sort((a, b) => b.mtime.getTime() - a.mtime.getTime());
  const mostRecent = withStats[0];

  const chunks: Buffer[] = [];
  let chunkIndex = 0;
  // eslint-disable-next-line no-constant-condition
  while (true) {
    const chunkPath = path.join(cacheDir, `${mostRecent.key}-${chunkIndex}`);
    try {
      const chunk = await fs.promises.readFile(chunkPath);
      chunks.push(chunk);
      chunkIndex++;
    } catch {
      break;
    }
  }

  if (chunks.length === 0) {
    throw new Error('Failed to read BundleGraph chunks from cache.');
  }

  const file = Buffer.concat(chunks);
  const obj = v8.deserialize(file);
  invariant(obj.bundleGraph != null, 'BundleGraph data is null');
  const bundleGraph = InternalBundleGraph.deserialize(obj.bundleGraph.value);

  return {bundleGraph, cache};
}

/**
 * Read the packaged bundle content from the cache.
 * Checks the filesystem first (native packager path), then falls back to LMDB.
 */
export async function readPackagedContent(
  cache: InstanceType<typeof LMDBLiteCache>,
  cacheKeys: {content: string; map: string; info: string},
): Promise<{content: Buffer; sourceMap: Buffer | null}> {
  let content: Buffer;
  const filePath = path.join(cache.dir, cacheKeys.content);

  try {
    content = await fs.promises.readFile(filePath);
  } catch {
    content = await cache.getBlob(cacheKeys.content);
  }

  let sourceMap: Buffer | null = null;
  try {
    const mapFilePath = path.join(cache.dir, cacheKeys.map);
    sourceMap = await fs.promises.readFile(mapFilePath);
  } catch {
    try {
      if (await cache.has(cacheKeys.map)) {
        sourceMap = await cache.getBlob(cacheKeys.map);
      }
    } catch {
      // Source map not available
    }
  }

  return {content, sourceMap};
}

/**
 * Write packaged bundle content to an output file, optionally with a source map.
 */
export async function writePackagedContent(
  content: Buffer,
  outputPath: string,
  sourceMap?: Buffer | null,
): Promise<void> {
  const outputDir = path.dirname(outputPath);
  await fs.promises.mkdir(outputDir, {recursive: true});
  await fs.promises.writeFile(outputPath, content);
  if (sourceMap) {
    await fs.promises.writeFile(outputPath + '.map', sourceMap);
  }
}
