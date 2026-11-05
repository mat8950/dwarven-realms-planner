import { api } from './api'
import { debug } from './debug.svelte'
import { i18n, t } from './i18n.svelte'
import { forgeExempt, heartFactor, scaleHearts } from './forge'
import { isMod, loadFonts, noForge } from './game'
import { isLootMod, modCondition, type Active, type Condition, type Probe } from './conditions'
import { autoKind, type Kind } from './history'
import { probeKey, skillProbes, treeProbes } from './tree'
import { storedMax } from './skills'
import type { Attr, BuildInput, Candidate, Catalog, CharacterView, Container, DpsView, Impact, InvItem, ItemView, Objective, SaveEntry, SetupStatus, Snapshot, StatsView, TreeOption, WeaponInfo } from './types'

export type Selection = number | 'weapon' | null
export type ForgeMode = 'current' | 'none' | 'max'

const BUILDS_KEY = 'dr-planner.builds'
const KINDS_KEY = 'dr-planner.historyKinds'
const VOID = 'EasyRPG.Items.Modifiers.DamageCircle.Base'

export interface SavedBuild {
  name: string
  save: string
  edits: Record<number, Attr[]>
  weapon: Attr[] | null
  weaponRow?: string | null
  hearts?: Record<string, number>
  forgeMode?: ForgeMode
  tree?: string[] | null
  skillLevels?: Record<string, number>
  statusEffects?: Record<string, number>
  states?: { orbs: number; colossus: boolean; unholyVoid: boolean }
  stance: number | null
  potions: number | null
  effects: Attr[]
}

const key = (sel: Selection) => String(sel)

class Planner {
  catalog = $state<Catalog | null>(null)
  saves = $state<SaveEntry[]>([])
  character = $state<CharacterView | null>(null)

  /** Edited attribute lists per equipment slot, at the item's original hearts. */
  edits = $state<Record<number, Attr[]>>({})
  weaponEdit = $state<Attr[] | null>(null)
  /** Replacement weapon (DT_Items row); keeps the current weapon's affixes unless edited. */
  weaponRow = $state<string | null>(null)
  /** Inventory items tried in a slot (Inventory view): shown instead of the equipped item. */
  tried = $state<Record<number, { name: string; icon: string | null; rarity: number; tier: number }>>({})
  /** Target gambler hearts per item (key = slot or "weapon"); missing = the item's own. */
  hearts = $state<Record<string, number>>({})
  forgeMode = $state<ForgeMode>('current')
  /** Owned rows of the main skill tree (null: the save's tree). */
  tree = $state<string[] | null>(null)
  /** Levels of other skills that differ from the save, by `table/row`. */
  skillLevels = $state<Record<string, number>>({})
  stance = $state<number | null>(null)
  potions = $state<number | null>(null)
  effects = $state<Attr[]>([])
  /** Active status effects (row -> stacks), like the buffs shown in game. */
  statusEffects = $state<Record<string, number>>({})
  /** Rupture essence orbs held and Berserker Fury (Colossus), read by some mods. */
  ruptureOrbs = $state(0)
  colossus = $state(false)
  /** Unholy Void active (the game marks it with the `DamageCircle.Base` attribute). */
  unholyVoid = $state(false)

  stats = $state<StatsView | null>(null)
  /** Save's items, same situation: what the deltas compare against. */
  baseline = $state<StatsView | null>(null)
  selected = $state<Selection>(null)
  error = $state<string | null>(null)
  loading = $state(true)

  /** What the impacts rank stats by. The DPS total follows the DPS tab: it counts the
   *  offhands (elemental) as well as the weapon hits (physical). */
  // Boss damage counts against bosses and elites (elite health bar): the default objective.
  objective = $state<Objective>(loadObjective())
  /** Impact of one typical roll of each stat on the current build (by tag). */
  impacts = $state<Record<string, Impact>>({})
  /** Impact of removing each attribute the items carry (by probeKey(tag, -value)). */
  removals = $state<Record<string, Impact>>({})
  /** Impact of the affix picker's choices at the open item's roll (by probeKey(tag, value)). */
  pickerImpacts = $state<Record<string, Impact>>({})

  statMods = $derived(new Set(this.catalog?.stat_mods ?? []))

