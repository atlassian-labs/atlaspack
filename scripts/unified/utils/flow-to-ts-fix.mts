import {readToString, writeString} from './fs-extra.mts';

export function flowToFlowFix(contents: string): string {
  // Some fixups of flow-to-ts output
  contents = contents.replace(
    'Record<string, JSONValue>',
    '{[key: string]: JSONValue}',
  );
  contents = contents.replace(/\$ReadOnlyMap/g, 'ReadonlyMap');
  contents = contents.replace(/\$ReadOnlySet/g, 'ReadonlySet');
  contents = contents.replace(/\$Partial/g, 'Partial');

  return contents;
}

export async function flowToFlowFixFile(filepath: string): Promise<void> {
  const contents = await readToString(filepath);
  const updated = flowToFlowFix(contents);
  await writeString(filepath, updated);
}
