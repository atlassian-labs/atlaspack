/* eslint-disable monorepo/no-internal-import */
/* eslint-disable no-console */

const program = require('commander');
const path = require('path');
const {LMDBLiteCache} = require('@atlaspack/cache');
const {loadGraphs} = require('@atlaspack/query');
const express = require('express');
const {spawn} = require('child_process');
const cors = require('cors');

function take(iterable, n) {
  const result = [];
  for (const item of iterable) {
    result.push(item);
    if (result.length >= n) {
      break;
    }
  }
  return result;
}

function getCacheStats(cache) {
  const stats = {
    size: 0,
    count: 0,
    keySize: 0,
    assetContentCount: 0,
    assetContentSize: 0,
    assetMapCount: 0,
    assetMapSize: 0,
  };

  for (const key of cache.keys()) {
    const value = cache.getBlobSync(key);
    stats.size += value.length;
    stats.keySize += Buffer.from(key).length;
    stats.count++;
    if (key.endsWith(':content')) {
      stats.assetContentCount++;
      stats.assetContentSize += value.length;
    } else if (key.endsWith(':map')) {
      stats.assetMapCount++;
      stats.assetMapSize += value.length;
    }
  }

  return stats;
}

async function main() {
  const command = program
    .requiredOption('-t, --target <path>', 'Path to the target cache')
    .parse(process.argv);

  const options = command.opts();
  const cache = new LMDBLiteCache(options.target);

  const {assetGraph, bundleGraph, requestTracker, bundleInfo, cacheInfo} =
    await loadGraphs(options.target);

  process.chdir(path.join(__dirname, 'frontend'));

  const child = spawn(
    'atlaspack',
    ['serve', '--no-cache', path.join(__dirname, './frontend/index.html')],
    {
      stdio: 'inherit',
      env: {
        ...process.env,
        PORT: '3333',
      },
    },
  );
  process.on('beforeExit', () => {
    child.kill();
  });

  const app = express();

  app.use(
    cors({
      origin: 'http://localhost:3333',
      credentials: true,
    }),
  );

  app.use((req, res, next) => {
    if (res.headersSent) {
      console.log(req.method, req.url, res.statusCode);
    } else {
      res.on('finish', function () {
        console.log(req.method, req.url, res.statusCode);
      });
    }
    next();
  });

  app.get('/', (req, res) => {
    res.sendFile(path.join(process.cwd(), './dist/index.html'));
  });
  // catch all in /app*
  app.get('/app/{*path}', (req, res) => {
    res.sendFile(path.join(process.cwd(), './dist/index.html'));
  });
  app.use(express.static(path.join(process.cwd(), './dist')));

  app.get('/api/asset-graph', (req, res) => {});

  app.get('/api/stats', (req, res) => {
    const stats = getCacheStats(cache);
    res.json(stats);
  });

  app.get('/api/cache-keys/', (req, res) => {
    const sortBy = req.query.sortBy;

    let keys = Array.from(cache.keys());

    if (sortBy === 'size') {
      const sizes = keys.map((key) => [key, cache.getBlobSync(key).length]);
      sizes.sort((a, b) => b[1] - a[1]);
      keys = sizes.map(([key]) => key);
    }

    res.json({
      keys,
      count: keys.length,
    });
  });

  app.get('/api/cache-value/:key', (req, res) => {
    const key = req.params.key;

    // const value = cache.getBlobSync(key);
    try {
      const value = cache.getBlobSync(key);
      // bigger than 1MB
      if (value.length > 1024 * 1024) {
        res.json({
          size: value.length,
          value: value.slice(0, 1024 * 1024).toString('utf-8'),
        });
        return;
      }

      res.json({
        size: value.length,
        value: value.toString('utf-8'),
      });
    } catch (error) {
      res.status(500).json({error: error.message});
    }
  });

  app.listen(3000, () => {
    // eslint-disable-next-line no-console
    console.log('Server is running on http://localhost:3000');
  });
}

main();
