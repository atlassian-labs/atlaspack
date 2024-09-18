import assert from 'assert';
import {join} from 'path';
import {
  bundle,
  describe,
  fsFixture,
  isAtlaspackV3,
  it,
  overlayFS,
  removeDistDirectory,
  run,
} from '@atlaspack/test-utils';

describe('data-url:', function () {
  beforeEach(async () => {
    await removeDistDirectory();
  });

  it('inlines text content as a data url', async () => {
    await fsFixture(overlayFS, __dirname)`
      index.js:
        import svg from 'data-url:./img.svg';
        export default svg;

      img.svg:
        <svg width="120" height='120' xmlns="http://www.w3.org/2000/svg">
          <filter id="blur-_.!~*">
            <feGaussianBlur stdDeviation="5"/>
          </filter>
          <circle cx="60" cy="60" r="50" fill="green" filter="url(#blur-_.!~*)" />
        </svg>
    `;

    let b = await bundle(join(__dirname, 'index.js'), {
      inputFS: overlayFS,
    });

    let svg = (await run(b)).default;

    if (isAtlaspackV3) {
      // The value is the same when rendered and matches the input exactly
      assert.equal(
        svg,
        'data:image/svg+xml,%3Csvg%20width%3D%22120%22%20height%3D%22120%22%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%3E%0A%20%20%3Cfilter%20id%3D%22blur-_.%21~%2a%22%3E%0A%20%20%20%20%3CfeGaussianBlur%20stdDeviation%3D%225%22%3E%3C%2FfeGaussianBlur%3E%0A%20%20%3C%2Ffilter%3E%0A%20%20%3Ccircle%20cx%3D%2260%22%20cy%3D%2260%22%20r%3D%2250%22%20fill%3D%22green%22%20filter%3D%22url%28%23blur-_.%21~%2a%29%22%3E%3C%2Fcircle%3E%0A%3C%2Fsvg%3E',
      );
    } else {
      // The output has extra quotes inside the url filter
      assert.equal(
        svg,
        'data:image/svg+xml,%3Csvg%20width%3D%22120%22%20height%3D%22120%22%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%3E%0A%20%20%3Cfilter%20id%3D%22blur-_.%21~%2a%22%3E%0A%20%20%20%20%3CfeGaussianBlur%20stdDeviation%3D%225%22%3E%3C%2FfeGaussianBlur%3E%0A%20%20%3C%2Ffilter%3E%0A%20%20%3Ccircle%20cx%3D%2260%22%20cy%3D%2260%22%20r%3D%2250%22%20fill%3D%22green%22%20filter%3D%22url%28%27%23blur-_.%21~%2a%27%29%22%3E%3C%2Fcircle%3E%0A%3C%2Fsvg%3E',
      );
    }
  });

  it('inlines binary content as a data url', async () => {
    let b = await bundle(join(__dirname, '../data/integration/data-url/binary.js'));
    let binary = (await run(b)).default;

    assert(binary.startsWith('data:image/webp;base64,UklGR'));
  });
});
