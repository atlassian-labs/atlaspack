import assert from 'assert';
import path from 'path';
import {
  bundle,
  describe,
  it,
  fsFixture,
  overlayFS,
} from '@atlaspack/test-utils';

describe('tesseract-resolver', function () {
  it('should not fail when resolving protocol-relative URLs in CSS', async function () {
    // Ensures protocol-relative URLs (//example.com) in CSS are not incorrectly treated as absolute file paths by TesseractResolver.
    await fsFixture(overlayFS, __dirname)`
      resolver-protocol-relative-url
        package.json:
          {
            "name": "resolver-protocol-relative-url",
            "version": "1.0.0"
          }

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "resolvers": ["@atlaspack/resolver-tesseract"]
          }

        index.html:
          <!DOCTYPE html>
          <html>
            <head>
              <style>
                @font-face {
                  font-family: 'Test';
                  src: url('//example.com/font.woff2') format('woff2');
                }
              </style>
            </head>
            <body>
              <h1>Test</h1>
            </body>
          </html>
    `;

    let b = await bundle(
      path.join(__dirname, 'resolver-protocol-relative-url/index.html'),
      {
        inputFS: overlayFS,
      },
    );

    // The build should succeed without throwing an error
    assert(b !== null);
  });
});
