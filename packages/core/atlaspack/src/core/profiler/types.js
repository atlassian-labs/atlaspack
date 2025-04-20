// @flow

export type {TraceMeasurement} from '../types-internal/index.js';

export type TraceMeasurementData = {|
  +categories: string[],
  +args?: {[key: string]: mixed},
|};
