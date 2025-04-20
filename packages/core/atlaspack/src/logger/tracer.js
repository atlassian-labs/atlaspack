// @flow strict-local

import {AtlaspackTracer} from '../rust/index.js';

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
  let result;
  try {
    result = await fn();
  } finally {
    tracer.exit(span);
  }
  return result;
}