  /** Tags that can still grow: a rollable affix, an unowned tree node or a skill below its
   *  max level. Without one (e.g. projectile bonuses once their tree nodes are all taken),
   *  a stat is not a priority however much it weighs. */
  sourceTags = $derived.by(() => {
    const c = this.catalog
    const out = new Set<string>()
    if (!c) return out
    for (const a of c.affixes) if (a.can_roll) out.add(a.tag)
    const owned = new Set(this.treeRows())
    for (const n of c.skill_tree.nodes) {
      if (n.root || n.options.some((o) => owned.has(o.row))) continue
      for (const o of n.options) for (const [t] of o.attributes) out.add(t)
    }
    for (const s of c.skill_sets) {
      for (const k of s.skills) {
        const max = storedMax(s.table, k.max_level)
        if (max > 0 && this.skillLevel(s.table, k.row) >= max) continue
        for (const [t] of k.attributes) out.add(t)
      }
    }
    return out
  })
  analyzing = $state(false)
  /** Why the last stat analysis failed (shown with the key stats; the details are in the debug panel). */
  analyzeError = $state<string | null>(null)
  /** When the running analysis started (ms), to tell how long it takes. */
  analyzeStarted = $state(0)
  hideUseless = $state(false)
  /** DPS tab: main attack ("basic" combo or a weapon skill row) and offhand choices by row. */
  dpsMain = $state<string>('basic')
  dpsOffhands = $state<Record<string, boolean>>({})
  /** Skill tree view: open, and the impact of each node bonus (by probeKey). */
  treeOpen = $state(false)
  /** Progression history view, and the open character's snapshots (oldest first). */
  historyOpen = $state(false)
  /** Play session view: gains since a starting point, from the save each time the game writes it. */
  sessionOpen = $state(false)
  /** Inventory and stash of the open character (Inventory view). */
  inventoryOpen = $state(false)
  inventory = $state<Container[]>([])
  inventoryLoading = $state(false)
  /** Gain of each equipment item on the objective if equipped (key = container id + index), and the
   *  build it was measured on (gains are stale once the build changes). */
  itemGains = $state<Record<string, { impact: Impact; slot: number | null }>>({})
  itemGainsFor = $state('')
  evaluating = $state(false)

  /** Game data setup (desktop app): installation, mappings, extracted data. Null in a browser. */
  setup = $state<SetupStatus | null>(null)
  setupOpen = $state(false)
  history = $state<Snapshot[]>([])
  /** Gear set of history points set by hand (`save|save_modified`), over the automatic one. */
  historyKinds = $state<Record<string, Kind>>(loadKinds())

  kindOf(s: Snapshot): Kind {
    return this.historyKinds[`${this.character?.save}|${s.save_modified}`] ?? autoKind(s)
  }

  setKind(s: Snapshot, kind: Kind) {
    const key = `${this.character?.save}|${s.save_modified}`
    const { [key]: _, ...rest } = this.historyKinds
    this.historyKinds = kind === autoKind(s) ? rest : { ...rest, [key]: kind }
    try {
      localStorage.setItem(KINDS_KEY, JSON.stringify(this.historyKinds))
    } catch {
      // storage unavailable: the choice lasts for this session
    }
  }
  /** Tab of the skills view: the main tree, or stance skills / cards / crafting. */
  skillsTab = $state<'tree' | 'skills'>('tree')
  treeImpacts = $state<Record<string, Impact>>({})
  treeAnalyzing = $state(false)
  /** Why the last skill analysis failed (the details are in the debug panel). */
  treeAnalyzeError = $state<string | null>(null)

  builds = $state<SavedBuild[]>(loadBuilds())

  async init() {
    await this.refreshSetup()
    // No game data yet (first run of an installed app): the setup screen extracts it.
    if (this.setup && !this.setup.data_root) {
      this.setupOpen = true
      this.loading = false
      return
    }
    await this.load()
    this.startHistory()
  }

  async refreshSetup() {
    try {
      this.setup = await api.setupStatus()
    } catch (e) {
      debug.log('error', 'setup', String(e))
    }
  }

  /** Loads the catalog and the saves, and opens the first character. */
  private async load() {
    this.loading = true
    try {
      this.catalog = await api.catalog()
      if (this.catalog.lang !== i18n.language.game) this.catalog = await api.setLanguage(i18n.language.game)
      this.saves = await api.saves()
      if (this.saves.length) await this.open(this.saves[0].path)
    } catch (e) {
      this.error = String(e)
    } finally {
      this.loading = false
    }
  }

  /** After an extraction (or a new save folder): the backend has new data, start again from it. */
  async reloadData() {
    await this.refreshSetup()
    loadFonts()
    this.error = null
    this.character = null
    this.stats = null
    await this.load()
    this.startHistory()
  }

  private historyTimer: ReturnType<typeof setInterval> | undefined
  /** History: one point per save change, checked now and every 2 minutes. */
  private startHistory() {
    void this.captureHistory()
    if (this.historyTimer === undefined) this.historyTimer = setInterval(() => void this.captureHistory(), 120_000)
  }

