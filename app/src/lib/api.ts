import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { debug } from './debug.svelte'
import type { Probe } from './conditions'
import type { BuildInput, Catalog, CharacterView, ExtractProgress, Impact, SaveEntry, SetupStatus, Snapshot, StatsView } from './types'

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

/** Tauri command, or the HTTP bridge (`cargo run --bin serve`) when running in a browser.
 *  Every call is timed; failures are journaled with the command name. */
async function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const t = performance.now()
  try {
    let out: T
    if (inTauri) out = await invoke<T>(cmd, args)
    else {
      const r = await fetch(`/api/${cmd}`, { method: 'POST', body: JSON.stringify(args) })
      const body = await r.json()
      if (!r.ok) throw new Error(String(body))
      out = body as T
    }
    debug.call(cmd, performance.now() - t, true)
    return out
  } catch (e) {
    const ms = performance.now() - t
    debug.call(cmd, ms, false)
    debug.log('error', cmd, `${e instanceof Error ? e.message : String(e)} (${ms.toFixed(0)} ms)`)
    throw e
  }
}

export const api = {
  catalog: () => call<Catalog>('catalog'),
  saves: () => call<SaveEntry[]>('saves'),
  open: (path: string) => call<CharacterView>('open_save', { path }),
  compute: (build: BuildInput) => call<StatsView>('compute', { build }),
  analyze: (save: string, build: BuildInput, probes: Probe[]) => call<Impact[]>('analyze', { save, build, probes }),
  /** Reloads the game texts in another language (locres folder name); returns the new catalog. */
  setLanguage: (lang: string) => call<Catalog>('set_language', { lang }),
  /** Snapshots every character whose save changed; returns their history keys. */
  historyCapture: () => call<string[]>('history_capture'),
  history: (save: string) => call<Snapshot[]>('history', { save }),
  // Game data setup. Only the desktop app has it: the HTTP bridge serves the repository's data.
  setupStatus: () => (inTauri ? call<SetupStatus>('setup_status') : Promise.resolve(null)),
  setGameDir: (dir: string | null) => call<void>('set_game_dir', { dir }),
  setSaveDir: (dir: string | null) => call<void>('set_save_dir', { dir }),
  importMappings: (path: string) => call<string>('import_mappings', { path }),
  /** Installs RE-UE4SS with the DRDump mod in the game (it writes the mappings), or removes it. */
  dumperInstall: () => call<void>('dumper_install'),
  dumperRemove: () => call<void>('dumper_remove'),
  /** Extracts the game data from the installation in use and reloads it; returns the data folder. */
  extract: () => call<string>('extract'),
  onExtractProgress: (cb: (p: ExtractProgress) => void) => (inTauri ? listen<ExtractProgress>('extract-progress', (e) => cb(e.payload)) : Promise.resolve(() => {})),
}
