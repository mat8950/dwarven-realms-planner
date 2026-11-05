// Debug journal of the UI: errors and timings, mirrored to the backend log file
// (~/.cache/dr-planner/app.log) so issues can be diagnosed after the fact.
import { invoke } from '@tauri-apps/api/core'

export type Level = 'info' | 'warn' | 'error'

export interface Entry {
  time: string
  level: Level
  source: string
  message: string
}

export interface CallStat {
  count: number
  errors: number
  lastMs: number
  maxMs: number
}

export interface BackendInfo {
  version: string
  log_path: string
  data_root: string
  save_dir: string
  recent: string[]
}

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
const MAX_ENTRIES = 400

class Debug {
  entries = $state<Entry[]>([])
  calls = $state<Record<string, CallStat>>({})
  open = $state(false)
  backend = $state<BackendInfo | null>(null)
  errors = $derived(this.entries.filter((e) => e.level === 'error').length)

  log(level: Level, source: string, message: string) {
    const e: Entry = { time: new Date().toLocaleTimeString('fr-FR', { hour12: false }) + '.' + String(Date.now() % 1000).padStart(3, '0'), level, source, message }
    this.entries = [...this.entries.slice(-(MAX_ENTRIES - 1)), e]
    if (level !== 'info') this.send(level, source, message)
  }

  /** Writes to the backend journal; never throws (the journal must not create errors). */
  send(level: Level, source: string, message: string) {
    const args = { level, source, message }
    const p = inTauri ? invoke('log_event', args) : fetch('/api/log_event', { method: 'POST', body: JSON.stringify(args) })
    p.catch(() => {})
  }

  call(cmd: string, ms: number, ok: boolean) {
    const c = this.calls[cmd] ?? { count: 0, errors: 0, lastMs: 0, maxMs: 0 }
    this.calls = { ...this.calls, [cmd]: { count: c.count + 1, errors: c.errors + (ok ? 0 : 1), lastMs: ms, maxMs: Math.max(c.maxMs, ms) } }
  }

  async refreshBackend() {
    try {
      this.backend = inTauri ? await invoke<BackendInfo>('debug_info') : await (await fetch('/api/debug_info', { method: 'POST', body: '{}' })).json()
    } catch (e) {
      this.log('warn', 'debug', `debug_info unavailable: ${e}`)
    }
  }

  clear() {
    this.entries = []
    this.calls = {}
  }
}

export const debug = new Debug()

/** Captures uncaught errors and rejected promises of the page. */
export function installGlobalHandlers() {
  window.addEventListener('error', (e) => debug.log('error', 'js', `${e.message} (${e.filename}:${e.lineno}:${e.colno})${e.error?.stack ? '\n' + e.error.stack : ''}`))
  window.addEventListener('unhandledrejection', (e) => debug.log('error', 'promise', String(e.reason?.stack ?? e.reason)))
  // Images and other resources that fail to load (their error does not bubble): one line per URL.
  const failed = new Set<string>()
  window.addEventListener(
    'error',
    (e) => {
      const el = e.target as HTMLImageElement | null
      const url = el && el !== (window as unknown) && 'src' in el ? el.src : null
      if (url && !failed.has(url) && failed.size < 20) {
        failed.add(url)
        debug.log('warn', 'asset', `failed to load ${url}`)
      }
    },
    true,
  )
  window.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 'd') {
      debug.open = !debug.open
      e.preventDefault()
    }
  })
}

// Dev server: this module holds app-wide state, so an edit reloads the page instead of swapping the module (a
// swapped copy would leave the views on two different states).
if (import.meta.hot) import.meta.hot.accept(() => location.reload())
