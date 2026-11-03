// Game states some mods need to do anything, read from the stat code (BP_AttributesComponent):
// a buff (status effect and its stacks), Unholy Void, Berserker Fury, rupture essence orbs,
// a ranged weapon. Such a mod is measured with its state active; a mod that creates the
// state itself (`grants`) is measured from "no state" to "state + mod".
import { t } from './i18n.svelte'
import type { Catalog } from './types'

export interface When {
  status?: [string, number][]
  attrs?: [string, number][]
  orbs?: number
  colossus?: boolean
  stance?: number
  grants?: boolean
}

export type Probe = [string, number] | { tag: string; delta: number; when: When }

export interface Condition {
  when: When
  /** Short label of the state ("Bloodlust ×100"). */
  label: string
}

/** What the build already has active: a condition already met needs no special measure. */
export interface Active {
  status: Record<string, number>
  orbs: number
  colossus: boolean
  stance: number | undefined
  attrs: Set<string>
}

const MOD = 'EasyRPG.Items.Modifiers.'
const VOID = MOD + 'DamageCircle.Base'

export function modCondition(tag: string, catalog: Catalog | null, active: Active): Condition | null {
  if (!tag.startsWith(MOD)) return null
  const m = tag.slice(MOD.length)
  const maxStack = (row: string) => catalog?.status_effects.find((e) => e.row === row)?.max_stack ?? 1
  const status = (rows: string[], grants = false): Condition | null => {
    const missing = rows.filter((r) => !(active.status[r] > 0))
    if (!missing.length) return null
    const st = missing.map((r) => [r, maxStack(r)] as [string, number])
    return { when: { status: st, grants }, label: st.map(([r, n]) => `${t(`cond.${r}`, {}, r)} ×${n}`).join(' + ') }
  }
  if (m === 'AllowPhasing') return status(['Buff_PhasingTrance'], true)
  if (m.startsWith('Phasing.')) return status(['Buff_PhasingTrance'])
  if (m === 'Bloodlust.Base') return status(['Buff_Bloodlust'], true)
  if (/^Bloodlust\.(DrawBlood|MoreBlood)/.test(m)) return status(['Buff_Bloodlust', 'Buff_Blood'])
  if (/^Bloodlust\.(DrawLife|MoreLife)/.test(m)) return status(['Buff_Bloodlust', 'Buff_Life'])
  if (m.startsWith('Bloodlust.')) return status(['Buff_Bloodlust'])
  // Shroud: moving builds Light Shroud (Buff_Shroud); a hit turns it into Dark Shroud (Buff_ExposedStrike).
  if (m === 'Shroud') return status(['Buff_ExposedStrike'], true)
  if (m === 'Shroud.damageScale.HighestStat') return status(['Buff_ExposedStrike'])
  if (m.startsWith('Shroud.')) return status(['Buff_Shroud'])
  if (m === 'DamageCircle' || m.startsWith('DamageCircle.')) {
    if (active.attrs.has(VOID)) return null
    return { when: { attrs: [[VOID, 1]], grants: m === 'DamageCircle' }, label: t('cond.void') }
  }
  if (m.startsWith('Colossus.')) {
    if (active.colossus) return null
    return { when: { colossus: true, grants: m === 'Colossus.Base' }, label: t('cond.colossus') }
  }
  if (/Essence/.test(m)) {
    if (active.orbs > 0) return null
    return { when: { orbs: 100 }, label: t('cond.orbs', { n: 100 }) }
  }
  if (m.startsWith('RangedParagon.')) {
    const bow = catalog?.stances.find(([, n]) => n === 'Bow')?.[0]
    if (bow === undefined || active.stance === bow) return null
    return { when: { stance: bow }, label: t('cond.ranged') }
  }
  return null
}

/** Mods about loot, currency, experience or gathering: no combat value to measure. */
export function isLootMod(tag: string): boolean {
  return /Gold|Elite(?!Buff)|Container|Loot|Xp|XP|Experience|Fellowship|MagicFind|ItemFind|Mining|Node|Resource|Chest|Disenchant|Fish|Craft|Essence.*Drop|Coins|Elven/i.test(tag.replace(MOD, ''))
}
