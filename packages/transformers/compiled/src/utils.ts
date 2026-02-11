import fs from 'fs';
import {dirname} from 'path';

import type {Resolver} from '@compiled/babel-plugin';
import {CachedInputFileSystem, ResolverFactory} from 'enhanced-resolve';

import type {CompiledTransformerOpts} from './types';

export function createDefaultResolver(
  config: CompiledTransformerOpts,
): Resolver {
  const resolver = ResolverFactory.createResolver({
    fileSystem: new CachedInputFileSystem(fs, 4000),
    ...(config.extensions && {
      extensions: config.extensions,
    }),
    ...(config.resolve ?? {}),
    // This makes the resolver invoke the callback synchronously
    useSyncFileSystemCalls: true,
  });

  return {
    // The resolver needs to be synchronous, as babel plugins must be synchronous
    resolveSync(context: string, request: string) {
      return resolver.resolveSync({}, dirname(context), request) as string;
    },
  };
}

export const DEFAULT_IMPORT_SOURCES = ['@compiled/react', '@atlaskit/css'];

type Falsy = false | null | undefined | '' | 0;

export function toBoolean<T>(value: T): value is Exclude<T, Falsy> {
  return Boolean(value);
}
