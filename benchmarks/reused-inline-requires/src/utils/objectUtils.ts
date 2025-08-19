import * as _ from 'lodash';

export const pick = <T, K extends keyof T>(object: T, keys: K[]): Pick<T, K> => {
  return _.pick(object, keys);
};

export const omit = <T, K extends keyof T>(object: T, keys: K[]): Omit<T, K> => {
  return _.omit(object, keys);
};

export const merge = <T>(...objects: Partial<T>[]): T => {
  return _.merge({}, ...objects);
};

export const cloneDeep = <T>(object: T): T => {
  return _.cloneDeep(object);
};

export const isEqual = (object1: any, object2: any): boolean => {
  return _.isEqual(object1, object2);
};

export const has = (object: any, path: string): boolean => {
  return _.has(object, path);
};

export const get = <T>(object: any, path: string, defaultValue?: T): T => {
  return _.get(object, path, defaultValue);
};

export const set = (object: any, path: string, value: any): any => {
  return _.set(object, path, value);
};

export const keys = (object: any): string[] => {
  return _.keys(object);
};

export const values = <T>(object: Record<string, T>): T[] => {
  return _.values(object);
};

export const entries = <T>(object: Record<string, T>): [string, T][] => {
  return _.toPairs(object);
};

export const mapValues = <T, U>(object: Record<string, T>, iteratee: (value: T, key: string) => U): Record<string, U> => {
  return _.mapValues(object, iteratee);
};

export default {
  pick,
  omit,
  merge,
  cloneDeep,
  isEqual,
  has,
  get,
  set,
  keys,
  values,
  entries,
  mapValues
};
