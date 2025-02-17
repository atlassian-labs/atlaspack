// @flow strict-local

import path from 'path';
import {NodeFS} from './NodeFS';
import {getVcsStateSnapshot, getEventsSince} from '@atlaspack/rust';
import type {FilePath} from '@atlaspack/types-internal';
import type {Event, Options as WatcherOptions} from '@parcel/watcher';
import {registerSerializableClass} from '@atlaspack/build-cache';
import {instrument, instrumentAsync} from '@atlaspack/logger';

// $FlowFixMe
import packageJSON from '../package.json';

export interface NodeVCSAwareFSOptions {
  gitRepoPath: FilePath;
  excludePatterns: Array<string>;
  logEventDiff: (watcherEvents: Event[], vcsEvents: Event[]) => void;
}

export class NodeVCSAwareFS extends NodeFS {
  #options: NodeVCSAwareFSOptions;

  constructor(options: NodeVCSAwareFSOptions) {
    super();
    this.#options = options;
  }

  async getEventsSince(
    dir: FilePath,
    snapshot: FilePath,
    opts: WatcherOptions,
  ): Promise<Array<Event>> {
    const {nativeSnapshotPath, vcsState} = await instrumentAsync(
      'NodeVCSAwareFS.readSnapshot',
      async () => {
        // Note: can't use toString() directly, or it won't resolve the promise
        const snapshotFile = await this.readFile(snapshot);
        const snapshotFileContent = snapshotFile.toString();
        return JSON.parse(snapshotFileContent);
      },
    );

    const watcherEventsSince = await instrumentAsync(
      'NodeVCSAwareFS::watchman.getEventsSince',
      () => this.watcher().getEventsSince(dir, nativeSnapshotPath, opts),
    );
    const vcsEventsSince = instrument(
      'NodeVCSAwareFS::rust.getEventsSince',
      () => getEventsSince(this.#options.gitRepoPath, vcsState.gitHash),
    ).map((e) => ({path: e.path, type: e.changeType}));
    this.#options.logEventDiff(watcherEventsSince, vcsEventsSince);

    return watcherEventsSince;
  }

  async writeSnapshot(
    dir: FilePath,
    snapshot: FilePath,
    opts: WatcherOptions,
  ): Promise<void> {
    const snapshotDirectory = path.dirname(snapshot);
    const filename = path.basename(snapshot, '.txt');
    const nativeSnapshotPath = path.join(
      snapshotDirectory,
      `${filename}.native-snapshot.txt`,
    );

    console.log('watchman.writeSnapshot');
    await instrumentAsync(
      'NodeVCSAwareFS::watchman.writeSnapshot',
      async () => {
        await this.watcher().writeSnapshot(dir, nativeSnapshotPath, opts);
      },
    );
    console.log('watchman.writeSnapshot.end');

    const vcsState = instrument('NodeVCSAwareFS::getVcsStateSnapshot', () =>
      getVcsStateSnapshot(
        this.#options.gitRepoPath,
        this.#options.excludePatterns,
      ),
    );

    const snapshotContents = {
      vcsState,
      nativeSnapshotPath,
    };
    await this.writeFile(snapshot, JSON.stringify(snapshotContents));
  }
}

registerSerializableClass(
  `${packageJSON.version}:NodeVCSAwareFS`,
  NodeVCSAwareFS,
);
