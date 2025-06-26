import path from 'path';
import {NodeFS} from './NodeFS';
import {getVcsStateSnapshot, getEventsSince} from '@atlaspack/rust';
import type {FilePath} from '@atlaspack/types-internal';
import type {Event, Options as WatcherOptions} from '@parcel/watcher';
import {registerSerializableClass} from '@atlaspack/build-cache';
import logger, {instrumentAsync} from '@atlaspack/logger';
import {getFeatureFlagValue} from '@atlaspack/feature-flags';

import packageJSON from '../package.json';

export interface NodeVCSAwareFSOptions {
  gitRepoPath: null | FilePath;
  excludePatterns: Array<string>;
  logEventDiff: null | ((watcherEvents: Event[], vcsEvents: Event[]) => void);
}

export type SerializedNodeVCSAwareFS = NodeVCSAwareFSOptions;

export class NodeVCSAwareFS extends NodeFS {
  /**
   * These files are excluded from 'dirty file' tracking even if they are
   * modified.
   */
  #excludePatterns: Array<string>;
  /**
   * Logging function for the diff between watcher events and vcs events.
   */
  #logEventDiff: null | ((watcherEvents: Event[], vcsEvents: Event[]) => void);
  /**
   * The path of the git repository containing the project root. Null if the
   * project is not a git repository.
   */
  #gitRepoPath: null | FilePath;

  constructor(options: NodeVCSAwareFSOptions) {
    super();
    this.#excludePatterns = options.excludePatterns;
    this.#logEventDiff = options.logEventDiff;
    this.#gitRepoPath = options.gitRepoPath;
  }

  static deserialize(data: SerializedNodeVCSAwareFS): NodeVCSAwareFS {
    const fs = new NodeVCSAwareFS({
      excludePatterns: data.excludePatterns,
      logEventDiff: null,
      gitRepoPath: data.gitRepoPath,
    });
    return fs;
  }

  serialize(): SerializedNodeVCSAwareFS {
    return {
      excludePatterns: this.#excludePatterns,
      logEventDiff: null,
      gitRepoPath: this.#gitRepoPath,
    };
  }

  setGitRepoPath(gitRepoPath: null | FilePath) {
    this.#gitRepoPath = gitRepoPath;
  }

  async getEventsSince(dir: FilePath, snapshot: FilePath, opts: WatcherOptions): Promise<Array<Event>> {
    const gitRepoPath = this.#gitRepoPath;
    if (gitRepoPath == null) {
      return this.watcher().getEventsSince(dir, snapshot, opts);
    }

    const {nativeSnapshotPath, vcsState} = await instrumentAsync(
      'NodeVCSAwareFS.readSnapshot',
      async () => {
        // Note: can't use toString() directly, or it won't resolve the promise
        const snapshotFile = await this.readFile(snapshot);
        const snapshotFileContent = snapshotFile.toString();
        return JSON.parse(snapshotFileContent);
      },
    );
    let watcherEventsSince: Array<Event> = [];

    const vcsEventsSince =
      vcsState != null
        ? (
            await instrumentAsync('NodeVCSAwareFS::rust.getEventsSince', () =>
              getEventsSince(gitRepoPath, vcsState, null),
            )
          ).map((e) => ({
            path: e.path,
            type: e.changeType,
          }))
        : null;

    if (getFeatureFlagValue('vcsMode') !== 'NEW' && vcsEventsSince != null) {
      watcherEventsSince = await instrumentAsync(
        'NodeVCSAwareFS::watchman.getEventsSince',
        () => this.watcher().getEventsSince(dir, nativeSnapshotPath, opts),
      );
      this.#logEventDiff?.(watcherEventsSince, vcsEventsSince);
    }

    if (['NEW_AND_CHECK', 'NEW'].includes(getFeatureFlagValue('vcsMode'))) {
      if (vcsEventsSince == null) {
        logger.error({
          origin: '@atlaspack/fs',
          message:
            'Missing VCS state. There was an error when writing the snapshot. Please clear your cache.',
          meta: {
            trackableEvent: 'vcs_state_snapshot_read_failed',
          },
        });

        return [];
      }

      return vcsEventsSince;
    }

    return watcherEventsSince;
  }

  async writeSnapshot(dir: FilePath, snapshot: FilePath, opts: WatcherOptions): Promise<void> {
    const gitRepoPath = this.#gitRepoPath;
    if (gitRepoPath == null) {
      await this.watcher().writeSnapshot(dir, snapshot, opts);
      return;
    }

    const snapshotDirectory = path.dirname(snapshot);
    await this.mkdirp(snapshotDirectory);
    const filename = path.basename(snapshot, '.txt');
    const nativeSnapshotPath = path.join(
      snapshotDirectory,
      `${filename}.native-snapshot.txt`,
    );

    if (getFeatureFlagValue('vcsMode') !== 'NEW') {
      await instrumentAsync(
        'NodeVCSAwareFS::watchman.writeSnapshot',
        async () => {
          await this.watcher().writeSnapshot(dir, nativeSnapshotPath, opts);
        },
      );
    }

    let vcsState = null;
    try {
      vcsState = await instrumentAsync(
        'NodeVCSAwareFS::getVcsStateSnapshot',
        () => getVcsStateSnapshot(gitRepoPath, this.#excludePatterns),
      );

      logger.verbose({
        origin: '@atlaspack/fs',
        message: 'Expose VCS timing metrics',
        meta: {
          trackableEvent: 'vcs_timing_metrics',
          dirtyFilesExecutionTime: vcsState?.dirtyFilesExecutionTime,
          yarnStatesExecutionTime: vcsState?.yarnStatesExecutionTime,
        },
      });
    } catch (err: any) {
      logger.error({
        origin: '@atlaspack/fs',
        message: `Failed to get VCS state snapshot: ${err.message}`,
        meta: {
          trackableEvent: 'vcs_state_snapshot_failed',
          error: err,
        },
      });
    }

    const snapshotContents = {
      vcsState,
      nativeSnapshotPath,
    } as const;
    await this.writeFile(snapshot, JSON.stringify(snapshotContents));
  }
}

registerSerializableClass(
  `${packageJSON.version}:NodeVCSAwareFS`,
  NodeVCSAwareFS,
);
