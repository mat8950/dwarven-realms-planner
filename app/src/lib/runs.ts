// Runs found in a character's session journal (one entry per save the game wrote).
//
// What the game does (measured on fortune ruptures, 2026-11-04): it writes the save when the player leaves a map,
// with that map as `CurrentLevel`, and nothing during a run. A fortune run is launched from the fort with an
// Infused Time Rupture Beetle (save in the fort, one beetle less); the cards the NPC reveals cost a Fortune key.
// A player who dislikes the cards leaves the rupture (save on the rupture map, the key spent) and launches again
// (save in the fort, another beetle). The last launch's rupture is played: its save comes when the player leaves
// it, minutes later, with all the run's gains.
import type { SessionSnapshot } from './types'

/** Saves this close to each other belong to the same launch (the cards are judged within this time). */
const LAUNCH_GAP = 180
/** A run lasts at least this long. */
const MIN_RUN = 120

export const isFort = (zone: string) => /HomeFort/i.test(zone)
export const isRupture = (zone: string) => /_rupture_/i.test(zone)

export interface Run {
  /** The end save's time in whole seconds: the key of the player's note. */
  id: string
  /** State before the run: just before the first launch's rupture (what the player did in the fort before stays out). */
  base: SessionSnapshot
  end: SessionSnapshot
  /** When the played rupture started: the last save before the end (the player leaving the fort). */
  start: number
  /** A fortune rupture (an `_Infused` map, or a Fortune key spent at the launch). */
  fortune: boolean
  /** Ruptures left before the played one: each was a new set of cards (a reroll). */
  rerolls: number
  /** Fortune keys spent on those left ruptures (the played one's key is in the run's gains). */
  keys: number
  /** Map of the end save. */
  zone: string
}

export function findRuns(journal: SessionSnapshot[]): Run[] {
  const runs: Run[] = []
  const key = (s: SessionSnapshot) => s.resources.CasinoKey ?? 0
  for (let i = 1; i < journal.length; i++) {
    const prev = journal[i - 1]
    const end = journal[i]
    if (isFort(end.zone) || end.experience <= prev.experience || end.time - prev.time < MIN_RUN) continue
    // The launch: the saves just before, each within a few minutes of the next, back to the previous run's end.
    let first = i - 1
    while (first > 0 && journal[first].time - journal[first - 1].time <= LAUNCH_GAP && !runs.some((r) => r.end === journal[first - 1])) first--
    // Elsewhere than in a rupture (a boss arena...), each save is its own run: no launch.
    if (!isRupture(end.zone)) {
      runs.push({ id: String(Math.round(end.time)), base: prev, end, start: prev.time, fortune: false, rerolls: 0, keys: 0, zone: end.zone })
      continue
    }
    const launch = journal.slice(first, i)
    // Ruptures left before the played one; the launch starts at the first of them.
    const left = launch.filter((s) => isRupture(s.zone))
    const onMap = launch.findIndex((s) => isRupture(s.zone))
    const base = onMap > 0 ? launch[onMap - 1] : onMap === 0 ? (journal[first - 1] ?? launch[0]) : prev
    let keys = 0
    for (let k = Math.max(first + Math.max(onMap, 0), 1); k < i; k++) keys += Math.max(0, key(journal[k - 1]) - key(journal[k]))
    const fortune = /Infused/i.test(end.zone) || left.some((s) => /Infused/i.test(s.zone)) || keys > 0
    runs.push({ id: String(Math.round(end.time)), base, end, start: prev.time, fortune, rerolls: left.length, keys, zone: end.zone })
  }
  return runs
}

export const gainOf = (r: Run, resource: string) => (r.end.resources[resource] ?? 0) - (r.base.resources[resource] ?? 0)

/** Every resource that moved during the run, biggest gain first. */
export function resourceGains(r: Run): [string, number][] {
  const keys = new Set([...Object.keys(r.base.resources), ...Object.keys(r.end.resources)])
  return [...keys].map((k) => [k, gainOf(r, k)] as [string, number]).filter(([, g]) => g !== 0).sort((a, b) => b[1] - a[1])
}
