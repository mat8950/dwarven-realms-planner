import { svelte } from '@sveltejs/vite-plugin-svelte'
import { existsSync, readdirSync, statSync } from 'node:fs'
import { homedir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'

/** The planner's data folder (dr_core::setup::data_dir). */
function appDataDir(): string {
  if (process.platform === 'win32') return join(process.env.APPDATA ?? join(homedir(), 'AppData/Roaming'), 'dr-planner')
  if (process.platform === 'darwin') return join(homedir(), 'Library/Application Support/dr-planner')
  return join(process.env.XDG_DATA_HOME ?? join(homedir(), '.local/share'), 'dr-planner')
}

/** Extracted textures and fonts, chosen like the app's debug build chooses its data: DR_PLANNER_ROOT, else this
 *  repository's data (tools/extract/extract.sh), else the newest extraction the app made. */
function assetsDir(): string | undefined {
  if (process.env.DR_PLANNER_ROOT) return join(process.env.DR_PLANNER_ROOT, 'data/assets')
  const repo = fileURLToPath(new URL('../data/assets', import.meta.url))
  if (existsSync(join(repo, '..', 'extracted', 'BUILD'))) return repo
  const game = join(appDataDir(), 'game')
  const builds = existsSync(game) ? readdirSync(game).map((b) => join(game, b, 'data')).filter((d) => existsSync(join(d, 'extracted', 'BUILD'))) : []
  builds.sort((a, b) => statSync(join(b, 'extracted', 'BUILD')).mtimeMs - statSync(join(a, 'extracted', 'BUILD')).mtimeMs)
  return builds.length ? join(builds[0], 'assets') : undefined
}

// The dev server serves the game textures and fonts as static files (a built app serves them itself, through
// its drassets protocol). A release build does not embed them: they are game content.
export default defineConfig(({ command }) => ({
  // Component CSS goes with the component's JS: the separate CSS modules come from a cache that
  // vite-plugin-svelte sometimes misses after a restart, and Vite then serves the .svelte source as CSS.
  plugins: [svelte({ emitCss: false })],
  publicDir: command === 'serve' ? (assetsDir() ?? false) : false,
  clearScreen: false,
  server: { port: 5173, strictPort: true, proxy: { '/api': 'http://127.0.0.1:5174' } },
}))
