import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    exclude: ['out/**', 'node_modules/**'],
    include: ['src/test/**/*.test.ts']
  }
});
