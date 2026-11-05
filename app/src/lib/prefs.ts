// The interface keeps its preferences in local storage (`dr-planner.*` keys: language, zoom, saved builds, wanted
// stats...). Local storage alone is fragile: the dev server and the installed app each have their own, and it
// goes with the webview's data. So the backend keeps a copy (`<data dir>/prefs.json`): it fills local storage
// before the interface starts, and every later write is sent to it.
import { invoke } from '@tauri-apps/api/core'

const PREFIX = 'dr-planner.'
const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

async function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (inTauri) return invoke<T>(cmd, args)
  const r = await fetch(`/api/${cmd}`, { method: 'POST', body: JSON.stringify(args) })
  if (!r.ok) throw new Error(await r.text())
  return r.json()
}

let pending: Record<string, string | null> = {}
let timer: ReturnType<typeof setTimeout> | undefined
function send(key: string, value: string | null) {
  pending[key] = value
  clearTimeout(timer)
  timer = setTimeout(() => {
    const prefs = pending
    pending = {}
    call('set_prefs', { prefs }).catch(() => {})
  }, 300)
}

/** Fills local storage from the backend copy (it wins), sends it what it lacks, then mirrors every write. */
export async function syncPrefs() {
  let stored: Record<string, string> = {}
  try {
    stored = await call<Record<string, string>>('prefs')
  } catch {
    // no backend copy (older backend, no data folder): local storage alone
  }
  try {
    for (const [k, v] of Object.entries(stored)) localStorage.setItem(k, v)
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i)
      if (k?.startsWith(PREFIX) && !(k in stored)) send(k, localStorage.getItem(k))
    }
  } catch {
    return // storage unavailable: nothing to keep
  }
  const setItem = Storage.prototype.setItem
  const removeItem = Storage.prototype.removeItem
  Storage.prototype.setItem = function (k: string, v: string) {
    setItem.call(this, k, v)
    if (this === localStorage && k.startsWith(PREFIX)) send(k, String(v))
  }
  Storage.prototype.removeItem = function (k: string) {
    removeItem.call(this, k)
    if (this === localStorage && k.startsWith(PREFIX)) send(k, null)
  }
}

// Dev server: this module holds app-wide state, so an edit reloads the page instead of swapping the module (a
// swapped copy would leave the views on two different states).
if (import.meta.hot) import.meta.hot.accept(() => location.reload())
