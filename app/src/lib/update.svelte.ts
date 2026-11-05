// Updates of the installed app. The app checks the latest GitHub release's `latest.json` (Tauri updater, signed with
// the project's key) a few seconds after start and every 6 hours. Installed copies update themselves (Windows setup,
// Linux AppImage, macOS app); the others (Windows zip, .deb and .rpm packages) open the download page.
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { inTauri } from './api'
import { debug } from './debug.svelte'

const RELEASES = 'https://github.com/mat8950/dwarven-realms-planner/releases/latest'

type Mode = 'installer' | 'appimage' | 'bundle' | 'manual'

class Updater {
  /** The newer version found, with its release notes. */
  available = $state<{ version: string; notes: string } | null>(null)
  mode = $state<Mode>('manual')
  /** Download progress (0 to 1) while installing; null when the size is unknown or nothing runs. */
  progress = $state<number | null>(null)
  installing = $state(false)
  error = $state<string | null>(null)
  private update: Update | null = null

  get automatic(): boolean {
    return this.mode !== 'manual'
  }

  async check() {
    // The dev build follows the repository, not the releases.
    if (!inTauri || import.meta.env.DEV || this.installing) return
    try {
      this.mode = await invoke<Mode>('update_mode')
      const u = await check()
      if (u) {
        this.update = u
        this.available = { version: u.version, notes: u.body ?? '' }
        debug.log('info', 'update', `version ${u.version} available (${this.mode})`)
      }
    } catch (e) {
      debug.log('warn', 'update', `update check failed: ${e}`)
    }
  }

  /** Installs the update and restarts, or opens the download page for copies that cannot update themselves. */
  async install() {
    if (!this.automatic || !this.update) {
      await openUrl(RELEASES).catch((e) => (this.error = String(e)))
      return
    }
    this.installing = true
    this.error = null
    let total = 0
    let done = 0
    try {
      await this.update.downloadAndInstall((ev) => {
        if (ev.event === 'Started') total = ev.data.contentLength ?? 0
        else if (ev.event === 'Progress') {
          done += ev.data.chunkLength
          this.progress = total ? done / total : null
        } else if (ev.event === 'Finished') this.progress = 1
      })
      await relaunch()
    } catch (e) {
      this.error = String(e)
      debug.log('error', 'update', `update failed: ${e}`)
    } finally {
      this.installing = false
    }
  }
}

export const updater = new Updater()

export function initUpdater() {
  setTimeout(() => void updater.check(), 5000)
  setInterval(() => void updater.check(), 6 * 3600 * 1000)
}

// Dev server: this module holds app-wide state, so an edit reloads the page instead of swapping the module (a
// swapped copy would leave the views on two different states).
if (import.meta.hot) import.meta.hot.accept(() => location.reload())
