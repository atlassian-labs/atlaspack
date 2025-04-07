// @flow strict-local
/* eslint-disable no-console */
import {readPackageJsonSync} from '@atlaspack/utils';

import type {LinkCommandOptions} from './link';
import type {UnlinkCommandOptions} from './unlink';

import {createLinkCommand} from './link';
import {createUnlinkCommand} from './unlink';

import commander from 'commander';

const {version} = readPackageJsonSync(__dirname);

export type ProgramOptions = {|...LinkCommandOptions, ...UnlinkCommandOptions|};

// $FlowFixMe[invalid-exported-annotation]
export function createProgram(opts?: ProgramOptions): commander.Command {
  let {fs, log = console.log, link, unlink} = opts ?? {};
  return new commander.Command()
    .version(version, '-V, --version')
    .description('A tool for linking a dev copy of Parcel into an app')
    .addHelpText('after', `\nThe link command is the default command.`)
    .addCommand(createLinkCommand({fs, log, link}), {isDefault: true})
    .addCommand(createUnlinkCommand({fs, log, unlink}));
}
