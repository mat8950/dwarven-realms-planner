import './app.css'
import { syncPrefs } from './lib/prefs'

// The preferences come first: the interface's modules read them when they load.
async function start() {
  await syncPrefs()
  const { mount } = await import('svelte')
  const { default: App } = await import('./App.svelte')
  const { debug, installGlobalHandlers } = await import('./lib/debug.svelte')
  const { loadFonts } = await import('./lib/game')
  const { initZoom } = await import('./lib/zoom.svelte')

  installGlobalHandlers()
  loadFonts()
  initZoom()
  debug.log('info', 'app', `UI démarrée (${'__TAURI_INTERNALS__' in window ? 'Tauri' : 'navigateur'})`)
  mount(App, { target: document.getElementById('app')! })
}

void start()
