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
  const output = createHash('sha256').update(target).digest('hex');
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
  const output = createHash('sha256').update(target).digest('hex');
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
  const output = createHash('sha256').update(target).digest('hex');
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
