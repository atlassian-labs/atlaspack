# Symbol Deduplication in Scope Hoisting Packager

## Problem

The scope hoisting packager was creating duplicate variable declarations when multiple assets in the same scope referenced the same dependency. This led to JavaScript syntax errors like:

```javascript
var $abc12 = parcelRequire('abc12');
var $abc12 = parcelRequire('abc12'); // Duplicate declaration error!
```

## Root Cause

The issue occurred in the `getSymbolResolution` method around line 1276, where hoisted `parcelRequire` calls were generated:

```typescript
hoisted.set(
  resolvedAsset.id,
  `var $${publicId} = parcelRequire(${JSON.stringify(publicId)});`,
);
```

When multiple assets in the same scope imported the same dependency, they would each try to create the same variable declaration, leading to duplicates.

## Solution

### 1. SymbolTracker Class

Created a new `SymbolTracker` class that:

- **Tracks scopes**: Maps assets to their execution scopes (top-level, wrapped, etc.)
- **Tracks symbols**: Maintains a set of declared symbols per scope
- **Prevents duplicates**: Checks if a symbol is already declared before allowing new declarations

### 2. Scope Detection

The tracker determines scopes based on:

- **Wrapped assets**: Get their own isolated scope (`wrapped:${assetId}`)
- **Hoisted assets**: Share the top-level scope or inherit from parent
- **Top-level scope**: Default scope for non-wrapped assets

### 3. Integration Points

#### Asset Registration

In `loadAssets()`, all assets are registered with the symbol tracker:

```typescript
this.bundle.traverseAssets((asset) => {
  let isWrapped = this.wrappedAssets.has(asset.id);
  this.symbolTracker.registerAsset(asset, isWrapped);
});
```

#### Symbol Declaration Prevention

In `getSymbolResolution()`, before creating hoisted variables:

```typescript
let variableName = `$${publicId}`;
if (!this.symbolTracker.isSymbolDeclared(parentAsset, variableName)) {
  this.symbolTracker.declareSymbol(parentAsset, variableName);
  hoisted.set(resolvedAsset.id, `var ${variableName} = parcelRequire(...);`);
}
```

#### Duplicate Filtering

In `getHoistedParcelRequires()`, filter out duplicates:

```typescript
let uniqueDeclarations = [...hoisted.values()].filter((declaration) => {
  let match = declaration.match(/^var (\$[a-zA-Z0-9]+) =/);
  if (match) {
    let varName = match[1];
    if (!this.symbolTracker.isSymbolDeclared(parentAsset, varName)) {
      this.symbolTracker.declareSymbol(parentAsset, varName);
      return true;
    }
    return false;
  }
  return true;
});
```

## Benefits

1. **Eliminates duplicate declarations**: No more `var $abc12` conflicts
2. **Maintains correct scoping**: Wrapped assets can still have their own variables
3. **Preserves functionality**: All imports still work correctly
4. **Performance**: Minimal overhead, only tracks what's necessary
5. **Testable**: Clear separation of concerns with dedicated test suite

## Testing

The solution includes comprehensive tests in `SymbolTracker.test.ts` that verify:

- Correct scope assignment for wrapped vs. hoisted assets
- Prevention of duplicate symbol declarations within scopes
- Allowance of same symbols across different scopes
- Proper tracking of scope membership and symbol existence

## Future Enhancements

This system could be extended to:

- Track other types of symbol conflicts (not just hoisted requires)
- Provide better error messages when conflicts are detected
- Optimize symbol naming to avoid conflicts proactively
- Support more complex scoping scenarios as the bundler evolves
