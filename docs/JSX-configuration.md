# JSX Configuration

Atlaspack provides flexible JSX configuration options through your project's package.json file. This allows you to control how JSX is transformed, customize pragma functions, and enable modern automatic runtime features.

## Feature Flag Requirement

The new JSX configuration system is currently experimental and requires the `newJsxConfig` feature flag to be enabled. You can enable this flag in your Atlaspack configuration:

```json
{
  "extends": "@atlaspack/config-default",
  "featureFlags": {
    "newJsxConfig": true
  }
}
```

**Behavior:**

- **With flag enabled**: JSX configuration comes from `@atlaspack/transformer-js` in package.json
- **With flag disabled**: Legacy behavior using automatic detection from dependencies and tsconfig.json

## Configuration Options

Configure JSX transformation by adding an `@atlaspack/transformer-js` section to your package.json:

```json
{
  "name": "my-app",
  "@atlaspack/transformer-js": {
    "jsx": {
      "pragma": "React.createElement",
      "pragmaFragment": "React.Fragment",
      "importSource": "react",
      "automaticRuntime": true
    }
  }
}
```

> **Note:** The JSX configuration options are currently experimental and require the `newJsxConfig` feature flag to be enabled. Without this flag, Atlaspack will use the legacy JSX detection based on your dependencies and tsconfig.json.

## JSX Configuration Options

### `pragma`

- **Type**: `string`
- **Default**: `"React.createElement"`
- **Description**: The function used to transform JSX elements in classic mode.

```json
{
  "@atlaspack/transformer-js": {
    "jsx": {
      "pragma": "React.createElement" // Standard React
    }
  }
}
```

**Examples:**

```json
// Preact
"pragma": "h"

// Custom function
"pragma": "createElement"

// Hyperapp
"pragma": "h"
```

### `pragmaFragment`

- **Type**: `string`
- **Default**: `"React.Fragment"`
- **Description**: The function used to transform JSX fragments in classic mode.

```json
{
  "@atlaspack/transformer-js": {
    "jsx": {
      "pragmaFragment": "React.Fragment" // Standard React
    }
  }
}
```

**Examples:**

```json
// Preact
"pragmaFragment": "Fragment"

// Fallback to div
"pragmaFragment": "'div'"
```

## Automatic JSX Runtime

The automatic JSX runtime transforms JSX without requiring `React` to be in scope. It uses `jsx()` and `jsxs()` functions from the import source, making your code cleaner and more efficient.

### `importSource`

- **Type**: `string`
- **Default**: `"react"`
- **Description**: The import source for automatic JSX runtime functions.

```json
{
  "@atlaspack/transformer-js": {
    "jsx": {
      "importSource": "react", // React 17+
      "automaticRuntime": true
    }
  }
}
```

**Examples:**

```json
// Preact
"importSource": "preact/jsx-runtime"

// Emotion
"importSource": "@emotion/react"

// Custom JSX implementation
"importSource": "./my-jsx-runtime"
```

### `automaticRuntime`

- **Type**: `boolean | {include: string[], exclude?: string[]}`
- **Default**: `false`
- **Description**: Controls when automatic JSX runtime is enabled.

## Automatic Runtime Configuration

### Global Enable/Disable

```json
{
  "@atlaspack/transformer-js": {
    "jsx": {
      "automaticRuntime": true // Always enabled
    }
  }
}
```

```json
{
  "@atlaspack/transformer-js": {
    "jsx": {
      "automaticRuntime": false // Always disabled (classic mode)
    }
  }
}
```

### File Pattern Matching

Enable automatic runtime only for specific file patterns using glob patterns with include and exclude rules:

```json
{
  "@atlaspack/transformer-js": {
    "jsx": {
      "importSource": "react",
      "automaticRuntime": {
        "include": [
          "src/components/**/*.tsx",
          "src/pages/**/*.jsx",
          "../shared/**/*.tsx"
        ],
        "exclude": ["src/components/legacy/**/*.tsx", "src/pages/old/**/*.jsx"]
      }
    }
  }
}
```

**Include/Exclude Rules:**

- **`include`**: Array of glob patterns that files must match to get automatic runtime
- **`exclude`** (optional): Array of glob patterns that override includes - if a file matches both include and exclude, it will NOT get automatic runtime
- **Exclude wins**: If a file matches both include and exclude patterns, exclude takes precedence

**Glob Pattern Examples:**

- `"src/components/**/*.tsx"` - All TypeScript JSX files in components directory
- `"**/*.jsx"` - All JSX files in any directory
- `"../other-project/**/*.tsx"` - Files outside project root
- `"src/modern/**/*"` - All files in modern directory

**Pattern Matching Rules:**

- Patterns are matched against the relative path from project root
- Supports `**` for recursive directory matching
- Supports `*` for single-level wildcards
- Supports `..` for paths outside project root
- Uses forward slashes `/` regardless of platform
