import type {PluginTracer as IPluginTracer} from '@atlaspack/types';

export class PluginTracer implements IPluginTracer {
  // @ts-expect-error TS2564
  enabled: false;

  // @ts-expect-error TS2416
  createMeasurement() {
    throw new Error('PluginTracer.createMeasurement');
  }
}
