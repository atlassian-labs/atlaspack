import { defineConfig, type RolldownOptions } from 'rolldown';

function preludeConfig(mode: 'dev' | 'prod'): RolldownOptions {
  return {
    input: 'src/prelude.ts',
    platform: 'neutral',
    transform: {
      target: 'es2019',
      define: {
        MODE: JSON.stringify(mode),
      }
    },
    output: {
      file: `lib/prelude.${mode}.js`,
      format: 'iife',
      minify: mode === 'prod',

    },
  };
}

export default defineConfig([
  preludeConfig('dev'),
  preludeConfig('prod'),
]);