  /** Switches the planner texts and the game texts; the build being edited is kept. */
  private lseq = 0
  async setLanguage(code: string) {
    i18n.set(code)
    const n = ++this.lseq
    try {
      const catalog = await api.setLanguage(i18n.language.game)
      // A quicker switch after this one wins.
      if (n !== this.lseq) return
      this.catalog = catalog
      // Stats and gains do not depend on the language: only the texts change. The backend's new
      // planner has no character, so reload it there (the view and the edits stay as they are).
      if (this.character) await api.open(this.character.save)
    } catch (e) {
      this.error = String(e)
    }
  }

  async captureHistory() {
    try {
      const added = await api.historyCapture()
      if (added.length) debug.log('info', 'history', `new snapshot(s): ${added.join(', ')}`)
      if (this.character && (added.length || !this.history.length)) await this.loadHistory()
    } catch (e) {
      debug.log('warn', 'history', String(e))
    }
  }

  async loadHistory() {
    if (!this.character) return
    try {
      this.history = await api.history(this.character.save)
    } catch (e) {
      debug.log('warn', 'history', String(e))
    }
  }

  async open(path: string) {
    this.error = null
    try {
      this.character = await api.open(path)
    } catch (e) {
      // The backend keeps the previous character: say why the switch failed.
      this.error = t('app.openFailed', { path, error: String(e) })
      return
    }
    debug.log('info', 'open', `${this.character.name} lvl ${this.character.level} — ${path}`)
    this.resetEdits()
    this.impacts = {}
    this.history = []
    void this.loadHistory()
  }

  /** Reads the current character's save again (the game wrote a new one: new gear, levels). The edits are dropped. */
  async reloadSave() {
    if (!this.character) return
    const path = this.character.save
    try {
      this.saves = await api.saves()
    } catch (e) {
      debug.log('warn', 'open', `save list failed: ${e}`)
    }
    await this.open(path)
  }

  resetEdits() {
    this.edits = {}
    this.tried = {}
    this.weaponEdit = null
    this.weaponRow = null
    this.hearts = {}
    this.forgeMode = 'current'
    this.tree = null
    this.skillLevels = {}
    this.stance = null
    this.potions = null
    this.effects = []
    this.statusEffects = {}
    this.ruptureOrbs = 0
    this.colossus = false
    this.unholyVoid = false
    this.selected = null
  }

  item(sel: Selection): ItemView | null {
    const c = this.character
    if (!c || sel === null) return null
    return sel === 'weapon' ? c.weapon : (c.items[sel] ?? null)
  }

  weaponInfo(): WeaponInfo | null {
    return this.weaponRow ? (this.catalog?.weapons.find((w) => w.row === this.weaponRow) ?? null) : null
  }

  /** Attributes at the item's original hearts (edited or from the save). */
  attrs(sel: Selection): Attr[] {
    if (sel === null) return []
    if (sel === 'weapon') return this.weaponEdit ?? this.character?.weapon?.attributes ?? []
    return this.edits[sel] ?? this.item(sel)?.attributes ?? []
  }

  originalHearts(sel: Selection): number {
    return this.item(sel)?.hearts ?? 0
  }

  /** Hearts the build uses for an item (forge mode overrides the per-item choice).
   *  The gambler does not take pets or offhands (goblet, horn, trinket, belt). */
  heartsOf(sel: Selection): number {
    if (noForge(sel)) return this.originalHearts(sel)
    if (this.forgeMode === 'none') return 0
    if (this.forgeMode === 'max') return 10
    return this.hearts[key(sel)] ?? this.originalHearts(sel)
  }

  setHearts(sel: Selection, n: number) {
    this.forgeMode = 'current'
    this.hearts = { ...this.hearts, [key(sel)]: n }
  }

  /** Attributes as the item has them in this build (hearts applied). */
  effAttrs(sel: Selection): Attr[] {
    return scaleHearts(this.attrs(sel), this.originalHearts(sel), this.heartsOf(sel))
  }

  setAttrs(sel: Selection, attrs: Attr[]) {
    if (sel === null) return
    if (sel === 'weapon') this.weaponEdit = attrs
    else this.edits = { ...this.edits, [sel]: attrs }
  }

  /** Sets attribute `index` of an item from the value shown (with hearts applied).
   *  Items can carry the same tag twice (stacked mods), hence indexes. */
  setEffValue(sel: Selection, index: number, value: number) {
    if (!Number.isFinite(value)) return
    this.setAttrs(
      sel,
      this.attrs(sel).map((a, i) => {
        if (i !== index) return a
        const f = forgeExempt(a.tag) ? 1 : heartFactor(this.originalHearts(sel), this.heartsOf(sel))
        return { tag: a.tag, value: value / f }
      }),
    )
  }

