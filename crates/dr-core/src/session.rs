//! Play session: what a character save says about the character's activity, read each time the game writes
//! the save. The gains between two snapshots are what the player earned in between (a run, a session).
//!
//! The game writes `<id>.sav` now and then while playing; nothing here is live.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::save;
use crate::value::{StructVal, Value};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    /// When the game wrote the save (unix seconds, the file time).
    pub time: f64,
    pub name: String,
    /// Map the character is in (`CurrentLevel`, for example `HomeFort_Reworked`).
    pub zone: String,
    pub level: i64,
    pub experience: f64,
    pub deaths: i64,
    pub crafting_level: i64,
    pub crafting_experience: f64,
    /// Highest rupture level completed (`Completed Ruptures`).
    pub ruptures: i64,
    /// Weapon skill experience: `SpearSkill` -> `Spear`.
    pub skills: BTreeMap<String, f64>,
    /// Activity completions: `ElvenCounter` -> `Elven`.
    pub counters: BTreeMap<String, f64>,
    /// Resources by tag, without `EasyRPG.Items.Resource.`: `ElvenFragments`, `CasinoKey`...
    pub resources: BTreeMap<String, f64>,
}

fn field<'a>(s: &'a StructVal, prefix: &str) -> Option<&'a Value> {
    s.fields.iter().find(|(k, _)| k.starts_with(prefix)).map(|(_, v)| v)
}

/// The field's name without the `_<n>_<guid>` suffix blueprint structs add.
fn base_name(k: &str) -> &str {
    k.split('_').next().unwrap_or(k)
}

pub fn read(path: &Path) -> Result<Snapshot> {
    let sav = save::load(path).with_context(|| format!("reading {}", path.display()))?;
    let mut s = Snapshot::default();
    // The save's `DateTime` is the game's local wall clock (DateTime.Now): the file time is the real instant.
    s.time = file_time(path);
    s.zone = sav.get("CurrentLevel").map(|v| v.name().to_string()).unwrap_or_default();
    s.crafting_level = sav.get("Crafting Level").map(Value::i64).unwrap_or(0);
    s.crafting_experience = sav.get("Total Crafting Experience").map(Value::f64).unwrap_or(0.0);
    if let Some(Value::Array(done)) = sav.get("Completed Ruptures") {
        s.ruptures = done.iter().map(Value::i64).max().unwrap_or(0);
    }
    let Some(Value::Struct(p)) = sav.get("HostPlayerData") else { anyhow::bail!("{}: no character data", path.display()) };
    for (k, v) in &p.fields {
        match base_name(k) {
            "PlayerName" => s.name = v.name().to_string(),
            "Level" => s.level = v.i64(),
            "Experience" => s.experience = v.f64(),
            "Deaths" => s.deaths = v.i64(),
            "GainedSkillPoints" => {}
            n if n.ends_with("Skill") => {
                s.skills.insert(n.trim_end_matches("Skill").to_string(), v.f64());
            }
            n if n.ends_with("Counter") => {
                s.counters.insert(n.trim_end_matches("Counter").to_string(), v.f64());
            }
            _ => {}
        }
    }
    if let Some(Value::Array(res)) = field(p, "Resources") {
        for r in res {
            let Value::Struct(r) = r else { continue };
            let tag = match field(r, "ResourceTypeTag") {
                Some(Value::Struct(t)) => t.get("TagName").map(|n| n.name().to_string()).unwrap_or_default(),
                _ => continue,
            };
            let tag = tag.trim_start_matches("EasyRPG.Items.Resource.").to_string();
            if !tag.is_empty() && tag != "None" {
                s.resources.insert(tag, field(r, "Value").map(Value::f64).unwrap_or(0.0));
            }
        }
    }
    Ok(s)
}

impl Snapshot {
    /// Same map and activity values (the time aside).
    fn same_values(&self, o: &Snapshot) -> bool {
        self.zone == o.zone && changes(self, o).is_empty()
    }
}

