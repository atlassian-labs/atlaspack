# @atlaspack/integration-tests

Contains an E2E / integration test suite.

Most work by creating an `Atlaspack` instance and running against some entries on disk then running
assertions on the output data structures and output bundles.

## Running integration tests

```
yarn test:integration
```

## Profiling integration tests

```
ATLAPACK_PROFILE_MOCHA=true yarn test:integration
```

You should see a CPU profile appear on `./mocha-cpu-profiles/*`

## Fixtures and file-system access

Currently most tests run against an overlay of the `./integration-tests/test` fixtures directory
and a `MemoryFS` instance used for writing.

Some tests, like some cache tests will use the real file-system, and thus end-up E2E testing the
watcher implementation.
