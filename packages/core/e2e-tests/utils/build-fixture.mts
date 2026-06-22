import {createHash} from 'node:crypto';
import * as path from 'node:path';
import * as fs from 'node:fs';
import * as url from 'node:url';
import {Atlaspack} from '@atlaspack/core';
import type {ServeContext} from './server.mts';
import type {
  BuildSuccessEvent,
  InitialAtlaspackOptions,
} from '@atlaspack/types';
import {setupThreeJsProject, cleanupThreeJsProject} from './three-js-setup.mts';
import {THREE_JS_CONFIG} from '../benchmarks/config.mts';

/**
 * Derive a readable output directory name from the fixture target path.
 *
 * Uses the fixture's directory name (e.g. `simple-project-with-css-map-scoped-extracted`)
 * so the build output is easy to locate under `dist/`, falling back to a sha256 hash
 * suffix when the same fixture is built more than once in a single test run (rare —
 * only happens if two distinct targets share a folder name).
 */
function fixtureOutputName(target: string): string {
  const segments = target.split(/[\\/]/).filter(Boolean);
  // Use the parent dir of the entry file (e.g. `.../foo/index.html` → `foo`).
  const fixtureDir =
    segments.length >= 2
      ? segments[segments.length - 2]
      : (segments[0] ?? 'fixture');
  const safe = fixtureDir.replace(/[^a-zA-Z0-9._-]/g, '_');
  // Append a short hash only when the target is more complex than `<dir>/index.html`,
  // to disambiguate without making the typical case unreadable.
  const isSimpleEntry = segments.length === 2 && segments[1] === 'index.html';
  if (isSimpleEntry) return safe;
  const suffix = createHash('sha256').update(target).digest('hex').slice(0, 8);
  return `${safe}-${suffix}`;
}

const __filename = url.fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const __root = path.dirname(__dirname);

export function mergeParcelOptions(
  optsOne: InitialAtlaspackOptions,
  optsTwo?: InitialAtlaspackOptions | null | undefined | void,
): InitialAtlaspackOptions {
  if (!optsTwo) {
    return optsOne;
  }

  return {
    ...optsOne,
    ...optsTwo,
    defaultTargetOptions: {
      ...optsOne?.defaultTargetOptions,
      ...optsTwo?.defaultTargetOptions,
    },
    featureFlags: {
      ...optsOne?.featureFlags,
      ...optsTwo?.featureFlags,
    },
  };
}

export async function buildFixture(
  target: string,
  config: InitialAtlaspackOptions = {},
): Promise<{
  outputDir: string;
  buildResult: BuildSuccessEvent;
}> {
  // Handle three.js project specially
  if (target.includes('three-js-project')) {
    return buildThreeJsFixture(target, config);
  }
  const output = fixtureOutputName(target);
  const outputDir = path.join(__root, 'dist', output);

  if (fs.existsSync(outputDir)) {
    fs.rmSync(outputDir, {
      recursive: true,
      force: true,
    });
  }

  const atlaspack = new Atlaspack(
    mergeParcelOptions(
      {
        entries: [path.join(__root, 'test', 'data', target)],
        defaultTargetOptions: {
          distDir: outputDir,
        },
        defaultConfig: url.fileURLToPath(
          import.meta.resolve('@atlaspack/config-default'),
        ),
      },
      config,
    ),
  );

  const buildResult = await atlaspack.run();
  return {outputDir, buildResult};
}

export async function serveFixture(target: string): Promise<ServeContext> {
  const output = fixtureOutputName(target);
  const outputDir = path.join(__root, 'dist', output);
  const randomPort = Math.floor(Math.random() * 10000) + 10000;

  if (fs.existsSync(outputDir)) {
    fs.rmSync(outputDir, {
      recursive: true,
      force: true,
    });
  }

  const atlaspack = new Atlaspack({
    entries: [path.join(__root, 'test', 'data', target)],
    defaultTargetOptions: {
      distDir: outputDir,
    },
    serveOptions: {
      port: randomPort,
    },
    defaultConfig: url.fileURLToPath(
      import.meta.resolve('@atlaspack/config-default'),
    ),
  });

  const subscription = await atlaspack.watch();

  return {
    address: `http://localhost:${randomPort}`,
    close() {
      subscription.unsubscribe();
    },
  };
}

async function buildThreeJsFixture(
  target: string,
  config: InitialAtlaspackOptions = {},
): Promise<{
  outputDir: string;
  buildResult: BuildSuccessEvent;
}> {
  const output = fixtureOutputName(target);
  const outputDir = path.join(__root, 'dist', output);

  if (fs.existsSync(outputDir)) {
    fs.rmSync(outputDir, {
      recursive: true,
      force: true,
    });
  }

  // Setup the three.js project (this will clone the repo if needed)
  const threeJsProjectDir = await setupThreeJsProject({
    copies: THREE_JS_CONFIG.copies,
    branch: THREE_JS_CONFIG.branch,
    repoUrl: THREE_JS_CONFIG.repoUrl,
  });

  try {
    const atlaspack = new Atlaspack(
      mergeParcelOptions(
        {
          entries: [path.join(threeJsProjectDir, 'index.html')],
          defaultTargetOptions: {
            distDir: outputDir,
          },
          defaultConfig: url.fileURLToPath(
            import.meta.resolve('@atlaspack/config-default'),
          ),
        },
        config,
      ),
    );

    const buildResult = await atlaspack.run();
    return {outputDir, buildResult};
  } catch (error) {
    // Clean up on error
    await cleanupThreeJsProject(threeJsProjectDir);
    throw error;
  }
}
