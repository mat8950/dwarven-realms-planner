import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { debug, installGlobalHandlers } from './lib/debug.svelte'

installGlobalHandlers()
debug.log('info', 'app', `UI démarrée (${'__TAURI_INTERNALS__' in window ? 'Tauri' : 'navigateur'})`)

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
