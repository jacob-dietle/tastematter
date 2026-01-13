import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { resolve } from 'path'

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      '$lib': resolve(__dirname, './src/lib')
    }
  },
  server: {
    proxy: {
      // Proxy /api/* to context-os HTTP server for browser dev mode
      '/api': {
        target: 'http://localhost:3001',
        changeOrigin: true
      }
    }
  }
})
