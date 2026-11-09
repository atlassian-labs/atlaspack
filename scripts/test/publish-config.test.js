const assert = require('assert');
const fs = require('fs');
const path = require('path');
const glob = require('glob');

const root = path.resolve(__dirname, '../..');
const registry = 'https://packages.atlassian.com/api/npm/npm-public/';
const read = (file) => fs.readFileSync(path.join(root, file), 'utf8');

describe('package publishing configuration', () => {
  const {workspaces} = JSON.parse(read('package.json'));
  const manifests = new Set(
    workspaces.flatMap((workspace) =>
      glob.sync(`${workspace}/package.json`, {cwd: root}),
    ),
  );

  for (const manifest of manifests) {
    const pkg = JSON.parse(read(manifest));
    if (pkg.private) continue;

    it(`${pkg.name} publishes publicly through the forwarding registry`, () => {
      assert.strictEqual(pkg.publishConfig?.access, 'public', manifest);
      assert.strictEqual(pkg.publishConfig?.registry, registry, manifest);
    });
  }

  const workflow = read('.github/workflows/release.yml');

  it('does not require a stored npm token', () => {
    assert.ok(!workflow.includes('NPM_TOKEN'));
    assert.ok(!workflow.includes('registry.npmjs.org'));
    assert.ok(workflow.includes('id-token: write'));
  });

  for (const job of ['build-and-release', 'build-and-release-changesets']) {
    it(`${job} uses the generated npm credentials when publishing`, () => {
      const start = workflow.indexOf(`\n  ${job}:\n`);
      assert.notStrictEqual(start, -1);
      const rest = workflow.slice(start + 1);
      const nextJob = rest.search(/\n {2}[\w-]+:\n/);
      const config = nextJob === -1 ? rest : rest.slice(0, nextJob);

      assert.ok(
        config.includes('uses: atlassian-labs/artifact-publish-token@v1.0.1'),
      );
      assert.ok(config.includes('output-modes: npm'));
      assert.ok(config.includes(`@atlaspack:registry=${registry}`));
      assert.ok(
        config.includes(
          'NPM_CONFIG_USERCONFIG: ${{ github.workspace }}/.npmrc-public',
        ),
      );
    });
  }
});
