const featureLibraryFunctions = new Set(['getFeatureFlag']);

const isInFunctionLevel = context => {
  let scope = context.getScope();

  while (scope?.type !== 'module' && scope?.type !== 'global') {
    if (scope.type === 'function') {
      return true;
    }

    if (scope.type === 'class-field-initializer') {
      return !scope.block.parent.static;
    }

    scope = scope.upper;
  }

  return false;
};

module.exports = {
  meta: {
    docs: {
      description: 'Disallow feature flag usage at module level',
    },
    messages: {
      noModuleLevelEval:
        'Do not evaluate feature flags at module level. This causes integration tests to fail and feature flag values to be incorrect between different parcel configurations in the same process.',
    },
  },
  create(context) {
    return {
      'CallExpression[callee.type="Identifier"]': node => {
        if (
          featureLibraryFunctions.has(node.callee.name) &&
          !isInFunctionLevel(context)
        ) {
          context.report({
            messageId: 'noModuleLevelEval',
            node,
          });
        }
      },
    };
  },
};
