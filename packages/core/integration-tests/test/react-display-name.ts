import {bundle, fsFixture, overlayFS} from '@atlaspack/test-utils';
import assert from 'assert';

describe('react-display-name', () => {
  it('adds display name when addReactDisplayName is true', async function () {
    await fsFixture(overlayFS)`
    yarn.lock:

    package.json:
        {
            "@atlaspack/transformer-js": {
                "addReactDisplayName": true
            }
        }

    index.jsx:

        export default function Foo() {
          return <div>Foo</div>;
        }

        export const Bar = () => {
          return <div>Bar</div>;
        }

        function helper() {
          return 2 + 2;
        }
    `;

    const result = await bundle('./index.jsx', {
      inputFS: overlayFS,
      outputFS: overlayFS,
    });
    const bundles = result.getBundles();
    assert.equal(bundles.length, 1);
    const b = bundles.find((b) => b.filePath.includes('index'));
    let asset = null;

    b.traverseAssets((a) => {
      if (a.filePath.includes('index.jsx')) {
        asset = a;
      }
    });

    assert(asset != null);
    const code = await asset.getCode();
    assert(code.includes('displayName = "Foo"'));
    assert(code.includes('displayName = "Bar"'));
    assert(!code.includes('displayName = "helper"'));
  });

  describe('if the setting is off', () => {
    async function testDoesNotAddDisplayName() {
      const result = await bundle('./index.jsx', {
        inputFS: overlayFS,
        outputFS: overlayFS,
      });
      const bundles = result.getBundles();
      assert.equal(bundles.length, 1);
      const b = bundles.find((b) => b.filePath.includes('index'));
      let asset = null;

      b.traverseAssets((a) => {
        if (a.filePath.includes('index.jsx')) {
          asset = a;
        }
      });

      assert(asset != null);
      const code = await asset.getCode();
      assert(!code.includes('displayName = "Foo"'));
      assert(!code.includes('displayName = "Bar"'));
      assert(!code.includes('displayName = "helper"'));
    }

    it('does not add display name when addReactDisplayName is false', async function () {
      await fsFixture(overlayFS)`
    yarn.lock:

    package.json:
        {
            "@atlaspack/transformer-js": {
                "addReactDisplayName": false
            }
        }

    index.jsx:

        export default function Foo() {
          return <div>Foo</div>;
        }

        export const Bar = () => {
          return <div>Bar</div>;
        }

        function helper() {
          return 2 + 2;
        }
    `;

      await testDoesNotAddDisplayName();
    });

    it('does not add display name when addReactDisplayName is not set', async function () {
      await fsFixture(overlayFS)`
    yarn.lock:

    package.json:
        {
            "@atlaspack/transformer-js": {}
        }

    index.jsx:

        export default function Foo() {
          return <div>Foo</div>;
        }

        export const Bar = () => {
          return <div>Bar</div>;
        }

        function helper() {
          return 2 + 2;
        }
    `;

      await testDoesNotAddDisplayName();
    });
  });
});
