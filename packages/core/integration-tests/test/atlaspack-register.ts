import {execSync} from 'child_process';
import assert from 'assert';
import path from 'path';

describe.skip('@atlaspack/register', () => {
  it('can be required at an entry script and transform following requires', () => {
    assert.equal(
      execSync(
        `node ${path.join(
          __dirname,
          'integration',
          'atlaspack-register',
          'entry.js',
        )}`,
      ).toString(),
      '123',
    );
  });

  it('can transform with --r and --require', () => {
    assert.equal(
      execSync(
        `node -r @atlaspack/register ${path.join(
          __dirname,
          'integration',
          'atlaspack-register',
          'index.js',
        )}`,
      ).toString(),
      '123',
    );
  });

  it("enables Atlaspacks's resolver in node", () => {
    const [foo, resolved] = execSync(
      `node -r @atlaspack/register ${path.join(
        __dirname,
        'integration',
        'atlaspack-register',
        'resolver.js',
      )}`,
      {cwd: path.join(__dirname, 'integration', 'atlaspack-register')},
    )
      .toString()
      .split('\n');
    assert.equal(foo, 'foo');
    assert.equal(
      resolved,
      path.join(__dirname, 'integration', 'atlaspack-register', 'foo.js'),
    );
  });

  it('can be disposed of, which reverts resolving', () => {
    try {
      execSync(
        `node ${path.join(
          __dirname,
          'integration',
          'atlaspack-register',
          'dispose-resolve.js',
        )}`,
        {
          cwd: path.join(__dirname, 'integration', 'atlaspack-register'),
          stdio: 'pipe',
        },
      )
        .toString()
        .split('\n');
    } catch (e: unknown) {
      const error = e as {stdout: {toString: () => string}; stderr: string};
      assert.equal(
        error.stdout.toString().trim(),
        path.join(__dirname, 'integration', 'atlaspack-register', 'foo.js'),
      );
      assert(error.stderr.includes("Error: Cannot find module '~foo.js'"));
      return;
    }

    assert.fail();
  });

  it('can be disposed of, which reverts transforming', () => {
    try {
      execSync(
        `node ${path.join(
          __dirname,
          'integration',
          'atlaspack-register',
          'dispose-transform.js',
        )}`,
        {
          cwd: path.join(__dirname, 'integration', 'atlaspack-register'),
          stdio: 'pipe',
        },
      )
        .toString()
        .split('\n');
    } catch (e: unknown) {
      const error = e as {stdout: {toString: () => string}; stderr: string};
      assert.equal(error.stdout.toString().trim(), '123');
      assert(error.stderr.includes('SyntaxError: Unexpected identifier'));
      return;
    }

    assert.fail();
  });
});
