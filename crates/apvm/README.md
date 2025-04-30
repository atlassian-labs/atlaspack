# 📚 Atlaspack Version Manager 📚

Version manager and installer for Atlaspack

## Installation

```bash
# Install globally
npm install -g @atlaspack/apvm

# Install into current project
npm install --save-dev @atlaspack/apvm
yarn add -D @atlaspack/apvm
```

## Usage

### CLI

```bash
# Install a version of Atlaspack from npm
apvm install 2.14.0
apvm install 2.15.0

# Link into node_modules the version specified in the project config or default
# This will install the version if not already installed
apvm link

# Link into node_modules (overriding project config)
apvm link 2.14.0
apvm link 2.15.0
```

### Config

Config can be specified in a `package.json` or `.apvmrc`

```json
// package.json
{
  "atlaspack": {
    "version": "2.15.0"
  }
}
```

```bash
# Will install 2.15.0
apvm link
```

Or using aliased versions

```json
// package.json
{
  "atlaspack": {
    "versions": {
      "default": "2.15.0",
      "next": "2.16.0"
    }
  }
}
```

```bash
# Will install & link 2.15.0
apvm link

# Will install & link 2.16.0
apvm link next
```

### Install a version from git

Versions obtained from git will build after being fetched. This takes a while.

```bash
apvm install git:main
apvm install git:my-branch
apvm install git:1fb73643c
```

### Register a locally installed git repo

```bash
# Register your local Atlaspack sources
apvm install local:/Users/username/atlaspack
```
