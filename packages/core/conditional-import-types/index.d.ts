// eslint-disable-next-line @typescript-eslint/no-unused-vars
type ModuleRef<_PhantomModuleType> = string;

type NoImportErrorMessage =
  "You must annotate type with \"<typeof import('a'), typeof import('b')>\"";
type NoDefaultErrorMessage = 'Conditional imports must have a default export';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ESModuleWithDefaultExport = {[key: string]: any; default: any};

type ConditionalImport<
  CondT extends ESModuleWithDefaultExport,
  CondF extends ESModuleWithDefaultExport,
> = CondT['default'] | CondF['default'];

/**
 * Conditionally import a dependency, based on the specified condition.
 *
 * This is a synchronous import that differs from conditionally loading a dynamic import (`import()`)
 *
 * This function requires server to guarantee the dependency is loaded.
 *
 * @param condition Condition evaluated by the server
 * @param ifTrueDependency Dependency returned if the condition is true. This should be a relative file path like './ui/comment-component-new.tsx'
 * @param ifFalseDependency Dependency returned if the condition is false. This should be a relative file path like './ui/comment-component-old.tsx'
 */
declare function importCond<CondT, CondF>(
  condition: string,
  ifTrueDependency: CondT extends void
    ? NoImportErrorMessage
    : CondT extends ESModuleWithDefaultExport
      ? ModuleRef<CondT>
      : NoDefaultErrorMessage,
  ifFalseDependency: CondF extends void
    ? NoImportErrorMessage
    : CondF extends ESModuleWithDefaultExport
      ? ModuleRef<CondF>
      : NoDefaultErrorMessage,
): CondT extends ESModuleWithDefaultExport
  ? CondF extends ESModuleWithDefaultExport
    ? ConditionalImport<CondT, CondF>
    : never
  : never;

export {};
