import { defineConfig } from 'rolldown';

export default defineConfig({
  input: 'src/prelude.ts',
  platform: 'neutral',
  transform: {
    target: 'es2019'
  },
  output: {
    file: 'lib/prelude.js',
    format: 'iife',
    name: 'Atlaspack_ATLASPACK_PRELUDE_HASH',
  },
});
