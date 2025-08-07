import type {
  ServerOptions,
  PluginLogger,
  HMROptions,
  HTTPSOptions,
  FilePath,
  PackageManager,
} from '@atlaspack/types';
import type {FileSystem} from '@atlaspack/fs';
import type {HTTPServer} from '@atlaspack/utils';
import {IncomingMessage, ServerResponse} from 'http';

interface HTTPRequest extends IncomingMessage {
  originalUrl?: string;
}

export type Request = HTTPRequest;
export type Response = ServerResponse;

export type DevServerOptions = ServerOptions & {
  projectRoot: string;
  publicUrl: string;
  cacheDir: string;
  inputFS: FileSystem;
  outputFS: FileSystem;
  logger: PluginLogger;
  hmrOptions?: HMROptions | null | undefined;
  packageManager: PackageManager;
};

// TODO: Figure out if there is a node.js type that could be imported with a complete ServerError
export type ServerError = Error & {
  code: string;
};

export type HMRServerOptions = {
  devServer?: HTTPServer;
  addMiddleware?: (handler: (req: Request, res: Response) => boolean) => void;
  port: number;
  host?: string | null | undefined;
  logger: PluginLogger;
  https?: HTTPSOptions | boolean;
  cacheDir: FilePath;
  inputFS: FileSystem;
  outputFS: FileSystem;
};
