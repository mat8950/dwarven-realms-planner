// Mirrors the serde types of dr-core (catalog.rs, planner.rs).

export interface AttrInfo {
  name: string
  description: string
  /** 0 integer, 1 float, 2 percent (value * 100), 3 hidden */
  value_type: number
  digits: number
  icon: string | null
}

export type PanelEntry =
  | { kind: 'row'; tag: string; always_sign: boolean; collapse_if_zero: boolean }
  | { kind: 'sep' }

export interface SlotInfo {
  value: number
  name: string
  silhouette: string | null
}

export interface Affix {
  row: string
  tag: string
  value: number
  per_level: number
  min_item_level: number
  can_roll: boolean
}

export interface AffixPool {
  name: string
  item_type: string
  pool: string[]
  banned: string[]
}

export interface Modifier {
  row: string
  tag: string
  slot: number
  in_use: boolean
  weapon_affinity: number
  name: string
  description: string
}

export interface WeaponInfo {
  row: string
  name: string
  icon: string | null
  stance: number
  item_level: number
  attributes: [string, number][]
}

export interface AbilityInfo {
  row: string
  tag: string
  name: string
  description: string
  element: string
  affinities: string[]
  damage_tag: string
  damage_base: number
  bonus_tag: string
  icon: string | null
  cooldowns: [number, number, number] | null
}

export interface AttackInfo {
  row: string
  stance: number
  kind: number
  name: string
  description: string
  multiplier: number
  energy: number
  damage_bonus_tag: string
  energy_reduction_tag: string
  icon: string | null
  anim_finish: number
  anim_rate: number
  in_play_rate: number
  attack_speed: boolean
}

export interface TreeOption {
  row: string
  name: string
  description: string
  icon: string | null
  attributes: [string, number][]
}

export interface TreeNode {
  id: string
  /** Canvas position (top-left) and size, in widget units. */
  x: number
  y: number
  w: number
  h: number
  root: boolean
  links: number[]
  /** Rows the node can own: its own row, or one dropdown option. */
  options: TreeOption[]
}

export interface SkillTree {
  nodes: TreeNode[]
}

export interface SkillInfo {
  row: string
  name: string
  description: string
  icon: string | null
  /** 0 when unlimited. */
  max_level: number
  requires: string
  /** Bonus per level. */
  attributes: [string, number][]
}

export interface SkillSet {
  table: string
  name: string
  stance: number | null
  skills: SkillInfo[]
}

export interface SkillLevel {
  table: string
  row: string
  level: number
}

export interface StatusEffectInfo {
  row: string
  name: string
  description: string
  icon: string | null
  positive: boolean
  max_stack: number
  duration: number
  /** Stats per stack. */
  attributes: [string, number][]
}

export interface Catalog {
  build: string
  /** Language of the game texts (locres folder) and the languages the data has. */
  lang: string
  languages: string[]
  attributes: Record<string, AttrInfo>
  panel: PanelEntry[]
  slots: SlotInfo[]
  stances: [number, string][]
  rarities: [number, string][]
  affixes: Affix[]
  affix_pools: AffixPool[]
  affix_groups: Record<string, string[]>
  modifiers: Modifier[]
  weapons: WeaponInfo[]
  abilities: AbilityInfo[]
  attacks: AttackInfo[]
  skill_tree: SkillTree
  skill_sets: SkillSet[]
  status_effects: StatusEffectInfo[]
  /** Mod tags the stat code reads: a zero gain there means "no effect on this build". */
  stat_mods: string[]
}

export interface Attr {
  tag: string
  value: number
}

export interface ItemView {
  slot: number | null
  row: string
  name: string
  rarity: number
  tier: number
  hearts: number
  icon: string | null
  attributes: Attr[]
}

export interface CharacterView {
  save: string
  name: string
  level: number
  race: string
  default_stance: number
  items: ItemView[]
  weapon: ItemView | null
  /** Owned rows of the main skill tree, and unspent skill points. */
  tree: string[]
  skill_points: number
  /** Levels of the other skills (stance skills, cards, crafting). */
  skills: SkillLevel[]
}

export interface SaveEntry {
  path: string
  name: string
  level: number
  modified: number
}

export interface BuildInput {
  items: Record<number, Attr[]>
  weapon: Attr[] | null
  weapon_row: string | null
  tree: string[] | null
  skill_levels: SkillLevel[]
  stance: number | null
  potions: number | null
  effects: Attr[]
  /** Active status effects (DT_StatusEffects row -> stacks). */
  status_effects: Record<string, number>
  rupture_orbs: number
  colossus: boolean
}

export interface AttackDps {
  hit: number
  crit: number
  avg: number
  cast_time: number
  burst_rate: number
  energy_cost: number
  sustained_rate: number
  energy_limited: boolean
  dps: number
  dps_boss: number
}

export interface AbilityDps {
  /** Damage of one hit, and hits of one proc on one target (beams × ticks…). */
  per_proc: number
  per_proc_boss: number
  hits: number
  hits_detail: string | null
  offhands: number
  cooldown: number | null
  /** Time between procs: cooldown + wait for the next weapon hit. */
  cycle: number | null
  dps: number
  dps_boss: number
  /** Training dummy reading: damage in its 5 s window from the first hit / 5. */
  meter: number
  meter_boss: number
}

export interface BasicDps {
  stance: number
  hits: number
  cycle_time: number
  rate: number
  hit: number
  crit: number
  avg: number
  dps: number
  dps_boss: number
}

export interface DpsView {
  boss_mult: number
  /** Weapon hit crit chance (capped at 1), uncapped total, and the offhands' (OffhandCrit mod). */
  crit_chance: number
  crit_total: number
  offhand_crit_chance: number
  attack_speed_bonus: number
  cast_speed: number
  energy_regen: number
  attacks: AttackDps[]
  abilities: AbilityDps[]
  basic: BasicDps[]
}

export interface StatsView {
  attributes: Record<string, number>
  combat_rating: number
  abilities: number[]
  attacks: number[]
  dps: DpsView
  max_potions: number
  stance: number
  missing: string[]
  /** Debug: peak interpreted steps of one engine call, its limit, engine time (ms). */
  steps: number
  max_steps: number
  elapsed_ms: number
}

/** Relative change of each metric when `delta` of `tag` is added (0.01 = +1%). */
export interface Impact {
  tag: string
  delta: number
  combat_rating: number
  physical: number
  elemental: number
  armor: number
  health: number
  abilities: number[]
  attacks: number[]
  /** Attributes that changed (the probed one aside): 0 = invisible in the stats. */
  changed: number
  /** DPS of each offhand, weapon attack and basic combo (DpsView order), and the boss multiplier. */
  ability_dps: number[]
  attack_dps: number[]
  basic_dps: number[]
  boss_mult: number
}

/** `dps`: the DPS total of the DPS tab (chosen main attack + chosen offhands). */
/** One point of a character's progression history (dr_core::history::Snapshot). */
export interface Snapshot {
  save_modified: number
  captured: number
  name: string
  level: number
  race: string
  stance: number
  combat_rating: number
  dps: number
  dps_boss: number
  dummy_boss: number
  attributes: Record<string, number>
  items: ItemView[]
  tree: string[]
  skills: SkillLevel[]
  skill_points: number
}

export type Objective = { kind: 'dps'; boss: boolean } | { kind: 'cr' } | { kind: 'attack'; index: number } | { kind: 'ability'; index: number }
