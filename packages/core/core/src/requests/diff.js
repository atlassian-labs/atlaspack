const util = require('util');
function diff(obj1, obj2) {
  const colors = {
    ADDED: '\x1b[32m', // Green
    DELETED: '\x1b[31m', // Red
    RESET: '\x1b[0m', // Reset
  };
  function render(obj, prefix = '') {
    if (obj instanceof Map) {
      const objRepresentation = Object.fromEntries(obj);
      return render(objRepresentation, prefix);
    } else if (obj instanceof Set) {
      const arrayRepresentation = [...obj];
      return render(arrayRepresentation, prefix);
    } else if (typeof obj !== 'object' || obj === null) {
      return util.inspect(obj);
    }
    const isArray = Array.isArray(obj);
    const keys = Object.keys(obj);
    const entries = keys.map(key => {
      const value = render(obj[key], prefix + '  ');
      return `${prefix}  ${key}: ${value}`;
    });
    const opening = isArray ? '[' : '{';
    const closing = isArray ? ']' : '}';
    return `${opening}\n${entries.join(',\n')}\n${prefix}${closing}`;
  }
  function diffInternal(obj1 = {}, obj2 = {}, prefix = '') {
    if (obj1 instanceof Map && obj2 instanceof Map) {
      return diffMap(obj1, obj2, prefix);
    }
    if (obj1 instanceof Set && obj2 instanceof Set) {
      return diffSet(obj1, obj2, prefix);
    }
    const keys = new Set([
      ...Object.keys(obj1 || {}),
      ...Object.keys(obj2 || {}),
    ]);
    const diffs = [];
    keys.forEach(key => {
      const value1 = obj1[key];
      const value2 = obj2[key];
      if (value1 === undefined) {
        // Added
        diffs.push(
          `${prefix}  ${colors.ADDED}+ ${key}: ${render(
            value2,
            prefix + '  ',
          )}${colors.RESET}`,
        );
      } else if (value2 === undefined) {
        // Deleted
        diffs.push(
          `${prefix}  ${colors.DELETED}- ${key}: ${render(
            value1,
            prefix + '  ',
          )}${colors.RESET}`,
        );
      } else if (typeof value1 === 'object' && typeof value2 === 'object') {
        // Recurse for nested objects
        const nestedDiff = diffInternal(value1, value2, prefix + '  ');
        if (nestedDiff) {
          diffs.push(`${prefix}  ${key}: ${nestedDiff}`);
        }
      } else if (value1 !== value2) {
        // Changed
        diffs.push(
          `${prefix}  ${colors.DELETED}- ${key}: ${render(
            value1,
            prefix + '  ',
          )}${colors.RESET}`,
        );
        diffs.push(
          `${prefix}  ${colors.ADDED}+ ${key}: ${render(
            value2,
            prefix + '  ',
          )}${colors.RESET}`,
        );
      } else {
        // Unchanged
        diffs.push(`${prefix}  ${key}: ${render(value1, prefix + '  ')}`);
      }
    });
    if (diffs.length > 0) {
      const opening = Array.isArray(obj1) ? '[' : '{';
      const closing = Array.isArray(obj1) ? ']' : '}';
      return `${opening}\n${diffs.join(',\n')}\n${prefix}${closing}`;
    }
    return '';
  }
  function diffMap(map1, map2, prefix = '') {
    const diffs = [];
    const allKeys = new Set([...map1.keys(), ...map2.keys()]);
    allKeys.forEach(key => {
      const hasKey1 = map1.has(key);
      const hasKey2 = map2.has(key);
      if (!hasKey1) {
        // Added
        diffs.push(
          `${prefix}  ${colors.ADDED}+ ${key}: ${render(
            map2.get(key),
            prefix + '  ',
          )}${colors.RESET}`,
        );
      } else if (!hasKey2) {
        // Deleted
        diffs.push(
          `${prefix}  ${colors.DELETED}- ${key}: ${render(
            map1.get(key),
            prefix + '  ',
          )}${colors.RESET}`,
        );
      } else {
        const value1 = map1.get(key);
        const value2 = map2.get(key);
        if (typeof value1 === 'object' && typeof value2 === 'object') {
          const nestedDiff = diffInternal(value1, value2, prefix + ' ');
          if (nestedDiff) {
            diffs.push(`${prefix}  ${key}: ${nestedDiff}`);
          }
        } else if (value1 !== value2) {
          // Changed
          diffs.push(
            `${prefix}  ${colors.DELETED}- ${key}: ${render(
              value1,
              prefix + '  ',
            )}${colors.RESET}`,
          );
          diffs.push(
            `${prefix}  ${colors.ADDED}+ ${key}: ${render(
              value2,
              prefix + '  ',
            )}${colors.RESET}`,
          );
        } else {
          // Unchanged
          diffs.push(`${prefix}  ${key}: ${render(value1, prefix + '  ')}`);
        }
      }
    });
    if (diffs.length > 0) {
      return `{\n${diffs.join(',\n')}\n${prefix}}`;
    }
    return '';
  }
  function diffSet(set1, set2, prefix = '') {
    const diffs = [];
    const onlyInSet1 = [...set1].filter(x => !set2.has(x));
    const onlyInSet2 = [...set2].filter(x => !set1.has(x));
    onlyInSet1.forEach(item => {
      // Deleted
      diffs.push(
        `${prefix}  ${colors.DELETED}- ${render(item, prefix + '  ')}${
          colors.RESET
        }`,
      );
    });
    onlyInSet2.forEach(item => {
      // Added
      diffs.push(
        `${prefix}  ${colors.ADDED}+ ${render(item, prefix + '  ')}${
          colors.RESET
        }`,
      );
    });
    if (diffs.length > 0) {
      return `[\n${diffs.join(',\n')}\n${prefix}]`;
    }
    return '';
  }
  const result = diffInternal(obj1, obj2);
  // eslint-disable-next-line
  console.log(result);
}

module.exports = diff;
