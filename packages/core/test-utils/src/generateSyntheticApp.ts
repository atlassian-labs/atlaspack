import path from 'path';

import type {FileSystem} from '@atlaspack/types';

/**
 * Deterministic PRNG (mulberry32).
 * Returns a function that produces uniform [0,1) values.
 */
function mulberry32(a: number) {
  return function () {
    let t = (a += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * Generate a synthetic app fixture on the given filesystem.
 *
 * Creates a realistic dependency graph with entries, routes (lazy-loaded),
 * components, utilities, libs, and a CSS file. The graph proportions are
 * tuned to approximate a real large-scale app:
 * - ~3.7 deps/asset
 * - ~2.8% lazy deps
 * - Overlapping shared component subsets across routes
 * - Cycle back-edges (~2%)
 *
 * @param fs       The filesystem to write files to (e.g. overlayFS)
 * @param baseDir  The base directory for the fixture (e.g. __dirname)
 * @param targetAssetCount  Approximate number of assets to generate
 * @param seed     PRNG seed for deterministic output
 * @returns        The fixture directory name and entry file name
 */
export async function generateSyntheticApp(
  fs: FileSystem,
  baseDir: string,
  targetAssetCount: number,
  seed: number,
): Promise<{fixtureName: string; entryFile: string}> {
  const fixtureName = 'bundler-parity-generated-app';
  const entryFile = 'entry-0.js';

  const rand = mulberry32(seed);
  const randInt = (n: number) => Math.floor(rand() * n);

  const entryFiles = ['entry-0.js', 'entry-1.js'];
  const routeCount = 10;
  const componentCount = 100;
  const utilCount = 7;
  const cssFile = 'style.css';

  const baseAssetCount =
    entryFiles.length + routeCount + componentCount + utilCount + 1;
  const libCount = Math.max(0, targetAssetCount - baseAssetCount);

  const routes = Array.from({length: routeCount}, (_, i) => `route-${i}.js`);
  const components = Array.from(
    {length: componentCount},
    (_, i) => `component-${i}.js`,
  );
  const utils = Array.from({length: utilCount}, (_, i) => `util-${i}.js`);
  const libs = Array.from({length: libCount}, (_, i) => `lib-${i}.js`);

  // Dependency sets (sync vs lazy) for each JS file.
  const syncDeps: Record<string, Set<string>> = {};
  const lazyDeps: Record<string, Set<string>> = {};

  function ensure(name: string) {
    syncDeps[name] ??= new Set();
    lazyDeps[name] ??= new Set();
  }

  function addSync(from: string, to: string) {
    if (from === to) return;
    ensure(from);
    syncDeps[from].add(to);
  }

  function addLazy(from: string, to: string) {
    if (from === to) return;
    ensure(from);
    lazyDeps[from].add(to);
  }

  // Entries: include a second "entry" module and lazily load all routes.
  // (We only bundle from entry-0.js, so entry-0 wires the whole graph.)
  addSync('entry-0.js', 'entry-1.js');
  for (let r of routes) {
    addLazy('entry-0.js', r);
  }

  // Entry-1 also references a couple routes to keep it non-trivial.
  addLazy('entry-1.js', routes[0]);
  addLazy('entry-1.js', routes[1]);

  // Routes: import a shared core set + per-route random components and
  // a couple shared utils.
  const coreSharedComponents = components.slice(0, 8);
  const perRouteExtra = 6;
  const routeLazyComponentCount = 5; // ~5 lazy deps from routes

  for (let i = 0; i < routes.length; i++) {
    let route = routes[i];
    for (let c of coreSharedComponents) addSync(route, c);
    addSync(route, utils[i % utils.length]);
    addSync(route, utils[(i + 3) % utils.length]);

    let picks = new Set<string>();
    while (picks.size < perRouteExtra) {
      picks.add(components[8 + randInt(components.length - 8)]);
    }
    for (let c of picks) addSync(route, c);

    // A few routes have an additional lazy-loaded component (keeps lazy
    // deps ~2-3%).
    if (i < routeLazyComponentCount) {
      addLazy(route, components[50 + i]);
    }
  }

  // Utils: small shared chain.
  for (let i = 0; i < utils.length; i++) {
    ensure(utils[i]);
    if (i > 0) addSync(utils[i], utils[i - 1]);
  }

  // Components: realistic-ish fan-in/fan-out with overlap.
  // Aim for ~3.7 deps/asset overall by keeping components around
  // ~3.5-3.7 deps each.
  for (let i = 0; i < components.length; i++) {
    let comp = components[i];

    // Shared utilities.
    addSync(comp, utils[randInt(utils.length)]);
    addSync(comp, utils[randInt(utils.length)]);

    // Pull in a lib dependency for ~50% of components.
    if (libs.length > 0 && rand() < 0.5) {
      addSync(comp, libs[randInt(libs.length)]);
    }

    // Overlap: bias toward a shared subset plus some random spread.
    const other =
      rand() < 0.6
        ? components[randInt(30)]
        : components[randInt(components.length)];
    addSync(comp, other);

    // Small fraction pull in another component.
    if (rand() < 0.1) {
      addSync(comp, components[randInt(components.length)]);
    }
  }

  // One component imports CSS (type change).
  addSync(components[0], cssFile);

  // Back-edges / cycles (~2% of deps). Create several mutual component
  // imports.
  const cyclePairs = 6;
  for (let i = 0; i < cyclePairs; i++) {
    const a = components[10 + randInt(components.length - 10)];
    const b = components[10 + randInt(components.length - 10)];
    if (a !== b) {
      addSync(a, b);
      addSync(b, a);
    }
  }

  // Helpers to emit minimal, parseable JS.
  function jsFileContent(fileName: string): string {
    const s = Array.from(syncDeps[fileName] ?? []);
    const l = Array.from(lazyDeps[fileName] ?? []);

    let lines: Array<string> = [];

    // Sync imports.
    let importVars: string[] = [];
    for (let i = 0; i < s.length; i++) {
      let dep = s[i];
      if (dep.endsWith('.css')) {
        lines.push(`import './${dep}';`);
      } else {
        const v = `d${i}`;
        importVars.push(v);
        lines.push(`import ${v} from './${dep}';`);
      }
    }

    // Dynamic imports.
    for (let dep of l) {
      lines.push(`import('./${dep}');`);
    }

    // Export.
    if (importVars.length > 0) {
      lines.push(`export default ${importVars.join(' + ')} + '${fileName}';`);
    } else {
      lines.push(`export default '${fileName}';`);
    }

    return lines.join('\n');
  }

  // Write files directly to the filesystem.
  const fixtureDir = path.join(baseDir, fixtureName);
  await fs.mkdirp(fixtureDir);

  const allJsFiles = [
    ...entryFiles,
    ...routes,
    ...components,
    ...utils,
    ...libs,
  ];
  for (let f of allJsFiles) {
    await fs.writeFile(path.join(fixtureDir, f), jsFileContent(f));
  }

  await fs.writeFile(
    path.join(fixtureDir, cssFile),
    '.generated-app { color: red; }',
  );

  await fs.writeFile(
    path.join(fixtureDir, 'package.json'),
    JSON.stringify({
      '@atlaspack/bundler-default': {
        minBundles: 1,
        minBundleSize: 0,
        maxParallelRequests: 99999,
      },
    }),
  );

  await fs.writeFile(path.join(fixtureDir, 'yarn.lock'), '');

  return {fixtureName, entryFile};
}
