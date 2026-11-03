// Gambler forge (UI_ItemGambler.Upgrade): each heart multiplies every stat of the item by 1.1,
// except the tags CheckTagsForMonograms skips. Success chance of heart n+1 is (10 - n) / 10;
// a failed attempt destroys the item.
import type { Attr } from './types'

export const MAX_HEARTS = 10
const MULT = 1.1

const under = (tag: string, parent: string) => tag === parent || tag.startsWith(parent + '.')

/** Tags the gambler does not multiply (mods, monograms, global modifiers, mastery, ability flags). */
export function forgeExempt(tag: string): boolean {
  return (
    under(tag, 'EasyRPG.Items.Modifiers') ||
    under(tag, 'EasyRPG.Attributes.GlobalModifiers') ||
    under(tag, 'EasyRPG.Attributes.Base.Mastery') ||
    (under(tag, 'EasyRPG.Attributes.Abilities') && !tag.endsWith('.DamageMultiplier'))
  )
}

/** Attributes of an item at `from` hearts, rescaled to `to` hearts. */
export function scaleHearts(attrs: Attr[], from: number, to: number): Attr[] {
  if (from === to) return attrs
  const f = Math.pow(MULT, to - from)
  return attrs.map((a) => (forgeExempt(a.tag) ? a : { tag: a.tag, value: Math.fround(a.value * f) }))
}

export function heartFactor(from: number, to: number): number {
  return Math.pow(MULT, to - from)
}

/** Probability of going from `from` to `to` hearts without losing the item. */
export function successChance(from: number, to: number): number {
  let p = 1
  for (let i = from; i < to; i++) p *= (MAX_HEARTS - i) / MAX_HEARTS
  return p
}
