/* eslint-disable no-console */
import {execSync, execFileSync} from 'node:child_process';
import {existsSync} from 'node:fs';
import {mkdir, writeFile, rm} from 'node:fs/promises';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const __root = dirname(__dirname);

export interface ThreeJsSetupOptions {
  branch: string;
  repoUrl: string;
  copies: number;
}

export async function setupThreeJsProject(options: ThreeJsSetupOptions) {
  const {branch, repoUrl, copies} = options;

  console.log(`Setting up Three.js benchmark project...`);
  console.log(`  Branch: ${branch}`);
  console.log(`  Copies: ${copies}`);

  const outputDir = join(__root, 'test', 'data', 'three-js-project');
  // Clean up existing benchmark project directory
  if (existsSync(outputDir)) {
    await rm(outputDir, {recursive: true, force: true});
  }

  await mkdir(outputDir, {recursive: true});

  // Use cached three.js source if available
  const cacheDir = join(__root, '.three-js-cache');
  const cachedThreeJsDir = join(cacheDir, `three-js-${branch}`);
  const threeJsDir = join(outputDir, 'three-js-source');

  if (existsSync(cachedThreeJsDir)) {
    console.log(`Using cached three.js source from ${cachedThreeJsDir}...`);
    execSync(`cp -r "${cachedThreeJsDir}" "${threeJsDir}"`, {stdio: 'pipe'});
  } else {
    console.log(`Cloning three.js from ${repoUrl}...`);

    try {
      // Ensure cache directory exists
      await mkdir(cacheDir, {recursive: true});

      // Clone to cache first
      execFileSync(
        'git',
        ['clone', '--depth=1', `--branch=${branch}`, repoUrl, cachedThreeJsDir],
        {
          stdio: 'pipe',
          cwd: cacheDir,
        },
      );

      // Copy from cache to working directory
      execSync(`cp -r "${cachedThreeJsDir}" "${threeJsDir}"`, {stdio: 'pipe'});
    } catch (error) {
      throw new Error(`Failed to clone three.js repository: ${error}`);
    }
  }

  // Create the benchmark entry point
  await createBenchmarkProject(outputDir, threeJsDir, copies);

  console.log(`✅ Three.js benchmark project created successfully`);
  return outputDir;
}

async function createBenchmarkProject(
  outputDir: string,
  threeJsDir: string,
  copies: number,
) {
  // Create package.json
  const packageJson = {
    name: 'three-js-benchmark-project',
    private: true,
    version: '1.0.0',
    type: 'module',
    dependencies: {},
  };

  await writeFile(
    join(outputDir, 'package.json'),
    JSON.stringify(packageJson, null, 2),
  );

  // Create .parcelrc
  const parcelRc = {
    extends: '@atlaspack/config-default',
  };

  await writeFile(
    join(outputDir, '.parcelrc'),
    JSON.stringify(parcelRc, null, 2),
  );

  // Create index.html
  const indexHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Three.js Benchmark</title>
</head>
<body>
    <div data-testid="content" id="output">Loading Three.js benchmark...</div>
    <script type="module" src="./src/index.js"></script>
</body>
</html>`;

  await writeFile(join(outputDir, 'index.html'), indexHtml);

  // Create src directory
  const srcDir = join(outputDir, 'src');
  await mkdir(srcDir, {recursive: true});

  // Copy three.js source files multiple times for bundling stress test
  const threeJsSrcDir = join(threeJsDir, 'src');
  const imports: string[] = [];
  const code: string[] = [];

  for (let i = 0; i < copies; i++) {
    const copyDir = join(srcDir, `three-js-copy-${i}`);

    // Copy the three.js src directory
    execSync(`cp -r "${threeJsSrcDir}" "${copyDir}"`, {stdio: 'pipe'});

    imports.push(
      `import * as THREE_${i} from './three-js-copy-${i}/Three.js';`,
    );
    code.push(`globalThis['THREE_COPY_${i}'] = THREE_${i};`);
    code.push(
      `console.log('Three.js copy ${i} loaded:', THREE_${i}.REVISION);`,
    );
  }

  // Create main index.js that imports all copies
  const mainIndexJs = `// Three.js Benchmark - Real three.js repository stress test
// This creates multiple copies of the three.js library to test bundling performance

${imports.join('\n')}

// Initialize all copies
${code.join('\n')}

// Benchmark initialization
function initThreeJsBenchmark() {
  console.log('Three.js benchmark initialized with ${copies} copies');

  const output = document.getElementById('output');
  if (output) {
    output.textContent = \`Three.js benchmark loaded successfully with \${${copies}} library copies\`;
  }

  // Create some basic three.js objects to ensure the code is used
  try {
    const scene = new THREE_0.Scene();
    const camera = new THREE_0.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
    const renderer = new THREE_0.WebGLRenderer();

    console.log('Basic Three.js objects created successfully');
    console.log('Scene:', scene);
    console.log('Camera:', camera);
    console.log('Renderer:', renderer);

    return {
      scene,
      camera,
      renderer,
      copiesLoaded: ${copies}
    };
  } catch (error) {
    console.error('Error creating Three.js objects:', error);
    return { error: error.message };
  }
}

// Auto-initialize when DOM is ready
if (typeof window !== 'undefined') {
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initThreeJsBenchmark);
  } else {
    initThreeJsBenchmark();
  }
}

export { initThreeJsBenchmark };
`;

  await writeFile(join(srcDir, 'index.js'), mainIndexJs);
}

export async function cleanupThreeJsProject(outputDir?: string) {
  const targetDir =
    outputDir ?? join(__root, 'test', 'data', 'three-js-project');

  if (existsSync(targetDir)) {
    await rm(targetDir, {recursive: true, force: true});
    console.log(`Cleaned up Three.js project at ${targetDir}`);
  }
}