/// `<history dir>/<save key>.session.jsonl`: one snapshot each time the save changed, oldest first.
fn journal(key: &str) -> PathBuf {
    crate::history::history_dir().join(format!("{key}.session.jsonl"))
}

/// The app records from two threads (the folder watch, the session view): one write at a time.
static WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The session journal of a character, oldest first (one entry per save time).
pub fn load(key: &str) -> Vec<Snapshot> {
    let Ok(text) = fs::read_to_string(journal(key)) else { return Vec::new() };
    let mut out: Vec<Snapshot> = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    out.dedup_by(|b, a| a.time == b.time);
    out
}

/// Reads the save when it was written since the journal's last entry, and appends it when a value moved.
/// Returns whether an entry was added.
pub fn record(path: &Path) -> Result<bool> {
    let _one = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    let key = crate::history::save_key(path);
    let mtime = file_time(path);
    let last = load(&key).pop();
    if last.as_ref().is_some_and(|l| l.time >= mtime) {
        return Ok(false);
    }
    let snap = read(path)?;
    if last.is_some_and(|l| l.same_values(&snap)) {
        return Ok(false);
    }
    fs::create_dir_all(crate::history::history_dir())?;
    let mut f = fs::OpenOptions::new().create(true).append(true).open(journal(&key))?;
    writeln!(f, "{}", serde_json::to_string(&snap)?)?;
    Ok(true)
}

/// Adds a copy of a save written at `time` to the journal of `key`, in time order (to fill the journal from
/// kept copies). Returns whether it was added.
pub fn import(key: &str, path: &Path, time: f64) -> Result<bool> {
    let _one = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    let mut snap = read(path)?;
    snap.time = time;
    let mut all = load(key);
    let at = all.partition_point(|s| s.time < time);
    let same = |o: Option<&Snapshot>| o.is_some_and(|o| (o.time - time).abs() < 1.0 || o.same_values(&snap));
    if same(at.checked_sub(1).and_then(|i| all.get(i))) || all.get(at).is_some_and(|o| (o.time - time).abs() < 1.0) {
        return Ok(false);
    }
    all.insert(at, snap);
    fs::create_dir_all(crate::history::history_dir())?;
    let text: String = all.iter().map(|s| serde_json::to_string(s).map(|l| l + "\n")).collect::<Result<_, _>>()?;
    fs::write(journal(key), text)?;
    Ok(true)
}

/// Character saves of a save folder (no parsing: backups, world saves and settings files are skipped by name).
fn character_saves(dir: &Path) -> Vec<PathBuf> {
    let skip = ["GameSettings.sav", "EnhancedInputUserSettings.sav", "perma_characters_list.sav"];
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            n.ends_with(".sav") && !n.contains("_BU") && !n.contains("_WRL") && !skip.contains(&n)
        })
        .collect()
}

/// Records every character save of `dir` written since the last call (`seen` keeps the file times). Meant to run
/// every few seconds: a run starts with three saves in about 20 seconds, and a save is overwritten by the next.
/// Returns the saves that got a new journal entry.
pub fn watch(dir: &Path, seen: &mut std::collections::HashMap<PathBuf, f64>) -> Vec<(PathBuf, Result<bool>)> {
    let mut out = Vec::new();
    for p in character_saves(dir) {
        let t = file_time(&p);
        if seen.get(&p) == Some(&t) {
            continue;
        }
        // The game writes the file in several steps: wait until it stopped changing.
        std::thread::sleep(std::time::Duration::from_millis(400));
        if file_time(&p) != t {
            continue;
        }
        seen.insert(p.clone(), t);
        let r = record(&p);
        out.push((p, r));
    }
    out
}

