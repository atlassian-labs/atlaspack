# Atlaspack CLI Documentation

This directory contains comprehensive documentation for all Atlaspack CLI commands and tools.

## Quick Reference

### Build Commands

- [`atlaspack serve`](build-commands.md#atlaspack-serve) - Start development server and watch for changes.
- [`atlaspack watch`](build-commands.md#atlaspack-watch) - Start bundler in watch mode, but does not start a dev server.
- [`atlaspack build`](build-commands.md#atlaspack-build) - Bundle for production with optimizations.

### Configuration

- [Environment Variables](environment-variables.md) - Configure Atlaspack behavior via environment variables

### Other

- [`apvm`](../../crates/apvm/README.md) - A version manager that allows you to install, manage, and switch between different versions of Atlaspack.
- [`atlaspack-link`](../../packages/dev/atlaspack-link/README.md) - Link dev copy of Atlaspack into an app
- [`parcel-to-atlaspack`](../../packages/migrations/parcel-to-atlaspack/README.md) - Migrate from Parcel to Atlaspack
