// @flow

import type {PluginTracer as IPluginTracer} from '../../../../types/index.js';

export class PluginTracer implements IPluginTracer {
  enabled: false;

  createMeasurement() {
    throw new Error('PluginTracer.createMeasurement');
  }
}