/// What the player tells about a run (the save does not have it).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct RunNote {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rupture: Option<i64>,
    /// `DT_RuptureFortunes` tags of the card taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bane: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub essence: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rerolls: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub containers: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elites: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mobs: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// The player's farming notes of a character: what each run was, and the farm level over time.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RunBook {
    /// By run id (the run's end time, in whole seconds).
    #[serde(default)]
    pub notes: BTreeMap<String, RunNote>,
    /// `(since, level)`: the rupture level the player farms from that time on (runs stay at one level until a
    /// power-up), oldest first. A run's own `rupture` note wins over it.
    #[serde(default)]
    pub levels: Vec<(f64, i64)>,
}

impl RunBook {
    /// Farm level in force at `time`.
    pub fn level_at(&self, time: f64) -> Option<i64> {
        self.levels.iter().rev().find(|(since, _)| *since <= time).map(|(_, l)| *l)
    }
}

/// `<history dir>/<save key>.runs.json`.
fn book_file(key: &str) -> PathBuf {
    crate::history::history_dir().join(format!("{key}.runs.json"))
}

pub fn book(key: &str) -> RunBook {
    fs::read_to_string(book_file(key)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save_book(key: &str, b: &RunBook) -> Result<()> {
    fs::create_dir_all(crate::history::history_dir())?;
    fs::write(book_file(key), serde_json::to_string_pretty(b)?)?;
    Ok(())
}

/// Stores the note of run `id` (an empty note removes it).
pub fn set_note(key: &str, id: &str, note: RunNote) -> Result<RunBook> {
    let _one = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    let mut b = book(key);
    if note == RunNote::default() {
        b.notes.remove(id);
    } else {
        b.notes.insert(id.to_string(), note);
    }
    save_book(key, &b)?;
    Ok(b)
}

/// The player farms rupture level `level` from `since` on.
pub fn set_level(key: &str, since: f64, level: i64) -> Result<RunBook> {
    let _one = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    let mut b = book(key);
    b.levels.retain(|(t, _)| *t < since);
    b.levels.push((since, level));
    save_book(key, &b)?;
    Ok(b)
}

fn file_time(path: &Path) -> f64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// What changed from `a` to `b`: `(group, key, before, after)` for every value that moved.
pub fn changes(a: &Snapshot, b: &Snapshot) -> Vec<(&'static str, String, f64, f64)> {
    let mut out = Vec::new();
    let mut one = |group: &'static str, key: &str, x: f64, y: f64| {
        if x != y {
            out.push((group, key.to_string(), x, y));
        }
    };
    one("character", "level", a.level as f64, b.level as f64);
    one("character", "experience", a.experience, b.experience);
    one("character", "deaths", a.deaths as f64, b.deaths as f64);
    one("character", "crafting level", a.crafting_level as f64, b.crafting_level as f64);
    one("character", "crafting experience", a.crafting_experience, b.crafting_experience);
    one("character", "ruptures", a.ruptures as f64, b.ruptures as f64);
    for (group, x, y) in [("skills", &a.skills, &b.skills), ("counters", &a.counters, &b.counters), ("resources", &a.resources, &b.resources)] {
        let keys: std::collections::BTreeSet<&String> = x.keys().chain(y.keys()).collect();
        for k in keys {
            one(group, k, x.get(k).copied().unwrap_or(0.0), y.get(k).copied().unwrap_or(0.0));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn farm_level_follows_the_last_change() {
        let b = RunBook { levels: vec![(100.0, 470), (500.0, 479)], ..Default::default() };
        assert_eq!(b.level_at(50.0), None);
        assert_eq!(b.level_at(300.0), Some(470));
        assert_eq!(b.level_at(500.0), Some(479));
    }

    #[test]
    fn changes_lists_moved_values_only() {
        let mut a = Snapshot::default();
        a.resources.insert("ElvenFragments".into(), 10.0);
        a.resources.insert("Coins".into(), 5.0);
        let mut b = a.clone();
        b.resources.insert("ElvenFragments".into(), 25.0);
        b.counters.insert("Elven".into(), 1.0);
        let c = changes(&a, &b);
        assert_eq!(c, vec![("counters", "Elven".to_string(), 0.0, 1.0), ("resources", "ElvenFragments".to_string(), 10.0, 25.0)]);
    }
}
