// @flow strict-local

import {AtlaspackTracer} from '@atlaspack/rust';

export const tracer: AtlaspackTracer = new AtlaspackTracer();

export function instrument<T>(label: string, fn: () => T): T {
  const span = tracer.enter(label);
  try {
    const result = fn();
    return result;
  } finally {
    tracer.exit(span);
  }
}

export async function instrumentAsync<T>(
  label: string,
  fn: () => Promise<T>,
): Promise<T> {
  const span = tracer.enter(label);
  try {
    const result = await fn();
    return result;
  } finally {
    tracer.exit(span);
  }
}
