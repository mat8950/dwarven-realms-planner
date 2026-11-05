//! Items a character carries (its inventory, in `<id>.sav`) and keeps in the home fort stash (the
//! `BP_Container_Home_Fort_stash` containers of the world save `<id>_WRL.sav`).
//!
//! Each generated item keeps its affix pools (`AffixesPool`: Inherent, Pool1-3, rows of
//! DT_Base_Item_Attributes): the affixes a reroll can give in each pool. An item is worth keeping
//! for a stat when one of its pools can roll it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;

use crate::catalog::{asset_path, ftext, table_rows};
use crate::gamedata::GameData;
use crate::planner::Attr;
use crate::save;
use crate::value::{StructVal, Value};

/// Equipment slot tags by `E_EquipmentSlot` value (BP_ItemsLibrary `EquipmentSlotToTag`).
const SLOT_TAGS: [&str; 16] = [
    "Head", "Body", "Pants", "Hands", "Feet", "Backpack", "Ring1", "Ring2", "Neck", "Bracers", "Heart", "Goblet", "Horn", "Trinket", "Pet", "Belt",
];

#[derive(Serialize, Clone, Debug)]
pub struct InvItem {
    /// Position in its container.
    pub index: usize,
    pub row: String,
    pub name: String,
    pub icon: Option<String>,
    pub rarity: u8,
    pub tier: i64,
    pub hearts: i64,
    pub amount: i64,
    pub locked: bool,
    /// Rolled stats (`GeneratedAttributes`).
    pub attributes: Vec<Attr>,
    /// Affix rows (DT_Base_Item_Attributes): the inherent ones and each reroll pool.
    pub inherent: Vec<String>,
    pub pools: Vec<Vec<String>>,
    /// `E_EquipmentSlot` values the item fits (rings fit both ring slots).
    pub slots: Vec<u8>,
    pub weapon: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct Container {
    /// `inventory`, or the stash container's component name.
    pub id: String,
    /// `inventory` or `stash`.
    pub kind: String,
    /// 1-based stash tab number (0 for the inventory).
    pub number: usize,
    pub size: usize,
    /// Used slots only.
    pub items: Vec<InvItem>,
    /// When the game wrote the save this comes from (unix seconds): the game saves when the player changes map,
    /// so loot picked up since is not there yet.
    pub saved: u64,
}

/// What the inventory needs from DT_Items: display name, icon and tags of each row.
pub struct ItemIndex {
    names: HashMap<String, String>,
    pub icons: HashMap<String, String>,
    tags: HashMap<String, Vec<String>>,
}

impl ItemIndex {
    pub fn new(gd: &GameData) -> Result<ItemIndex> {
        let (mut names, mut icons, mut tags) = (HashMap::new(), HashMap::new(), HashMap::new());
        for (row, r) in table_rows(gd, "EasySurvivalRPG/Blueprints/DataTables/DT_Items.json")? {
            if let Some(p) = r["Icon"]["ObjectPath"].as_str() {
                icons.insert(row.clone(), asset_path(p));
            }
            names.insert(row.clone(), ftext(gd, &r["InGameName"]));
            tags.insert(row, r["ItemTags"].as_array().into_iter().flatten().filter_map(|t| t.as_str().map(str::to_string)).collect());
        }
        Ok(ItemIndex { names, icons, tags })
    }

    /// Equipment slots of an item row, and whether it is a weapon (ItemHasTag: a tag matches its children).
    fn slots(&self, row: &str) -> (Vec<u8>, bool) {
        let tags = self.tags.get(row).map(Vec::as_slice).unwrap_or_default();
        let has = |t: &str| tags.iter().any(|x| x == t || x.starts_with(&format!("{t}.")));
        let slots = SLOT_TAGS.iter().enumerate().filter(|(_, t)| has(&format!("EasyRPG.Items.Equipment.{t}"))).map(|(i, _)| i as u8).collect();
        (slots, has("EasyRPG.Items.Weapon"))
    }

