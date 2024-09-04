type ModuleRef<_> = string;
type ErrorMessage =
  "You must annotate type with \"<typeof import('a'), typeof import('b')>\"";

type ConditionalImport<CondT, CondF> = CondT | CondF;

/**
 * **IMPORTANT: This API is currently a no-op. Do not use until this message is removed.**
 *
 * Conditionally import a dependency, based on the specified condition.
 *
 * This is a synchronous import that differs from conditionally loading a dynamic import (`import()`)
 *
 * This function requires server to guarantee the dependency is loaded.
 *
 * @param condition Condition evaluated by the server
 * @param ifTrueDependency Dependency returned if the condition is true
 * @param ifFalseDependency Dependency returned if the condition is false
 */
declare function importCond<CondT, CondF>(
  condition: string,
  ifTrueDependency: CondT extends void ? ErrorMessage : ModuleRef<CondT>,
  ifFalseDependency: CondF extends void ? ErrorMessage : ModuleRef<CondF>,
): CondT extends void
  ? never
  : CondF extends void
  ? never
  : ConditionalImport<CondT, CondF>;
