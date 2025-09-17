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