    fn item(&self, index: usize, v: &Value) -> Option<InvItem> {
        let Value::Struct(s) = v else { return None };
        let f = |n: &str| s.fields.iter().find(|(k, _)| k.starts_with(n)).map(|(_, v)| v.clone()).unwrap_or_default();
        let row = match f("ItemHandle") {
            Value::Struct(h) => h.get("RowName").map(|r| r.name().to_string()).unwrap_or_default(),
            _ => String::new(),
        };
        if row.is_empty() || row == "None" {
            return None;
        }
        let rows = |v: &Value| -> Vec<String> {
            match v {
                Value::Array(a) => a
                    .iter()
                    .filter_map(|h| match h {
                        Value::Struct(h) => h.get("RowName").map(|r| r.name().to_string()),
                        _ => None,
                    })
                    .filter(|r| r != "None")
                    .collect(),
                _ => Vec::new(),
            }
        };
        let (inherent, pools) = match f("AffixesPool") {
            Value::Struct(p) => {
                let g = |n: &str| p.fields.iter().find(|(k, _)| k.starts_with(n)).map(|(_, v)| rows(v)).unwrap_or_default();
                (g("Inherent"), vec![g("Pool1"), g("Pool2"), g("Pool3")])
            }
            _ => (Vec::new(), Vec::new()),
        };
        let generated = f("GeneratedName").name().to_string();
        let (slots, weapon) = self.slots(&row);
        Some(InvItem {
            index,
            name: if generated.is_empty() { self.names.get(&row).cloned().unwrap_or_else(|| row.clone()) } else { generated },
            icon: self.icons.get(&row).cloned(),
            rarity: f("Rarity").i64() as u8,
            tier: f("ItemTier").i64(),
            hearts: f("GabmleUpgradeAmount").i64(),
            amount: f("Amount").i64(),
            locked: matches!(f("IsLocked"), Value::Bool(true)),
            attributes: attrs(&f("GeneratedAttributes")),
            inherent,
            pools,
            slots,
            weapon,
            row,
        })
    }

    fn container(&self, id: String, kind: &str, number: usize, items: &[Value], saved: u64) -> Container {
        Container { id, kind: kind.into(), number, size: items.len(), items: items.iter().enumerate().filter_map(|(i, v)| self.item(i, v)).collect(), saved }
    }

    /// The character's inventory, then its stash tabs (when the world save is there).
    pub fn read(&self, save_path: &Path) -> Result<Vec<Container>> {
        let mut out = Vec::new();
        let sav = save::load(save_path)?;
        if let Some(Value::Struct(p)) = sav.get("HostPlayerData") {
            if let Some(Value::Array(items)) = field(p, "InventoryItems") {
                out.push(self.container("inventory".into(), "inventory", 0, items, file_time(save_path)));
            }
        }
        let world = world_save(save_path);
        if world.is_file() {
            out.extend(self.stash(&save::load(&world)?, file_time(&world)));
        }
        Ok(out)
    }

    /// Stash tabs of a world save: `..._BP_ContainerComponent_Stash`, `..._Stash1`, ... in order.
    fn stash(&self, world: &StructVal, saved: u64) -> Vec<Container> {
        let mut tabs: Vec<(usize, String, &Vec<Value>)> = Vec::new();
        for (_, v) in &world.fields {
            let Value::Array(levels) = v else { continue };
            for level in levels {
                let Value::Struct(l) = level else { continue };
                let Some(Value::Map(containers)) = field(l, "ContainerComponents") else { continue };
                for (name, c) in containers {
                    let name = name.name().to_string();
                    let Some(rest) = name.split("_BP_ContainerComponent_Stash").nth(1).filter(|_| name.contains("Home_Fort_stash")) else { continue };
                    let Value::Struct(c) = c else { continue };
                    if let Some(Value::Array(items)) = field(c, "Items") {
                        tabs.push((rest.parse::<usize>().map(|n| n + 1).unwrap_or(0), name, items));
                    }
                }
            }
        }
        tabs.sort_by_key(|t| t.0);
        tabs.into_iter().enumerate().map(|(i, (_, name, items))| self.container(name, "stash", i + 1, items, saved)).collect()
    }
}

fn file_time(p: &Path) -> u64 {
    p.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0)
}

fn field<'a>(s: &'a StructVal, prefix: &str) -> Option<&'a Value> {
    s.fields.iter().find(|(k, _)| k.starts_with(prefix)).map(|(_, v)| v)
}

/// `<id>.sav` -> `<id>_WRL.sav`.
pub fn world_save(save_path: &Path) -> PathBuf {
    let stem = save_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    save_path.with_file_name(format!("{stem}_WRL.sav"))
}

fn attrs(v: &Value) -> Vec<Attr> {
    let Value::Array(a) = v else { return Vec::new() };
    a.iter()
        .filter_map(|e| match e {
            Value::Struct(s) => Some(Attr {
                tag: s.fields.iter().find(|(k, _)| k.starts_with("GameplayTag"))?.1.name().to_string(),
                value: s.fields.iter().find(|(k, _)| k.starts_with("Value"))?.1.f64(),
            }),
            _ => None,
        })
        .filter(|a| a.tag != "None")
        .collect()
}
