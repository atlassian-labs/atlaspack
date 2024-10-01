import {Resolver} from '@atlaspack/rust';

export const ResolverBase: typeof Resolver = Resolver;

export {default} from './Wrapper';
// @ts-expect-error - TS2305 - Module '"@atlaspack/rust"' has no exported member 'init'.
export {init} from '@atlaspack/rust';
