import {Resolver} from '@atlaspack/rust';

export const ResolverBase: typeof Resolver = Resolver;

export {default} from './Wrapper';
// @ts-expect-error TS2305
export {init} from '@atlaspack/rust';
