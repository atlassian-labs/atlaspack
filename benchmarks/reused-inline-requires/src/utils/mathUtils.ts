import * as _ from 'lodash';

export const add = (a: number, b: number): number => {
  return _.add(a, b);
};

export const subtract = (a: number, b: number): number => {
  return _.subtract(a, b);
};

export const multiply = (a: number, b: number): number => {
  return _.multiply(a, b);
};

export const divide = (a: number, b: number): number => {
  return _.divide(a, b);
};

export const ceil = (number: number, precision?: number): number => {
  return _.ceil(number, precision);
};

export const floor = (number: number, precision?: number): number => {
  return _.floor(number, precision);
};

export const round = (number: number, precision?: number): number => {
  return _.round(number, precision);
};

export const random = (lower?: number, upper?: number, floating?: boolean): number => {
  return _.random(lower, upper, floating);
};

export const clamp = (number: number, lower: number, upper: number): number => {
  return _.clamp(number, lower, upper);
};

export const inRange = (number: number, start: number, end?: number): boolean => {
  return _.inRange(number, start, end);
};

export const mean = (array: number[]): number => {
  return _.mean(array);
};

export const sum = (array: number[]): number => {
  return _.sum(array);
};

export const max = (array: number[]): number => {
  return _.max(array) || 0;
};

export const min = (array: number[]): number => {
  return _.min(array) || 0;
};

export default {
  add,
  subtract,
  multiply,
  divide,
  ceil,
  floor,
  round,
  random,
  clamp,
  inRange,
  mean,
  sum,
  max,
  min
};
