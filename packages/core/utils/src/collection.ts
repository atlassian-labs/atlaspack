export function unique<T>(array: Array<T>): Array<T> {
  return [...new Set(array)];
}

export function objectSortedEntries(
  obj: {
    readonly [key: string]: unknown;
  },
): Array<[string, unknown]> {
  return Object.entries(obj).sort(([keyA]: [any], [keyB]: [any]) => keyA.localeCompare(keyB));
}

export function objectSortedEntriesDeep(
  object: {
    readonly [key: string]: unknown;
  },
): Array<[string, unknown]> {
  let sortedEntries = objectSortedEntries(object);
  for (let i = 0; i < sortedEntries.length; i++) {
    sortedEntries[i][1] = sortEntry(sortedEntries[i][1]);
  }
  return sortedEntries;
}

function sortEntry(entry: unknown) {
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
export function setDifference<T>(a: ReadonlySet<T>, b: ReadonlySet<T>): Set<T> {
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
export function setSymmetricDifference<T>(a: ReadonlySet<T>, b: ReadonlySet<T>): Set<T> {
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

export function setIntersect<T>(a: Set<T>, b: ReadonlySet<T>): void {
  for (let entry of a) {
    if (!b.has(entry)) {
      a.delete(entry);
    }
  }
}

export function setIntersectStatic<T>(a: Set<T>, b: Set<T>): Set<T> {
  let intersection = new Set();
  for (let entry of a) {
    if (b.has(entry)) {
      intersection.add(entry);
    }
  }
  return intersection;
}

export function setUnion<T>(a: Iterable<T>, b: Iterable<T>): Set<T> {
  return new Set([...a, ...b]);
}

export function setEqual<T>(a: ReadonlySet<T>, b: ReadonlySet<T>): boolean {
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
