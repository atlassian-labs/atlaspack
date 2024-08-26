# @atlaspack/parcel-to-atlaspack

This package exposes a CLI that can be used to migrate a Parcel application to Atlaspack

## Installation

```sh
npm install -g @atlaspack/parcel-to-atlaspack
```

## Usage

```sh
parcel-to-atlaspack
parcel-to-atlaspack --dry-run
parcel-to-atlaspack --help
```

## Features

This CLI supports the following features:

- Replace parcel dependencies with atlaspack in `package.json`
- Rename `.parcelrc` files to `.atlaspackrc`
- Update `@parcel/` plugin references to `@atlaspack` in both `.atlaspackrc` and `package.json`
- Update `package.json#engines` to reference atlaspack instead of parcel

## Development Workflow

```sh
yarn workspace @atlaspack/parcel-to-atlaspack start
yarn workspace @atlaspack/parcel-to-atlaspack start --dry-run
yarn workspace @atlaspack/parcel-to-atlaspack start --help
```
