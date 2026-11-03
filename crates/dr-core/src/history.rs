//! Progression history: one snapshot of each character every time its save changes
//! (level, combat rating, DPS, every attribute, items, skill tree, skills), appended to
//! `~/.local/share/dr-planner/history/<save id>.jsonl`.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::planner::{BuildInput, ItemView, Planner, SkillLevel};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Snapshot {
    /// Save file modification time and capture time (Unix seconds).
    pub save_modified: u64,
    pub captured: u64,
    pub name: String,
    pub level: i64,
    pub race: String,
    pub stance: u8,
    pub combat_rating: f64,
    /// DPS total as the DPS tab computes it by default: the stance's basic combo plus the
    /// equipped offhands; on a normal enemy, on a boss or elite, and the dummy's boss reading.
    pub dps: f64,
    pub dps_boss: f64,
    pub dummy_boss: f64,
    pub attributes: BTreeMap<String, f64>,
    /// Equipped items (slot = E_EquipmentSlot) and the weapon in hand (slot = None).
    pub items: Vec<ItemView>,
    pub tree: Vec<String>,
    pub skills: Vec<SkillLevel>,
    pub skill_points: i64,
}

impl Snapshot {
    /// Whether anything worth a history entry differs (XP alone does not count).
    fn same_build(&self, o: &Snapshot) -> bool {
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0);
        self.level == o.level
            && close(self.combat_rating, o.combat_rating)
            && close(self.dps_boss, o.dps_boss)
            && self.items == o.items
            && self.tree == o.tree
            && self.skills.iter().map(|s| (&s.table, &s.row, s.level)).eq(o.skills.iter().map(|s| (&s.table, &s.row, s.level)))
    }
}

/// `$XDG_DATA_HOME/dr-planner/history` (default `~/.local/share/...`).
pub fn history_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share"));
    base.join("dr-planner/history")
}

/// History key of a save: its file name without extension (one file per character).
pub fn save_key(path: &Path) -> String {
    path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string()
}

fn file_of(key: &str) -> PathBuf {
    history_dir().join(format!("{key}.jsonl"))
}

/// All snapshots of a character, oldest first.
pub fn load(key: &str) -> Result<Vec<Snapshot>> {
    let Ok(text) = fs::read_to_string(file_of(key)) else { return Ok(Vec::new()) };
    Ok(text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect())
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn modified(path: &Path) -> u64 {
    path.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0)
}

/// Snapshot of a save as it is (no build edits).
pub fn snapshot(planner: &mut Planner, path: &Path) -> Result<Snapshot> {
    let ch = planner.open(path)?;
    let st = planner.compute(&BuildInput::default())?;
    let d = &st.dps;
    let basic = d.basic.iter().find(|b| b.stance == st.stance);
    let off = d.abilities.iter().filter(|a| a.offhands > 0);
    let mut items: Vec<ItemView> = ch.items.clone();
    items.extend(ch.weapon.clone());
    Ok(Snapshot {
        save_modified: modified(path),
        captured: now(),
        name: ch.name,
        level: ch.level,
        race: ch.race,
        stance: st.stance,
        combat_rating: st.combat_rating,
        dps: basic.map(|b| b.dps).unwrap_or(0.0) + off.clone().map(|a| a.dps).sum::<f64>(),
        dps_boss: basic.map(|b| b.dps_boss).unwrap_or(0.0) + off.clone().map(|a| a.dps_boss).sum::<f64>(),
        dummy_boss: basic.map(|b| b.dps_boss).unwrap_or(0.0) + off.map(|a| a.meter_boss).sum::<f64>(),
        attributes: st.attributes,
        items,
        tree: ch.tree,
        skills: ch.skills,
        skill_points: ch.skill_points,
    })
}

/// Records a snapshot of `path` under `key` when the save changed since the last capture
/// (`seen` remembers the save times already checked). Returns whether one was added.
pub fn capture(planner: &mut Planner, path: &Path, key: &str, seen: &mut HashMap<String, u64>) -> Result<bool> {
    let mtime = modified(path);
    if seen.get(key) == Some(&mtime) {
        return Ok(false);
    }
    let last = load(key)?.pop();
    if last.as_ref().is_some_and(|l| l.save_modified >= mtime) {
        seen.insert(key.to_string(), mtime);
        return Ok(false);
    }
    let snap = snapshot(planner, path)?;
    seen.insert(key.to_string(), mtime);
    if last.is_some_and(|l| l.same_build(&snap)) {
        return Ok(false);
    }
    fs::create_dir_all(history_dir())?;
    let mut f = fs::OpenOptions::new().create(true).append(true).open(file_of(key))?;
    writeln!(f, "{}", serde_json::to_string(&snap)?)?;
    Ok(true)
}
