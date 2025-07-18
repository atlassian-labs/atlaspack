import {diff as jestDiff} from 'jest-diff';

export function diff(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  chalk: any,
  original: string,
  modified: string,
): ReturnType<typeof jestDiff> {
  return jestDiff(original, modified, {
    aAnnotation: 'Original',
    aColor: chalk.red,
    bAnnotation: 'Modified',
    bColor: chalk.green,
    expand: false,
    omitAnnotationLines: true,
  });
}
