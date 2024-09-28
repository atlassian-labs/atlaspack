// @flow strict-local

import path from 'path';
import Git from 'nodegit';
import {NodeFS} from './NodeFS';
import {getVcsStateSnapshot, getEventsSince} from '@atlaspack/rust';
import type {FilePath} from '@atlaspack/types-internal';
import type {Event, Options as WatcherOptions} from '@parcel/watcher';

export class NodeVCSAwareFS extends NodeFS {
  getEventsSince(
    dir: FilePath,
    snapshot: FilePath,
    opts: WatcherOptions,
  ): Promise<Array<Event>> {
    const snapshotFile = await this.readFile(snapshot).toString();
    const {nativeSnapshotPath, vcsState} = JSON.parse(snapshotFile);

    const watcherEventsSince = await this.watcher().getEventsSince(
      dir,
      nativeSnapshotPath,
      opts,
    );
    // TODO: we need the git repo path
    const vcsEventsSince = getEventsSince();

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
    await this.watcher().writeSnapshot(dir, nativeSnapshotPath, opts);

    // TODO: we need the git repo path, pass the exclude patterns
    const vcsState = await getVcsStateSnapshot();

    const snapshotContents = {
      vcsState,
      nativeSnapshotPath,
    };
    await this.writeFile(snapshot, JSON.stringify(snapshotContents));
  }
}
