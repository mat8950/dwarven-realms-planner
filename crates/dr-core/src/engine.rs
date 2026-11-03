//! Stat engine: wires host objects like the game does (controller -> manager ->
//! character -> components) and runs the game's own `CalculateAttributes`.

use std::collections::BTreeMap;

use anyhow::{Context, Result};

use crate::gamedata::GameData;
use crate::value::{ObjId, StructVal, Value};
use crate::vm::Vm;

/// Everything the stat code reads that is not in the save, or that the user overrides.
#[derive(Clone, Debug, Default)]
pub struct Situation {
    /// `E_Stance` index (0 fists, 1 one-hand, 3 spear, ...). `None` = from equipped weapon.
    pub stance: Option<u8>,
    /// Potions in the belt. `None` = full (the game clamps to its computed max).
    pub available_potions: Option<i64>,
    pub rupture_orbs: i64,
    /// Active status effects: DT row name -> stacks (bloodlust, phasing...).
    pub status_effects: BTreeMap<String, i64>,
    /// Attributes granted by active buffs (what the game keeps in `EffectAttributes`).
    pub effect_attributes: Vec<(String, f64)>,
    /// `is Colossus Warrior Active?` (BP_Character_Player), read by the Colossus mods.
    pub colossus: bool,
}

#[derive(Debug, Default)]
pub struct Stats {
    /// Attribute tag -> value, as the stats panel reads them.
    pub attributes: BTreeMap<String, f64>,
    pub combat_rating: f64,
    /// Physical damage of a critical hit (`Get Base Player Damage FORCED(true)`).
    pub phys_crit: f64,
    /// Animation speed bonus from attack speed (`CalculateAttackSpeedIncrease`).
    pub attack_speed_bonus: f64,
    /// Cooldown speed factor (`CalculateCooldownReduction` "Cast Speed" = 1 + CDR%).
    pub cast_speed: f64,
    /// Unknown properties / calls hit during evaluation.
    pub missing: Vec<String>,
}

pub struct Engine<'g> {
    pub vm: Vm<'g>,
    pub attr: ObjId,
    pub character: ObjId,
    pub manager: ObjId,
    pub controller: ObjId,
    pub stance: ObjId,
    pub potions: ObjId,
    pub orbs: ObjId,
    pub equipment: ObjId,
    pub abilities: ObjId,
    pub race: ObjId,
    /// `ItemData` per equipment slot (index = `E_EquipmentSlot`), and the weapon in hand.
    pub items: Vec<Value>,
    pub weapon: Option<Value>,
    /// Stance of the weapon in hand.
    pub default_stance: u8,
    /// The save's `CharacterSkills` (STR_Skill: row handle + level) and stat point attributes.
    pub skills: Vec<Value>,
    stat_points: Value,
    /// `PlayerAttributes` computed from the save's skills.
    player_attributes: Value,
    /// Bonuses of the save's skills outside the main tree (stance skills, cards, crafting).
    other_skill_attributes: Value,
    /// `MakeSkillsAttributes` of single skills, by (table, row, level), computed on demand.
    skill_bonus: std::collections::HashMap<(String, String, i64), Value>,
}

/// DataTable of the main skill tree (one row per node, or per dropdown option).
pub const SKILL_TREE_TABLE: &str = "DT_GENERATED_SkillTree_Main";

