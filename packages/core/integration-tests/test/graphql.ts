import assert from 'assert';
import path from 'path';
import {
  bundle,
  describe,
  isAtlaspackV3,
  it,
  run,
  sleep,
} from '@atlaspack/test-utils';
import {parse, print} from 'graphql/language';

describe('graphql', function () {
  // This is very very shit, but prevents an issue where ThreadSafeFunctions that are kicked off but not complete when
  // the build ends result in a fatal NAPI error which hangs the build. It seems to mostly affect this particular suite for
  // some reason, but we have not found the root cause yet.
  //
  // This "only" adds ~2s to the integration test suite.
  afterEach(async () => {
    if (isAtlaspackV3) {
      await sleep(500);
    }
  });

  it('should support requiring graphql files', async function () {
    let b = await bundle(path.join(__dirname, '/integration/graphql/index.js'));

    let output = await run(b);
    assert.equal(typeof output, 'function');
    assert.deepEqual(
      normalize(output()),
      normalize(`
        {
          user(id: 5) {
            ...UserFragment
          }
        }

        fragment UserFragment on User {
          firstName
          lastName
        }
      `),
    );
  });

  it('should support importing other graphql files from a graphql file', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/graphql-import/index.js'),
    );

    let output = await run(b);
    assert.equal(typeof output, 'function');
    assert.deepEqual(
      normalize(output()),
      normalize(`
        {
          user(id: 6) {
            ...UserFragment
            ...AnotherUserFragment
          }
        }

        fragment UserFragment on User {
          firstName
          lastName
        }

        fragment AnotherUserFragment on User {
          address
          email
        }
      `),
    );
  });

  it('should support importing fragments in other graphql files by name', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/graphql-named-import/index.js'),
    );

    let output = await run(b);
    assert.equal(typeof output, 'function');
    assert.deepEqual(
      normalize(output()),
      normalize(`
      query MyQuery {
        user(id: 6) {
          ...UserFragment
          address {
            ...Address
          }
        }
      }

      fragment UserFragment on User {
        firstName
        lastName
        ...AnotherUserFragment
      }

      fragment Address on Address {
        line1
        county
        postalCode
      }

      fragment AnotherUserFragment on User {
        address
        email
      }

      fragment otherUserFragment on User {
        friends {
          edges {
            nodes {
              name
            }
          }
        }
      }
      `),
    );
  });
});

function normalize(body) {
  return print(parse(body));
}
