import type {TSModuleGraph} from './TSModuleGraph';

import nullthrows from 'nullthrows';
import ts, {EntityName} from 'typescript';
import {TSModule} from './TSModule';
import {getExportedName, isDeclaration} from './utils';

export function collect(
  moduleGraph: TSModuleGraph,
  context: any,
  sourceFile: any,
): any {
  // Factory only exists on TS >= 4.0
  const {factory = ts} = context;

  // When module definitions are nested inside each other (e.g with module augmentation),
  // we want to keep track of the hierarchy so we can associated nodes with the right module.
  const moduleStack: Array<TSModule | null | undefined> = [];
  let _currentModule: TSModule | null | undefined;
  let visit = (node: any): any => {
    if (ts.isBundle(node)) {
      // @ts-expect-error - TS2345 - Argument of type 'readonly SourceFile[]' is not assignable to parameter of type 'NodeArray<any>'.
      return factory.updateBundle(node, ts.visitNodes(node.sourceFiles, visit));
    }

    if (ts.isModuleDeclaration(node)) {
      moduleStack.push(_currentModule);
      _currentModule = new TSModule();
      moduleGraph.addModule(node.name.text, _currentModule);
    }

    if (!_currentModule) {
      return ts.visitEachChild(node, visit, context);
    }

    let currentModule = nullthrows(_currentModule);
    if (ts.isImportDeclaration(node) && node.importClause) {
      if (node.importClause.namedBindings) {
        // @ts-expect-error - TS2339 - Property 'elements' does not exist on type 'NamedImportBindings'.
        if (node.importClause.namedBindings.elements) {
          // @ts-expect-error - TS2339 - Property 'elements' does not exist on type 'NamedImportBindings'.
          for (let element of node.importClause.namedBindings.elements) {
            currentModule.addImport(
              element.name.text,
              // @ts-expect-error - TS2339 - Property 'text' does not exist on type 'Expression'.
              node.moduleSpecifier.text,
              (element.propertyName ?? element.name).text,
            );
          }
          // @ts-expect-error - TS2339 - Property 'name' does not exist on type 'NamedImportBindings'.
        } else if (node.importClause.namedBindings.name) {
          currentModule.addImport(
            // @ts-expect-error - TS2339 - Property 'name' does not exist on type 'NamedImportBindings'.
            node.importClause.namedBindings.name.text,
            // @ts-expect-error - TS2339 - Property 'text' does not exist on type 'Expression'.
            node.moduleSpecifier.text,
            '*',
          );
        }
      }

      if (node.importClause.name) {
        currentModule.addImport(
          node.importClause.name.text,
          // @ts-expect-error - TS2339 - Property 'text' does not exist on type 'Expression'.
          node.moduleSpecifier.text,
          'default',
        );
      }
    }

    if (ts.isExportDeclaration(node)) {
      if (node.exportClause) {
        // @ts-expect-error - TS2339 - Property 'elements' does not exist on type 'NamedExportBindings'.
        for (let element of node.exportClause.elements) {
          if (node.moduleSpecifier) {
            currentModule.addExport(
              element.name.text,
              (element.propertyName ?? element.name).text,
              // @ts-expect-error - TS2339 - Property 'text' does not exist on type 'Expression'.
              node.moduleSpecifier.text,
            );
          } else {
            currentModule.addExport(
              element.name.text,
              (element.propertyName ?? element.name).text,
            );
          }
        }
      } else {
        // @ts-expect-error - TS2532 - Object is possibly 'undefined'. | TS2339 - Property 'text' does not exist on type 'Expression'.
        currentModule.addWildcardExport(node.moduleSpecifier.text);
      }
    }

    node = ts.visitEachChild(node, visit, context);

    if (
      ts.isImportTypeNode(node) &&
      ts.isLiteralTypeNode(node.argument) &&
      ts.isStringLiteral(node.argument.literal)
    ) {
      let local = `$$parcel$import$${moduleGraph.syntheticImportCount++}`;
      let [specifier, entity] = getImportName(node.qualifier, local, factory);
      currentModule.addImport(local, node.argument.literal.text, specifier);
      return factory.createTypeReferenceNode(entity, node.typeArguments);
    }

    // Handle `export default name;`
    if (ts.isExportAssignment(node) && ts.isIdentifier(node.expression)) {
      currentModule.addExport('default', node.expression.text);
    }

    if (isDeclaration(node)) {
      if (node.name) {
        currentModule.addLocal(node.name.text, node);
      }

      let name = getExportedName(node);
      if (name) {
        currentModule.addLocal(name, node);
        currentModule.addExport(name, name);
      }
    }

    if (ts.isVariableStatement(node) && node.modifiers) {
      let isExported = node.modifiers.some(
        (m) => m.kind === ts.SyntaxKind.ExportKeyword,
      );
      for (let v of node.declarationList.declarations) {
        // @ts-expect-error - TS2339 - Property 'text' does not exist on type 'BindingName'.
        currentModule.addLocal(v.name.text, v);
        if (isExported) {
          // @ts-expect-error - TS2339 - Property 'text' does not exist on type 'BindingName'. | TS2339 - Property 'text' does not exist on type 'BindingName'.
          currentModule.addExport(v.name.text, v.name.text);
        }
      }
    }

    // After we finish traversing the children of a module definition,
    // we need to make sure that subsequent nodes get associated with the next-highest level module.
    if (ts.isModuleDeclaration(node)) {
      _currentModule = moduleStack.pop();
    }
    return node;
  };

  return ts.visitNode(sourceFile, visit);
}

// Traverse down an EntityName to the root identifier. Return that to use as the named import specifier,
// and collect the remaining parts into a new QualifiedName with the local replacement at the root.
// import('react').JSX.Element => import {JSX} from 'react'; JSX.Element
// @ts-expect-error - TS7023 - 'getImportName' implicitly has return type 'any' because it does not have a return type annotation and is referenced directly or indirectly in one of its return expressions.
function getImportName(
  qualifier: EntityName | null | undefined,
  local: string,
  factory: typeof ts,
) {
  if (!qualifier) {
    // @ts-expect-error - TS2339 - Property 'createIdentifier' does not exist on type 'typeof ts'.
    return ['*', factory.createIdentifier(local)];
  }

  if (qualifier.kind === ts.SyntaxKind.Identifier) {
    // @ts-expect-error - TS2339 - Property 'createIdentifier' does not exist on type 'typeof ts'.
    return [qualifier.text, factory.createIdentifier(local)];
  }

  // @ts-expect-error - TS7022 - 'name' implicitly has type 'any' because it does not have a type annotation and is referenced directly or indirectly in its own initializer. | TS7022 - 'entity' implicitly has type 'any' because it does not have a type annotation and is referenced directly or indirectly in its own initializer.
  let [name, entity] = getImportName(qualifier.left, local, factory);
  // @ts-expect-error - TS2339 - Property 'createQualifiedName' does not exist on type 'typeof ts'.
  return [name, factory.createQualifiedName(entity, qualifier.right)];
}
