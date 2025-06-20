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
  port: number;
  close(): void;
  server: Server;
};

export async function serve(directory: string): Promise<ServeContext> {
  const server = createServer((req, res) => {
    let relPath = !req.url || req.url === '/' ? 'index.html' : req.url;
    const target = path.join(directory, path.normalize(relPath));
    const extname = path.extname(target);
    const contentType = contentTypes[extname] || 'application/octet-stream';
    res.setHeader('Content-Type', contentType);
    fs.createReadStream(target).pipe(res);
  });

  await new Promise<void>((res) => server.listen(0, '127.0.0.1', res));

  return {
    address: `http://localhost:${(server.address() as any).port}`,
    port: (server.address() as any).port,
    close() {
      server.close();
    },
    server,
  };
}
