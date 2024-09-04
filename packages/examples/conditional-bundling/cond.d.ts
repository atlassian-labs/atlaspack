type ModuleRef<_> = string;
type ErrorMessage =
  "You must annotate type with \"<typeof import('a'), typeof import('b')>\"";

type ConditionalImport<CondT, CondF> = CondT | CondF;

/**
 * Conditionally import a dependency, based on the specified condition.
 *
 * This is a synchronous import that differs from conditionally loading a dynamic import.
 *
 * This function requires server to guarantee the dependency
 *
 * @param condition
 * @param ifTrueDependency
 * @param ifFalseDependency
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