impl<'g> Engine<'g> {
    pub fn new(game: &'g GameData) -> Result<Engine<'g>> {
        let mut vm = Vm::new(game);
        let attr = vm.spawn("BP_AttributesComponent_C");
        let character = vm.spawn("BP_Character_Player_C");
        let manager = vm.spawn("BP_PlayerManagerComponent_C");
        let controller = vm.spawn("BP_PlayerController_C");
        let stance = vm.spawn("BP_PlayerStanceSkillManagerComponent_C");
        let potions = vm.spawn("BP_PotionManager_C");
        let orbs = vm.spawn("BP_RuptureOrbsComponent_C");
        let equipment = vm.spawn("BP_EquipmentComponent_C");
        let inventory = vm.spawn("BP_ContainerComponent_C");
        let hotbar = vm.spawn("BP_HotbarComponent_C");
        let abilities = vm.spawn("BP_AbilitySystemComponent_C");
        let race = vm.spawn("BP_RaceManager_C");
        let pets = vm.spawn("BP_PetManager_C");

        for (prop, obj) in [
            ("AttributesComponent", attr),
            ("BP_PlayerStanceSkillManagerComponent", stance),
            ("AbilitySystemComponent", abilities),
            ("BP_RaceProgressionManager", race),
            ("EquipmentContainer", equipment),
            ("InventoryContainer", inventory),
            ("WeaponContainer", hotbar),
            ("PlayerManager", manager),
            ("MyController", controller),
            ("PetManager", pets),
        ] {
            vm.set(character, prop, Value::Object(obj));
            // Components keep the overrides set in the character blueprint (base attributes...).
            for (k, v) in game.component_template("BP_Character_Player_C", prop) {
                vm.objects[obj].props.insert(k, v);
            }
        }
        vm.set(manager, "PlayerCharacter", Value::Object(character));
        vm.set(controller, "ManagerComponent", Value::Object(manager));
        vm.set(controller, "BP_PotionManager", Value::Object(potions));
        vm.set(controller, "RuptureOrbsComponent", Value::Object(orbs));
        vm.controller = Value::Object(controller);

        // Engine-side calls with no blueprint body.
        vm.stub("GetOwner", move |_, _, _| Ok(Value::Object(character)));
        vm.stub("GetOwningPlayer", move |_, _, _| Ok(Value::Object(controller)));
        vm.stub("GetController", move |_, _, _| Ok(Value::Object(controller)));
        vm.stub("HasAuthority", |_, _, _| Ok(Value::Bool(true)));
        vm.stub("IsLocallyControlled", |_, _, _| Ok(Value::Bool(true)));

        let mut e = Engine { vm, attr, character, manager, controller, stance, potions, orbs, equipment, abilities, race, items: Vec::new(), weapon: None, default_stance: 0, skills: Vec::new(), stat_points: Value::None, player_attributes: Value::None, other_skill_attributes: Value::None, skill_bonus: Default::default() };
        e.vm.invoke(attr, "InitAttributesDependencies", &mut vec![Value::None])?;
        // Adds the default inventory slots to the base attributes.
        e.vm.invoke(character, "UserConstructionScript", &mut vec![])?;
        // Offhand ability base multipliers: the game adds one per DT_PlayerAbilities row
        // (verified against a live dump of BaseAttributes).
        let mut extra = Vec::new();
        for (_, row) in game.table("DT_PlayerAbilities")?.iter() {
            if let Value::Struct(r) = row {
                if let Some((_, Value::Struct(dm))) = r.fields.iter().find(|(k, _)| k.starts_with("DmgModifierAttribute")) {
                    let tag = dm.fields.iter().find(|(k, _)| k.starts_with("GameplayTag")).map(|(_, v)| v.name().to_string());
                    let val = dm.fields.iter().find(|(k, _)| k.starts_with("Value")).map(|(_, v)| v.f64());
                    if let (Some(t), Some(v)) = (tag, val) {
                        extra.push(attribute(&t, v));
                    }
                }
            }
        }
        if let Value::Array(base) = e.vm.objects[attr].props.entry("BaseAttributes".into()).or_insert(Value::Array(Vec::new())) {
            base.extend(extra);
        }
        Ok(e)
    }

    /// Builds an `ItemData` for a DT_Items row with the given attributes, like the game's
    /// `Make Item Data by Handle` (used to swap the weapon or another item).
    pub fn make_item(&mut self, row: &str, attrs: &[(String, f64)], rarity: u8, tier: i64, hearts: i64) -> Result<Value> {
        let itemslib = self.library("BP_ItemsLibrary_C");
        let handle = Value::Struct(Box::new(StructVal {
            ty: "DataTableRowHandle".into(),
            fields: vec![("DataTable".into(), Value::Asset("DT_Items".into())), ("RowName".into(), Value::Name(row.into()))],
        }));
        let list = Value::Array(attrs.iter().map(|(t, v)| attribute(t, *v)).collect());
        let name = self
            .vm
            .game
            .table("DT_Items")?
            .iter()
            .find(|(k, _)| &**k == row)
            .and_then(|(_, v)| match v {
                Value::Struct(s) => s.fields.iter().find(|(k, _)| k.starts_with("InGameName")).map(|(_, n)| n.clone()),
                _ => None,
            })
            .unwrap_or(Value::Str(row.into()));
        let r = self.call(itemslib, "Make Item Data by Handle", &[
            ("ItemHandle", handle),
            ("Amount", Value::Int(1)),
            ("Charges", Value::Int(0)),
            ("Generated Name", name),
            ("Generated Attributes", list),
            ("is Generated?", Value::Bool(true)),
            ("Generated Rarity", Value::Byte(rarity)),
            ("Item Tier", Value::Int(tier)),
            ("CraftingSpecks", Value::Int(0)),
        ])?;
        let mut item = r.get("Item").cloned().unwrap_or_default();
        if let Value::Struct(s) = &mut item {
            *s.get_mut(s.fields.iter().find(|(k, _)| k.starts_with("GabmleUpgradeAmount")).map(|(k, _)| k.clone()).as_deref().unwrap_or("GabmleUpgradeAmount")) = Value::Int(hearts);
        }
        Ok(item)
    }

    /// Stance an item puts the character in (`GetItemStance`).
    pub fn item_stance(&mut self, item: &Value) -> Result<u8> {
        let itemslib = self.library("BP_ItemsLibrary_C");
        Ok(self.call(itemslib, "GetItemStance", &[("ItemData", item.clone())])?.get("Stance").map(|v| v.i64() as u8).unwrap_or(0))
    }

    /// Runs a blueprint function with named inputs; returns all parameters after the call.
    /// `PlayerAttributes` for a skill list: skill bonuses plus the save's stat points.
    fn skill_attributes(&mut self, skills: &[Value]) -> Result<Value> {
        let attrs = self.skills_bonus(skills)?;
        self.sum_attributes(attrs, self.stat_points.clone())
    }

    fn skills_bonus(&mut self, skills: &[Value]) -> Result<Value> {
        let gamelib = self.library("BP_GameLibrary_C");
        let attrs = self.call(gamelib, "MakeSkillsAttributes", &[("Skills", Value::Array(skills.to_vec()))])?;
        Ok(attrs.get("Attributes").cloned().unwrap_or_default())
    }

    fn sum_attributes(&mut self, a: Value, b: Value) -> Result<Value> {
        let gamelib = self.library("BP_GameLibrary_C");
        let summed = self.call(gamelib, "SummAttributes", &[("Attributes1", a), ("Attributes2", b)])?;
        Ok(summed.get("ResultAttributes").cloned().unwrap_or_default())
    }

    /// Rows of the main skill tree owned in the save.
    pub fn tree_rows(&self) -> Vec<String> {
        self.skills.iter().filter(|s| is_tree_skill(s)).filter_map(skill_handle).map(|(_, r)| r).collect()
    }

    /// (table, row, level) of the save's skills outside the main tree.
    pub fn other_skills(&self) -> Vec<(String, String, i64)> {
        self.skills
            .iter()
            .filter(|s| !is_tree_skill(s))
            .filter_map(|s| {
                let (t, r) = skill_handle(s)?;
                Some((t, r, skill_level(s)))
            })
            .collect()
    }

    /// Replaces the main skill tree nodes of the save (None: the save's own tree) and the
    /// levels of other skills (`levels`: (table, row, level), 0 removes the skill).
    pub fn set_skills(&mut self, tree: Option<&[String]>, levels: &[(String, String, i64)]) -> Result<()> {
        if tree.is_none() && levels.is_empty() {
            self.vm.set(self.attr, "PlayerAttributes", self.player_attributes.clone());
            return Ok(());
        }
        // MakeSkillsAttributes sums the skills tag by tag, which takes ~0.4 s in the VM for a
        // whole save. Each skill's bonus is cached instead (and the save's other skills' total).
        let others = if levels.is_empty() {
            self.other_skill_attributes.clone()
        } else {
            let mut list = self.other_skills();
            for (t, r, l) in levels {
                match list.iter_mut().find(|(t2, r2, _)| t2 == t && r2 == r) {
                    Some(e) => e.2 = *l,
                    None => list.push((t.clone(), r.clone(), *l)),
                }
            }
            list.retain(|e| e.2 > 0);
            self.bonuses(&list)?
        };
        let rows: Vec<String> = match tree {
            Some(r) => r.to_vec(),
            None => self.tree_rows(),
        };
        let tree: Vec<(String, String, i64)> = rows.into_iter().map(|r| (SKILL_TREE_TABLE.to_string(), r, 1)).collect();
        let tree = self.bonuses(&tree)?;
        let skills = self.sum_attributes(others, tree)?;
        let attrs = self.sum_attributes(skills, self.stat_points.clone())?;
        self.vm.set(self.attr, "PlayerAttributes", attrs);
        Ok(())
    }

    /// Tag-wise sum of the cached bonuses of single skills.
    fn bonuses(&mut self, skills: &[(String, String, i64)]) -> Result<Value> {
        let template = self.skills.first().cloned();
        for key in skills {
            if !self.skill_bonus.contains_key(key) {
                let bonus = self.skills_bonus(&[make_skill(template.as_ref(), &key.0, &key.1, key.2)])?;
                self.skill_bonus.insert(key.clone(), bonus);
            }
        }
        Ok(merge_attributes(skills.iter().filter_map(|k| self.skill_bonus.get(k))))
    }

    pub fn call(&mut self, obj: ObjId, func: &str, inputs: &[(&str, Value)]) -> Result<BTreeMap<String, Value>> {
        let class = self.vm.objects[obj].class.clone();
        let (_, f) = self.vm.game.find_function(&class, func).with_context(|| format!("{class}::{func} not found"))?;
        let params: Vec<_> = f.params.iter().filter(|p| !p.ret).cloned().collect();
        let mut args: Vec<Value> = params
            .iter()
            .map(|p| match inputs.iter().find(|(n, _)| *n == &*p.name) {
                Some((_, v)) => v.clone(),
                None if &*p.name == "__WorldContext" => Value::Object(self.character),
                None => Value::None,
            })
            .collect();
        let ret = self.vm.invoke(obj, func, &mut args)?;
        let mut out: BTreeMap<String, Value> = params.iter().zip(args).map(|(p, v)| (p.name.to_string(), v)).collect();
        out.insert("ReturnValue".into(), ret);
        Ok(out)
    }

    fn library(&mut self, class: &str) -> ObjId {
        match self.vm.objects.iter().position(|o| &*o.class == class) {
            Some(o) => o,
            None => self.vm.spawn(class),
        }
    }

    /// Loads stat points, skills and equipped items from a save's `HostPlayerData`.
    pub fn load_player(&mut self, player: &StructVal) -> Result<()> {
        let field = |name: &str| player.fields.iter().find(|(k, _)| k.starts_with(name)).map(|(_, v)| v.clone()).unwrap_or_default();
        // Level gates racial traits; the race picks the DT_PlayerRaces row (E_CharacterRace order).
        let level = field("Level_").i64();
        self.vm.set(self.manager, "CurrentLevel", Value::Int(level));
        let race = match field("CustumizationData_") {
            Value::Struct(c) => c.fields.iter().find(|(k, _)| k.starts_with("Race_")).map(|(_, v)| v.i64()).unwrap_or(0),
            _ => 0,
        };
        let row = ["HUMAN", "ORC", "DWARF", "UNDEAD"].get(race as usize).copied().unwrap_or("HUMAN");
        let desc = self.vm.game.table("DT_PlayerRaces")?.iter().find(|(k, _)| &**k == row).map(|(_, v)| v.clone()).unwrap_or_default();
        self.vm.set(self.race, "LocalRaceDesc", desc);
        let end_zone = field("EndZoneLevel_").i64();
        self.vm.stub("Get End Zone Reached Level", move |_, _, a| {
            if let Some(out) = a.get_mut(1) {
                *out = Value::Int(end_zone);
            }
            Ok(Value::None)
        });

        let skill = |n: &str| field(n);
        self.call(self.stance, "Initialize Skill Levels", &[
            ("Spear Skill", skill("SpearSkill_")),
            ("Mauls Skill", skill("MaulsSkill_")),
            ("OneHand Skill", skill("OneHandSkill_")),
            ("TwoHand Skill", skill("TwoHandSkill_")),
            ("Bow Skill", skill("BowSkill_")),
            ("Magery Skill", skill("MagerySkill_")),
            ("Unarmed Skill", skill("UnarmedSkill_")),
            ("Scyth Skill", skill("ScytheSkill_")),
        ])?;

        let itemslib = self.library("BP_ItemsLibrary_C");

        self.skills = match field("CharacterSkills_") {
            Value::Array(a) => a,
            _ => Vec::new(),
        };
        self.stat_points = field("Attributes_");
        self.player_attributes = self.skill_attributes(&self.skills.clone())?;
        let other: Vec<Value> = self.skills.iter().filter(|s| !is_tree_skill(s)).cloned().collect();
        self.other_skill_attributes = self.skills_bonus(&other)?;
        self.vm.set(self.attr, "PlayerAttributes", self.player_attributes.clone());

        // Equipment index = E_EquipmentSlot value; the weapon in hand is the first hotbar slot.
        self.items.clear();
        if let Value::Array(saved) = field("EquipmentItems_") {
            for it in saved {
                let r = self.call(itemslib, "ItemTransferToData", &[("Item", it), ("GabmleUpgradeAmount", Value::Int(0))])?;
                self.items.push(r.get("ItemData").cloned().unwrap_or_default());
            }
        }
        self.weapon = None;
        if let Value::Array(hotbar) = field("HotbarItems_") {
            if let Some(it) = hotbar.into_iter().next() {
                let r = self.call(itemslib, "ItemTransferToData", &[("Item", it), ("GabmleUpgradeAmount", Value::Int(0))])?;
                self.weapon = r.get("ItemData").cloned();
            }
        }
        self.default_stance = match self.weapon.clone() {
            Some(w) => self.call(itemslib, "GetItemStance", &[("ItemData", w)])?.get("Stance").map(|v| v.i64() as u8).unwrap_or(0),
            None => 0,
        };
        self.apply_items()
    }

    /// Recomputes `EquipmentAttributes` / `SelectedItemAttributes` from the current items,
    /// through the game's own item functions.
    pub fn apply_items(&mut self) -> Result<()> {
        let itemslib = self.library("BP_ItemsLibrary_C");
        let eq = self.call(itemslib, "GetItemsEquipmentAttributes", &[("Items", Value::Array(self.items.clone()))])?;
        self.vm.set(self.attr, "EquipmentAttributes", eq.get("EquipmentAttributes").cloned().unwrap_or_default());
        let selected = match self.weapon.clone() {
            Some(w) => self.call(itemslib, "GetItemEquipmentAttributes", &[("Item", w)])?.get("EquipmentAttributes").cloned().unwrap_or_default(),
            None => Value::Array(Vec::new()),
        };
        self.vm.set(self.attr, "SelectedItemAttributes", selected);
        Ok(())
    }

    /// `ItemData` of equipment slot `slot`, or of the weapon when `slot` is `None`.
    pub fn item(&self, slot: Option<usize>) -> Option<&Value> {
        match slot {
            Some(i) => self.items.get(i),
            None => self.weapon.as_ref(),
        }
    }

    pub fn item_mut(&mut self, slot: Option<usize>) -> Option<&mut Value> {
        match slot {
            Some(i) => self.items.get_mut(i),
            None => self.weapon.as_mut(),
        }
    }

    /// Replaces an item's attributes (affixes and modifiers). Call `apply_items` afterwards.
    pub fn set_item_attributes(&mut self, slot: Option<usize>, attrs: &[(String, f64)]) {
        let list = Value::Array(attrs.iter().map(|(t, v)| attribute(t, *v)).collect());
        if let Some(Value::Struct(it)) = self.item_mut(slot) {
            if let Some((_, v)) = it.fields.iter_mut().find(|(k, _)| k.starts_with("EquipmentAttributes")) {
                *v = list;
            }
        }
    }

    pub fn compute(&mut self, s: &Situation) -> Result<Stats> {
        let (vm, stance) = (&mut self.vm, self.stance);
        vm.set(stance, "CurrentStance", Value::Byte(s.stance.unwrap_or(self.default_stance)));
        vm.set(self.potions, "Available Potions", Value::Int(s.available_potions.unwrap_or(9999)));
        vm.set(self.orbs, "Amount of Orbs", Value::Int(s.rupture_orbs));
        vm.set(self.character, "is Colossus Warrior Active?", Value::Bool(s.colossus));
        let effects: Vec<Value> = s.effect_attributes.iter().map(|(t, v)| attribute(t, *v)).collect();
        vm.set(self.attr, "EffectAttributes", Value::Array(effects));
        let effects = s.status_effects.clone();
        vm.stub("CheckStatusEffectByHandle", move |_, _, a| {
            let row = match a.first() {
                Some(Value::Struct(h)) => h.get("RowName").map(|v| v.name().to_string()).unwrap_or_default(),
                _ => String::new(),
            };
            let stacks = effects.get(&row).copied().unwrap_or(0);
            if a.len() > 2 {
                a[1] = Value::Bool(stacks > 0);
                a[2] = Value::Int(stacks);
            }
            Ok(Value::None)
        });
        vm.missing.clear();

        self.calculate()?;
        if s.available_potions.is_none() {
            // Potion mods read the belt before ModifyMaxPotions clamps it: recompute
            // with the belt full at the max the first pass found, as in game.
            let max = self.vm.get(self.potions, "Max Potions");
            self.vm.set(self.potions, "Available Potions", max);
            self.calculate()?;
        }

        let mut stats = Stats::default();
        if let Value::Map(m) = self.vm.get(self.attr, "AttributesNamed") {
            for (k, v) in m {
                let val = match &v {
                    Value::Struct(s) => s.fields.iter().find(|(f, _)| f.starts_with("Value")).map(|(_, x)| x.f64()).unwrap_or(0.0),
                    other => other.f64(),
                };
                stats.attributes.insert(k.name().to_string(), val);
            }
        }
        let cr = self.call(self.manager, "Get Combat Rating Attribute", &[])?;
        stats.combat_rating = cr.get("Combat Rating").map(Value::f64).unwrap_or(0.0);
        let crit = self.call(self.character, "Get Base Player Damage FORCED", &[("CriticalHit", Value::Bool(true))])?;
        stats.phys_crit = crit.get("Damage").map(Value::f64).unwrap_or(0.0);
        let gamelib = self.library("BP_GameLibrary_C");
        stats.attack_speed_bonus = self.call(gamelib, "CalculateAttackSpeedIncrease", &[])?.get("ReturnValue").map(Value::f64).unwrap_or(0.0);
        stats.cast_speed = self.call(gamelib, "CalculateCooldownReduction", &[])?.get("Cast Speed").map(Value::f64).unwrap_or(1.0);
        stats.missing = self.vm.missing.iter().cloned().collect();
        Ok(stats)
    }

    fn calculate(&mut self) -> Result<()> {
        let mut args = vec![Value::Bool(false), Value::Byte(0), Value::None, Value::None];
        self.vm.invoke(self.attr, "CalculateAttributes", &mut args)?;
        Ok(())
    }
}

/// An `STR_Attribute` value (tag + float value).
pub fn attribute(tag: &str, value: f64) -> Value {
    Value::Struct(Box::new(StructVal {
        ty: "STR_Attribute".into(),
        fields: vec![
            ("GameplayTag_2_9682C35B48A5160809F4BFB3A6AECB2C".into(), Value::tag(tag)),
            ("Value_5_43BD33CA455ADE70795B28BCBCDB22CD".into(), Value::Float(value as f32 as f64)),
        ],
    }))
}

/// (table, row) of an STR_Skill save entry.
fn skill_handle(skill: &Value) -> Option<(String, String)> {
    let Value::Struct(s) = skill else { return None };
    let Value::Struct(h) = &s.fields.iter().find(|(k, _)| k.starts_with("Handle"))?.1 else { return None };
    Some((h.get("DataTable")?.name().to_string(), h.get("RowName")?.name().to_string()))
}

/// Tag-wise sum of attribute arrays (FAttribute structs), like AddAttributesGameLibrary.
fn merge_attributes<'a>(lists: impl Iterator<Item = &'a Value>) -> Value {
    let mut out: Vec<Value> = Vec::new();
    let mut index: std::collections::HashMap<String, usize> = Default::default();
    let parts = |v: &Value| -> Option<(String, f64)> {
        let Value::Struct(s) = v else { return None };
        let tag = s.fields.iter().find(|(k, _)| k.starts_with("GameplayTag"))?.1.name().to_string();
        Some((tag, s.fields.iter().find(|(k, _)| k.starts_with("Value"))?.1.f64()))
    };
    for list in lists {
        let Value::Array(a) = list else { continue };
        for e in a {
            let Some((tag, v)) = parts(e) else { continue };
            match index.get(&tag) {
                Some(&i) => {
                    if let Value::Struct(s) = &mut out[i] {
                        if let Some(f) = s.fields.iter_mut().find(|(k, _)| k.starts_with("Value")) {
                            f.1 = Value::Float(f.1.f64() + v);
                        }
                    }
                }
                None => {
                    index.insert(tag, out.len());
                    out.push(e.clone());
                }
            }
        }
    }
    Value::Array(out)
}

