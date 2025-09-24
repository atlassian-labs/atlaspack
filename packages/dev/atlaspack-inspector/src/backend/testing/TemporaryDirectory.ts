import {existsSync, mkdirSync, rmSync} from 'fs';
import {randomUUID} from 'crypto';
import path from 'path';

afterAll(() => {
  const tmpDir = path.join(__dirname, 'tmp');
  if (existsSync(tmpDir)) {
    rmSync(tmpDir, {recursive: true, force: true});
  }
});

/**
 * Testing helper to create a temporary directory and remove it when
 * tests complete.
 *
 * @example
 * ```ts
 * {
 *   const tempDir = new TemporaryDirectory();
 *   console.log(tempDir.get());
 * }
 * // the directory will be automatically removed
 * // both by testing hooks and by custom dispose logic.
 * ```
 */
export class TemporaryDirectory {
  private readonly path: string;

  constructor() {
    const randomId = randomUUID();
    this.path = path.join(
      __dirname,
      'tmp',
      'atlaspack-inspector-tests',
      `${randomId}`,
    );
    mkdirSync(this.path, {recursive: true});
  }

  /**
   * The file-path to the temporary directory.
   */
  get(): string {
    return this.path;
  }

  [Symbol.dispose]() {
    rmSync(this.path, {recursive: true, force: true});
  }
}
