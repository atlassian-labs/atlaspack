// @flow strict-local

import commander, {
  type commander$Command,
  type commander$Option,
} from 'commander';

export interface OptionsDefinition {
  [key: string]: string | mixed[] | commander$Option;
}

export function applyOptions(
  cmd: commander$Command,
  options: OptionsDefinition,
) {
  for (let opt in options) {
    const option = options[opt];
    if (option instanceof commander.Option) {
      cmd.addOption(option);
    } else if (Array.isArray(option)) {
      cmd.option(opt, ...option);
    } else if (typeof option === 'string') {
      cmd.option(opt, option);
    }
  }
}
