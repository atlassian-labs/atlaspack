import {createHash} from 'node:crypto';
import * as path from 'node:path';
import * as fs from 'node:fs';
import * as url from 'node:url';
import {Atlaspack} from '@atlaspack/core';
import type {ServeContext} from './server.mts';
import type {AsyncSubscription} from '@atlaspack/types';
import type {FeatureFlags} from '@atlaspack/feature-flags';

const __filename = url.fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const __root = path.dirname(__dirname);

export async function buildFixture(target: string): Promise<string> {
  const output = createHash('sha256').update(target).digest('hex');
  const outputDir = path.join(__root, 'dist', output);

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
    defaultConfig: url.fileURLToPath(
      import.meta.resolve('@atlaspack/config-default'),
    ),
  });

  await atlaspack.run();
  return outputDir;
}

export async function serveFixture(
  target: string,
  options: {
    featureFlags?: Partial<FeatureFlags>;
  } = {},
): Promise<ServeContext> {
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
    featureFlags: {
      ...options.featureFlags,
    },
    serveOptions: {
      port: randomPort,
    },
    defaultConfig: url.fileURLToPath(
      import.meta.resolve('@atlaspack/config-default'),
    ),
  });

  let resolveReady: () => void;
  let rejectReady: (reason?: any) => void;
  const buildReady = new Promise<void>((resolve, reject) => {
    resolveReady = resolve;
    rejectReady = reject;
  });

  const subscriptionPromise: Promise<AsyncSubscription> = atlaspack.watch(
    (err, event) => {
      if (err) {
        rejectReady(err);
        return;
      }
      if (event?.type === 'buildSuccess') {
        resolveReady();
      }
    },
  );

  const subscription: AsyncSubscription = await subscriptionPromise;
  await buildReady;

  return {
    address: `http://localhost:${randomPort}`,
    async close() {
      await subscription.unsubscribe();
    },
  };
}
