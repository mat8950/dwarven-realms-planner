// Number formatting identical to the game (BP_GameLibrary.GetAttributeValueText):
// integer -> no decimals, float -> up to `digits` decimals, percent -> value*100 with "%".
import type { AttrInfo } from './types'

const nf = new Map<number, Intl.NumberFormat>()
function fmt(n: number, maxFrac: number): string {
  let f = nf.get(maxFrac)
  if (!f) {
    f = new Intl.NumberFormat('en-US', { maximumFractionDigits: maxFrac, minimumFractionDigits: 0 })
    nf.set(maxFrac, f)
  }
  return f.format(n)
}

/** Value as the stats panel / tooltips show it. */
export function formatValue(v: number, info: AttrInfo | undefined, alwaysSign = true): string {
  const type = info?.value_type ?? 1
  const digits = info?.digits ?? 2
  if (type === 3) return ''
  const shown = type === 2 ? v * 100 : v
  const text = type === 0 ? fmt(Math.round(shown), 0) : fmt(shown, digits)
  const sign = alwaysSign && shown > 0 ? '+' : ''
  return sign + text + (type === 2 ? '%' : '')
}

/** Display units <-> stored value (percent attributes are edited as "35" for 0.35). */
export function toDisplay(v: number, info: AttrInfo | undefined): number {
  return info?.value_type === 2 ? v * 100 : v
}
export function fromDisplay(v: number, info: AttrInfo | undefined): number {
  return info?.value_type === 2 ? v / 100 : v
}

export function formatDelta(d: number, info: AttrInfo | undefined): string {
  if (Math.abs(d) < 1e-9) return ''
  return (d > 0 ? '+' : '−') + formatValue(Math.abs(d), info, false)
}

const SUFFIXES = ['', 'K', 'M', 'B', 'T', 'Qa', 'Qi', 'Sx', 'Sp', 'Oc', 'No', 'Dc']

/** Big numbers the way the in-game damage meter shows them (66.3B, 4.30Qa): 3 significant digits. */
export function shortNumber(n: number): string {
  const a = Math.abs(n)
  if (a < 1000) return fmt(n, a < 10 ? 2 : a < 100 ? 1 : 0)
  const k = Math.min(SUFFIXES.length - 1, Math.floor(Math.log10(a) / 3))
  const v = n / 10 ** (3 * k)
  const digits = Math.abs(v) < 10 ? 2 : Math.abs(v) < 100 ? 1 : 0
  return v.toFixed(digits) + SUFFIXES[k]
}

/** Unreal linear colour -> CSS sRGB. */
export function linearToCss(r: number, g: number, b: number, a = 1): string {
  const s = (c: number) => Math.round(255 * (c <= 0.0031308 ? 12.92 * c : 1.055 * Math.pow(c, 1 / 2.4) - 0.055))
  return `rgba(${s(r)}, ${s(g)}, ${s(b)}, ${a})`
}
