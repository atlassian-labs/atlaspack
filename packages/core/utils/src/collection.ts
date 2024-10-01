export function unique<T>(array: Array<T>): Array<T> {
  return [...new Set(array)];
}

export function objectSortedEntries(obj: {
  readonly [key: string]: unknown;
}): Array<[string, unknown]> {
  // @ts-expect-error - TS2345 - Argument of type '([keyA]: [any], [keyB]: [any]) => any' is not assignable to parameter of type '(a: [string, unknown], b: [string, unknown]) => number'.
  return Object.entries(obj).sort(([keyA]: [any], [keyB]: [any]) =>
    keyA.localeCompare(keyB),
  );
}

export function objectSortedEntriesDeep(object: {
  readonly [key: string]: unknown;
}): Array<[string, unknown]> {
  let sortedEntries = objectSortedEntries(object);
  for (let i = 0; i < sortedEntries.length; i++) {
    sortedEntries[i][1] = sortEntry(sortedEntries[i][1]);
  }
  return sortedEntries;
}

// @ts-expect-error - TS7023 - 'sortEntry' implicitly has return type 'any' because it does not have a return type annotation and is referenced directly or indirectly in one of its return expressions.
function sortEntry(entry: unknown) {
  if (Array.isArray(entry)) {
    return entry.map(sortEntry);
  }

  if (typeof entry === 'object' && entry != null) {
    // @ts-expect-error - TS2345 - Argument of type 'object' is not assignable to parameter of type '{ readonly [key: string]: unknown; }'.
    return objectSortedEntriesDeep(entry);
  }

  return entry;
}

export function setDifference<T>(
  // @ts-expect-error - TS2552 - Cannot find name '$ReadOnlySet'. Did you mean 'ReadonlySet'?
  a: $ReadOnlySet<T>,
  // @ts-expect-error - TS2552 - Cannot find name '$ReadOnlySet'. Did you mean 'ReadonlySet'?
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
  // @ts-expect-error - TS2322 - Type 'Set<unknown>' is not assignable to type 'Set<T>'.
  return difference;
}

// @ts-expect-error - TS2552 - Cannot find name '$ReadOnlySet'. Did you mean 'ReadonlySet'?
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

// @ts-expect-error - TS2552 - Cannot find name '$ReadOnlySet'. Did you mean 'ReadonlySet'? | TS2552 - Cannot find name '$ReadOnlySet'. Did you mean 'ReadonlySet'?
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
