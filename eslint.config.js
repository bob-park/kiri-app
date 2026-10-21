import eslintConfig from '@bob-park/eslint-config-bobpark';

import { defineConfig } from 'eslint/config';

// `no-restricted-imports` banning `../*` comes from the shared config.
export default defineConfig([
  // Legacy code is exempt until it moves under src/app, src/domain, src/shared, src/utils (Task 7).
  {
    ignores: [
      'src/components/**',
      'src/lib/**',
      'src/locales/**',
      'src/pages/**',
      'src/test/**',
      'src/main.tsx',
      'src/vite-env.d.ts',
    ],
  },
  { extends: [eslintConfig] },
]);
