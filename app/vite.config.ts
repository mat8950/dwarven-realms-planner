import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

// In development the game textures and fonts extracted by tools/extract are served as static
// files. A release build does not embed them (they are game content): the app reads them from
// the data the player extracted from their own game.
export default defineConfig(({ command }) => ({
  // Component CSS goes with the component's JS: the separate CSS modules come from a cache that
  // vite-plugin-svelte sometimes misses after a restart, and Vite then serves the .svelte source as CSS.
  plugins: [svelte({ emitCss: false })],
  publicDir: command === 'serve' ? '../data/assets' : false,
  clearScreen: false,
  server: { port: 5173, strictPort: true, proxy: { '/api': 'http://127.0.0.1:5174' } },
}))
