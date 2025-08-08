#!/usr/bin/env -S node
/* eslint-disable no-console */

import {$} from 'zx';
import {parseArgs} from 'util';
import {writeFileSync, unlinkSync} from 'fs';
import {join} from 'path';
import {tmpdir} from 'os';

async function countTSSuppressions(basePath: string): Promise<number> {
  const count = Number(
    (
      await $`rg --no-heading -o '@ts-expect-error' ${basePath} | wc -l`.text()
    ).trim(),
  );
  return count;
}

async function saveMetricsToGitNote(
  commitHash: string,
  count: number,
): Promise<void> {
  // Try to get existing note
  let existingNote = '';
  try {
    existingNote = (await $`git notes show ${commitHash}`.text()).trim();
  } catch (error) {
    // Note doesn't exist, that's fine
  }

  // Parse existing note or create new metrics object
  let metrics = {tsErrorSuppressions: count};
  let updatedNote = '';

  if (existingNote) {
    const lines = existingNote.split('\n');
    let metricsLineFound = false;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      if (line.startsWith('METRICS: ')) {
        try {
          const existingMetrics = JSON.parse(line.substring(9));
          metrics = {...existingMetrics, tsErrorSuppressions: count};
          lines[i] = `METRICS: ${JSON.stringify(metrics)}`;
          metricsLineFound = true;
        } catch (error) {
          // If parsing fails, replace the line
          lines[i] = `METRICS: ${JSON.stringify(metrics)}`;
          metricsLineFound = true;
        }
        break;
      }
    }

    if (metricsLineFound) {
      updatedNote = lines.join('\n');
    } else {
      // Add METRICS line at the end
      updatedNote = existingNote + '\nMETRICS: ' + JSON.stringify(metrics);
    }
  } else {
    // No existing note, create new one
    updatedNote = `METRICS: ${JSON.stringify(metrics)}`;
  }

  // Add or update the note using a temporary file to avoid command line issues
  const tempFile = join(tmpdir(), `git-notes-${Date.now()}.txt`);
  try {
    writeFileSync(tempFile, updatedNote);
    await $`git notes add -f ${commitHash} -F ${tempFile}`;
  } finally {
    try {
      unlinkSync(tempFile);
    } catch (cleanupError) {
      // Ignore cleanup errors
    }
  }
}

async function showTSSuppressions(days: number = 30) {
  try {
    // Find the commit that is the oldest within the specified days
    const cutoffDate = new Date();
    cutoffDate.setDate(cutoffDate.getDate() - days);
    const cutoffTimestamp = Math.floor(cutoffDate.getTime() / 1000);

    // Get commits from the cutoff date to now
    const commits = (
      await $`git log --since="${days} days ago" --format="%H %at" --reverse`.text()
    )
      .trim()
      .split('\n')
      .filter((line) => line.length > 0)
      .map((line) => {
        const [hash, timestamp] = line.split(' ');
        return {hash, timestamp: parseInt(timestamp)};
      });

    console.log('date,commitid,ts errors metric');

    for (const commit of commits) {
      const commitDate = new Date(commit.timestamp * 1000)
        .toISOString()
        .split('T')[0];

      try {
        // Try to get the git note for this commit
        const note = (
          await $`git notes show ${commit.hash} 2>/dev/null`.text()
        ).trim();

        // Parse the METRICS line if it exists
        const metricsLine = note
          .split('\n')
          .find((line) => line.startsWith('METRICS: '));
        if (metricsLine) {
          try {
            const metrics = JSON.parse(metricsLine.substring(9));
            const tsErrors = metrics.tsErrorSuppressions || 0;
            if (tsErrors > 0) {
              console.log(`${commitDate},${commit.hash},${tsErrors}`);
            }
          } catch (parseError) {
            // If JSON parsing fails, skip output (equivalent to 0)
          }
        }
        // No METRICS line found - skip output (equivalent to 0)
      } catch (noteError) {
        // No note exists for this commit - skip output (equivalent to 0)
      }
    }
  } catch (error) {
    console.error('Failed to show TS suppressions history:');
    if (error instanceof Error) {
      console.error(`  Error: ${error.message}`);
      if ('stderr' in error && typeof (error as any).stderr === 'string') {
        console.error(`  Details: ${(error as any).stderr}`);
      }
    } else {
      console.error(`  Error: ${error}`);
    }
  }
}

