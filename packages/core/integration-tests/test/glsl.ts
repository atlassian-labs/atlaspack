import assert from 'assert';
import path from 'path';
import fs from 'fs';
import {
  bundle,
  describe,
  it,
  run,
  normaliseNewlines,
} from '@atlaspack/test-utils';

describe('glsl', function () {
  it('should support requiring GLSL files via glslify', async function () {
    const b = await bundle(path.join(__dirname, '/integration/glsl/index.js'));

    const shader = fs.readFileSync(
      path.join(__dirname, '/integration/glsl/compiled.glsl'),
      'utf8',
    );

    const output = await run(b);

    assert.equal(typeof output, 'function');
    assert.ok(
      output().reduce((acc: boolean, requiredShader: string) => {
        return (
          acc && normaliseNewlines(shader) === normaliseNewlines(requiredShader)
        );
      }, true),
    );
  });

  it.v2('should correctly resolve relative GLSL imports', async function () {
    const b = await bundle(
      path.join(__dirname, '/integration/glsl-relative-import/index.js'),
    );

    const output = (await run(b)).trim();

    assert.strictEqual(
      output,
      `
#define GLSLIFY 1
float b(float p) { return p*2.0; }

float c(float p) { return b(p)*3.0; }

varying float x;

void main() { gl_FragColor = vec4(c(x)); }
      `.trim(),
    );
  });
});
