import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { resolve } from 'path';

export default defineConfig({
  plugins: [svelte()],
  test: {
    include: ['tests/unit/**/*.test.ts'],
    environment: 'happy-dom',
    globals: true,
    setupFiles: ['./tests/setup.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      include: ['src/lib/**/*.ts', 'src/lib/**/*.svelte']
    },
    alias: {
      // Force Svelte to use client-side bundle for testing
      'svelte': 'svelte'
    }
  },
  resolve: {
    conditions: ['browser', 'development'],
    alias: {
      '$lib': resolve(__dirname, './src/lib')
    }
  },
  ssr: {
    noExternal: ['svelte']
  }
});
