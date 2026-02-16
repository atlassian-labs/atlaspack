import { defineConfig, type RolldownOptions } from 'rolldown';

export function preludeConfig(mode: 'debug' | 'dev' | 'prod'): RolldownOptions {
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
  preludeConfig('debug'),
  preludeConfig('dev'),
  preludeConfig('prod'),
]);
