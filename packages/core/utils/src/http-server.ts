import type {
  Server as HTTPOnlyServer,
  IncomingMessage as HTTPRequest,
  ServerResponse as HTTPResponse,
} from 'http';
import type {
  Server as HTTPSServer,
  // @ts-expect-error - TS2305 - Module '"https"' has no exported member 'IncomingMessage'.
  IncomingMessage as HTTPSRequest,
  // @ts-expect-error - TS2305 - Module '"https"' has no exported member 'ServerResponse'.
  ServerResponse as HTTPSResponse,
} from 'https';
import type {Socket} from 'net';
import type {FilePath, HTTPSOptions} from '@atlaspack/types';
import type {FileSystem} from '@atlaspack/fs';

import http from 'http';
import https from 'https';
import nullthrows from 'nullthrows';
import {getCertificate, generateCertificate} from './';

type CreateHTTPServerOpts = {
  listener?: (
    arg1: HTTPRequest | HTTPSRequest,
    arg2: HTTPResponse | HTTPSResponse,
  ) => void;
  host?: string;
} & (
  | {
      https: HTTPSOptions | boolean | null | undefined;
      inputFS: FileSystem;
      outputFS: FileSystem;
      cacheDir: FilePath;
    }
  | Record<any, any>
);

export type HTTPServer = HTTPOnlyServer | HTTPSServer;

// Creates either an http or https server with an awaitable dispose
// that closes any connections
export async function createHTTPServer(options: CreateHTTPServerOpts): Promise<{
  stop: () => Promise<void>;
  server: HTTPServer;
}> {
  // @ts-expect-error - TS7034 - Variable 'server' implicitly has type 'any' in some locations where its type cannot be determined.
  let server;
  if (!options.https) {
    server = http.createServer(options.listener);
  } else if (options.https === true) {
    let {cert, key} = await generateCertificate(
      options.outputFS,
      options.cacheDir,
      options.host,
    );

    server = https.createServer({cert, key}, options.listener);
  } else {
    let {cert, key} = await getCertificate(options.inputFS, options.https);

    server = https.createServer({cert, key}, options.listener);
  }

  // HTTPServer#close only stops accepting new connections, and does not close existing ones.
  // Before closing, destroy any active connections through their sockets. Additionally, remove sockets when they close:
  // https://stackoverflow.com/questions/18874689/force-close-all-connections-in-a-node-js-http-server
  // https://stackoverflow.com/questions/14626636/how-do-i-shutdown-a-node-js-https-server-immediately/14636625#14636625
  let sockets: Set<Socket> = new Set();
  server.on('connection', (socket: Socket) => {
    nullthrows(sockets).add(socket);
    socket.on('close', () => {
      nullthrows(sockets).delete(socket);
    });
  });
  return {
    server,
    stop() {
      return new Promise(
        (
          resolve: (result: Promise<undefined> | undefined) => void,
          reject: (error?: any) => void,
        ) => {
          for (let socket of nullthrows(sockets)) {
            socket.destroy();
          }
          sockets = new Set();

          // @ts-expect-error - TS7005 - Variable 'server' implicitly has an 'any' type. | TS7006 - Parameter 'err' implicitly has an 'any' type.
          server.close((err) => {
            if (err != null) {
              reject(err);
              return;
            }

            // @ts-expect-error - TS2794 - Expected 1 arguments, but got 0. Did you forget to include 'void' in your type argument to 'Promise'?
            resolve();
          });
        },
      );
    },
  };
}
