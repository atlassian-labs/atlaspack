import * as _ from 'lodash';

export const capitalize = (str: string): string => {
  return _.capitalize(str);
};

export const kebabCase = (str: string): string => {
  return _.kebabCase(str);
};

export const camelCase = (str: string): string => {
  return _.camelCase(str);
};

export const truncate = (str: string, length: number = 50): string => {
  return _.truncate(str, { length });
};

export const repeat = (str: string, count: number): string => {
  return _.repeat(str, count);
};

export const padStart = (str: string, length: number, chars?: string): string => {
  return _.padStart(str, length, chars);
};

export const padEnd = (str: string, length: number, chars?: string): string => {
  return _.padEnd(str, length, chars);
};

export const slugify = (str: string): string => {
  return str
    .toLowerCase()
    .trim()
    .replace(/[^\w\s-]/g, '')
    .replace(/[\s_-]+/g, '-')
    .replace(/^-+|-+$/g, '');
};

export default {
  capitalize,
  kebabCase,
  camelCase,
  truncate,
  repeat,
  padStart,
  padEnd,
  slugify
};
