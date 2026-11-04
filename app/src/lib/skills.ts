// Levels of the skills outside the main tree, as the game shows them and as the save stores them.
//
// Crystal cards have three levels (5 quartz each) and the third doubles the effect: the save stores
// levels 1, 2 and 6, and a skill's bonus is its base bonus x the stored level (GetSkillDataAttributes).
// Stored levels 4 and 7 are a bug the game repairs to 3 ("DATA FIXER - Card Amount BUG" in
// BP_PlayerManagerComponent). Other tables store the level they show.

export const CARD_TABLE = 'DT_Crystal_Cards_Skills'
const CARD_LEVELS = [0, 1, 2, 6]

const isCards = (table: string) => table === CARD_TABLE

/** Level shown for a stored level. */
export const shownLevel = (table: string, stored: number) => (isCards(table) ? Math.min(stored, 3) : stored)

/** Stored level for a shown level. */
export const storedLevel = (table: string, shown: number) => (isCards(table) ? CARD_LEVELS[Math.max(0, Math.min(shown, 3))] : shown)

/** Highest shown level (0: unlimited). */
export const shownMax = (table: string, max: number) => (isCards(table) ? 3 : max)

/** Highest stored level (0: unlimited). */
export const storedMax = (table: string, max: number) => storedLevel(table, shownMax(table, max))

/** Stored levels one shown level adds (`up`) or removes (`down`) from `stored`. */
export function levelSteps(table: string, stored: number, max: number): { up: number; down: number } {
  const shown = shownLevel(table, stored)
  const top = shownMax(table, max)
  return {
    up: top > 0 && shown >= top ? 0 : storedLevel(table, shown + 1) - stored,
    down: shown > 0 ? stored - storedLevel(table, shown - 1) : 0,
  }
}
