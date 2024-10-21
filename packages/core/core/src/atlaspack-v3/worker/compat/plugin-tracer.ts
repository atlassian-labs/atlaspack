import type {PluginTracer as IPluginTracer} from '@atlaspack/types';

export class PluginTracer implements IPluginTracer {
  enabled: false;

  createMeasurement() {
    throw new Error('PluginTracer.createMeasurement');
  }
}
