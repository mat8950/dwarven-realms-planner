//! Static data the UI needs: attribute names / formats / icons, the stats panel
//! layout, equipment slots, affixes and equipment modifiers (monograms).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value as Json;

use crate::gamedata::{GameData, enum_value};
use crate::value::short_field;

#[derive(Serialize, Clone, Debug)]
pub struct AttrInfo {
    pub name: String,
    pub description: String,
    /// 0 integer, 1 float, 2 percent (value * 100), 3 hidden.
    pub value_type: u8,
    pub digits: u8,
    /// Asset path under `data/assets` (`NhanceSpellIconsMegaBundle/CircleIcons/198.webp`).
    pub icon: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PanelEntry {
    Row { tag: String, always_sign: bool, collapse_if_zero: bool },
    Sep,
}

#[derive(Serialize, Clone, Debug)]
pub struct SlotInfo {
    pub value: u8,
    pub name: String,
    pub silhouette: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Affix {
    pub row: String,
    pub tag: String,
    pub value: f64,
    pub per_level: f64,
    pub min_item_level: i64,
    pub can_roll: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct AffixPool {
    pub name: String,
    pub item_type: String,
    pub pool: Vec<String>,
    pub banned: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Modifier {
    pub row: String,
    pub tag: String,
    pub slot: u8,
    pub in_use: bool,
    pub weapon_affinity: u8,
    pub name: String,
    pub description: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct WeaponInfo {
    pub row: String,
    pub name: String,
    pub icon: Option<String>,
    /// `E_Stance` value of the weapon.
    pub stance: u8,
    pub item_level: i64,
    /// Attributes of the un-rolled item (`EquipmentAttributes` of the DT_Items row).
    pub attributes: Vec<(String, f64)>,
}

#[derive(Serialize, Clone, Debug)]
pub struct AbilityInfo {
    pub row: String,
    pub tag: String,
    pub name: String,
    pub description: String,
    /// `Fire`, `Lightning`, `Arcane` (from its damage type class), or empty.
    pub element: String,
    pub affinities: Vec<String>,
    pub damage_tag: String,
    pub damage_base: f64,
    pub bonus_tag: String,
    pub icon: Option<String>,
    /// Base cooldown (s) with 1, 2 or 3+ offhands granting the ability; `None` = no cooldown.
    pub cooldowns: Option<[f64; 3]>,
}

#[derive(Serialize, Clone, Debug)]
pub struct AttackInfo {
    pub row: String,
    pub stance: u8,
    /// 0 primary, 1 secondary (`E_AttackType`).
    pub kind: u8,
    pub name: String,
    pub description: String,
    pub multiplier: f64,
    pub energy: f64,
    pub damage_bonus_tag: String,
    pub energy_reduction_tag: String,
    pub icon: Option<String>,
    /// Animation timing: the attack ends at `anim_finish` s of an animation played at
    /// `anim_rate` x (`in_play_rate` + attack speed bonus when `attack_speed`).
    pub anim_finish: f64,
    pub anim_rate: f64,
    pub in_play_rate: f64,
    pub attack_speed: bool,
}

/// One hit of a basic-attack combo: the next hit starts `time` s into this animation
/// (its ContinueAttack window), played at `rate` x (`in_play_rate` + attack speed bonus).
#[derive(Serialize, Clone, Debug)]
pub struct ComboStep {
    pub anim: String,
    pub time: f64,
    pub rate: f64,
    pub in_play_rate: f64,
    pub attack_speed: bool,
}

/// Basic attack (left click) of a stance: `TryAttack` plays `<Stance>FirstAttackAnim`, then
/// each animation's ContinueAttack notify chains the next one.
#[derive(Serialize, Clone, Debug)]
pub struct BasicAttack {
    pub stance: u8,
    pub steps: Vec<ComboStep>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Catalog {
    pub build: String,
    /// Language of the game texts, and the languages the extracted data has.
    pub lang: String,
    pub languages: Vec<String>,
    pub attributes: BTreeMap<String, AttrInfo>,
    pub panel: Vec<PanelEntry>,
    pub slots: Vec<SlotInfo>,
    pub stances: Vec<(u8, String)>,
    pub rarities: Vec<(u8, String)>,
    pub affixes: Vec<Affix>,
    pub affix_pools: Vec<AffixPool>,
    /// Named groups of affix rows (`RandomStat`, `BaseModifiers`, ...).
    pub affix_groups: BTreeMap<String, Vec<String>>,
    pub modifiers: Vec<Modifier>,
    pub weapons: Vec<WeaponInfo>,
    pub abilities: Vec<AbilityInfo>,
    pub attacks: Vec<AttackInfo>,
    pub basic_attacks: Vec<BasicAttack>,
    pub skill_tree: SkillTree,
    pub skill_sets: Vec<SkillSet>,
    pub status_effects: Vec<StatusEffectInfo>,
    /// Mod tags the stat code reads (BP_AttributesComponent): a zero gain for one of them means
    /// "no effect on this build", not "an effect the stats cannot show".
    pub stat_mods: Vec<String>,
}

fn stat_mods(gd: &GameData) -> Vec<String> {
    let path = gd.root.join("data/extracted/blueprints/EasySurvivalRPG/Blueprints/Components/BP_AttributesComponent.json");
    let text = fs::read_to_string(path).unwrap_or_default();
    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut rest = text.as_str();
    while let Some(i) = rest.find("Items.Modifiers.") {
        let tail = &rest[i..];
        let end = tail.find(|c: char| c == '"' || c == '\\' || c == ',' || c.is_whitespace()).unwrap_or(tail.len());
        out.insert(format!("EasyRPG.{}", &tail[..end]));
        rest = &tail[end..];
    }
    out.into_iter().collect()
}

/// A buff or debuff (`DT_StatusEffects`). Its stats are `CharacterAttributes` x stacks
/// (BP_StatusEffect_Base.GetCharacterAttributes); some are also checked by name in the
/// stat code (`CheckStatusEffectByHandle`).
#[derive(Serialize, Clone, Debug)]
pub struct StatusEffectInfo {
    pub row: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub positive: bool,
    pub max_stack: i64,
    pub duration: f64,
    pub attributes: Vec<(String, f64)>,
}

fn status_effects(gd: &GameData) -> Result<Vec<StatusEffectInfo>> {
    Ok(table_rows(gd, "EasySurvivalRPG/Blueprints/DataTables/LocalizedDataTables/DT_StatusEffects.json")?
        .into_iter()
        .map(|(row, r)| StatusEffectInfo {
            name: ftext(gd, &r["AttributeName"]),
            description: ftext(gd, &r["Description"]),
            icon: icon_of(&r),
            positive: r["IsPositiveEffect"].as_bool().unwrap_or(true),
            max_stack: r["MaxStack"].as_i64().unwrap_or(1),
            duration: r["Duration"].as_f64().unwrap_or(0.0),
            attributes: attrs_json(&r["CharacterAttributes"]).into_iter().filter(|(t, _)| t != "None").collect(),
            row,
        })
        .collect())
}

/// Skills outside the main tree, by DataTable: stance skills, crystal cards, crafting.
#[derive(Serialize, Clone, Debug)]
pub struct SkillSet {
    pub table: String,
    /// English key, translated by the UI (`Spear`, `Cards`, `Crafting`...).
    pub name: String,
    /// `E_Stance` value of a stance skill table.
    pub stance: Option<u8>,
    pub skills: Vec<SkillInfo>,
}

#[derive(Serialize, Clone, Debug)]
pub struct SkillInfo {
    pub row: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    /// 0 when unlimited.
    pub max_level: i64,
    pub requires: String,
    /// Bonus per level (`GetSkillDataAttributes`: Attribute and SkillLevels[0].BonusAttributes, x level).
    pub attributes: Vec<(String, f64)>,
}

fn skill_sets(gd: &GameData) -> Result<Vec<SkillSet>> {
    let stance = |n: &str| gd.enums.get("E_Stance").and_then(|e| e.iter().find(|(_, s)| s == n).map(|(v, _)| *v));
    let dir = "EasySurvivalRPG/Blueprints/DataTables/StanceSkills";
    let sets = [
        ("DT_Skills_Spear", "Spear", Some("Spear")),
        ("DT_Skills_OneHand", "Sword", Some("Sword")),
        ("DT_Skills_TwoHand", "TwoHanded", Some("TwoHanded")),
        ("DT_Skills_Mauls", "Polearm", Some("Polearm")),
        ("DT_Skills_Scythe", "Scythe", Some("Scythe")),
        ("DT_Skills_Archery", "Bow", Some("Bow")),
        ("DT_Skills_Magery", "Magic", Some("Magic")),
        ("DT_Skills_Unarmed", "Common", Some("Common")),
        ("DT_Crystal_Cards_Skills", "Cards", None),
        ("Localized/DT_Skills_Crafting_S24", "Crafting", None),
    ];
    let mut out = Vec::new();
    for (file, name, st) in sets {
        let Ok(rows) = table_rows(gd, &format!("{dir}/{file}.json")) else { continue };
        let skills = rows
            .into_iter()
            .map(|(row, r)| {
                let mut attributes: Vec<(String, f64)> = Vec::new();
                let a = &r["Attribute"];
                if tag_of(&a["GameplayTag"]) != "None" {
                    attributes.push((tag_of(&a["GameplayTag"]), a["Value"].as_f64().unwrap_or(0.0)));
                }
                attributes.extend(attrs_json(&r["SkillLevels"][0]["BonusAttributes"]).into_iter().filter(|(t, _)| t != "None"));
                // Cards carry a placeholder title.
                let title = r["Title"].as_str().filter(|t| !t.starts_with("INVTEXT")).unwrap_or("").to_string();
                SkillInfo {
                    name: title,
                    description: ftext(gd, &r["Description"]).trim_start_matches("Desc").to_string(),
                    icon: icon_of(&r),
                    max_level: r["MaxLevel"].as_i64().unwrap_or(0),
                    requires: ftext(gd, &r["RequiredDescription"]),
                    attributes,
                    row,
                }
            })
            .collect();
        let table = file.rsplit('/').next().unwrap_or(file).to_string();
        out.push(SkillSet { table, name: name.to_string(), stance: st.and_then(stance), skills });
    }
    Ok(out)
}

/// The main skill tree (`UI_SkillTree_S4realzies`): node layout and what each node grants.
#[derive(Serialize, Clone, Debug, Default)]
pub struct SkillTree {
    pub nodes: Vec<TreeNode>,
}

#[derive(Serialize, Clone, Debug)]
pub struct TreeNode {
    pub id: String,
    /// Canvas position (top-left) and size, in widget units.
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// The start node: always owned, never refunded.
    pub root: bool,
    /// Indexes of the neighbour nodes. A node can be bought next to an owned one.
    pub links: Vec<usize>,
    /// Rows the node can own (`DT_GENERATED_SkillTree_Main`): its own row, or one dropdown option.
    pub options: Vec<TreeOption>,
}

#[derive(Serialize, Clone, Debug)]
pub struct TreeOption {
    pub row: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub attributes: Vec<(String, f64)>,
}

/// Joins the extracted layout (`skill_tree.json`, see tools/extract/skilltree.py) with the
/// node rows. Base nodes own the row of their name; dropdown nodes offer the rows
/// `<node>_<n>` (so `UI_SkillTreeNode_DropDown_6` is an option of `UI_SkillTreeNode_DropDown`,
/// not the row of the `UI_SkillTreeNode_DropDown_6` node).
fn skill_tree(gd: &GameData) -> Result<SkillTree> {
    let Ok(text) = fs::read_to_string(gd.root.join("data/extracted/skill_tree.json")) else { return Ok(SkillTree::default()) };
    let layout: Json = serde_json::from_str(&text)?;
    let rows: HashMap<String, Json> = table_rows(gd, "EasySurvivalRPG/Blueprints/UserInterface/SkillTree_S3/SkillDataTables/DT_GENERATED_SkillTree_Main.json")?.into_iter().collect();
    let option = |row: &str| -> Option<TreeOption> {
        let r = rows.get(row)?;
        Some(TreeOption {
            row: row.to_string(),
            name: ftext(gd, &r["SkillName"]),
            description: ftext(gd, &r["Description"]),
            icon: icon_of(r),
            attributes: attrs_json(&r["SkillLevels"][0]["BonusAttributes"]).into_iter().filter(|(t, _)| t != "None").collect(),
        })
    };
    let raw = layout["nodes"].as_array().cloned().unwrap_or_default();
    let index: HashMap<&str, usize> = raw.iter().enumerate().filter_map(|(i, n)| Some((n["id"].as_str()?, i))).collect();
    let mut links: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); raw.len()];
    for (i, n) in raw.iter().enumerate() {
        for l in n["links"].as_array().into_iter().flatten().filter_map(|l| l.as_str().and_then(|l| index.get(l))) {
            links[i].insert(*l);
            links[*l].insert(i);
        }
    }
    let nodes = raw
        .iter()
        .zip(links)
        .map(|(n, links)| {
            let id = n["id"].as_str().unwrap_or("").to_string();
            let options: Vec<TreeOption> = if n["kind"] == "dropdown" {
                let mut numbered: Vec<(u32, &String)> = rows.keys().filter_map(|r| Some((r.strip_prefix(&id)?.strip_prefix('_')?.parse().ok()?, r))).collect();
                numbered.sort();
                numbered.into_iter().filter_map(|(_, r)| option(r)).collect()
            } else {
                option(&id).into_iter().collect()
            };
            TreeNode {
                x: n["x"].as_f64().unwrap_or(0.0),
                y: n["y"].as_f64().unwrap_or(0.0),
                w: n["w"].as_f64().unwrap_or(30.0),
                h: n["h"].as_f64().unwrap_or(30.0),
                root: n["root"].as_bool().unwrap_or(false),
                links: links.into_iter().collect(),
                options,
                id,
            }
        })
        .collect();
    Ok(SkillTree { nodes })
}

/// Basic-attack combos per stance, from BP_Character_Player's first-attack animations
/// (the stance switch of `TryAttack`) chained through the animations' ContinueAttack notifies.
fn basic_attacks(gd: &GameData, anims: &Json) -> Vec<BasicAttack> {
    let path = gd.root.join("data/extracted/blueprints/EasySurvivalRPG/Blueprints/Characters/Base/BP_Character_Player.json");
    let Some(cdo) = fs::read_to_string(path).ok().and_then(|s| serde_json::from_str::<Vec<Json>>(&s).ok()).and_then(|d| d.into_iter().find(|e| e["Name"].as_str().is_some_and(|n| n.starts_with("Default__")))) else {
        return Vec::new();
    };
    let props = shorten(&cdo["Properties"]);
    let first = [
        ("Common", "CommonFirstAttackAnim"),
        ("Sword", "SwordFirstAttackAnim"),
        ("Spear", "SpearFirstAttackAnim"),
        ("TwoHanded", "TwoHandFirstAttackAnim"),
        ("Polearm", "PolearmFirstAttackAnim"),
        ("Bow", "BowStartShootAnim"),
        ("Magic", "WandFirstAttackAnim"),
        ("Scythe", "ScytheFirstAttackAnim"),
    ];
    let stances = gd.enums.get("E_Stance").cloned().unwrap_or_default();
    let mut out = Vec::new();
    for (stance_name, prop) in first {
        let Some(stance) = stances.iter().find(|(_, n)| n == stance_name).map(|(v, _)| *v) else { continue };
        let start = &props[prop];
        let mut anim = start["Anim"]["ObjectPath"].as_str().unwrap_or("").split('.').next().unwrap_or("").to_string();
        let mut play = (start["InPlayRate"].as_f64().unwrap_or(1.0), start["AffectedByAttackSpeed?"].as_bool().unwrap_or(true));
        let mut steps: Vec<ComboStep> = Vec::new();
        while !anim.is_empty() && steps.len() < 12 && !steps.iter().any(|s| s.anim == anim) {
            let t = &anims[anim.as_str()];
            if t.is_null() {
                break;
            }
            steps.push(ComboStep {
                time: t["continue"].as_f64().or(t["finish"].as_f64()).unwrap_or(1.0),
                rate: t["rate"].as_f64().unwrap_or(1.0),
                in_play_rate: play.0,
                attack_speed: play.1,
                anim: anim.clone(),
            });
            play = (t["next_play_rate"].as_f64().unwrap_or(1.0), t["next_attack_speed"].as_bool().unwrap_or(true));
            anim = t["next"].as_str().unwrap_or("").to_string();
        }
        if !steps.is_empty() {
            out.push(BasicAttack { stance, steps });
        }
    }
    out
}

/// The game's French text drops "physique" ("Dégâts", "Bonus Dégâts") and has no entry for
/// the elemental ones, so both families read the same. Explicit names, plus entries for the
/// panel totals (`*.Display`) that have no settings of their own.
/// The game names physical damage "Damage": the planner says which damage it is. Languages
/// without an override keep the game's names.
fn apply_label_overrides(attrs: &mut BTreeMap<String, AttrInfo>, lang: &str) {
    let names: &[(&str, &str)] = match lang {
        "fr" => &[
            ("EasyRPG.Attributes.Base.Damage", "Dégâts physiques"),
            ("EasyRPG.Attributes.Base.Damage%", "Bonus dégâts physiques"),
            ("EasyRPG.Attributes.Base.ElementalDamage", "Dégâts élémentaires"),
            ("EasyRPG.Attributes.Base.ElementalDamage%", "Bonus dégâts élémentaires"),
        ],
        "en" => &[
            ("EasyRPG.Attributes.Base.Damage", "Physical Damage"),
            ("EasyRPG.Attributes.Base.Damage%", "Physical Damage Bonus"),
            ("EasyRPG.Attributes.Base.ElementalDamage", "Elemental Damage"),
            ("EasyRPG.Attributes.Base.ElementalDamage%", "Elemental Damage Bonus"),
        ],
        _ => &[],
    };
    for (tag, name) in names {
        if let Some(a) = attrs.get_mut(*tag) {
            a.name = name.to_string();
        }
    }
    let (total, description) = match lang {
        "fr" => ("total", "Valeur affichée par le jeu : base x (1 + bonus + posture)."),
        _ => ("total", "Value shown by the game: base x (1 + bonus + stance)."),
    };
    for (tag, base) in [
        ("EasyRPG.Attributes.Base.Damage.Display", "EasyRPG.Attributes.Base.Damage"),
        ("EasyRPG.Attributes.Base.ElementalDamage.Display", "EasyRPG.Attributes.Base.ElementalDamage"),
    ] {
        let Some(b) = attrs.get(base).cloned() else { continue };
        let name = format!("{} ({total})", b.name);
        attrs.insert(tag.to_string(), AttrInfo { name, description: description.to_string(), value_type: 1, digits: 2, icon: b.icon });
    }
}

/// Localised FText: string-table entry, locres entry, or its source string.
pub fn ftext(gd: &GameData, v: &Json) -> String {
    if let (Some(t), Some(k)) = (v["TableId"].as_str(), v["Key"].as_str()) {
        let table = t.rsplit('.').next().unwrap_or(t);
        if let Some(s) = gd.text(table, k) {
            return s.to_string();
        }
    }
    if let (Some(ns), Some(k)) = (v["Namespace"].as_str(), v["Key"].as_str()) {
        if let Some(s) = gd.text(ns, k) {
            return s.to_string();
        }
    }
    v["LocalizedString"].as_str().filter(|s| !s.is_empty()).or(v["SourceString"].as_str()).or(v["CultureInvariantString"].as_str()).or(v.as_str()).unwrap_or("").to_string()
}

fn attrs_json(v: &Json) -> Vec<(String, f64)> {
    v.as_array().into_iter().flatten().map(|a| (tag_of(&a["GameplayTag"]), a["Value"].as_f64().unwrap_or(0.0))).collect()
}

/// `E_Stance` value from item tags (`EasyRPG.Items.Stance.Spear` -> Spear), as GetStanceFromTags does.
fn stance_from_tags(gd: &GameData, tags: &[String]) -> u8 {
    let stances = gd.enums.get("E_Stance").cloned().unwrap_or_default();
    tags.iter()
        .filter_map(|t| t.strip_prefix("EasyRPG.Items.Stance."))
        .find_map(|n| stances.iter().find(|(_, s)| s == n).map(|(v, _)| *v))
        .unwrap_or(0)
}

/// `/Game/A/B/C.C` -> `A/B/C.webp`.
pub fn asset_path(game_path: &str) -> String {
    let p = game_path.split('.').next().unwrap_or(game_path);
    format!("{}.webp", p.trim_start_matches("/Game/"))
}

fn icon_of(row: &Json) -> Option<String> {
    row["Icon"]["ObjectPath"].as_str().map(asset_path)
}

fn tag_of(v: &Json) -> String {
    v["TagName"].as_str().unwrap_or("None").to_string()
}

/// Rows of an extracted table with short field names (`Value_5_43BD...` -> `Value`).
pub fn table_rows(gd: &GameData, rel: &str) -> Result<Vec<(String, Json)>> {
    let path = gd.root.join("data/extracted/tables").join(rel);
    let d: Vec<Json> = serde_json::from_str(&fs::read_to_string(&path).with_context(|| path.display().to_string())?)?;
    Ok(d[0]["Rows"].as_object().into_iter().flatten().map(|(k, v)| (k.clone(), shorten(v))).collect())
}

fn shorten(v: &Json) -> Json {
    match v {
        Json::Object(m) => Json::Object(m.iter().map(|(k, x)| (short_field(k).to_string(), shorten(x))).collect()),
        Json::Array(a) => Json::Array(a.iter().map(shorten).collect()),
        other => other.clone(),
    }
}

fn handle_row(v: &Json) -> String {
    v["RowName"].as_str().unwrap_or("None").to_string()
}

impl Catalog {
    pub fn build(gd: &GameData) -> Result<Catalog> {
        let text = |key: String| gd.text_checked("ST_Attributes", &key).unwrap_or_default();

        // Attribute settings (format) + icons from the item attribute table.
        let mut attributes = BTreeMap::new();
        for (_, r) in table_rows(gd, "EasySurvivalRPG/Blueprints/DataTables/LocalizedDataTables/DT_Attributes.json")? {
            let tag = tag_of(&r["Tag"]);
            let value_type = r["ValueType"].as_str().and_then(enum_value).unwrap_or(0);
            attributes.insert(
                tag.clone(),
                AttrInfo {
                    name: text(format!("{tag}_Name")),
                    description: text(format!("{tag}_Description")),
                    value_type,
                    digits: r["MinFractinalDigits"].as_u64().unwrap_or(0) as u8,
                    icon: None,
                },
            );
        }
        let mut affixes = Vec::new();
        for (row, r) in table_rows(gd, "EasySurvivalRPG/Blueprints/DataTables/GeneratedItems/DT_Base_Item_Attributes.json")? {
            let tag = tag_of(&r["AttributeTag"]);
            if let Some(a) = attributes.get_mut(&tag) {
                if a.icon.is_none() {
                    a.icon = icon_of(&r);
                }
            }
            affixes.push(Affix {
                row,
                tag,
                value: r["Value"].as_f64().unwrap_or(0.0),
                per_level: r["AddedValuePerLevel"].as_f64().unwrap_or(0.0),
                min_item_level: r["MinIRequiredtemLevel"].as_i64().unwrap_or(0),
                can_roll: r["CanBeRollerOnItem"].as_bool().unwrap_or(false),
            });
        }

        let affix_pools = table_rows(gd, "EasySurvivalRPG/Blueprints/Misc/ItemizationV2/DT_EquipmentAffixes.json")?
            .into_iter()
            .map(|(name, r)| AffixPool {
                name,
                item_type: tag_of(&r["ItemTypeTag"]),
                pool: r["AffixesPool"].as_array().into_iter().flatten().map(handle_row).collect(),
                banned: r["BannedAffixes"].as_array().into_iter().flatten().map(handle_row).collect(),
            })
            .collect();

        let mut affix_groups = BTreeMap::new();
        for (name, r) in table_rows(gd, "EasySurvivalRPG/Blueprints/DataTables/GeneratedItems/DT_Yellow_Orange_Modifiers.json")? {
            affix_groups.insert(name, r["AttributesAbilityHandles"].as_array().into_iter().flatten().map(handle_row).collect());
        }
        for (name, r) in table_rows(gd, "EasySurvivalRPG/Blueprints/DataTables/GeneratedItems/DT_RandomAffixPools_V2.json")? {
            let rows = r["Handles"].as_array().into_iter().flatten().flat_map(|h| h["Handles"].as_array().cloned().unwrap_or_default()).map(|h| handle_row(&h)).collect();
            affix_groups.insert(name, rows);
        }

        let modifiers = table_rows(gd, "EasySurvivalRPG/Blueprints/Equipment_Modifiers/DT_Equipment_Modifiers.json")?
            .into_iter()
            .map(|(row, r)| {
                let tag = tag_of(&r["ModifierTag"]);
                Modifier {
                    name: text(format!("{tag}_Name")),
                    description: text(format!("{tag}_Description")),
                    slot: r["ModifierSlot"].as_str().and_then(enum_value).unwrap_or(0),
                    in_use: r["inUse?"].as_bool().unwrap_or(false),
                    weapon_affinity: r["bCanUseOnWeapon"].as_str().and_then(enum_value).unwrap_or(0),
                    row,
                    tag,
                }
            })
            .collect();

        let silhouettes = [
            ("Head", "sil-helmet"),
            ("Body", "sil-chest"),
            ("Pants", "sil-pants"),
            ("Hands", "sil-gloves"),
            ("Feet", "sil-boots"),
            ("Backpack", "sil-crest"),
            ("Ring1", "sil-ring"),
            ("Ring2", "sil-ring"),
            ("Amulet", "sil-jewelery"),
            ("Bracer", "sil-bracers"),
            ("Heart", "heart-of-dwarf-slot-empty"),
            ("Goblet", "sil-goblet"),
            ("Horn", "sil-horn"),
            ("Trinket", "sil-trinket"),
            ("Pet", "sil-pet"),
            ("Belt", "empty-belt"),
        ];
        let enum_list = |n: &str| gd.enums.get(n).cloned().unwrap_or_default();
        let slots = enum_list("E_EquipmentSlot")
            .into_iter()
            .map(|(value, name)| {
                let silhouette = silhouettes.iter().find(|(s, _)| *s == name).map(|(_, f)| format!("EasySurvivalRPG/Textures/UI/{f}.webp"));
                SlotInfo { value, name, silhouette }
            })
            .collect();

        let mut weapons = Vec::new();
        for (row, r) in table_rows(gd, "EasySurvivalRPG/Blueprints/DataTables/DT_Items.json")? {
            let tags: Vec<String> = r["ItemTags"].as_array().into_iter().flatten().map(|t| t.as_str().map(str::to_string).unwrap_or_else(|| tag_of(t))).collect();
            // Only `Items.Weapon.*` rows: recipes, drinks and buildings carry stance tags too.
            if !tags.iter().any(|t| t.starts_with("EasyRPG.Items.Weapon.")) {
                continue;
            }
            weapons.push(WeaponInfo {
                name: ftext(gd, &r["InGameName"]),
                icon: icon_of(&r),
                stance: stance_from_tags(gd, &tags),
                item_level: r["ItemLevel"].as_str().and_then(enum_value).map(i64::from).or(r["ItemLevel"].as_i64()).unwrap_or(0),
                attributes: attrs_json(&r["EquipmentAttributes"]),
                row,
            });
        }

        let abilities = table_rows(gd, "EasySurvivalRPG/Blueprints/Abilities/SkillAbilities/Localized/DT_PlayerAbilities.json")?
            .into_iter()
            .map(|(row, r)| {
                let dtype = r["DamageType"]["ObjectPath"].as_str().or(r["DamageType"]["ObjectName"].as_str()).unwrap_or("").to_string();
                let element = ["Fire", "Lightning", "Arcane"].into_iter().find(|e| dtype.contains(e)).unwrap_or("").to_string();
                AbilityInfo {
                    tag: tag_of(&r["IdentifierTag"]),
                    name: ftext(gd, &r["Title"]),
                    description: ftext(gd, &r["Description"]),
                    element,
                    affinities: r["Affinities"].as_array().into_iter().flatten().map(tag_of).collect(),
                    damage_tag: tag_of(&r["DmgModifierAttribute"]["GameplayTag"]),
                    damage_base: r["DmgModifierAttribute"]["Value"].as_f64().unwrap_or(0.0),
                    bonus_tag: tag_of(&r["DmgBonusModifierAttribute"]["GameplayTag"]),
                    icon: icon_of(&r),
                    cooldowns: r["CooldownSettings"]["HasCooldown?"].as_bool().unwrap_or(false).then(|| {
                        let c = &r["CooldownSettings"];
                        [c["Initial"].as_f64().unwrap_or(0.0), c["TwoOffhandsCooldown"].as_f64().unwrap_or(0.0), c["ThreeOffhandsCooldown"].as_f64().unwrap_or(0.0)]
                    }),
                    row,
                }
            })
            .collect();

        let anims: Json = fs::read_to_string(gd.root.join("data/extracted/attack_anims.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Json::Null);
        let attacks = table_rows(gd, "EasySurvivalRPG/Blueprints/Misc/WeaponAttacksSystem/DT_WeaponAttacks.json")?
            .into_iter()
            .map(|(row, r)| {
                let anim = &r["Animation"];
                let timing = &anims[anim["Anim"]["ObjectPath"].as_str().unwrap_or("").split('.').next().unwrap_or("")];
                AttackInfo {
                anim_finish: timing["finish"].as_f64().unwrap_or(1.0),
                anim_rate: timing["rate"].as_f64().unwrap_or(1.0),
                in_play_rate: anim["InPlayRate"].as_f64().unwrap_or(1.0),
                attack_speed: anim["AffectedByAttackSpeed?"].as_bool().unwrap_or(true),
                stance: r["Stance"].as_str().and_then(enum_value).unwrap_or(0),
                kind: r["Type"].as_str().and_then(enum_value).unwrap_or(0),
                name: ftext(gd, &r["AttributeName"]),
                description: ftext(gd, &r["Description"]),
                multiplier: r["DamageMultiplier"].as_f64().unwrap_or(1.0),
                energy: r["RequiredEnergy"].as_f64().unwrap_or(0.0),
                damage_bonus_tag: tag_of(&r["BonusTags"]["DamageBonusTag"]),
                energy_reduction_tag: tag_of(&r["BonusTags"]["EnergyReductionTag"]),
                icon: icon_of(&r),
                row,
            }})
            .collect();

        let basic_attacks = basic_attacks(gd, &anims);
        apply_label_overrides(&mut attributes, &gd.lang);

        Ok(Catalog {
            build: fs::read_to_string(gd.root.join("data/extracted/BUILD")).unwrap_or_default().trim().to_string(),
            lang: gd.lang.clone(),
            languages: crate::gamedata::languages(&gd.root),
            attributes,
            panel: stats_panel(gd)?,
            slots,
            stances: enum_list("E_Stance"),
            rarities: enum_list("E_Rarity"),
            affixes,
            affix_pools,
            affix_groups,
            modifiers,
            weapons,
            abilities,
            attacks,
            basic_attacks,
            skill_tree: skill_tree(gd)?,
            skill_sets: skill_sets(gd)?,
            status_effects: status_effects(gd)?,
            stat_mods: stat_mods(gd),
        })
    }
}

/// Rows of the in-game STATS panel, in display order, read from `UI_Character`'s widget tree.
fn stats_panel(gd: &GameData) -> Result<Vec<PanelEntry>> {
    let path = gd.root.join("data/extracted/blueprints/EasySurvivalRPG/Blueprints/UserInterface/HUD/UI_Character.json");
    let d: Vec<Json> = serde_json::from_str(&fs::read_to_string(&path)?)?;
    let index = |o: &Json| o["ObjectPath"].as_str().and_then(|p| p.rsplit_once('.')).and_then(|(_, i)| i.parse::<usize>().ok());
    let root = d.iter().position(|e| e["Name"] == "StatsInfoVBoxWrapper").context("StatsInfoVBoxWrapper not found")?;
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![root];
    // Depth-first, children in slot order.
    while let Some(i) = stack.pop() {
        if !seen.insert(i) || i >= d.len() {
            continue;
        }
        let w = &d[i];
        let p = &w["Properties"];
        match w["Type"].as_str().unwrap_or("") {
            "UI_AttributeValue_C" => out.push(PanelEntry::Row {
                tag: tag_of(&p["Attribute"]),
                always_sign: p["AlwaysSign"].as_bool().unwrap_or(true),
                collapse_if_zero: p["CollapseIfZero"].as_bool().unwrap_or(false),
            }),
            "Image" if w["Name"].as_str().is_some_and(|n| n.starts_with("Line")) => out.push(PanelEntry::Sep),
            _ => {}
        }
        let mut children: Vec<usize> = p["Slots"].as_array().into_iter().flatten().filter_map(index).filter_map(|s| d.get(s).and_then(|slot| index(&slot["Properties"]["Content"]))).collect();
        if let Some(c) = index(&p["Content"]) {
            children.push(c);
        }
        stack.extend(children.into_iter().rev());
    }
    Ok(out)
}
