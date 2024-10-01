import * as path from 'path';
import * as Mocha from 'mocha';
import * as glob from 'glob';

export function run(): Promise<void> {
  // Create the mocha test
  // @ts-expect-error - TS2351 - This expression is not constructable.
  const mocha = new Mocha({
    ui: 'tdd',
    color: true,
  });

  const testsRoot = path.resolve(__dirname, '..');

  return new Promise((c, e) => {
    // @ts-expect-error - TS2349 - This expression is not callable. | TS7006 - Parameter 'err' implicitly has an 'any' type. | TS7006 - Parameter 'files' implicitly has an 'any' type.
    glob('**/**.test.js', {cwd: testsRoot}, (err, files) => {
      if (err) {
        return e(err);
      }

      // Add files to the test suite
      // @ts-expect-error - TS7006 - Parameter 'f' implicitly has an 'any' type.
      files.forEach((f) => mocha.addFile(path.resolve(testsRoot, f)));

      try {
        // Run the mocha test
        // @ts-expect-error - TS7006 - Parameter 'failures' implicitly has an 'any' type.
        mocha.run((failures) => {
          if (failures > 0) {
            e(new Error(`${failures} tests failed.`));
          } else {
            c();
          }
        });
      } catch (err) {
        console.error(err);
        e(err);
      }
    });
  });
}
