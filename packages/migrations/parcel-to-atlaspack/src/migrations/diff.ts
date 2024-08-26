import {diff as jestDiff} from 'jest-diff';

export function diff(
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