  addAttr(sel: Selection, tag: string, shownValue: number) {
    if (!Number.isFinite(shownValue)) return
    const f = forgeExempt(tag) ? 1 : heartFactor(this.originalHearts(sel), this.heartsOf(sel))
    this.setAttrs(sel, [...this.attrs(sel), { tag, value: shownValue / f }])
  }

  removeAttr(sel: Selection, index: number) {
    this.setAttrs(
      sel,
      this.attrs(sel).filter((_, i) => i !== index),
    )
  }

  resetItem(sel: Selection) {
    if (sel === 'weapon') {
      this.weaponEdit = null
      this.weaponRow = null
    } else if (sel !== null) {
      const { [sel]: _, ...rest } = this.edits
      this.edits = rest
      const { [sel]: _t, ...tried } = this.tried
      this.tried = tried
    }
    const { [key(sel)]: _h, ...hearts } = this.hearts
    this.hearts = hearts
  }

  isEdited(sel: Selection): boolean {
    const own = this.heartsOf(sel) !== this.originalHearts(sel)
    if (sel === 'weapon') return own || this.weaponEdit !== null || this.weaponRow !== null
    return own || (sel !== null && sel in this.edits)
  }

  buildInput(withEdits: boolean): BuildInput {
    const items: Record<number, Attr[]> = {}
    let weapon: Attr[] | null = null
    if (withEdits && this.character) {
      this.character.items.forEach((_, i) => {
        if (this.isEdited(i)) items[i] = $state.snapshot(this.effAttrs(i))
      })
      if (this.isEdited('weapon')) weapon = $state.snapshot(this.effAttrs('weapon'))
    }
    return {
      items,
      weapon,
      weapon_row: withEdits ? this.weaponRow : null,
      tree: withEdits && this.tree ? $state.snapshot(this.tree) : null,
      skill_levels: withEdits ? Object.entries(this.skillLevels).map(([k, level]) => ({ table: k.split('/')[0], row: k.slice(k.indexOf('/') + 1), level })) : [],
      stance: this.stance,
      potions: this.potions,
      effects: [...$state.snapshot(this.effects), ...(this.unholyVoid ? [{ tag: VOID, value: 1 }] : [])],
      status_effects: $state.snapshot(this.statusEffects),
      rupture_orbs: this.ruptureOrbs,
      colossus: this.colossus,
    }
  }

  private seq = 0
  async recompute() {
    if (!this.character) return
    const n = ++this.seq
    try {
      const [stats, baseline] = await Promise.all([api.compute(this.buildInput(true)), api.compute(this.buildInput(false))])
      if (n !== this.seq) return
      this.stats = stats
      this.baseline = baseline
      this.error = null
      if (stats.steps * 2 > stats.max_steps) debug.log('warn', 'engine', `compute close to the step limit: ${stats.steps}/${stats.max_steps}`)
    } catch (e) {
      this.error = String(e)
    }
  }

  /** One probe per stat: a typical end-game roll (DT_Base_Item_Attributes value at item level 80). */
  probes(): [string, number][] {
    const c = this.catalog
    if (!c || !this.character) return []
    const out = new Map<string, number>()
    for (const a of c.affixes) {
      if (!a.can_roll || a.tag.startsWith('EasyRPG.Items.Modifiers.')) continue
      out.set(a.tag, Math.max(out.get(a.tag) ?? 0, a.value + a.per_level * 80))
    }
    const fallback = (tag: string) => (c.attributes[tag]?.value_type === 2 ? 0.1 : 50)
    const extra = [
      ...this.character.items.flatMap((i) => i.attributes.map((a) => a.tag)),
      ...(this.character.weapon?.attributes.map((a) => a.tag) ?? []),
      ...c.panel.flatMap((e) => (e.kind === 'row' ? [e.tag] : [])),
    ]
    for (const t of extra) {
      if (t.startsWith('EasyRPG.Items.Modifiers.') || t.endsWith('.Display') || t.endsWith('CombatRating')) continue
      if (!out.has(t)) out.set(t, fallback(t))
    }
    return [...out.entries()].filter(([, v]) => v > 0)
  }

  /** States the build already has: their mods need no special measure. */
  active(): Active {
    const attrs = new Set([...this.effects.map((e) => e.tag), ...(this.unholyVoid ? [VOID] : [])])
    return { status: this.statusEffects, orbs: this.ruptureOrbs, colossus: this.colossus, stance: this.stats?.stance, attrs }
  }

  /** State a mod needs to do anything (null: none, or already active). */
  condition(tag: string): Condition | null {
    return modCondition(tag, this.catalog, this.active())
  }

