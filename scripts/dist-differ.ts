/* eslint-disable no-console */

/**
 * Diff the results of two different directories
 *
 * 1. Check if they have the same number of files
 * 2. Report the diffreences in files and stop if they differ
 * 3. Check if the files have the same names
 * 4. Check that the file contents is the same
 * 5. Report the diff on the files that differ
 */

import * as fs from 'fs';
import * as path from 'path';
import {execSync} from 'child_process';

interface FileInfo {
  relativePath: string;
  fullPath: string;
  size: number;
}

function getAllFiles(dir: string, baseDir: string = dir): FileInfo[] {
  const files: FileInfo[] = [];

  try {
    const entries = fs.readdirSync(dir, {withFileTypes: true});

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (fullPath.endsWith('.js.map')) {
        // Skip .js.map files
        continue;
      }

      if (entry.isDirectory()) {
        files.push(...getAllFiles(fullPath, baseDir));
      } else if (entry.isFile()) {
        const relativePath = path.relative(baseDir, fullPath);
        const stats = fs.statSync(fullPath);
        files.push({
          relativePath,
          fullPath,
          size: stats.size,
        });
      }
    }
  } catch (error) {
    console.error(`Error reading directory ${dir}:`, error);
    process.exit(1);
  }

  return files;
}

function compareFileContents(file1: string, file2: string): string | null {
  try {
    const content1 = fs.readFileSync(file1);
    const content2 = fs.readFileSync(file2);

    if (Buffer.compare(content1, content2) === 0) {
      return null; // Files are identical
    }

    // Try to get a readable diff using the system diff command
    try {
      const diffOutput = execSync(`diff -u "${file1}" "${file2}"`, {
        encoding: 'utf8',
        maxBuffer: 50 * 1024 * 1024, // 50MB buffer
      });
      return diffOutput;
    } catch (diffError) {
      // diff command returns non-zero exit code when files differ
      if (diffError instanceof Error && 'stdout' in diffError) {
        return (diffError as any).stdout;
      }
      return `Files differ but unable to generate readable diff. File sizes: ${content1.length} vs ${content2.length} bytes`;
    }
  } catch (error) {
    return `Error comparing files: ${error}`;
  }
}

function main() {
  const args = process.argv.slice(2);

  if (args.length !== 2) {
    console.error(
      'Usage: node --experimental-strip-types dist-differ.ts <dir1> <dir2>',
    );
    console.error(
      'Example: node --experimental-strip-types dist-differ.ts packages/entry-point/dist packages/entry-point/dist-control',
    );
    process.exit(1);
  }

  const [dir1, dir2] = args;

  // Check if directories exist
  if (!fs.existsSync(dir1)) {
    console.error(`Directory does not exist: ${dir1}`);
    process.exit(1);
  }

  if (!fs.existsSync(dir2)) {
    console.error(`Directory does not exist: ${dir2}`);
    process.exit(1);
  }

  console.log(`Comparing directories:`);
  console.log(`  Dir 1: ${dir1}`);
  console.log(`  Dir 2: ${dir2}`);
  console.log();

  // Get all files from both directories
  const files1 = getAllFiles(dir1);
  const files2 = getAllFiles(dir2);

  // Step 1: Check if they have the same number of files
  console.log(`Step 1: Checking file counts...`);
  console.log(`  Dir 1: ${files1.length} files`);
  console.log(`  Dir 2: ${files2.length} files`);

  console.log(`\nStep 2: Reporting file differences if any...`);
  if (files1.length !== files2.length) {
    console.log(`❌ File count mismatch: ${files1.length} vs ${files2.length}`);

    // Step 2: Report the differences in files
    const paths1 = new Set(files1.map((f) => f.relativePath));
    const paths2 = new Set(files2.map((f) => f.relativePath));

    const onlyInDir1 = [...paths1].filter((p) => !paths2.has(p));
    const onlyInDir2 = [...paths2].filter((p) => !paths1.has(p));

    if (onlyInDir1.length > 0) {
      console.log(`\nFiles only in ${dir1}:`);
      onlyInDir1.forEach((file) => console.log(`  - ${file}`));
    }

    if (onlyInDir2.length > 0) {
      console.log(`\nFiles only in ${dir2}:`);
      onlyInDir2.forEach((file) => console.log(`  - ${file}`));
    }

    process.exit(1);
  }

  console.log(`✅ File counts match`);

  // Step 3: Check if the files have the same names
  console.log(`\nStep 3: Checking file names...`);
  const paths1 = files1.map((f) => f.relativePath).sort();
  const paths2 = files2.map((f) => f.relativePath).sort();

  const pathsMatch = paths1.every((path, index) => path === paths2[index]);

  if (!pathsMatch) {
    console.log(`❌ File names don't match`);

    const set1 = new Set(paths1);
    const set2 = new Set(paths2);

    const onlyInDir1 = paths1.filter((p) => !set2.has(p));
    const onlyInDir2 = paths2.filter((p) => !set1.has(p));

    if (onlyInDir1.length > 0) {
      console.log(`\nFiles only in ${dir1}:`);
      onlyInDir1.forEach((file) => console.log(`  - ${file}`));
    }

    if (onlyInDir2.length > 0) {
      console.log(`\nFiles only in ${dir2}:`);
      onlyInDir2.forEach((file) => console.log(`  - ${file}`));
    }

    process.exit(1);
  }

  console.log(`✅ File names match`);

  // Step 4 & 5: Check file contents and report differences
  console.log(`\nStep 4: Checking file contents...`);

  const fileMap1 = new Map(files1.map((f) => [f.relativePath, f]));
  const fileMap2 = new Map(files2.map((f) => [f.relativePath, f]));

  let differingFiles = 0;

  for (const relativePath of paths1) {
    const file1 = fileMap1.get(relativePath)!;
    const file2 = fileMap2.get(relativePath)!;

    // Quick size check first
    if (file1.size !== file2.size) {
      console.log(
        `❌ ${relativePath}: Size mismatch (${file1.size} vs ${file2.size} bytes)`,
      );
      differingFiles++;
      continue;
    }

    // Content comparison
    const diff = compareFileContents(file1.fullPath, file2.fullPath);
    if (diff !== null) {
      console.log(`❌ ${relativePath}: Content differs`);
      console.log(`Diff:`);
      console.log(diff);
      console.log(`${'='.repeat(80)}`);
      differingFiles++;
    }
  }

  if (differingFiles === 0) {
    console.log(`✅ All file contents match`);
    console.log(`\n🎉 Directories are identical!`);
  } else {
    console.log(`\n❌ Found ${differingFiles} differing file(s)`);
    process.exit(1);
  }
}

main();
