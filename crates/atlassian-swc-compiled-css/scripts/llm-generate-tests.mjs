import rootPath from '@af/root-path';
import { $, cd } from 'zx';
import { join } from 'node:path';
import { existsSync } from 'node:fs';

const PROMPT = `
We are building a Compiled CSS-in-JS swc plugin to replace the babel plugin '@compiled/babel-plugin'  
  
We need to develop as many fixtures as possible for consumption by another LLM that are mostly unique and only create them for things that cause errors.

If the build has a conflicting port issue, you can run 'pkill -f 'pillar|atlaspack|parcel' to kill the conflicting processes and restart the build. Only run this if there is the conflicting port error as it takes a long time.

Make sure to import { token } from '@atlaskit/tokens' for any fixtures that use tokens.

If the yarn ssr:start logs the message '[Pillar] 🎉 All servers started successfully!' and there are no errors, create a file called 'fixtures-done.md' in platform/crates/compiled/crates/compiled_swc_plugin.

To find fixtures to create do the following continuously:

- In 'jira/' run 'yarn ssr:start' and observe any errors associated with Compiled. You may need to wait for the build to complete which can take a few minutes. Go to the relevant from the error (platform/packages/design-system/lozenge/src/lozenge.tsx is a good example but it already has a fixture).
- You can create a fixture by creating a new file at 'platform/crates/compiled/crates/compiled_swc_plugin/tests/fixtures/FIXTURE_NAME/in.jsx' where FIXTURE_NAME is a name for the fixture
- Remove any imports that are not required. If any imports are required, create new files in 'platform/crates/compiled/crates/compiled_swc_plugin/tests/fixtures/FIXTURE_NAME/' that are imported from 'in.jsx'
- Run 'node platform/crates/compiled/crates/compiled_swc_plugin/scripts/build-fixture.mjs FIXTURE_NAME' . If the babel fixture causes an error change 'in.jsx' so it can be built successfully. If the swc plugin causes an error, leave it as that's the behaviour we want
- When the babel build works correctly, then adjust the relevant file that originally failed by deleting the problematic code. This should mean we can run yarn ssr:start again to find a new fixture to generate
`;

async function main() {
  cd(rootPath.afmRoot());
  while (true) {
    if (
      existsSync(
        join(
          rootPath.afmRoot(),
          'platform/crates/compiled/crates/compiled_swc_plugin',
          'fixtures-done.md'
        )
      )
    ) {
      console.log('Detected done build marker file');
      break;
    }
    await $`acli rovodev run --yolo "${PROMPT}"`.nothrow();
  }
}

main();
