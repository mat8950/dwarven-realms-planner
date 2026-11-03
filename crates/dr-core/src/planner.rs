//! Serializable layer between the stat engine and the UI: character and item
//! views read from a save, and build edits sent back for recomputation.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::catalog::{AbilityInfo, AttackInfo, BasicAttack, Catalog, asset_path, table_rows};
use crate::engine::{Engine, Situation};
use crate::gamedata::GameData;
use crate::save;
use crate::value::{StructVal, Value};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Attr {
    pub tag: String,
    pub value: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ItemView {
    /// `E_EquipmentSlot` value; `None` for the weapon in hand.
    pub slot: Option<u8>,
    pub row: String,
    pub name: String,
    pub rarity: u8,
    pub tier: i64,
    /// Gambler hearts (`GabmleUpgradeAmount`, 0-10): each one multiplied the item's stats by 1.1.
    pub hearts: i64,
    pub icon: Option<String>,
    pub attributes: Vec<Attr>,
}

#[derive(Serialize, Clone, Debug)]
pub struct CharacterView {
    pub save: String,
    pub name: String,
    pub level: i64,
    pub race: String,
    pub default_stance: u8,
    pub items: Vec<ItemView>,
    pub weapon: Option<ItemView>,
    /// Owned rows of the main skill tree, and the save's unspent skill points.
    pub tree: Vec<String>,
    pub skill_points: i64,
    /// Levels of the other skills in the save.
    pub skills: Vec<SkillLevel>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SkillLevel {
    pub table: String,
    pub row: String,
    pub level: i64,
}

#[derive(Serialize, Clone, Debug)]
pub struct SaveEntry {
    pub path: String,
    pub name: String,
    pub level: i64,
    pub modified: u64,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct BuildInput {
    /// Attributes per equipment slot; slots not listed keep the save's items.
    #[serde(default)]
    pub items: BTreeMap<u8, Vec<Attr>>,
    #[serde(default)]
    pub weapon: Option<Vec<Attr>>,
    /// Replace the weapon by another DT_Items row (keeps rarity, tier and hearts).
    #[serde(default)]
    pub weapon_row: Option<String>,
    /// Owned rows of the main skill tree (None: the save's tree).
    #[serde(default)]
    pub tree: Option<Vec<String>>,
    /// Levels of other skills (stance skills, crystal cards, crafting) that differ from the save.
    #[serde(default)]
    pub skill_levels: Vec<SkillLevel>,
    #[serde(default)]
    pub stance: Option<u8>,
    #[serde(default)]
    pub potions: Option<i64>,
    #[serde(default)]
    pub effects: Vec<Attr>,
    /// Active status effects of `DT_StatusEffects` (row -> stacks).
    #[serde(default)]
    pub status_effects: BTreeMap<String, i64>,
    /// Rupture essence orbs held, and the Colossus Warrior state.
    #[serde(default)]
    pub rupture_orbs: i64,
    #[serde(default)]
    pub colossus: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct StatsView {
    pub attributes: BTreeMap<String, f64>,
    pub combat_rating: f64,
    /// Damage index per offhand ability / weapon attack, in catalog order.
    pub abilities: Vec<f64>,
    pub attacks: Vec<f64>,
    pub dps: DpsView,
    pub max_potions: i64,
    pub stance: u8,
    /// Engine calls / properties the extracted blueprints lacked (stubbed as empty).
    pub missing: Vec<String>,
    /// Debug info: peak interpreted steps of one call (limit `MAX_STEPS`) and engine time.
    pub steps: u64,
    pub max_steps: u64,
    pub elapsed_ms: f64,
}

/// Concrete damage of one weapon attack, against normal enemies and bosses.
#[derive(Serialize, Clone, Debug, Default)]
pub struct AttackDps {
    pub hit: f64,
    pub crit: f64,
    /// Expected damage per attack (crit chance capped at 100%).
    pub avg: f64,
    pub cast_time: f64,
    /// Attacks per second allowed by the animation, and sustainable with energy regen.
    pub burst_rate: f64,
    pub energy_cost: f64,
    pub sustained_rate: f64,
    pub energy_limited: bool,
    pub dps: f64,
    pub dps_boss: f64,
}

/// Basic-attack combo of a stance: hits loop through the combo animations.
#[derive(Serialize, Clone, Debug, Default)]
pub struct BasicDps {
    pub stance: u8,
    pub hits: usize,
    /// Seconds for one full combo, and hits per second.
    pub cycle_time: f64,
    pub rate: f64,
    pub hit: f64,
    pub crit: f64,
    pub avg: f64,
    pub dps: f64,
    pub dps_boss: f64,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct AbilityDps {
    /// Damage of one hit, and the hits of one proc on one target (beams x ticks...).
    pub per_proc: f64,
    pub per_proc_boss: f64,
    pub hits: f64,
    /// How the hits are counted ("5 rayons x 11 ticks"), when the ability actor is modelled.
    pub hits_detail: Option<String>,
    /// Equipped offhand items granting the ability (sets the base cooldown).
    pub offhands: usize,
    pub cooldown: Option<f64>,
    /// Time between procs: cooldown + wait for the next weapon hit (and proc chance roll).
    pub cycle: Option<f64>,
    pub dps: f64,
    pub dps_boss: f64,
    /// What the training dummy shows: damage counted in its 5 s window from the first hit / 5.
    pub meter: f64,
    pub meter_boss: f64,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct DpsView {
    pub boss_mult: f64,
    /// Crit chance of weapon hits (capped at 1), the uncapped total, and the offhands' one.
    pub crit_chance: f64,
    pub crit_total: f64,
    pub offhand_crit_chance: f64,
    pub attack_speed_bonus: f64,
    pub cast_speed: f64,
    pub energy_regen: f64,
    pub attacks: Vec<AttackDps>,
    pub abilities: Vec<AbilityDps>,
    /// One entry per stance with a basic-attack combo (catalog order).
    pub basic: Vec<BasicDps>,
}

/// Impact of adding `delta` of `tag` to the build, as relative changes (0.01 = +1%).
#[derive(Serialize, Clone, Debug)]
pub struct Impact {
    pub tag: String,
    pub delta: f64,
    pub combat_rating: f64,
    pub physical: f64,
    pub elemental: f64,
    pub armor: f64,
    pub health: f64,
    pub abilities: Vec<f64>,
    pub attacks: Vec<f64>,
    /// DPS of each offhand ability, weapon attack and basic combo (same order as DpsView),
    /// and the boss multiplier. The UI combines them into the DPS total it shows.
    pub ability_dps: Vec<f64>,
    pub attack_dps: Vec<f64>,
    pub basic_dps: Vec<f64>,
    pub boss_mult: f64,
    /// Attributes whose value changed (the probed one aside). 0: the effect is invisible in the
    /// stats (a proc or a behaviour the game handles in combat).
    pub changed: usize,
}

/// A stat to probe, possibly in a game state (`when`).
#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum Probe {
    Plain(String, f64),
    When { tag: String, delta: f64, when: When },
}

impl Probe {
    fn parts(&self) -> (&str, f64, Option<&When>) {
        match self {
            Probe::Plain(t, d) => (t, *d, None),
            Probe::When { tag, delta, when } => (tag, *delta, Some(when)),
        }
    }
}

/// A game state some mods need: active status effects (row, stacks), attributes the state
/// grants (Unholy Void's `DamageCircle.Base`), rupture essence orbs, Colossus Warrior, a stance.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct When {
    #[serde(default)]
    pub status: Vec<(String, i64)>,
    #[serde(default)]
    pub attrs: Vec<(String, f64)>,
    #[serde(default)]
    pub orbs: Option<i64>,
    #[serde(default)]
    pub colossus: bool,
    #[serde(default)]
    pub stance: Option<u8>,
    /// The mod itself grants the state (AllowPhasing grants the Phasing Trance): the state is
    /// active only with the mod, the base build stays as it is.
    #[serde(default)]
    pub grants: bool,
}

impl When {
    /// Sets the state; status effects also bring their attributes x stacks, like in `prepare`.
    fn apply(&self, s: &mut Situation, status_attributes: &HashMap<String, Vec<(String, f64)>>) {
        for (row, n) in &self.status {
            s.status_effects.insert(row.clone(), *n);
            for (tag, v) in status_attributes.get(row).into_iter().flatten() {
                s.effect_attributes.push((tag.clone(), v * *n as f64));
            }
        }
        s.effect_attributes.extend(self.attrs.iter().cloned());
        if let Some(o) = self.orbs {
            s.rupture_orbs = o;
        }
        s.colossus |= self.colossus;
        if self.stance.is_some() {
            s.stance = self.stance;
        }
    }
}

/// Relative changes between a base and a probed build.
fn impact(tag: &str, delta: f64, base: (&crate::engine::Stats, &Metrics, &DpsView), probe: (&crate::engine::Stats, &Metrics, &DpsView)) -> Impact {
    let (st0, m0, d0) = base;
    let (st, m, d) = probe;
    // From a zero baseline any gain counts as +100% (it must not look useless).
    let rel = |a: f64, b: f64| if b.abs() > 1e-12 { a / b - 1.0 } else if a.abs() > 1e-12 { a.signum() } else { 0.0 };
    let each = |a: Vec<f64>, b: Vec<f64>| a.iter().zip(&b).map(|(a, b)| rel(*a, *b)).collect();
    let changed = st
        .attributes
        .iter()
        .filter(|(k, v)| k.as_str() != tag && (st0.attributes.get(*k).copied().unwrap_or(0.0) - **v).abs() > 1e-9 * v.abs().max(1.0))
        .count();
    Impact {
        ability_dps: each(d.abilities.iter().map(|x| x.dps).collect(), d0.abilities.iter().map(|x| x.dps).collect()),
        attack_dps: each(d.attacks.iter().map(|x| x.dps).collect(), d0.attacks.iter().map(|x| x.dps).collect()),
        basic_dps: each(d.basic.iter().map(|x| x.dps).collect(), d0.basic.iter().map(|x| x.dps).collect()),
        boss_mult: rel(d.boss_mult, d0.boss_mult),
        changed,
        tag: tag.to_string(),
        delta,
        combat_rating: rel(m.cr, m0.cr),
        physical: rel(m.phys, m0.phys),
        elemental: rel(m.ele, m0.ele),
        armor: rel(m.armor, m0.armor),
        health: rel(m.hp, m0.hp),
        abilities: m.abilities.iter().zip(&m0.abilities).map(|(a, b)| rel(*a, *b)).collect(),
        attacks: m.attacks.iter().zip(&m0.attacks).map(|(a, b)| rel(*a, *b)).collect(),
    }
}

struct Metrics {
    cr: f64,
    phys: f64,
    ele: f64,
    armor: f64,
    hp: f64,
    abilities: Vec<f64>,
    attacks: Vec<f64>,
}

/// Holds the loaded character and recomputes stats for edited builds.
pub struct Planner<'g> {
    pub game: &'g GameData,
    abilities: Vec<AbilityInfo>,
    /// `CharacterAttributes` per stack of each status effect.
    status_attributes: HashMap<String, Vec<(String, f64)>>,
    attacks: Vec<AttackInfo>,
    basic: Vec<BasicAttack>,
    engine: Engine<'g>,
    /// Original items, to reset slots the build does not override.
    original: (Vec<Value>, Option<Value>),
    icons: HashMap<String, String>,
}

impl<'g> Planner<'g> {
    pub fn new(game: &'g GameData, catalog: &Catalog) -> Result<Planner<'g>> {
        let mut icons = HashMap::new();
        for (row, r) in table_rows(game, "EasySurvivalRPG/Blueprints/DataTables/DT_Items.json")? {
            if let Some(p) = r["Icon"]["ObjectPath"].as_str() {
                icons.insert(row, asset_path(p));
            }
        }
        let status_attributes = catalog.status_effects.iter().map(|e| (e.row.clone(), e.attributes.clone())).collect();
        Ok(Planner { game, status_attributes, abilities: catalog.abilities.clone(), attacks: catalog.attacks.clone(), basic: catalog.basic_attacks.clone(), engine: Engine::new(game)?, original: (Vec::new(), None), icons })
    }

    pub fn open(&mut self, path: &Path) -> Result<CharacterView> {
        let sav = save::load(path)?;
        let Some(Value::Struct(player)) = sav.get("HostPlayerData").cloned() else { bail!("no HostPlayerData in {}", path.display()) };
        // Load into a fresh engine first: a save that fails keeps the previous character.
        let mut engine = Engine::new(self.game)?;
        engine.load_player(&player)?;
        self.engine = engine;
        self.original = (self.engine.items.clone(), self.engine.weapon.clone());

        let field = |p: &StructVal, n: &str| p.fields.iter().find(|(k, _)| k.starts_with(n)).map(|(_, v)| v.clone()).unwrap_or_default();
        let race = match field(&player, "CustumizationData_") {
            Value::Struct(c) => field(&c, "Race_").i64() as u8,
            _ => 0,
        };
        let race = self.game.enums.get("E_CharacterRace").and_then(|e| e.iter().find(|(v, _)| *v == race)).map(|(_, n)| n.clone()).unwrap_or_default();
        let items = self.engine.items.iter().enumerate().map(|(i, it)| self.item_view(Some(i as u8), it)).collect();
        let weapon = self.engine.weapon.as_ref().map(|w| self.item_view(None, w));
        Ok(CharacterView {
            save: path.display().to_string(),
            name: field(&player, "PlayerName_").name().to_string(),
            level: field(&player, "Level_").i64(),
            race,
            default_stance: self.engine.default_stance,
            items,
            weapon,
            tree: self.engine.tree_rows(),
            skills: self.engine.other_skills().into_iter().map(|(table, row, level)| SkillLevel { table, row, level }).collect(),
            skill_points: field(&player, "AvailableSkillPoints_").i64(),
        })
    }

    fn item_view(&self, slot: Option<u8>, item: &Value) -> ItemView {
        let Value::Struct(s) = item else {
            return ItemView { slot, row: String::new(), name: String::new(), rarity: 0, tier: 0, hearts: 0, icon: None, attributes: Vec::new() };
        };
        let f = |n: &str| s.fields.iter().find(|(k, _)| k.starts_with(n)).map(|(_, v)| v.clone()).unwrap_or_default();
        let row = match f("ItemHandle") {
            Value::Struct(h) => h.get("RowName").map(|v| v.name().to_string()).unwrap_or_default(),
            _ => String::new(),
        };
        ItemView {
            slot,
            icon: self.icons.get(&row).cloned(),
            name: f("Name_").name().to_string(),
            rarity: f("Rarity").i64() as u8,
            tier: f("ItemTier").i64(),
            hearts: f("GabmleUpgradeAmount").i64(),
            attributes: attrs_of(&f("EquipmentAttributes")),
            row,
        }
    }

    /// Applies a build's items and situation to the engine; returns the situation used.
    fn prepare(&mut self, b: &BuildInput) -> Result<Situation> {
        self.engine.items = self.original.0.clone();
        self.engine.weapon = self.original.1.clone();
        let levels: Vec<(String, String, i64)> = b.skill_levels.iter().map(|s| (s.table.clone(), s.row.clone(), s.level)).collect();
        self.engine.set_skills(b.tree.as_deref(), &levels)?;
        for (slot, attrs) in &b.items {
            self.engine.set_item_attributes(Some(*slot as usize), &to_pairs(attrs));
        }
        let mut stance = b.stance;
        if let Some(row) = &b.weapon_row {
            let (rarity, tier, hearts) = match &self.original.1 {
                Some(Value::Struct(w)) => {
                    let f = |n: &str| w.fields.iter().find(|(k, _)| k.starts_with(n)).map(|(_, v)| v.i64()).unwrap_or(0);
                    (f("Rarity") as u8, f("ItemTier"), f("GabmleUpgradeAmount"))
                }
                _ => (3, 1, 0),
            };
            let attrs = b.weapon.clone().map(|w| to_pairs(&w)).unwrap_or_else(|| self.original.1.as_ref().map(|w| to_pairs(&attrs_of(&item_attrs(w)))).unwrap_or_default());
            let item = self.engine.make_item(row, &attrs, rarity, tier, hearts)?;
            if stance.is_none() {
                stance = Some(self.engine.item_stance(&item)?);
            }
            self.engine.weapon = Some(item);
        } else if let Some(w) = &b.weapon {
            self.engine.set_item_attributes(None, &to_pairs(w));
        }
        self.engine.apply_items()?;
        // Status effects add their CharacterAttributes x stacks to EffectAttributes, like the
        // ability system does; the stub of CheckStatusEffectByHandle reports their stacks.
        let mut effect_attributes = to_pairs(&b.effects);
        for (row, stacks) in b.status_effects.iter().filter(|(_, n)| **n > 0) {
            for (tag, v) in self.status_attributes.get(row).into_iter().flatten() {
                effect_attributes.push((tag.clone(), v * *stacks as f64));
            }
        }
        Ok(Situation { stance, available_potions: b.potions, effect_attributes, status_effects: b.status_effects.clone(), rupture_orbs: b.rupture_orbs, colossus: b.colossus })
    }

    pub fn compute(&mut self, b: &BuildInput) -> Result<StatsView> {
        let t = std::time::Instant::now();
        self.engine.vm.peak_steps = 0;
        let situation = self.prepare(b)?;
        let stats = self.engine.compute(&situation)?;
        let m = self.metrics(&stats);
        let dps = self.dps(&stats, &m, situation.stance.unwrap_or(self.engine.default_stance));
        Ok(StatsView {
            dps,
            attributes: stats.attributes,
            combat_rating: stats.combat_rating,
            abilities: m.abilities,
            attacks: m.attacks,
            max_potions: self.engine.vm.get(self.engine.potions, "Max Potions").i64(),
            stance: situation.stance.unwrap_or(self.engine.default_stance),
            missing: stats.missing,
            steps: self.engine.vm.peak_steps,
            max_steps: crate::vm::MAX_STEPS,
            elapsed_ms: t.elapsed().as_secs_f64() * 1000.0,
        })
    }

    /// For each probe, the relative change of every metric when `delta` of its tag is added to
    /// the build (as an extra flat bonus, like an affix). A probe with a `when` condition is
    /// measured with that game state active, against the build in the same state.
    pub fn analyze(&mut self, b: &BuildInput, probes: &[Probe]) -> Result<Vec<Impact>> {
        let mut situation = self.prepare(b)?;
        let first = self.engine.compute(&situation)?;
        // Pin the potion count so each probe is a single pass.
        situation.available_potions = Some(situation.available_potions.unwrap_or_else(|| self.engine.vm.get(self.engine.potions, "Max Potions").i64()));
        let mut bases: HashMap<String, (crate::engine::Stats, Metrics, DpsView)> = HashMap::new();
        let mut out = Vec::with_capacity(probes.len());
        let mut first = Some(first);
        for p in probes {
            let (tag, delta, when) = p.parts();
            let mut s = situation.clone();
            // The base is measured in the state, unless the mod is what grants it.
            let base_when = when.filter(|w| !w.grants);
            if let Some(w) = base_when {
                w.apply(&mut s, &self.status_attributes);
            }
            let key = base_when.map(|w| serde_json::to_string(w).unwrap_or_default()).unwrap_or_default();
            if !bases.contains_key(&key) {
                let st = match (key.is_empty(), first.take()) {
                    (true, Some(st)) => st,
                    _ => self.engine.compute(&s)?,
                };
                let m = self.metrics(&st);
                let d = self.dps(&st, &m, s.stance.unwrap_or(self.engine.default_stance));
                bases.insert(key.clone(), (st, m, d));
            }
            if let Some(w) = when.filter(|w| w.grants) {
                w.apply(&mut s, &self.status_attributes);
            }
            s.effect_attributes.push((tag.to_string(), delta));
            let st = self.engine.compute(&s)?;
            let m = self.metrics(&st);
            let d = self.dps(&st, &m, s.stance.unwrap_or(self.engine.default_stance));
            let (st0, m0, d0) = &bases[&key];
            out.push(impact(tag, delta, (st0, m0, d0), (&st, &m, &d)));
        }
        Ok(out)
    }

    /// Damage indexes. Offhand abilities follow BP_Character_Player `Try to Spawn Player Ability`:
    /// `Get Base Player Elemental Damage` (on average ElementalDamage.Display: elemental damage with
    /// its % bonus) x `GetDamageMultiplierForOffhandAbility`. They crit only with the OffhandCrit
    /// mod (chance = total crit chance x 0.1), and deal 0 with the elemental-as-physical mods.
    /// Weapon attacks are estimated as the average weapon hit x attack
    /// multiplier x (1 + attack damage bonus): the multiplier is applied in combat actors not
    /// extracted yet.
    fn metrics(&self, st: &crate::engine::Stats) -> Metrics {
        let (a, cr) = (&st.attributes, st.combat_rating);
        let g = |t: &str| a.get(t).copied().unwrap_or(0.0);
        let vm = &self.engine.vm;
        let elem = |e: &str| match e {
            "Fire" | "Lightning" | "Arcane" => vm.get(self.engine.attr, &format!("{e} Damage Multiplier")).f64(),
            _ => 1.0,
        };
        let p = offhand_crit_chance(a);
        let crit = 1.0 - p + p * g("EasyRPG.Attributes.Base.CriticalDamage");
        // PhysAsElementalCheck: these mods turn the offhands' own damage off.
        let phys_as_ele = g("EasyRPG.Items.Modifiers.BonusEleAsBonusPhys") >= 1.0 || g("EasyRPG.Items.Modifiers.EleAsBasePhys") >= 1.0;
        let ele_base = if phys_as_ele { 0.0 } else { g("EasyRPG.Attributes.Base.ElementalDamage.Display") };
        let abilities = self
            .abilities
            .iter()
            .map(|ab| {
                let aff: f64 = ab.affinities.iter().map(|c| g(&format!("{c}.Damage%Bonus"))).sum();
                ele_base * (g(&ab.damage_tag) + g(&ab.bonus_tag) + aff) * elem(&ab.element) * crit
            })
            .collect();
        let hit = weapon_hit(st);
        let attacks = self.attacks.iter().map(|at| hit * at.multiplier * (1.0 + g(&at.damage_bonus_tag))).collect();
        Metrics {
            cr,
            phys: g("EasyRPG.Attributes.Base.Damage.Display"),
            ele: g("EasyRPG.Attributes.Base.ElementalDamage.Display"),
            armor: g("EasyRPG.Attributes.Base.Armor"),
            hp: g("EasyRPG.Attributes.Base.MaxHealth"),
            abilities,
            attacks,
        }
    }
}

impl Planner<'_> {
    /// Per-attack and per-offhand damage and DPS (normal enemy / boss).
    /// `stance`: the stance in use, whose basic combo triggers the offhand procs.
    fn dps(&self, st: &crate::engine::Stats, m: &Metrics, stance: u8) -> DpsView {
        let a = &st.attributes;
        let g = |t: &str| a.get(t).copied().unwrap_or(0.0);
        let boss_mult = 1.0 + g("EasyRPG.Attributes.DamageSystem.Damage.Boss%");
        let c = g("EasyRPG.Attributes.Base.CriticalChance").clamp(0.0, 1.0);
        let regen = g("EasyRPG.Attributes.Base.EnergyRegeneration");
        // Weapon hits (TraceDealDamage_*, GetBaseDamage) deal `Get Base Player Damage`: physical
        // damage only. Elemental damage feeds offhands and mods, not weapon hits.
        let base_hit = g("EasyRPG.Attributes.Base.Damage.Display");
        let avg_hit = weapon_hit(st);
        let has = |t: &str| g(t) >= 1.0;
        let attacks = self
            .attacks
            .iter()
            .map(|at| {
                let mult = at.multiplier * (1.0 + g(&at.damage_bonus_tag));
                let (hit, crit) = (base_hit * mult, st.phys_crit * mult);
                let avg = avg_hit * mult;
                let play = at.in_play_rate + if at.attack_speed { st.attack_speed_bonus } else { 0.0 };
                let cast_time = at.anim_finish / (at.anim_rate * play).max(1e-6);
                let burst = 1.0 / cast_time.max(1e-6);
                // Start Weapon Attack / Start Secondary Ability Anim / Reduce Energy.
                let secondary = if at.kind == 1 { 1.0 + g("EasyRPG.Items.Modifiers.SecondaryBuild.DoubleEnergyMoreDamage") } else { 1.0 };
                let cost = if has("EasyRPG.Items.Modifiers.ProcsTake100EnergyHighDamage") { 100.0 } else { at.energy * secondary + g(&at.energy_reduction_tag) };
                let cost = cost.max(0.0) * if has("EasyRPG.Items.Modifiers.EnergyConsumptionReduceHalf") { 0.5 } else { 1.0 };
                let sustained = if cost > 0.0 && regen > 0.0 { burst.min(regen / cost) } else { burst };
                AttackDps {
                    hit,
                    crit,
                    avg,
                    cast_time,
                    burst_rate: burst,
                    energy_cost: cost,
                    sustained_rate: sustained,
                    energy_limited: sustained < burst,
                    dps: avg * sustained,
                    dps_boss: avg * sustained * boss_mult,
                }
            })
            .collect();
        let basic: Vec<BasicDps> = self
            .basic
            .iter()
            .map(|b| {
                let cycle: f64 = b.steps.iter().map(|s| s.time / (s.rate * (s.in_play_rate + if s.attack_speed { st.attack_speed_bonus } else { 0.0 })).max(1e-6)).sum();
                let rate = b.steps.len() as f64 / cycle.max(1e-6);
                BasicDps {
                    stance: b.stance,
                    hits: b.steps.len(),
                    cycle_time: cycle,
                    rate,
                    hit: base_hit,
                    crit: st.phys_crit,
                    avg: avg_hit,
                    dps: avg_hit * rate,
                    dps_boss: avg_hit * rate * boss_mult,
                }
            })
            .collect();
        // Offhands proc on weapon hits (Try to Spawn All On Hit Offhand Procs): once the cooldown
        // is over, the next hit (and the proc chance roll) starts the next one.
        let hit_rate = basic.iter().find(|b| b.stance == stance).map(|b| b.rate).unwrap_or(0.0);
        let item_tags: Vec<Vec<String>> = self.engine.items.iter().map(|it| attrs_of(&item_attrs(it)).into_iter().map(|x| x.tag).collect()).collect();
        let abilities = self
            .abilities
            .iter()
            .zip(&m.abilities)
            .map(|(ab, per_proc)| {
                // Goblet, horn, trinket and belt are the offhand slots.
                let n = [11usize, 12, 13, 15].iter().filter(|s| item_tags.get(**s).is_some_and(|t| t.iter().any(|x| x.starts_with(&format!("{}.", ab.tag)) || *x == ab.damage_tag))).count();
                let cdb: f64 = ab.affinities.iter().map(|c| g(&format!("{c}.CooldownBonus"))).sum();
                // Calculate Offhand Ability Cooldown: the 1/2/3-offhand cooldown, x0.9 with a 4th one,
                // / (1 + affinity cooldown bonus) / (1 + cooldown reduction).
                let four = if n >= 4 { 0.9 } else { 1.0 };
                let cooldown = ab.cooldowns.map(|cd| cd[n.clamp(1, 3) - 1] * four / (st.cast_speed.max(1e-6) * (1.0 + cdb)));
                let ph = proc_hits(&ab.row, &g);
                // Not equipped: the hypothetical ability procs on every hit.
                let chance = match g(&format!("{}.ProcChance", ab.tag)) {
                    p if p > 0.0 && n > 0 => p.min(1.0),
                    _ => 1.0,
                };
                let wait = if hit_rate > 0.0 { (1.0 / chance - 0.5) / hit_rate } else { 0.0 };
                let cycle = cooldown.map(|cd| cd + wait);
                let dps = cycle.map(|cy| per_proc * ph.hits / cy.max(1e-6)).unwrap_or(0.0);
                let meter = cycle.map(|cy| per_proc * meter_hits(&ph, cy, DUMMY_SECONDS) / DUMMY_SECONDS).unwrap_or(0.0);
                AbilityDps {
                    per_proc: *per_proc,
                    per_proc_boss: per_proc * boss_mult,
                    hits: ph.hits,
                    hits_detail: ph.detail,
                    offhands: n,
                    cooldown,
                    cycle,
                    dps,
                    dps_boss: dps * boss_mult,
                    meter,
                    meter_boss: meter * boss_mult,
                }
            })
            .collect();

        DpsView {
            boss_mult,
            crit_chance: c,
            crit_total: g("EasyRPG.Attributes.Base.CriticalChance"),
            offhand_crit_chance: offhand_crit_chance(a),
            attack_speed_bonus: st.attack_speed_bonus,
            cast_speed: st.cast_speed,
            energy_regen: regen,
            attacks,
            abilities,
            basic,
        }
    }
}

/// Hits of one proc on one target, from the ability actors. Only Lightning Plasma is modelled
/// (BP_PlayerSkill_LightningPlasma, BP_LightningBeamSingle); the others count one hit.
fn proc_hits(row: &str, g: &impl Fn(&str) -> f64) -> ProcHits {
    // IsPlayerSkillLearned: the value counts from 1.
    let learned = |t: &str| {
        let v = g(t);
        if v >= 1.0 { v } else { 0.0 }
    };
    match row {
        "LightningPlasma" => {
            let m = |x: &str| format!("EasyRPG.Attributes.Abilities.LightningPlasma.Modifier.{x}");
            let extra = learned(&m("AdditionalBeam")).trunc();
            let grow = learned(&m("GrowLarger"));
            let tick_mod = learned(&m("Ticks"));
            // The plasma spawns 1 + AdditionalBeam beams; with GrowLarger each deals +5% per extra beam and level.
            let beams = extra + 1.0;
            let beam_damage = if extra >= 1.0 && grow > 0.0 { 1.0 + 0.05 * extra * grow } else { 1.0 };
            // Each beam ticks 8 + Ticks times (+GrowLarger, at most +3), every 0.33 s - 0.044 x GrowLarger
            // (0.198 s from 3), each tick x (1 + 0.1 x Ticks), hitting every overlapping enemy.
            let base = (8.0 + tick_mod).trunc();
            let ticks = if grow < 3.0 { (base + grow).trunc() } else { base + 3.0 };
            let interval = if grow < 3.0 { 0.33 - 0.044 * grow } else { 0.198 };
            let per_tick = beams * beam_damage * (1.0 + 0.1 * tick_mod);
            ProcHits {
                hits: per_tick * ticks,
                ticks,
                interval,
                detail: Some(format!("{beams} rayon{} × {ticks} ticks ({interval:.3} s), ×{:.2} par tick", if beams > 1.0 { "s" } else { "" }, beam_damage * (1.0 + 0.1 * tick_mod))),
            }
        }
        _ => ProcHits { hits: 1.0, ticks: 1.0, interval: 0.0, detail: None },
    }
}

/// Hits of one proc on one target, spread over `ticks` ticks `interval` s apart.
struct ProcHits {
    hits: f64,
    ticks: f64,
    interval: f64,
    detail: Option<String>,
}

/// BP_Building_Dummy counts damage for `Seconds to Timer` (5 s) from the first hit.
const DUMMY_SECONDS: f64 = 5.0;

/// Hits counted by the dummy: procs every `cycle` s from t = 0, each tick inside the window.
fn meter_hits(p: &ProcHits, cycle: f64, window: f64) -> f64 {
    let per_tick = p.hits / p.ticks.max(1.0);
    let mut hits = 0.0;
    let mut t = 0.0;
    while t < window {
        let ticks = if p.interval > 0.0 { ((window - t) / p.interval).floor() + 1.0 } else { 1.0 };
        hits += per_tick * ticks.min(p.ticks);
        t += cycle.max(1e-3);
    }
    hits
}

/// GetCritDamageForOffhandCrit: with the OffhandCrit mod, offhands crit with 10% of the total
/// crit chance (so the total matters up to 1000%).
fn offhand_crit_chance(a: &BTreeMap<String, f64>) -> f64 {
    let g = |t: &str| a.get(t).copied().unwrap_or(0.0);
    if g("EasyRPG.Items.Modifiers.OffhandCrit") >= 1.0 { (g("EasyRPG.Attributes.Base.CriticalChance") * 0.1).clamp(0.0, 1.0) } else { 0.0 }
}

/// Expected weapon hit: physical damage, critical with the capped crit chance.
fn weapon_hit(st: &crate::engine::Stats) -> f64 {
    let a = &st.attributes;
    let c = a.get("EasyRPG.Attributes.Base.CriticalChance").copied().unwrap_or(0.0).clamp(0.0, 1.0);
    (1.0 - c) * a.get("EasyRPG.Attributes.Base.Damage.Display").copied().unwrap_or(0.0) + c * st.phys_crit
}

fn item_attrs(item: &Value) -> Value {
    match item {
        Value::Struct(s) => s.fields.iter().find(|(k, _)| k.starts_with("EquipmentAttributes")).map(|(_, v)| v.clone()).unwrap_or_default(),
        _ => Value::None,
    }
}

fn to_pairs(a: &[Attr]) -> Vec<(String, f64)> {
    a.iter().map(|x| (x.tag.clone(), x.value)).collect()
}

fn attrs_of(v: &Value) -> Vec<Attr> {
    let Value::Array(a) = v else { return Vec::new() };
    a.iter()
        .filter_map(|e| match e {
            Value::Struct(s) => Some(Attr {
                tag: s.fields.iter().find(|(k, _)| k.starts_with("GameplayTag"))?.1.name().to_string(),
                value: s.fields.iter().find(|(k, _)| k.starts_with("Value"))?.1.f64(),
            }),
            _ => None,
        })
        .collect()
}

/// Default Proton save folder of the game.
pub fn default_save_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join("Games/steamapps/compatdata/2015240/pfx/drive_c/users/steamuser/AppData/Local/ProjectAlpha/Saved/SaveGames")
}

/// Character saves in `dir` (skips backups, world saves and settings).
pub fn list_saves(dir: &Path) -> Result<Vec<SaveEntry>> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if !n.ends_with(".sav") || n.contains("_BU") || n.contains("_WRL") || ["GameSettings.sav", "EnhancedInputUserSettings.sav", "perma_characters_list.sav"].contains(&n.as_str()) {
            continue;
        }
        let modified = p.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
        let (name, level) = match save::load(&p) {
            Ok(s) => match s.get("HostPlayerData") {
                Some(Value::Struct(h)) => {
                    let f = |n: &str| h.fields.iter().find(|(k, _)| k.starts_with(n)).map(|(_, v)| v.clone()).unwrap_or_default();
                    (f("PlayerName_").name().to_string(), f("Level_").i64())
                }
                _ => continue,
            },
            Err(_) => continue,
        };
        out.push(SaveEntry { path: p.display().to_string(), name, level, modified });
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(out)
}
