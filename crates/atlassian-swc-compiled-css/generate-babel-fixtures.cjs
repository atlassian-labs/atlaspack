#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

// Try multiple paths to find the dependencies
const possibleBabelPaths = [
	path.join(__dirname, '../../node_modules/@babel/core'),
	path.join(__dirname, '../../mercury/platform/node_modules/@babel/core'),
	'@babel/core',
];

const possibleCompiledPaths = [
	path.join(__dirname, '../../node_modules/@compiled/babel-plugin'),
	path.join(__dirname, '../../mercury/platform/node_modules/@compiled/babel-plugin'),
	'@compiled/babel-plugin',
];

let babel, compiledPlugin;

function tryRequire(paths, name) {
	for (const p of paths) {
		try {
			return require(p);
		} catch (e) {
			continue;
		}
	}
	throw new Error(`Could not find ${name} in any of the expected locations`);
}

try {
	babel = tryRequire(possibleBabelPaths, '@babel/core');
	compiledPlugin = tryRequire(possibleCompiledPaths, '@compiled/babel-plugin');
	console.log('✓ Found required dependencies');
} catch (e) {
	console.error('✗ Could not find required dependencies:', e.message);
	console.error('\nPlease install the dependencies:');
	console.error(
		'cd /Users/jlane2/atlassian/atlassian-frontend-monorepo && npm install @babel/core @compiled/babel-plugin',
	);
	process.exit(1);
}

const fixturesDir = path.join(__dirname, 'tests', 'fixtures');

function processFixture(fixturePath) {
	const fixtureName = path.basename(fixturePath);
	const inFile = path.join(fixturePath, 'in.jsx');
	const babelOutFile = path.join(fixturePath, 'babel.js');

	if (!fs.existsSync(inFile)) {
		console.log(`⚠ Skipping ${fixtureName} - no in.jsx file found`);
		return;
	}

	try {
		const input = fs.readFileSync(inFile, 'utf8');

		// Configure babel transformation
		const result = babel.transformSync(input, {
			plugins: [
				[
					compiledPlugin,
					{
						// Add any necessary plugin options here
					},
				],
			],
			parserOpts: {
				plugins: ['jsx', 'typescript'],
			},
			filename: inFile,
			babelrc: false,
			configFile: false,
			compact: false,
			retainLines: false,
		});

		if (result && result.code) {
			// Format the output to match the expected format
			let formattedCode = result.code;

			// Basic formatting to match fixture style
			formattedCode = formattedCode
				.split('\n')
				.map((line) => (line.trim() ? '\t' + line.trim() : line))
				.join('\n')
				.replace(/^\t/, '') // Remove leading tab from first line
				.trim();

			fs.writeFileSync(babelOutFile, formattedCode, 'utf8');
			console.log(`✓ Generated ${fixtureName}/babel.js`);
		} else {
			console.error(`✗ Failed to transform ${fixtureName}/in.jsx - no output generated`);
		}
	} catch (error) {
		console.error(`✗ Error processing ${fixtureName}/in.jsx:`);
		console.error(`   ${error.message}`);

		// Log more details for debugging
		if (error.code === 'BABEL_TRANSFORM_ERROR') {
			console.error('   Babel transform error - check plugin configuration');
		}
	}
}

function main() {
	if (!fs.existsSync(fixturesDir)) {
		console.error(`Fixtures directory not found: ${fixturesDir}`);
		process.exit(1);
	}

	const fixtures = fs
		.readdirSync(fixturesDir, { withFileTypes: true })
		.filter((dirent) => dirent.isDirectory())
		.map((dirent) => dirent.name)
		.sort();

	console.log(`Found ${fixtures.length} fixtures to process\n`);

	let successful = 0;
	let failed = 0;

	fixtures.forEach((fixture) => {
		const fixturePath = path.join(fixturesDir, fixture);
		try {
			processFixture(fixturePath);
			successful++;
		} catch (e) {
			failed++;
		}
	});

	console.log(`\nResults: ${successful} successful, ${failed} failed`);
	console.log(
		'Generated babel.js files can be compared with out.js files to see expected vs actual output.',
	);

	// Run Prettier to format all generated babel.js files in the fixtures
	const { execSync } = require('child_process');
	try {
		execSync(`yarn prettier -w ${__dirname}/tests/fixtures/*/babel.js`, {
			stdio: 'inherit',
		});
		console.log('Prettier formatting complete.');
	} catch (err) {
		console.error('Error running Prettier on generated babel.js files:', err.message);
	}
}

if (require.main === module) {
	main();
}
