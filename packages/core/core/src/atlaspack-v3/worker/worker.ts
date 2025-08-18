import * as napi from '@atlaspack/rust';
import {parentPort} from 'worker_threads';

import {AtlaspackWorker} from './AtlaspackWorker';

export const CONFIG = Symbol.for('parcel-plugin-config');

// Create napi worker and send it back to main thread
const worker = new AtlaspackWorker();
const napiWorker = napi.newNodejsWorker(worker);
parentPort?.postMessage(napiWorker);
