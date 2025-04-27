/* eslint-disable no-console */
import * as path from 'node:path';
import * as process from 'node:process';
import * as child_process from 'node:child_process';
import * as url from 'node:url';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);

for (const foundRel of glob.sync("packages/**/*.node", { cwd: __root, ignore: '**/node_modules/**'  })) {
  const found = path.join(__root, foundRel)
  if (process.platform === 'linux') {
    console.log(`Stripping:     ${found}`);
    cmd(
      `objcopy --only-keep-debug --compress-debug-sections=zlib ${found} ${found}.debug`,
    );
    cmd(`objcopy --strip-debug --strip-unneeded ${found}`);
    cmd(`objcopy --add-gnu-debuglink=${found}.debug ${found}`);
    console.log(`  ➜ Generated: ${found}.debug`);
  }
  if (process.platform === 'darwin') {
    cmd(`dsymutil ${found}`);
    cmd(`strip -x ${found}`);
  }
}

function cmd(command, options) {
  try {
    const [arg0, ...args] = command
      .split(' ')
      .filter((v) => v !== '')
      .map((v) => v.trim());
    child_process.execFileSync(arg0, args, {
      stdio: 'inherit',
      shell: true,
      ...options,
    });
  } catch (error) {
    console.error(`Failed: ${command}`);
    console.error(error);
    process.exit(1);
  }
}
