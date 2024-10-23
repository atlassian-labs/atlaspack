type CondType = 'exp' | 'fg';

interface ImportTypeObject {
  [key: string]: ESModuleWithDefaultExport;
  default: ESModuleWithDefaultExport;
}

type ImportObject<T extends ImportTypeObject> = {[key in keyof T]: string};
type NoImportObjectTypeErrorMessage =
  "You must annotate type with \"<{ someVariant: typeof import ('./some-import'); default: typeof import('./x') }>\"";

declare function importCond<T>(
  type: CondType,
  condition: string,
  imports: T extends ImportTypeObject ? ImportObject<T> : NoImportErrorMessage,
): void;

const Component = importCond<{
  a: typeof import('./ComponentA');
  b: typeof import('./ComponentB');
  default: typeof import('./ComponentDefault');
}>('exp', 'my-cond', {
  a: 'string',
  b: 'string',
  default: './ComponentDefault',
});
