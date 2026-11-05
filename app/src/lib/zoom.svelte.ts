// Interface zoom. The layout is drawn for a window of about 1680 x 1000 CSS pixels: "auto" scales it to the
// window (smaller on a laptop, larger on a 1440p or 4K screen at 100 % system scaling); the player can also pick a
// fixed zoom. The desktop app zooms the webview itself (like a browser's Ctrl +); the browser version (dev, tests)
// sets CSS `zoom` on the page.
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { inTauri } from './api'

const KEY = 'dr-planner.zoom'
/** Fixed zooms offered, and the steps of Ctrl + / Ctrl -. */
export const ZOOMS = [0.6, 0.67, 0.75, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5]
const DESIGN = { w: 1680, h: 1000 }

type Choice = 'auto' | number

function stored(): Choice {
  try {
    const v = localStorage.getItem(KEY)
    if (v && v !== 'auto' && Number.isFinite(Number(v))) return Number(v)
  } catch {
    // storage unavailable: auto
  }
  return 'auto'
}

class Zoom {
  choice = $state<Choice>(stored())
  /** Zoom in force. */
  applied = $state(1)

  /** Window size in CSS pixels at zoom 1 (the webview zoom shrinks `innerWidth`, CSS zoom does not). */
  private base() {
    const k = inTauri ? this.applied : 1
    return { w: window.innerWidth * k, h: window.innerHeight * k }
  }

  /** Zoom that fits the layout to the window, in 5 % steps; 100 % when it is close. */
  get auto(): number {
    const { w, h } = this.base()
    const fit = Math.min(w / DESIGN.w, h / DESIGN.h)
    if (fit > 0.95 && fit < 1.15) return 1
    return Math.min(2.5, Math.max(0.6, Math.round(fit * 20) / 20))
  }

  get target(): number {
    return this.choice === 'auto' ? this.auto : this.choice
  }

  private done = false

  async apply() {
    const z = this.target
    if (this.done && Math.abs(z - this.applied) < 0.001) return
    try {
      if (inTauri) await getCurrentWebview().setZoom(z)
      else document.documentElement.style.zoom = z === 1 ? '' : String(z)
      this.applied = z
      this.done = true
    } catch (e) {
      console.warn('zoom unavailable', e)
    }
  }

  set(c: Choice) {
    this.choice = c
    try {
      localStorage.setItem(KEY, String(c))
    } catch {
      // the choice lasts for this session
    }
    void this.apply()
  }

  /** One step bigger (+1) or smaller (-1) than the zoom in force. */
  step(dir: 1 | -1) {
    const cur = this.applied
    const next = dir > 0 ? ZOOMS.find((z) => z > cur + 0.001) : [...ZOOMS].reverse().find((z) => z < cur - 0.001)
    if (next !== undefined) this.set(next)
  }
}

export const zoom = new Zoom()

/** Applies the stored zoom, follows the window size in auto, and binds Ctrl + / Ctrl - / Ctrl 0 (back to auto). */
export function initZoom() {
  void zoom.apply()
  let timer: ReturnType<typeof setTimeout> | undefined
  window.addEventListener('resize', () => {
    if (zoom.choice !== 'auto') return
    clearTimeout(timer)
    timer = setTimeout(() => void zoom.apply(), 200)
  })
  window.addEventListener('keydown', (e) => {
    if (!(e.ctrlKey || e.metaKey) || e.altKey) return
    if (e.key === '+' || e.key === '=') zoom.step(1)
    else if (e.key === '-' || e.key === '_') zoom.step(-1)
    else if (e.key === '0') zoom.set('auto')
    else return
    e.preventDefault()
  })
}

// Dev server: this module holds app-wide state, so an edit reloads the page instead of swapping the module (a
// swapped copy would leave the views on two different states).
if (import.meta.hot) import.meta.hot.accept(() => location.reload())
