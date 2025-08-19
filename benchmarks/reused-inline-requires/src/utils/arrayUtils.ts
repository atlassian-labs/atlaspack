import * as _ from 'lodash';

export const chunk = <T>(array: T[], size: number): T[][] => {
  return _.chunk(array, size);
};

export const compact = <T>(array: (T | null | undefined | false | 0 | '')[]): T[] => {
  return _.compact(array);
};

export const flatten = <T>(array: any[]): T[] => {
  return _.flatten(array);
};

export const uniq = <T>(array: T[]): T[] => {
  return _.uniq(array);
};

export const sortBy = <T>(array: T[], iteratee: keyof T | ((item: T) => any)): T[] => {
  return _.sortBy(array, iteratee);
};

export const groupBy = <T>(array: T[], iteratee: keyof T | ((item: T) => any)): Record<string, T[]> => {
  return _.groupBy(array, iteratee);
};

export const partition = <T>(array: T[], predicate: (item: T) => boolean): [T[], T[]] => {
  return _.partition(array, predicate);
};

export const intersection = <T>(...arrays: T[][]): T[] => {
  return _.intersection(...arrays);
};

export const difference = <T>(array: T[], ...others: T[][]): T[] => {
  return _.difference(array, ...others);
};

export const shuffle = <T>(array: T[]): T[] => {
  return _.shuffle(array);
};

export default {
  chunk,
  compact,
  flatten,
  uniq,
  sortBy,
  groupBy,
  partition,
  intersection,
  difference,
  shuffle
};
