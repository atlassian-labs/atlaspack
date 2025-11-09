import nodeAssert, {AssertionError} from 'assert';

export function assert(
  value: unknown,
  message?: string | (() => string),
): asserts value {
  if (value) return;

  throw new AssertionError({
    message: typeof message === 'function' ? message() : message,
    expected: true,
    actual: value,
  });
}

assert.equal = function <T>(
  actual: T,
  expected: T,
  message?: string | (() => string),
) {
  if (actual == expected) return;
  throw new AssertionError({
    message: typeof message === 'function' ? message() : message,
    expected,
    actual,
  });
};

assert.fail = function (message: Error | string | (() => string)): never {
  if (message instanceof Error) {
    throw message;
  }

  throw new AssertionError({
    message: typeof message === 'function' ? message() : message,
  });
};

assert.deepEqual = function <T>(
  actual: T,
  expected: T,
  message?: string | (() => string),
) {
  // Just intercept the message rather than re-implementing deepEqual
  try {
    nodeAssert.deepEqual(actual, expected);
  } catch (e) {
    if (e instanceof AssertionError && message != null) {
      e.message = typeof message === 'function' ? message() : message;
    }
    throw e;
  }
};