  /** Probe of what `value` of `tag` brings. A conditional mod is measured in its state; a mod
   *  that creates its state is measured as "state on" against the build without it. */
  private probeOf(tag: string, delta: number, removal: boolean): Probe {
    const c = this.condition(tag)
    if (!c) return [tag, delta]
    if (c.when.grants && removal) return { tag, delta: 0, when: c.when }
    return { tag, delta, when: c.when }
  }

  /** One removal probe (-value) per attribute the build's items carry, mods included. */
  removalProbes(): [string, Probe][] {
    if (!this.character) return []
    const out = new Map<string, Probe>()
    const sels: Selection[] = [...this.character.items.map((_, i) => i), 'weapon']
    for (const sel of sels) for (const a of this.effAttrs(sel)) if (a.value) out.set(probeKey(a.tag, -a.value), this.probeOf(a.tag, -a.value, true))
    return [...out.entries()]
  }

  private aseq = 0
  async analyze() {
    if (!this.character) return
    const n = ++this.aseq
    this.analyzing = true
    this.analyzeStarted = Date.now()
    try {
      const add = this.probes()
      const removals = this.removalProbes()
      const probes: Probe[] = [...add, ...removals.map(([, p]) => p)]
      const t = performance.now()
      const res = await api.analyze(this.character.save, this.buildInput(true), probes)
      if (n !== this.aseq) return
      debug.log('info', 'analyze', `${probes.length} stats analysed in ${(performance.now() - t).toFixed(0)} ms`)
      this.impacts = Object.fromEntries(res.slice(0, add.length).map((r) => [r.tag, r]))
      // Keyed by the request's values: the backend's JSON round trip may change the last digits.
      this.removals = Object.fromEntries(res.slice(add.length).map((r, k) => [removals[k][0], r]))
      this.analyzeError = null
    } catch (e) {
      if (n !== this.aseq) return
      this.analyzeError = String(e)
      debug.log('error', 'analyze', `stat analysis failed: ${e}`)
    } finally {
      if (n === this.aseq) this.analyzing = false
    }
  }

  /** Relative gain of the objective for one typical roll of `tag` (undefined = not analysed). */
  impact(tag: string): number | undefined {
    const i = this.impacts[tag]
    return i && this.objectiveGain(i)
  }

  /** Offhand ability `i` counts in the DPS total: ticked in the DPS tab, else when equipped. */
  offhandIncluded(i: number): boolean {
    const a = this.catalog?.abilities[i]
    return !!a && (this.dpsOffhands[a.row] ?? (this.stats?.dps.abilities[i]?.offhands ?? 0) > 0)
  }

  /** Main attack of the DPS total in a computed view: the stance combo or a weapon skill. */
  dpsMainRef(v: DpsView): { list: 'basic' | 'attacks'; i: number } {
    if (this.dpsMain === 'basic') return { list: 'basic', i: v.basic.findIndex((x) => x.stance === this.stats?.stance) }
    return { list: 'attacks', i: this.catalog?.attacks.findIndex((a) => a.row === this.dpsMain) ?? -1 }
  }

  /** DPS total of a computed view, split into main attack and offhands. */
  dpsParts(v: DpsView | undefined, boss: boolean): { main: number; off: number } {
    if (!v) return { main: 0, off: 0 }
    const pick = (x: { dps: number; dps_boss: number } | undefined) => (x ? (boss ? x.dps_boss : x.dps) : 0)
    const ref = this.dpsMainRef(v)
    let off = 0
    v.abilities.forEach((a, i) => this.offhandIncluded(i) && (off += pick(a)))
    return { main: pick(ref.list === 'basic' ? v.basic[ref.i] : v.attacks[ref.i]), off }
  }

  /** What the training dummy would show (5 s window): main attack + chosen offhands' meter. */
  dpsMeter(v: DpsView | undefined, boss: boolean): number {
    if (!v) return 0
    const ref = this.dpsMainRef(v)
    const main = ref.list === 'basic' ? v.basic[ref.i] : v.attacks[ref.i]
    let total = main ? (boss ? main.dps_boss : main.dps) : 0
    v.abilities.forEach((a, i) => this.offhandIncluded(i) && (total += boss ? a.meter_boss : a.meter))
    return total
  }

  /** Relative change of the DPS total for one probe (from the per-component changes). */
  private dpsGain(i: Impact, boss: boolean): number | undefined {
    const v = this.stats?.dps
    if (!v || !i.ability_dps) return undefined
    let base = 0
    let next = 0
    const add = (b: number | undefined, r: number | undefined) => {
      base += b ?? 0
      next += (b ?? 0) * (1 + (r ?? 0))
    }
    const ref = this.dpsMainRef(v)
    if (ref.list === 'basic') add(v.basic[ref.i]?.dps, i.basic_dps[ref.i])
    else add(v.attacks[ref.i]?.dps, i.attack_dps[ref.i])
    v.abilities.forEach((a, k) => this.offhandIncluded(k) && add(a.dps, i.ability_dps[k]))
    if (base <= 0) return undefined
    const g = next / base - 1
    return boss ? (1 + g) * (1 + i.boss_mult) - 1 : g
  }