fn is_tree_skill(skill: &Value) -> bool {
    skill_handle(skill).is_some_and(|(t, _)| t == SKILL_TREE_TABLE)
}

fn skill_level(skill: &Value) -> i64 {
    match skill {
        Value::Struct(s) => s.fields.iter().find(|(k, _)| k.starts_with("Level")).map(|(_, v)| v.i64()).unwrap_or(0),
        _ => 0,
    }
}

/// An STR_Skill entry (row handle + level), shaped like the save's entries.
fn make_skill(template: Option<&Value>, table: &str, row: &str, level: i64) -> Value {
    let (handle_key, level_key) = match template {
        Some(Value::Struct(t)) => (
            t.fields.iter().find(|(k, _)| k.starts_with("Handle")).map(|(k, _)| k.clone()).unwrap_or_else(|| "Handle".into()),
            t.fields.iter().find(|(k, _)| k.starts_with("Level")).map(|(k, _)| k.clone()).unwrap_or_else(|| "Level".into()),
        ),
        _ => ("Handle".into(), "Level".into()),
    };
    let handle = StructVal { ty: "DataTableRowHandle".into(), fields: vec![("DataTable".into(), Value::Asset(table.into())), ("RowName".into(), Value::Name(row.into()))] };
    Value::Struct(Box::new(StructVal { ty: "STR_Skill".into(), fields: vec![(handle_key, Value::Struct(Box::new(handle))), (level_key, Value::Int(level))] }))
}
