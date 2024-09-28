# atlaspack_vcs

This crate provides integration with `git` and `yarn.lock` such that
**atlaspack** can perform cache invalidation based on version-control
information, as opposed to filesystem events.

There are a few motivations to do this:

- It is significantly faster, in certain cases, to query `git` than it is to
  fetch large lists of events from the watcher
- This allows the current atlaspack caching system to work on CI

## Implementation overview

### Yarn lock and yarn state

`yarn` writes `yarn.lock` files, containing package resolutions. On `yarn v2`,
this file is YAML, and on `yarn v1` it is a custom format.

The file contains mappings of dependency requirements to what package they've
resolved to. For example it might contain (some fields omitted):

```yaml
'lodash@npm:^3':
  resolution: 'lodash@npm:3.10.1'
  checksum: 10c0/f5f6d3d87503c3f1db27d49b30a00bb38dc1bd9de716c5febe8970259cc7b447149a0e320452ccaf5996a7a4abd63d94df341bb91bd8d336584ad518d8eab144
```

Here `'lodash@npm:^3'` is the **requirement**, and `lodash@npm:3.10.1` is the
**resolution**.

Furthermore, on the `node_modules/.yarn-state.yml` file, `yarn` stores all the
filepaths for each **resolution**. The `.yarn-state.yml` might look like:

```yaml
'lodash@npm:3.10.1':
  locations:
    - 'node_modules/lodash'
```

### Overview

The overall idea would be to modify the `getEventsSince` and `writeSnapshot`
filesystem functions.

The snapshot file will be modified to contain,
**in addition to the current watcher snapshot** some git/yarn related metadata.
The metadata stored will be:

1. the current git revision SHA hash
2. a list of the dirty files and their content hashes
3. if any yarn.lock file is dirty, its "yarn snapshot"
   - the yarn.lock file contents
   - the .yarn-state.yaml contents
   - the filepath

When we switch branches, we will read this new snapshot, and query git for the
files that have changed between revisions. This list will not contain untracked
files, such as `node_modules`, hence we will integrate with `yarn`.

If a `yarn.lock` file has changed between the revisions, we will parse its
state at the current revision and the snapshots revision. We will then diff the
`yarn.lock` files looking for changed **resolutions**.

Once we have all changed resolutions, we will use the current `.yarn-state.yml`
file to expand them into file-paths. The old state file could be used to
this, because we do not necessarily need to mark removed dependency paths as
deleted.

#### Untracked files

In order to support cases where the server starts between two uncommitted
changesets, which would not be visible on the git diff, we will store the
content hashes of the uncommitted files. We will perform exclusion over this
list to only consider relevant files. This will also handle all other cases of
**untracked files** that git does not track, but that might be relevant to the
build, such as generated files. Even on large repositories, this has manageable
size, and it can be reduced by avoiding to have such code-gen assets outside of
VCS.

In order to support cases where the user starts-up a build in a dirty repository
which has uncommitted changes to `yarn.lock` and the dependencies, we will store
the **contents of the yarn.lock and yarn-state.yml** files in the snapshot. This
can be done only when the `yarn.lock` file is dirty, since we can otherwise read
its contents from git; but we might want to always store the `yarn-state.yml`
file in order to support marking excluded dependencies as deleted.

#### git integration

The crate integrates with `libgit` and the `git` binary. Currently `libgit` is
linked dynamically with the binary, which means it must be present on the client
machine, but doesn't require us to bump `atlaspack` whenever security fixes are
done to `git`.

Git is used to:

- List files that are dirty/untracked/removed/modified in a repository
- Diff two revisions to find changed files between revisions
- Get the contents of the `yarn.lock` files at different revisions

## Roll-out and validation

We will validate that this implementation is correct by:

- Integrating into a new `FileSystem` implementation under a feature-flag
- Initially we will simply write the snapshots but still return the native
  watcher events ; however, we will diff the two events lists and report
  mismatches
- Once there are no mismatches found in production roll-outs or our testing we
  will stop querying watcher for the initial events list when starting-up a
  development build
- We will then try to implement CI caching using this implementation
- To achieve that we will perform similar comparison to guarantee we are
  producing equivalent build outputs
