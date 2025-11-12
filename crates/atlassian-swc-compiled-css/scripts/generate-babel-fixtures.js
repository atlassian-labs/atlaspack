#!/usr/bin/env node
/* eslint-disable no-console */
const fs = require('fs');
const path = require('path');
const { transformSync } = require('@babel/core');
const jsxTransform = require('@babel/plugin-transform-react-jsx');

function loadPlugin(moduleId) {
  const plugin = require(moduleId);
  return plugin && plugin.default ? plugin.default : plugin;
}

const fixturesRoot = path.join(__dirname, '..', 'tests', 'fixtures');

function loadFixtureConfig(fixtureDir) {
  const configPath = path.join(fixtureDir, 'config.json');
  if (!fs.existsSync(configPath)) {
    return {
      pluginOptions: { extract: false },
      nodeEnv: undefined,
      babelEnv: undefined,
    };
  }

  const raw = fs.readFileSync(configPath, 'utf8');
  const parsed = JSON.parse(raw);
  const { nodeEnv, babelEnv, ...pluginOptions } = parsed;
  return {
    pluginOptions: {
      extract: parsed.extract ?? false,
      ...pluginOptions,
    },
    nodeEnv,
    babelEnv,
  };
}

function syntaxPluginsForExtension(ext) {
  switch (ext) {
    case '.ts':
    case '.cts':
      return ['typescript'];
    case '.tsx':
      return ['jsx', 'typescript'];
    default:
      return ['jsx'];
  }
}

function applyEnv(env, callback) {
  const prevNode = process.env.NODE_ENV;
  const prevBabel = process.env.BABEL_ENV;

  if (env.nodeEnv !== undefined) {
    process.env.NODE_ENV = env.nodeEnv;
  }
  if (env.babelEnv !== undefined) {
    process.env.BABEL_ENV = env.babelEnv;
  }

  try {
    return callback();
  } finally {
    if (prevNode === undefined) {
      delete process.env.NODE_ENV;
    } else {
      process.env.NODE_ENV = prevNode;
    }

    if (prevBabel === undefined) {
      delete process.env.BABEL_ENV;
    } else {
      process.env.BABEL_ENV = prevBabel;
    }
  }
}

function transformFixture(inputPath, source, pluginOptions) {
  const ext = path.extname(inputPath);
  const parserPlugins = syntaxPluginsForExtension(ext);
  const result = transformSync(source, {
    filename: inputPath,
    babelrc: false,
    configFile: false,
    parserOpts: { plugins: parserPlugins },
    sourceType: 'module',
    plugins: [[loadPlugin('@compiled/babel-plugin'), pluginOptions]],
    generatorOpts: {
      retainLines: false,
    },
  });

  if (!result || typeof result.code !== 'string') {
    throw new Error(`Failed to transform ${inputPath}`);
  }

  return result.code + '\n';
}

function transformForSwcOutput(inputPath, compiledCode) {
  const ext = path.extname(inputPath);
  const parserPlugins = syntaxPluginsForExtension(ext);
  const result = transformSync(compiledCode, {
    filename: inputPath,
    babelrc: false,
    configFile: false,
    parserOpts: { plugins: parserPlugins },
    plugins: [[jsxTransform, { runtime: 'automatic' }]],
    generatorOpts: {
      retainLines: false,
    },
  });

  if (!result || typeof result.code !== 'string') {
    throw new Error(`Failed to lower JSX for ${inputPath}`);
  }

  let code = result.code;
  // Drop the leading banner comment so the snapshot mirrors the SWC output.
  code = code.replace(/^\/\*.*?\*\/\n?/, '');
  // Normalize runtime helper imports and identifiers to match the Rust emitter.
  code = code.replace(/import \{([^}]*)\} from "react\/jsx-runtime";/g, (statement, specifiers) => {
    const cleaned = specifiers
      .split(',')
      .map((part) => part.trim())
      .map((part) => part.replace(/jsx as _jsx/, 'jsx').replace(/jsxs as _jsxs/, 'jsxs'))
      .join(', ');
    return `import { ${cleaned} } from "react/jsx-runtime";`;
  });
  code = code.replace(/_jsx/g, 'jsx').replace(/_jsxs/g, 'jsxs');
  code = code.replace(/\/\*#__PURE__\*\//g, '');
  return code.trimEnd() + '\n';
}

const fixtureName = process.argv[2];

fs.readdirSync(fixturesRoot, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .filter((entry) => !fixtureName || entry.name === fixtureName)
  .forEach((entry) => {
    const fixtureDir = path.join(fixturesRoot, entry.name);
    const { pluginOptions, nodeEnv, babelEnv } = loadFixtureConfig(fixtureDir);
    const inputCandidates = ['in.tsx', 'in.ts', 'in.jsx', 'in.js'];
    const inputPath = inputCandidates
      .map((candidate) => path.join(fixtureDir, candidate))
      .find((candidatePath) => fs.existsSync(candidatePath));

    if (!inputPath) {
      return;
    }

    const outputPath = path.join(fixtureDir, 'babel-out.js');
    const source = fs.readFileSync(inputPath, 'utf8');
    const compiledCode = applyEnv({ nodeEnv, babelEnv }, () =>
      transformFixture(inputPath, source, pluginOptions)
    );
    fs.writeFileSync(outputPath, compiledCode);
    console.log(`Updated ${path.relative(process.cwd(), outputPath)}`);

    const swcOutputPath = path.join(fixtureDir, 'out.js');
    const swcCode = applyEnv({ nodeEnv, babelEnv }, () =>
      transformForSwcOutput(inputPath, compiledCode)
    );
    fs.writeFileSync(swcOutputPath, swcCode);
    console.log(`Updated ${path.relative(process.cwd(), swcOutputPath)}`);
  });