  /** Gain of an impact on the chosen objective. */
  gainOf(i: Impact): number | undefined {
    return this.objectiveGain(i)
  }

  async loadInventory() {
    if (!this.character) return
    this.inventoryLoading = true
    try {
      this.inventory = await api.inventory(this.character.save)
      this.itemGains = {}
      this.itemGainsFor = ''
    } catch (e) {
      this.error = String(e)
    } finally {
      this.inventoryLoading = false
    }
  }

  /** Reads the saves again and takes them only when the game wrote one since (the Inventory view polls). */
  async refreshInventory() {
    if (!this.character || this.inventoryLoading) return
    try {
      const fresh = await api.inventory(this.character.save)
      const stamp = (cs: Container[]) => cs.map((c) => `${c.id}:${c.saved}`).join(',')
      if (stamp(fresh) === stamp(this.inventory)) return
      // Gains stay for the items that did not move or change.
      const same = (cs: Container[]) => new Map<string, string>(cs.flatMap((c) => c.items.map((it) => [`${c.id}#${it.index}`, JSON.stringify(it)] as [string, string])))
      const before = same(this.inventory)
      const after = same(fresh)
      this.itemGains = Object.fromEntries(Object.entries(this.itemGains).filter(([k]) => before.get(k) !== undefined && before.get(k) === after.get(k)))
      this.inventory = fresh
      debug.log('info', 'inventory', 'the game saved: inventory read again')
    } catch (e) {
      debug.log('warn', 'inventory', `refresh failed: ${e}`)
    }
  }

  buildKey(): string {
    return JSON.stringify(this.buildInput(true))
  }

  /** Measures every equipment item of the inventory and stash in its slot (rings: the better ring slot). */
  async evaluateInventory() {
    if (!this.character || this.evaluating) return
    this.evaluating = true
    try {
      const cands: { key: string; c: Candidate }[] = []
      for (const tab of this.inventory) {
        for (const it of tab.items) {
          const key = `${tab.id}#${it.index}`
          const base = { row: it.row, attributes: $state.snapshot(it.attributes), rarity: it.rarity, tier: it.tier, hearts: it.hearts }
          if (it.weapon) cands.push({ key, c: { ...base, slot: null } })
          else for (const slot of it.slots) cands.push({ key, c: { ...base, slot } })
        }
      }
      const build = this.buildInput(true)
      const res = await api.evaluateItems(this.character.save, build, cands.map((x) => x.c))
      const gains: Record<string, { impact: Impact; slot: number | null }> = {}
      cands.forEach(({ key, c }, i) => {
        const prev = gains[key]
        if (!prev || (this.objectiveGain(res[i]) ?? -Infinity) > (this.objectiveGain(prev.impact) ?? -Infinity)) gains[key] = { impact: res[i], slot: c.slot }
      })
      this.itemGains = gains
      this.itemGainsFor = JSON.stringify(build)
    } catch (e) {
      this.error = String(e)
    } finally {
      this.evaluating = false
    }
  }

  /** Puts an inventory item in the build: its stats in its slot (a weapon also replaces the weapon row). */
  tryItem(it: InvItem, slot: number | null) {
    const attrs = $state.snapshot(it.attributes)
    // The item's stats already include its own hearts: keep the forge factor at 1 (current hearts mode).
    if (slot === null) {
      this.weaponRow = it.row
      this.weaponEdit = attrs
      this.hearts = { ...this.hearts, weapon: this.originalHearts('weapon') }
    } else {
      this.edits = { ...this.edits, [slot]: attrs }
      this.tried = { ...this.tried, [slot]: { name: it.name, icon: it.icon, rarity: it.rarity, tier: it.tier } }
      this.hearts = { ...this.hearts, [slot]: this.originalHearts(slot) }
    }
    this.selected = slot === null ? 'weapon' : slot
    this.inventoryOpen = false
  }

  private objectiveGain(i: Impact): number | undefined {
    const o = this.objective
    if (o.kind === 'dps') return this.dpsGain(i, o.boss)
    if (o.kind === 'attack') return i.attacks[o.index]
    if (o.kind === 'ability') return i.abilities[o.index]
    return i.combat_rating
  }

  savedSkillLevel(table: string, row: string): number {
    return this.character?.skills.find((s) => s.table === table && s.row === row)?.level ?? 0
  }

