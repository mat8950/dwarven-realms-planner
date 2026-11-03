// What changed between two history snapshots, as display lines, and which gear set a
// snapshot uses: players swap between a loot ("farm") set and a damage set, so each set is
// followed on its own.
import { slotName, t } from './i18n.svelte'
import type { Catalog, ItemView, Snapshot } from './types'

const slotKey = (it: ItemView) => (it.slot === null ? 'weapon' : String(it.slot))

function slotLabel(catalog: Catalog | null, it: ItemView): string {
  if (it.slot === null) return t('slot.weapon')
  return slotName(catalog?.slots.find((s) => s.value === it.slot)?.name ?? String(it.slot))
}

const sameAttrs = (a: ItemView, b: ItemView) => a.attributes.length === b.attributes.length && a.attributes.every((x, i) => x.tag === b.attributes[i].tag && Math.abs(x.value - b.attributes[i].value) < 1e-6)

export type Kind = 'dps' | 'farm'

/** Mods that only pay off when killing (gold, extra elites, containers, item find): what a farm
 *  set stacks. Experience, fellowship or mining mods stay in every set, so they do not count. */
const FARM_MOD = /Gold|Elite(?!Buff)|Container|Loot|MagicFind|ItemFind|Chest/i

/** Farm set: at least 3 kill-loot mods equipped. */
export function autoKind(s: Snapshot): Kind {
  let loot = 0
  for (const it of s.items) {
    for (const a of it.attributes) {
      if (a.tag.startsWith('EasyRPG.Items.Modifiers.') && FARM_MOD.test(a.tag.slice(24))) loot += Math.max(1, Math.round(a.value))
    }
  }
  return loot >= 3 ? 'farm' : 'dps'
}

/** Changes since `prev`, the previous point of the same set (none: the set's first point). */
export function changes(catalog: Catalog | null, prev: Snapshot | undefined, cur: Snapshot, kind: Kind): string[] {
  if (!prev) return [t('history.firstKind', { kind: t(kind === 'farm' ? 'history.kindFarm' : 'history.kindDps') })]
  const out: string[] = []
  if (cur.level !== prev.level) out.push(t('history.levelUp', { from: prev.level, to: cur.level }))
  const before = new Map(prev.items.map((it) => [slotKey(it), it]))
  for (const it of cur.items) {
    const old = before.get(slotKey(it))
    const slot = slotLabel(catalog, it)
    if (!old || old.row !== it.row || old.name !== it.name) {
      out.push(t('history.itemChanged', { slot, from: old?.name || '—', to: it.name || '—' }))
    } else if (old.hearts !== it.hearts) {
      out.push(t('history.heartsChanged', { slot, name: it.name, from: old.hearts, to: it.hearts }))
    } else if (!sameAttrs(old, it)) {
      out.push(t('history.itemEdited', { slot, name: it.name }))
    }
  }
  const was = new Set(prev.tree)
  const now = new Set(cur.tree)
  const taken = cur.tree.filter((r) => !was.has(r)).length
  const removed = prev.tree.filter((r) => !now.has(r)).length
  if (taken || removed) out.push(t('history.treeChanged', { n: taken, m: removed }))
  const level = new Map(prev.skills.map((s) => [`${s.table}/${s.row}`, s.level]))
  const skills = cur.skills.filter((s) => (level.get(`${s.table}/${s.row}`) ?? 0) !== s.level).length
  if (skills) out.push(t('history.skillsChanged', { n: skills }))
  if (out.length === 0 || (out.length === 1 && cur.level !== prev.level)) out.push(t('history.noBuildChange'))
  return out
}
