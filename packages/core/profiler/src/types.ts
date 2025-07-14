export type {TraceMeasurement} from '@atlaspack/types-internal';

export type TraceMeasurementData = {
  readonly categories: string[];
  readonly args?: Record<string, unknown>;
};