  skillLevel(table: string, row: string): number {
    return this.skillLevels[`${table}/${row}`] ?? this.savedSkillLevel(table, row)
  }

  setSkillLevel(table: string, row: string, level: number, max: number) {
    if (!Number.isFinite(level)) return
    const l = Math.max(0, Math.min(Math.round(level), max > 0 ? max : Infinity))
    const { [`${table}/${row}`]: _, ...rest } = this.skillLevels
    this.skillLevels = l === this.savedSkillLevel(table, row) ? rest : { ...rest, [`${table}/${row}`]: l }
  }

  treeRows(): string[] {
    return this.tree ?? this.character?.tree ?? []
  }

  /** Skill points of the save: the ones spent in its tree plus the unspent ones. */
  treeBudget(): number {
    return (this.character?.tree.length ?? 0) + (this.character?.skill_points ?? 0)
  }

  /** Gain of the objective for a tree option (sum of its bonuses; undefined = not analysed).
   *  For an owned option (`owned`), what it brings: the loss when it is removed. */
  /** Gain of an option (`owned`: loss of removing it); `scale` levels at once (a crystal card's third level). */
  optionImpact(o: Pick<TreeOption, 'attributes'>, owned = false, scale = 1): number | undefined {
    let sum = 0
    let known = false
    for (const [t, v] of o.attributes) {
      if (!v) continue
      const i = this.treeImpacts[probeKey(t, (owned ? -v : v) * scale)]
      if (i && unquantified(t, i, this.statMods)) continue
      const g = i && this.objectiveGain(i)
      if (g === undefined) return undefined
      sum += owned ? -g : g
      known = true
    }
    // Only behaviour flags (procs, ability mods): the stats cannot tell their worth.
    return known || !o.attributes.length ? sum : NaN
  }

  private tseq = 0
  /** Impact of every bonus of the open tab on the current build (main tree: ~440 probes,
   *  other skills: ~250), only while the view is open. */
  async analyzeTree() {
    const tree = this.catalog?.skill_tree
    if (!this.character || !tree || !this.treeOpen) return
    const n = ++this.tseq
    this.treeAnalyzing = true
    try {
      const probes =
        this.skillsTab === 'tree' ? treeProbes(tree, new Set(this.treeRows())) : skillProbes(this.catalog?.skill_sets ?? [], (t, r) => this.skillLevel(t, r))
      const t = performance.now()
      const res = await api.analyze(this.character.save, this.buildInput(true), probes)
      if (n !== this.tseq) return
      debug.log('info', 'analyze', `skills: ${probes.length} bonuses analysed in ${(performance.now() - t).toFixed(0)} ms`)
      this.treeImpacts = { ...this.treeImpacts, ...Object.fromEntries(res.map((r, k) => [probeKey(...probes[k]), r])) }
      this.treeAnalyzeError = null
    } catch (e) {
      if (n !== this.tseq) return
      this.treeAnalyzeError = String(e)
      debug.log('error', 'analyze', `skill analysis failed: ${e}`)
    } finally {
      if (n === this.tseq) this.treeAnalyzing = false
    }
  }

  /** What an attribute of the build brings: the objective loss when it is removed
   *  (undefined = not analysed). Unlike `impact`, it sees caps: crit chance above a cap
   *  gains nothing when added but can cost a lot when removed. */
  contribution(tag: string, value: number): number | undefined {
    const i = this.removals[probeKey(tag, -value)]
    if (i && unquantified(tag, i, this.statMods)) return NaN
    const g = i && this.objectiveGain(i)
    if (g === undefined) return undefined
    // A mod that creates its state was measured as "state on": its gain is what it brings.
    return this.condition(tag)?.when.grants ? g : -g
  }

  /** Why a mod shows no number: loot/utility, or an effect handled in combat. */
  unquantifiedKind(tag: string): 'loot' | 'combat' {
    return isLootMod(tag) ? 'loot' : 'combat'
  }

  private pseq = 0
  /** Analyses the picker's choices (affix rolls of the open item, or mods) in their state. */
  async analyzePicker(choices: [string, number][]) {
    if (!this.character || !choices.length) return
    const n = ++this.pseq
    try {
      const res = await api.analyze(this.character.save, this.buildInput(true), choices.map(([t, v]) => this.probeOf(t, v, false)))
      if (n !== this.pseq) return
      this.pickerImpacts = Object.fromEntries(res.map((r, k) => [probeKey(...choices[k]), r]))
    } catch (e) {
      this.error = String(e)
    }
  }

  /** Gain of adding `value` of `tag` (the picker's own roll when analysed, else a typical roll). */
  addGain(tag: string, value: number): number | undefined {
    const i = this.pickerImpacts[probeKey(tag, value)]
    if (i && unquantified(tag, i, this.statMods)) return NaN
    return i ? this.objectiveGain(i) : this.impact(tag)
  }