async function backfillTSSuppressions(basePath: string, days: number = 30) {
  try {
    // Create a temporary worktree using mktemp
    const worktreePath = join(tmpdir(), `ts-metrics-backfill-${Date.now()}`);
    console.log(`Creating temporary worktree at ${worktreePath}`);

    try {
      // Create the worktree
      await $`git worktree add ${worktreePath} main`;

      // Get commits from the cutoff date to now, only on main branch
      const commits = (
        await $`git log --since="${days} days ago" --format="%H %at" --reverse --first-parent`.text()
      )
        .trim()
        .split('\n')
        .filter((line) => line.length > 0)
        .map((line) => {
          const [hash, timestamp] = line.split(' ');
          return {hash, timestamp: parseInt(timestamp)};
        });

      console.log(`Found ${commits.length} commits to process`);

      for (const commit of commits) {
        const commitDate = new Date(commit.timestamp * 1000)
          .toISOString()
          .split('T')[0];
        console.log(`Processing commit ${commit.hash} from ${commitDate}...`);

        try {
          // Check if this commit already has a note with metrics
          let existingNote = '';
          let hasMetrics = false;
          try {
            existingNote = (
              await $`git notes show ${commit.hash}`.text()
            ).trim();
            hasMetrics = existingNote
              .split('\n')
              .some((line) => line.startsWith('METRICS: '));
          } catch (noteError) {
            // No note exists, which is fine
            hasMetrics = false;
          }

          if (hasMetrics) {
            console.log(`  Skipping - already has metrics`);
            continue;
          }

          // Checkout the commit in the worktree
          try {
            await $`cd ${worktreePath} && git checkout ${commit.hash}`;
          } catch (checkoutError) {
            console.error(
              `  Failed to checkout commit ${commit.hash}:`,
              checkoutError,
            );
            continue;
          }

          // Count TS suppressions for this commit using the worktree path
          try {
            const count = Number(
              (
                await $`cd ${worktreePath} && rg --no-heading -o '@ts-expect-error' ${basePath} | wc -l`.text()
              ).trim(),
            );

            if (count > 0) {
              console.log(`  Found ${count} TS suppressions`);

              // Save the metrics to git note
              await saveMetricsToGitNote(commit.hash, count);
              console.log(`  Saved metrics to git note`);
            } else {
              console.log(`  No TS suppressions found`);
            }
          } catch (countError) {
            console.error(
              `  Failed to count suppressions for commit ${commit.hash}:`,
              countError,
            );
          }
        } catch (error) {
          console.error(`  Error processing commit ${commit.hash}:`, error);
          if (error instanceof Error && 'stderr' in error) {
            console.error(`  Details: ${(error as any).stderr}`);
          }
        }
      }
    } finally {
      // Clean up the worktree
      console.log(`Cleaning up worktree...`);
      try {
        await $`git worktree remove -f ${worktreePath}`;
      } catch (cleanupError) {
        console.error(
          `Warning: Could not remove worktree ${worktreePath}:`,
          cleanupError,
        );
      }
    }

    console.log(`Backfill completed`);
  } catch (error) {
    console.error('Failed to backfill TS suppressions:');
    if (error instanceof Error) {
      console.error(`  Error: ${error.message}`);
      if ('stderr' in error && typeof (error as any).stderr === 'string') {
        console.error(`  Details: ${(error as any).stderr}`);
      }
    } else {
      console.error(`  Error: ${error}`);
    }
  }
}

async function findTSSuppressions(basePath: string, save: boolean = false) {
  const count = await countTSSuppressions(basePath);
  console.log(count);

  if (save) {
    try {
      // Get the current commit hash
      const commitHash = (await $`git rev-parse HEAD`.text()).trim();

      await saveMetricsToGitNote(commitHash, count);
      console.log(`Saved metrics to git note for commit ${commitHash}`);
    } catch (error) {
      console.error('Failed to save metrics to git note:');
      if (error instanceof Error) {
        console.error(`  Error: ${error.message}`);
        // Check if it's a zx ProcessOutput error
        if ('stderr' in error && typeof (error as any).stderr === 'string') {
          console.error(`  Details: ${(error as any).stderr}`);
        }
      } else {
        console.error(`  Error: ${error}`);
      }
    }
  }
}

interface ParsedArgs {
  values: {
    save?: boolean;
    show?: boolean;
    backfill?: boolean;
    days?: string;
  };
  positionals: string[];
  basePath?: string;
  days: number;
}

function parseArguments(): ParsedArgs {
  const {values, positionals} = parseArgs({
    options: {
      save: {
        type: 'boolean',
        short: 's',
      },
      show: {
        type: 'boolean',
        short: 'w',
      },
      backfill: {
        type: 'boolean',
        short: 'b',
      },
      days: {
        type: 'string',
        short: 'd',
        default: '30',
      },
    },
    allowPositionals: true,
  });

  const basePath = positionals[0];
  const days = parseInt(values.days as string) || 30;

  // Check for mutually exclusive options
  const activeOptions = [values.save, values.show, values.backfill].filter(
    Boolean,
  );
  if (activeOptions.length > 1) {
    console.error(
      'Error: --save, --show, and --backfill are mutually exclusive',
    );
    process.exitCode = 1;
    return {values, positionals, days};
  }

  // Validate base path requirements
  if (values.backfill && !basePath) {
    console.error('Error: Base path is required for --backfill');
    process.exitCode = 1;
    return {values, positionals, days};
  }

  if (!values.show && !values.backfill && !basePath) {
    console.error('Error: Base path is required for --save or default mode');
    process.exitCode = 1;
    return {values, positionals, days};
  }

  return {values, positionals, basePath, days};
}

async function main() {
  const args = parseArguments();

  // Only proceed if no errors occurred during argument parsing
  if (process.exitCode && process.exitCode !== 0) {
    return;
  }

  try {
    if (args.values.backfill) {
      await backfillTSSuppressions(args.basePath!, args.days);
    } else if (args.values.show) {
      await showTSSuppressions(args.days);
    } else {
      await findTSSuppressions(args.basePath!, args.values.save as boolean);
    }
  } catch (err) {
    console.error(err);
    process.exitCode = 1;
  }
}

// Start the application
main();
