// @ts-expect-error TS2305
import commander, {commander$Command, commander$Option} from 'commander';

export interface OptionsDefinition {
  [key: string]: string | unknown[] | commander.Option;
}

export function applyOptions(
  cmd: commander.Command,
  options: OptionsDefinition,
) {
  for (let opt in options) {
    const option = options[opt];
    if (option instanceof commander.Option) {
      cmd.addOption(option);
    } else if (Array.isArray(option)) {
      // @ts-expect-error TS2345
      cmd.option(opt, ...option);
    } else if (typeof option === 'string') {
      cmd.option(opt, option);
    }
  }
}
