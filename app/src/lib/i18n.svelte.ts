// Languages of the planner. Two layers:
// - game texts (stats, items, skills, effects) come from the game's own language files,
//   loaded by the backend (`set_language`): every language the game ships works;
// - planner texts come from the dictionaries in ./locales. A language without its own
//   dictionary shows English planner texts. To add one: copy locales/en.ts, translate it,
//   and register it in DICTS below.
import en from './locales/en'
import fr from './locales/fr'

export type Key = keyof typeof fr

export interface Language {
  code: string
  name: string
  /** Locres folder of the game's texts. */
  game: string
  /** Locale for numbers, dates and plural rules. */
  intl: string
}

export const LANGUAGES: Language[] = [
  { code: 'fr', name: 'Français', game: 'fr', intl: 'fr-FR' },
  { code: 'en', name: 'English', game: 'en', intl: 'en-US' },
  { code: 'de', name: 'Deutsch', game: 'de-DE', intl: 'de-DE' },
  { code: 'es', name: 'Español', game: 'es-ES', intl: 'es-ES' },
  { code: 'pt', name: 'Português (BR)', game: 'pt-BR', intl: 'pt-BR' },
  { code: 'ru', name: 'Русский', game: 'ru-RU', intl: 'ru-RU' },
  { code: 'ja', name: '日本語', game: 'ja-JP', intl: 'ja-JP' },
  { code: 'ko', name: '한국어', game: 'ko-KR', intl: 'ko-KR' },
  { code: 'zh', name: '简体中文', game: 'zh-Hans-CN', intl: 'zh-CN' },
]

const DICTS: Record<string, Partial<Record<Key, string>>> = { fr, en }
const STORAGE_KEY = 'dr-planner.lang'

/** The language chosen in the app, else the system's when the game has it, else English. */
function initial(): string {
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    if (saved && LANGUAGES.some((l) => l.code === saved)) return saved
  } catch {
    // storage unavailable: use the system language
  }
  for (const tag of navigator.languages ?? [navigator.language]) {
    const code = tag.toLowerCase().split('-')[0]
    if (LANGUAGES.some((l) => l.code === code)) return code
  }
  return 'en'
}

class I18n {
  lang = $state(initial())

  get language(): Language {
    return LANGUAGES.find((l) => l.code === this.lang) ?? LANGUAGES[1]
  }

  /** Whether the planner texts are translated (else they are shown in English). */
  get translated(): boolean {
    return this.lang in DICTS
  }

  set(code: string) {
    this.lang = code
    try {
      localStorage.setItem(STORAGE_KEY, code)
    } catch {
      // storage unavailable: the choice lasts for this session
    }
  }
}

export const i18n = new I18n()

const plurals = new Map<string, Intl.PluralRules>()
function pluralOne(n: number): boolean {
  const loc = i18n.language.intl
  let p = plurals.get(loc)
  if (!p) {
    p = new Intl.PluralRules(loc)
    plurals.set(loc, p)
  }
  return p.select(n) === 'one'
}

/** Planner text `key`, with `{name}` values and `{n|one|other}` plural forms. Keys built at
 *  run time (`stance.${name}`) fall back to `fallback`, then to the key itself. */
export function t(key: Key | string, vars: Record<string, string | number> = {}, fallback?: string): string {
  const text = DICTS[i18n.lang]?.[key as Key] ?? en[key as Key] ?? fallback ?? key
  return text.replace(/\{(\w+)(?:\|([^|}]*)\|([^}]*))?\}/g, (m, name: string, one?: string, other?: string) => {
    const v = vars[name]
    if (v === undefined) return m
    if (one === undefined) return String(v)
    return pluralOne(Number(v)) ? one : (other ?? one)
  })
}

/** Number with the language's separators. */
export function fmtNumber(n: number, digits = 0): string {
  return new Intl.NumberFormat(i18n.language.intl, { maximumFractionDigits: digits }).format(n)
}

export function fmtDate(unixSeconds: number): string {
  return new Intl.DateTimeFormat(i18n.language.intl, { dateStyle: 'short', timeStyle: 'short' }).format(new Date(unixSeconds * 1000))
}

// Names the game keeps as enum identifiers.
export const stanceName = (enumName: string) => t(`stance.${enumName}`, {}, enumName)
export const slotName = (enumName: string) => t(`slot.${enumName}`, {}, enumName)
export const raceName = (enumName: string) => t(`race.${enumName}`, {}, enumName)
export const rarityName = (rarity: number) => t(`rarity.${rarity}`, {}, String(rarity))

// Dev server: this module holds app-wide state, so an edit reloads the page instead of swapping the module (a
// swapped copy would leave the views on two different states).
if (import.meta.hot) import.meta.hot.accept(() => location.reload())
