const {spawn} = require('child_process');
const http = require('http');
const fs = require('fs');
const path = require('path');

const TESSERACT_PORT = 8080;
const TESSERACT_HOST = 'localhost';
const READY_MESSAGE = 'Ready (script loaded)';

/**
 * Strip ANSI escape codes and clean up string for matching
 */
function cleanString(str) {
  // Remove ANSI escape codes (more comprehensive pattern)
  // eslint-disable-next-line no-control-regex
  return (
    str
      .replace(/\u001b\[[0-9;]*m/g, '')
      // Remove other control characters but keep newlines
      .replace(/[\x00-\x09\x0B-\x1F\x7F]/g, '')
  );
}

/**
 * Finds the SSR bundle in the dist directory
 */
function findBundle() {
  const distDir = path.join(__dirname, 'dist');

  if (!fs.existsSync(distDir)) {
    console.error('Error: dist directory not found. Did you run build first?');
    process.exitCode = 1;
    return null;
  }

  const files = fs.readdirSync(distDir);
  // Prefer bundles that start with 'ssr-app' or 'index'
  const bundleFile =
    files.find(
      (file) =>
        file.startsWith('ssr-app') &&
        file.endsWith('.js') &&
        !file.endsWith('.snapshot'),
    ) ||
    files.find(
      (file) =>
        file.startsWith('index') &&
        file.endsWith('.js') &&
        !file.endsWith('.snapshot'),
    ) ||
    files.find((file) => file.endsWith('.js') && !file.endsWith('.snapshot'));

  if (!bundleFile) {
    console.error('Error: No JavaScript bundle found in dist directory');
    process.exitCode = 1;
    return null;
  }

  return path.join(distDir, bundleFile);
}

/**
 * Spawns Tesseract server and waits for it to be ready
 */
function startTesseract(bundlePath) {
  return new Promise((resolve, reject) => {
    console.log(`Starting Tesseract with bundle: ${bundlePath}`);

    const tesseract = spawn(
      'atlas',
      ['tesseract', 'run', '--use-snapvm', bundlePath],
      {
        stdio: ['ignore', 'pipe', 'pipe'],
      },
    );

    let isReady = false;
    let outputBuffer = '';

    // Handle stdout
    tesseract.stdout.on('data', (data) => {
      const output = data.toString();
      process.stdout.write(output);
      outputBuffer += output;

      // Check if server is ready - clean string and check
      const cleanOutput = cleanString(output);
      if (!isReady && cleanOutput.includes(READY_MESSAGE)) {
        isReady = true;
        console.log('\n✓ Tesseract server is ready');
        resolve(tesseract);
      }
    });

    // Handle stderr
    tesseract.stderr.on('data', (data) => {
      const output = data.toString();
      process.stderr.write(output);
      outputBuffer += output;

      // Also check stderr for ready message
      const cleanOutput = cleanString(output);
      if (!isReady && cleanOutput.includes(READY_MESSAGE)) {
        isReady = true;
        console.log('\n✓ Tesseract server is ready');
        resolve(tesseract);
      }
    });

    // Handle process exit
    tesseract.on('exit', (code, signal) => {
      if (!isReady) {
        reject(
          new Error(
            `Tesseract exited before becoming ready (code: ${code}, signal: ${signal})`,
          ),
        );
      }
    });

    // Handle spawn errors
    tesseract.on('error', (err) => {
      reject(new Error(`Failed to start Tesseract: ${err.message}`));
    });

    // Timeout after 30 seconds
    setTimeout(() => {
      if (!isReady) {
        tesseract.kill();
        reject(new Error('Timeout waiting for Tesseract to become ready'));
      }
    }, 30000);
  });
}

/**
 * Sends a POST request to the render endpoint
 */
function testRenderEndpoint() {
  return new Promise((resolve, reject) => {
    const postData = JSON.stringify({
      input: {},
    });

    const options = {
      hostname: TESSERACT_HOST,
      port: TESSERACT_PORT,
      path: '/render',
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(postData),
      },
    };

    console.log(
      `\nSending POST request to http://${TESSERACT_HOST}:${TESSERACT_PORT}/render`,
    );
    console.log(`Request body: ${postData}`);

    const req = http.request(options, (res) => {
      console.log(`Received response with status: ${res.statusCode}`);
      let body = '';

      res.on('data', (chunk) => {
        body += chunk.toString();
        console.log(`Received ${chunk.length} bytes of data...`);
      });

      res.on('end', () => {
        console.log('Response complete.');
        if (res.statusCode === 200) {
          console.log('✓ Received response (status: 200)');
          console.log(`Response length: ${body.length} bytes`);

          // Try to parse as JSON first
          try {
            const parsed = JSON.parse(body);
            console.log('✓ Response is valid JSON');

            // Check if it contains HTML (might be in a property)
            if (parsed.html || parsed.output || typeof parsed === 'string') {
              console.log('✓ Response contains rendered content');
              resolve(true);
            } else {
              console.log(
                'Response structure:',
                JSON.stringify(parsed, null, 2).substring(0, 300),
              );
              console.log('✓ Response received successfully');
              resolve(true);
            }
          } catch (e) {
            // Not JSON, check if it's HTML directly
            if (body.trim().startsWith('<') || body.includes('html')) {
              console.log('✓ Response appears to be HTML');
              resolve(true);
            } else {
              console.error('✗ Response is neither JSON nor HTML');
              console.error('Response:', body.substring(0, 200));
              reject(new Error('Invalid response format'));
            }
          }
        } else {
          console.error(
            `✗ Received error response (status: ${res.statusCode})`,
          );
          console.error('Response:', body);
          reject(new Error(`HTTP ${res.statusCode}: ${body}`));
        }
      });
    });

    req.on('error', (err) => {
      console.error('✗ Request failed:', err.message);
      reject(err);
    });

    // Add timeout to the request
    req.setTimeout(10000, () => {
      req.destroy();
      console.error('✗ Request timed out after 10 seconds');
      reject(new Error('Request timeout'));
    });

    req.write(postData);
    req.end();
    console.log('Request sent, waiting for response...');
  });
}

/**
 * Main validation function
 */
async function validate() {
  let tesseractProcess = null;

  try {
    // Find the bundle
    const bundlePath = findBundle();
    if (!bundlePath) {
      return;
    }
    console.log(`Found bundle: ${bundlePath}\n`);

    // Start Tesseract
    tesseractProcess = await startTesseract(bundlePath);

    // Wait a bit for the server to fully initialize
    await new Promise((resolve) => setTimeout(resolve, 1000));

    // Test the render endpoint
    await testRenderEndpoint();

    console.log('\n✓ Validation successful!');
    process.exitCode = 0;
  } catch (err) {
    console.error('\n✗ Validation failed:', err.message);
    process.exitCode = 1;
  } finally {
    // Clean up: kill Tesseract process
    if (tesseractProcess) {
      console.log('\nCleaning up Tesseract process...');
      tesseractProcess.kill('SIGTERM');

      // Give it a moment to terminate gracefully
      await new Promise((resolve) => setTimeout(resolve, 1000));

      // Force kill if still running
      if (!tesseractProcess.killed) {
        tesseractProcess.kill('SIGKILL');
      }
    }

    // Force exit after cleanup
    process.exit(process.exitCode || 0);
  }
}

// Run validation
validate();
