// @flow strict-local

export function unique<T>(array: Array<T>): Array<T> {
  return [...new Set(array)];
}

export function objectSortedEntries(obj: {
  +[string]: mixed,
  ...
}): Array<[string, mixed]> {
  return Object.entries(obj).sort(([keyA], [keyB]) => keyA.localeCompare(keyB));
}

export function objectSortedEntriesDeep(object: {
  +[string]: mixed,
  ...
}): Array<[string, mixed]> {
  let sortedEntries = objectSortedEntries(object);
  for (let i = 0; i < sortedEntries.length; i++) {
    sortedEntries[i][1] = sortEntry(sortedEntries[i][1]);
  }
  return sortedEntries;
}

function sortEntry(entry: mixed) {
  if (Array.isArray(entry)) {
    return entry.map(sortEntry);
  }

  if (typeof entry === 'object' && entry != null) {
    return objectSortedEntriesDeep(entry);
  }

  return entry;
}

/**
 * Get the difference of A and B sets
 *
 * This is the set of elements which are in A but not in B
 * For example, the difference of the sets {1,2,3} and {3,4} is {1,2}
 *
 * @param {*} a Set A
 * @param {*} b Set B
 * @returns A \ B
 */
export function setDifference<T>(
  a: $ReadOnlySet<T>,
  b: $ReadOnlySet<T>,
): Set<T> {
  let difference = new Set();
  for (let e of a) {
    if (!b.has(e)) {
      difference.add(e);
    }
  }

  return difference;
}

/**
 * Get the symmetric difference of A and B sets
 *
 * This is the set of elements which are in either of the sets, but not in their intersection.
 * For example, the symmetric difference of the sets {1,2,3} and {3,4} is {1,2,4}
 *
 * @param {*} a Set A
 * @param {*} b Set B
 * @returns A Δ B
 */
export function setSymmetricDifference<T>(
  a: $ReadOnlySet<T>,
  b: $ReadOnlySet<T>,
): Set<T> {
  let difference = new Set();
  for (let e of a) {
    if (!b.has(e)) {
      difference.add(e);
    }
  }
  for (let d of b) {
    if (!a.has(d)) {
      difference.add(d);
    }
  }
  return difference;
}

export function setIntersect<T>(a: Set<T>, b: $ReadOnlySet<T>): void {
  for (let entry of a) {
    if (!b.has(entry)) {
      a.delete(entry);
    }
  }
}

export function setUnion<T>(a: Iterable<T>, b: Iterable<T>): Set<T> {
  return new Set([...a, ...b]);
}

export function setEqual<T>(a: $ReadOnlySet<T>, b: $ReadOnlySet<T>): boolean {
  if (a.size != b.size) {
    return false;
  }
  for (let entry of a) {
    if (!b.has(entry)) {
      return false;
    }
  }
  return true;
}
