import {createBuildCache} from './buildCache.mts';
import {serializeRaw, deserializeRaw} from './serializerCore.mts';

export {serializeRaw, deserializeRaw} from './serializerCore.mts';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Class = new (...args: any[]) => any;

const nameToCtor: Map<string, Class> = new Map();
const ctorToName: Map<Class, string> = new Map();

export function registerSerializableClass(name: string, ctor: Class) {
  if (ctorToName.has(ctor)) {
    throw new Error('Class already registered with serializer');
  }

  nameToCtor.set(name, ctor);
  ctorToName.set(ctor, name);
}

export function unregisterSerializableClass(name: string, ctor: Class) {
  if (nameToCtor.get(name) === ctor) {
    nameToCtor.delete(name);
  }

  if (ctorToName.get(ctor) === name) {
    ctorToName.delete(ctor);
  }
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function shallowCopy(object: any) {
  if (object && typeof object === 'object') {
    if (Array.isArray(object)) {
      return [...object];
    }

    if (object instanceof Map) {
      return new Map(object);
    }

    if (object instanceof Set) {
      return new Set(object);
    }

    return Object.create(
      Object.getPrototypeOf(object),
      Object.getOwnPropertyDescriptors(object),
    );
  }

  return object;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function isBuffer(object: any) {
  return (
    object.buffer instanceof ArrayBuffer ||
    (typeof SharedArrayBuffer !== 'undefined' &&
      object.buffer instanceof SharedArrayBuffer)
  );
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function shouldContinueMapping(value: any) {
  return value && typeof value === 'object' && value.$$raw !== true;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function mapObject(object: any, fn: (val: any) => any, preOrder = false): any {
  const cache = new Map();
  const memo = new Map();

  // Memoize the passed function to ensure it always returns the exact same
  // output by reference for the same input. This is important to maintain
  // reference integrity when deserializing rather than cloning.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const memoizedFn = (val: any) => {
    let res = memo.get(val);
    if (res == null) {
      res = fn(val);
      memo.set(val, res);
    }

    return res;
  };

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const walk = (object: any, shouldCopy = false) => {
    // Check the cache first, both for performance and cycle detection.
    if (cache.has(object)) {
      return cache.get(object);
    }

    let result = object;
    cache.set(object, result);

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const processKey = (key: any, value: any) => {
      let newValue = value;
      if (preOrder && value && typeof value === 'object') {
        newValue = memoizedFn(value);
      }

      // Recursively walk the children
      if (
        preOrder
          ? shouldContinueMapping(newValue)
          : newValue &&
            typeof newValue === 'object' &&
            shouldContinueMapping(object)
      ) {
        newValue = walk(newValue, newValue === value);
      }

      if (!preOrder && newValue && typeof newValue === 'object') {
        newValue = memoizedFn(newValue);
      }

      if (newValue !== value) {
        // Copy on write. We only need to do this when serializing, not deserializing.
        if (object === result && preOrder && shouldCopy) {
          result = shallowCopy(object);
          cache.set(object, result);
        }

        // Replace the key with the new value
        if (result instanceof Map) {
          result.set(key, newValue);
        } else if (result instanceof Set) {
          const _result = result; // For Flow
          // TODO: do we care about iteration order??
          _result.delete(value);
          _result.add(newValue);
        } else {
          result[key] = newValue;
        }
      }
    };

    // Iterate in various ways depending on type.
    if (Array.isArray(object)) {
      for (let i = 0; i < object.length; i++) {
        processKey(i, object[i]);
      }
    } else if (object instanceof Map || object instanceof Set) {
      for (const [key, val] of object.entries()) {
        processKey(key, val);
      }
    } else if (!isBuffer(object)) {
      for (const key in object) {
        processKey(key, object[key]);
      }
    }

    return result;
  };

  const mapped = memoizedFn(object);
  if (
    preOrder
      ? shouldContinueMapping(mapped)
      : mapped && typeof mapped === 'object' && shouldContinueMapping(object)
  ) {
    return walk(mapped, mapped === object);
  }

  return mapped;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function prepareForSerialization(object: any): any {
  if (object?.$$raw) {
    return object;
  }

  return mapObject(
    object,
    (value) => {
      // Add a $$type property with the name of this class, if any is registered.
      if (
        value &&
        typeof value === 'object' &&
        typeof value.constructor === 'function'
      ) {
        const type = ctorToName.get(value.constructor);
        if (type != null) {
          let serialized = value;
          let raw = false;
          if (value && typeof value.serialize === 'function') {
            // If the object has a serialize method, call it
            serialized = value.serialize();
            raw = (serialized && serialized.$$raw) ?? true;
            if (serialized) {
              delete serialized.$$raw;
            }
          }

          return {
            $$type: type,
            $$raw: raw,
            value: {...serialized},
          };
        }
      }

      return value;
    },
    true,
  );
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function restoreDeserializedObject(object: any): any {
  return mapObject(object, (value) => {
    // If the value has a $$type property, use it to restore the object type
    if (value && value.$$type) {
      const ctor = nameToCtor.get(value.$$type);
      if (ctor == null) {
        throw new Error(
          `Expected constructor ${value.$$type} to be registered with serializer to deserialize`,
        );
      }

      // @ts-expect-error weird type signature
      if (typeof ctor.deserialize === 'function') {
        // @ts-expect-error weird type signature
        return ctor.deserialize(value.value);
      }

      value = value.value;
      Object.setPrototypeOf(value, ctor.prototype);
    }

    return value;
  });
}

const serializeCache = createBuildCache();

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function serialize(object: any): Buffer {
  const cached = serializeCache.get(object);
  if (cached) {
    // @ts-expect-error unknown return type
    return cached;
  }

  const mapped = prepareForSerialization(object);
  return serializeRaw(mapped);
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function deserialize(buffer: Buffer): any {
  const obj = deserializeRaw(buffer);
  return restoreDeserializedObject(obj);
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function cacheSerializedObject(object: any, buffer?: Buffer): void {
  serializeCache.set(object, buffer || serialize(object));
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function deserializeToCache(buffer: Buffer): any {
  const deserialized = deserialize(buffer);
  serializeCache.set(deserialized, buffer);
  return deserialized;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function removeSerializedObjectFromCache(object: any) {
  serializeCache.delete(object);
}
