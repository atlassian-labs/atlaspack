import type {Server} from 'node:http';
import * as fs from 'node:fs';
import * as path from 'node:path';
import {createServer} from 'node:http';

const contentTypes: Record<string, string> = {
  '.json': 'application/json',
  '.html': 'text/html',
  '.css': 'text/css',
  '.js': 'application/javascript',
};

export type ServeContext = {
  address: string;
  close(): void;
  port?: number;
  server?: Server;
};

export async function serve(directory: string): Promise<ServeContext> {
  // Normalize and resolve the root directory only once
  const rootDir = path.resolve(directory);
  const server = createServer((req, res) => {
    const relPath = !req.url || req.url === '/' ? 'index.html' : req.url;
    // Use path.resolve to build an absolute path, preventing path traversal
    const target = path.resolve(rootDir, path.normalize(relPath));

    // Prevent directory traversal: ensure target is within rootDir
    if (!target.startsWith(rootDir + path.sep) && target !== rootDir) {
      res.writeHead(403, {'Content-Type': 'text/plain'});
      res.end('Forbidden');
      return;
    }

    // Check if file exists before trying to serve it
    if (!fs.existsSync(target)) {
      // Handle common browser requests gracefully
      if (relPath === '/favicon.ico') {
        res.writeHead(204, {'Content-Type': 'text/plain'});
        res.end();
        return;
      }

      res.writeHead(404, {'Content-Type': 'text/plain'});
      res.end('File not found');
      return;
    }

    const extname = path.extname(target);
    const contentType = contentTypes[extname] || 'application/octet-stream';
    res.setHeader('Content-Type', contentType);

    const stream = fs.createReadStream(target);
    stream.on('error', (err) => {
      res.writeHead(500, {'Content-Type': 'text/plain'});
      res.end('Internal server error');
    });
    stream.pipe(res);
  });

  await new Promise<void>((res) => server.listen(0, '127.0.0.1', res));

  const address = server.address();
  if (!address || typeof address === 'string') {
    throw new Error('Invalid server type');
  }

  return {
    address: `http://localhost:${address.port}`,
    port: address.port,
    close() {
      server.close();
    },
    server,
  };
}
