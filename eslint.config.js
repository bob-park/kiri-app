import eslintConfig from '@bob-park/eslint-config-bobpark';

import { defineConfig } from 'eslint/config';

// `no-restricted-imports` banning `../*` comes from the shared config.
export default defineConfig([
  { extends: [eslintConfig] },
]);
