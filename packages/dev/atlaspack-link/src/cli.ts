/* eslint-disable no-console */

import type {LinkCommandOptions} from './link';
import type {UnlinkCommandOptions} from './unlink';

import {version} from '../package.json';
import {createLinkCommand} from './link';
import {createUnlinkCommand} from './unlink';

import {Command} from 'commander';

export type ProgramOptions = LinkCommandOptions & UnlinkCommandOptions;

export function createProgram(opts?: ProgramOptions): Command {
  let {fs, log = console.log, link, unlink} = opts ?? {};
  return new Command()
    .version(version, '-V, --version')
    .description('A tool for linking a dev copy of Parcel into an app')
    .addHelpText('after', `\nThe link command is the default command.`)
    .addCommand(createLinkCommand({fs, log, link}), {isDefault: true})
    .addCommand(createUnlinkCommand({fs, log, unlink}));
}