  useless(tag: string): boolean {
    const v = this.impact(tag)
    return v !== undefined && Math.abs(v) < 1e-7
  }

  objectiveLabel(): string {
    const c = this.catalog
    const o = this.objective
    if (o.kind === 'dps') return o.boss ? t('objective.dpsBoss') : t('objective.dps')
    if (o.kind === 'attack') return c?.attacks[o.index]?.name ?? t('objective.attack')
    if (o.kind === 'ability') return c?.abilities[o.index]?.name ?? t('objective.ability')
    return t('objective.cr')
  }

  saveBuild(name: string) {
    if (!this.character) return
    const b: SavedBuild = {
      name,
      save: this.character.save,
      edits: $state.snapshot(this.edits),
      weapon: $state.snapshot(this.weaponEdit),
      weaponRow: this.weaponRow,
      hearts: $state.snapshot(this.hearts),
      forgeMode: this.forgeMode,
      tree: this.tree ? $state.snapshot(this.tree) : null,
      skillLevels: $state.snapshot(this.skillLevels),
      statusEffects: $state.snapshot(this.statusEffects),
      states: { orbs: this.ruptureOrbs, colossus: this.colossus, unholyVoid: this.unholyVoid },
      stance: this.stance,
      potions: this.potions,
      effects: $state.snapshot(this.effects),
    }
    this.builds = [...this.builds.filter((x) => x.name !== name), b]
    storeBuilds(this.builds)
  }

  loadBuild(build: SavedBuild) {
    // Builds live in $state: snapshot them (structuredClone cannot copy the proxies).
    const b = $state.snapshot(build)
    this.edits = b.edits
    this.tried = {}
    this.weaponEdit = b.weapon
    this.weaponRow = b.weaponRow ?? null
    this.hearts = b.hearts ?? {}
    this.forgeMode = b.forgeMode ?? 'current'
    this.tree = b.tree ?? null
    this.skillLevels = b.skillLevels ?? {}
    this.statusEffects = b.statusEffects ?? {}
    this.ruptureOrbs = b.states?.orbs ?? 0
    this.colossus = b.states?.colossus ?? false
    this.unholyVoid = b.states?.unholyVoid ?? false
    this.stance = b.stance
    this.potions = b.potions
    this.effects = b.effects
  }

  deleteBuild(name: string) {
    this.builds = this.builds.filter((x) => x.name !== name)
    storeBuilds(this.builds)
  }
}

/** A mod or skill flag whose effect never shows in the stats (loot, procs, ability behaviour
 *  the game handles in combat): no attribute moves, so "not quantified" rather than "useless".
 *  The DPS model can still see some (Lightning Plasma beams): those have a gain. */
function unquantified(tag: string, i: Impact, statMods: Set<string>): boolean {
  if (!isMod(tag) && !tag.startsWith('EasyRPG.Attributes.Skills.') && !tag.startsWith('EasyRPG.WeaponAttackModifier.')) return false
  // Read by the stat code yet nothing moved: no effect on this build (shown as 0).
  if (statMods.has(tag)) return false
  const all = [i.combat_rating, i.physical, i.elemental, i.armor, i.health, i.boss_mult, ...i.abilities, ...i.attacks, ...i.ability_dps, ...i.attack_dps, ...i.basic_dps]
  return i.changed === 0 && all.every((x) => Math.abs(x) < 1e-9)
}

/** The objective chosen last time (kept by App.svelte), else total DPS on bosses and elites. */
function loadObjective(): Objective {
  try {
    const o = JSON.parse(localStorage.getItem('dr-planner.objective') ?? 'null')
    if (o && typeof o === 'object' && typeof o.kind === 'string') return o
  } catch {
    // no storage or an old value: the default
  }
  return { kind: 'dps', boss: true }
}

function loadKinds(): Record<string, Kind> {
  try {
    return JSON.parse(localStorage.getItem(KINDS_KEY) ?? '{}')
  } catch {
    return {}
  }
}

function loadBuilds(): SavedBuild[] {
  try {
    return JSON.parse(localStorage.getItem(BUILDS_KEY) ?? '[]')
  } catch {
    return []
  }
}

function storeBuilds(b: SavedBuild[]) {
  try {
    localStorage.setItem(BUILDS_KEY, JSON.stringify(b))
  } catch {
    // storage unavailable: builds stay in memory for this session
  }
}

export const planner = new Planner()

// Dev server: this module holds app-wide state, so an edit reloads the page instead of swapping the module (a
// swapped copy would leave the views on two different states).
if (import.meta.hot) import.meta.hot.accept(() => location.reload())
