import {readToString, writeString} from './fs-extra.mts';

const RE = {
  dynamic: /((import|require)\s*\(\s*('|"|`))(.*)(('|"|`)\s*\))/g,
  static: /(import\s*(\w+)*('|"|`))(.*)(('|"|`))/g,
  named: /((import|export)\s*(.*)from\s*('|"|`))(.*)(('|"|`))/g,
};

function subpath(input, to) {
  const arr = input.split('/');
  if (input.startsWith('@')) {
    arr.shift();
  }
  arr[0] = to;
  return arr.join('/');
}

export function renameImports(
  contents: string,
  ...ops: Array<{from: string; to: string}>
) {
  for (const {from, to} of ops) {
    contents = contents.replace(RE.dynamic, (...match) => {
      if (match[4].startsWith(from)) {
        return match[1] + subpath(match[4], to) + match[5];
      } else {
        return match[0];
      }
    });

    contents = contents.replace(RE.static, (...match) => {
      if (match[4].startsWith(from)) {
        return match[1] + subpath(match[4], to) + match[6];
      } else {
        return match[0];
      }
    });

    contents = contents.replace(RE.named, (...match) => {
      if (match[5].startsWith(from)) {
        return match[1] + subpath(match[5], to) + match[7];
      } else {
        return match[0];
      }
    });
  }
  return contents;
}

export async function renameImportsFile(
  target: string,
  ...ops: Array<{from: string; to: string}>
): Promise<void> {
  let contents = await readToString(target);
  contents = renameImports(contents, ...ops);
  await writeString(target, contents);
}
