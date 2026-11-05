//! Where things are on the player's computer: the game installation (Steam or Epic), its saves,
//! the planner's own folders, and the game data the planner extracted for each game build.
//!
//! The planner ships no game data. It extracts it from the player's installation (the `dr-extract`
//! sidecar) into `<data dir>/game/<build>/data/`, the layout `GameData::load` reads.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const STEAM_APP_ID: &str = "2015240";

/// The planner's data folder: `~/.local/share/dr-planner` on Linux, `%APPDATA%\dr-planner` on Windows,
/// `~/Library/Application Support/dr-planner` on macOS.
pub fn data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("dr-planner")
}

/// The planner's cache folder (journal, Oodle library): `~/.cache/dr-planner` on Linux.
pub fn cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("dr-planner")
}

/// A game installation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameInstall {
    /// Folder holding `ProjectAlpha/`.
    pub dir: PathBuf,
    /// `steam`, `epic` or `manual`.
    pub store: String,
    /// Steam build id, Epic version, or a fingerprint of the pak: extracted data is kept per build.
    pub build: String,
    /// Folder of the character saves, when it is known.
    pub save_dir: Option<PathBuf>,
}

/// Whether `dir` is a game installation (it has the game's pak folder).
pub fn is_game_dir(dir: &Path) -> bool {
    dir.join("ProjectAlpha/Content/Paks").is_dir()
}

/// `"key"  "value"` pairs of a Valve KeyValues file (`.acf`, `.vdf`), in file order. Nesting is ignored:
/// the keys the planner needs are unique.
pub fn vdf_pairs(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut parts = Vec::new();
        let mut cur = String::new();
        let (mut quoted, mut escaped) = (false, false);
        for c in line.chars() {
            if !quoted {
                if c == '"' {
                    quoted = true;
                    cur.clear();
                }
                continue;
            }
            if escaped {
                cur.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                quoted = false;
                parts.push(std::mem::take(&mut cur));
            } else {
                cur.push(c);
            }
        }
        if let [k, v] = parts.as_slice() {
            out.push((k.clone(), v.clone()));
        }
    }
    out
}

fn vdf_value(text: &str, key: &str) -> Option<String> {
    vdf_pairs(text).into_iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
}

/// Steam folders to look in (each has `steamapps/libraryfolders.vdf`).
fn steam_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(windows) {
        if let Some(p) = windows_steam_path() {
            out.push(p);
        }
        for var in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(p) = std::env::var_os(var) {
                out.push(PathBuf::from(p).join("Steam"));
            }
        }
    } else if cfg!(target_os = "macos") {
        out.push(home().join("Library/Application Support/Steam"));
        // CrossOver bottles: the game is Windows-only, so on a Mac it runs in a Windows Steam.
        let bottles = home().join("Library/Application Support/CrossOver/Bottles");
        for b in fs::read_dir(bottles).into_iter().flatten().flatten() {
            out.push(b.path().join("drive_c/Program Files (x86)/Steam"));
        }
    } else {
        let h = home();
        for p in [".steam/steam", ".steam/root", ".local/share/Steam", ".var/app/com.valvesoftware.Steam/.local/share/Steam", ".var/app/com.valvesoftware.Steam/data/Steam"] {
            out.push(h.join(p));
        }
    }
    out
}

/// `HKCU\Software\Valve\Steam\SteamPath`, read with `reg query` (no registry crate needed).
fn windows_steam_path() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let mut cmd = std::process::Command::new("reg");
    cmd.args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"]);
    #[cfg(windows)]
    {
        // The app has no console: do not flash one.
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd.output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| l.contains("SteamPath"))?;
    let value = line.split("REG_SZ").nth(1)?.trim();
    (!value.is_empty()).then(|| PathBuf::from(value.replace('/', "\\")))
}

/// Steam library folders: the Steam folder itself and the ones listed in `libraryfolders.vdf`.
fn steam_libraries() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for root in steam_roots() {
        if !root.join("steamapps").is_dir() {
            continue;
        }
        out.push(root.clone());
        if let Ok(text) = fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
            out.extend(vdf_pairs(&text).into_iter().filter(|(k, _)| k == "path").map(|(_, v)| PathBuf::from(v)));
        }
    }
    dedupe(out)
}

fn dedupe(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    paths.into_iter().filter(|p| seen.insert(fs::canonicalize(p).unwrap_or_else(|_| p.clone()))).collect()
}

/// Save folder of a Windows install: `%LOCALAPPDATA%\ProjectAlpha\Saved\SaveGames`.
fn windows_save_dir() -> Option<PathBuf> {
    cfg!(windows).then(|| dirs::data_local_dir()).flatten().map(|d| d.join("ProjectAlpha/Saved/SaveGames"))
}

/// Save folder inside a Wine prefix (`pfx/` of Proton, or a CrossOver bottle).
fn prefix_save_dir(drive_c: &Path) -> Option<PathBuf> {
    let users = drive_c.join("users");
    fs::read_dir(users)
        .into_iter()
        .flatten()
        .flatten()
        .map(|u| u.path().join("AppData/Local/ProjectAlpha/Saved/SaveGames"))
        .find(|p| p.is_dir())
}

/// Save folder of a Steam install in `library` (Proton's prefix on Linux, CrossOver's bottle on a Mac).
fn steam_save_dir(library: &Path) -> Option<PathBuf> {
    if cfg!(windows) {
        return windows_save_dir();
    }
    let proton = library.join("steamapps/compatdata").join(STEAM_APP_ID).join("pfx/drive_c");
    if let Some(p) = prefix_save_dir(&proton) {
        return Some(p);
    }
    // CrossOver: <bottle>/drive_c/Program Files (x86)/Steam is the library, its drive_c is two levels up.
    library.ancestors().find(|a| a.file_name().is_some_and(|n| n == "drive_c")).and_then(prefix_save_dir)
}

fn steam_installs() -> Vec<GameInstall> {
    let mut out = Vec::new();
    for lib in steam_libraries() {
        let acf = lib.join("steamapps").join(format!("appmanifest_{STEAM_APP_ID}.acf"));
        let Ok(text) = fs::read_to_string(&acf) else { continue };
        let Some(installdir) = vdf_value(&text, "installdir") else { continue };
        let dir = lib.join("steamapps/common").join(installdir);
        if !is_game_dir(&dir) {
            continue;
        }
        let build = vdf_value(&text, "buildid").filter(|b| !b.is_empty()).unwrap_or_else(|| pak_fingerprint(&dir));
        out.push(GameInstall { save_dir: steam_save_dir(&lib), dir, store: "steam".into(), build });
    }
    out
}

/// Epic Games Launcher (Windows) and Heroic (Linux) installs.
fn epic_installs() -> Vec<GameInstall> {
    let mut out = Vec::new();
    let mut add = |dir: PathBuf, version: Option<String>, save_dir: Option<PathBuf>| {
        if is_game_dir(&dir) {
            let build = version.map(|v| format!("epic-{v}")).unwrap_or_else(|| pak_fingerprint(&dir));
            out.push(GameInstall { dir, store: "epic".into(), build, save_dir });
        }
    };
    if cfg!(windows) {
        let program_data = std::env::var_os("ProgramData").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
        for e in fs::read_dir(program_data.join(r"Epic\EpicGamesLauncher\Data\Manifests")).into_iter().flatten().flatten() {
            let Ok(text) = fs::read_to_string(e.path()) else { continue };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
            if v["DisplayName"].as_str().is_some_and(|n| n.contains("Dwarven Realms")) {
                if let Some(dir) = v["InstallLocation"].as_str() {
                    add(PathBuf::from(dir), v["AppVersionString"].as_str().map(str::to_string), windows_save_dir());
                }
            }
        }
    } else {
        let installed = dirs::config_dir().unwrap_or_default().join("heroic/legendaryConfig/legendary/installed.json");
        if let Ok(v) = fs::read_to_string(installed).map_err(anyhow::Error::from).and_then(|t| Ok(serde_json::from_str::<serde_json::Value>(&t)?)) {
            for game in v.as_object().into_iter().flat_map(|o| o.values()) {
                if game["title"].as_str().is_some_and(|n| n.contains("Dwarven Realms")) {
                    if let Some(dir) = game["install_path"].as_str() {
                        add(PathBuf::from(dir), game["version"].as_str().map(str::to_string), None);
                    }
                }
            }
        }
    }
    out
}

/// Build id of a folder the player picked: the Steam manifest next to it, else a fingerprint of the pak.
fn manual_build(dir: &Path) -> String {
    let acf = dir.join("../..").join(format!("appmanifest_{STEAM_APP_ID}.acf"));
    fs::read_to_string(acf).ok().and_then(|t| vdf_value(&t, "buildid")).filter(|b| !b.is_empty()).unwrap_or_else(|| pak_fingerprint(dir))
}

/// Size and modification time of the game's pak: changes with every patch.
pub fn pak_fingerprint(dir: &Path) -> String {
    let pak = dir.join("ProjectAlpha/Content/Paks/ProjectAlpha-Windows.pak");
    match fs::metadata(&pak) {
        Ok(m) => {
            let mtime = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
            format!("pak-{:x}-{mtime:x}", m.len())
        }
        Err(_) => "unknown".into(),
    }
}

/// A folder the player picked as the game folder.
pub fn manual_install(dir: &Path) -> Option<GameInstall> {
    if !is_game_dir(dir) {
        return None;
    }
    let library = dir.parent().and_then(Path::parent).and_then(Path::parent);
    let save_dir = if cfg!(windows) { windows_save_dir() } else { library.and_then(steam_save_dir) };
    Some(GameInstall { dir: dir.to_path_buf(), store: "manual".into(), build: manual_build(dir), save_dir })
}

/// Every installation found: Steam first, then Epic.
pub fn find_installs() -> Vec<GameInstall> {
    let mut out = steam_installs();
    out.extend(epic_installs());
    let mut seen = std::collections::HashSet::new();
    out.retain(|g| seen.insert(fs::canonicalize(&g.dir).unwrap_or_else(|_| g.dir.clone())));
    out
}

/// Choices the player made in the setup screen.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    /// Game folder picked by hand (else the first installation found).
    #[serde(default)]
    pub game_dir: Option<PathBuf>,
    /// Save folder picked by hand (else the installation's).
    #[serde(default)]
    pub save_dir: Option<PathBuf>,
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

/// The interface's preferences (`dr-planner.*` keys of the UI's local storage), kept in `<data dir>/prefs.json`:
/// the webview's own storage differs between the dev server and the installed app, and goes with its data.
fn prefs_path() -> PathBuf {
    data_dir().join("prefs.json")
}

static PREFS: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn prefs() -> std::collections::BTreeMap<String, String> {
    fs::read_to_string(prefs_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// Sets (`Some`) or removes (`None`) preferences.
pub fn set_prefs(changes: std::collections::BTreeMap<String, Option<String>>) -> Result<()> {
    let _one = PREFS.lock().unwrap_or_else(|e| e.into_inner());
    let mut all = prefs();
    for (k, v) in changes {
        match v {
            Some(v) => all.insert(k, v),
            None => all.remove(&k),
        };
    }
    let p = prefs_path();
    fs::create_dir_all(p.parent().unwrap())?;
    fs::write(&p, serde_json::to_string_pretty(&all)?).with_context(|| format!("writing {}", p.display()))
}

impl Settings {
    pub fn load() -> Settings {
        fs::read_to_string(settings_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let p = settings_path();
        fs::create_dir_all(p.parent().unwrap())?;
        fs::write(&p, serde_json::to_string_pretty(self)?).with_context(|| format!("writing {}", p.display()))
    }

    /// The installation in use: the picked folder, else the first one found.
    pub fn install(&self) -> Option<GameInstall> {
        match &self.game_dir {
            Some(d) => manual_install(d),
            None => find_installs().into_iter().next(),
        }
    }

    /// The save folder in use.
    pub fn save_dir(&self) -> Option<PathBuf> {
        self.save_dir.clone().or_else(|| self.install().and_then(|g| g.save_dir)).or_else(windows_save_dir).filter(|p| p.is_dir())
    }
}

/// Folder of the data extracted for game build `build` (it holds `data/extracted` and `data/assets`).
pub fn game_data_root(build: &str) -> PathBuf {
    let safe: String = build.chars().map(|c| if c.is_ascii_alphanumeric() || "-_.".contains(c) { c } else { '_' }).collect();
    data_dir().join("game").join(safe)
}

/// Layout version of the extracted data (`data/extracted/FORMAT`, written by dr-extract and extract.sh).
/// Data below it is extracted again. 2: elven_tree.json. 3: icons of the resources and rupture fortunes, and of the
/// textures inside the exported folders.
pub const DATA_FORMAT: u32 = 3;

/// Layout version of the data in `root` (1 when the file is missing).
pub fn data_format(root: &Path) -> u32 {
    fs::read_to_string(root.join("data/extracted/FORMAT")).ok().and_then(|t| t.trim().parse().ok()).unwrap_or(1)
}

/// Whether `root` holds a complete extraction (`BUILD` is written last).
pub fn data_ready(root: &Path) -> bool {
    root.join("data/extracted/BUILD").is_file() && root.join("data/assets").is_dir()
}

/// Build of the data in `root` (`data/extracted/BUILD`: "build <id>").
pub fn data_build(root: &Path) -> Option<String> {
    fs::read_to_string(root.join("data/extracted/BUILD")).ok().map(|t| t.trim().trim_start_matches("build").trim().to_string())
}

/// Extracted builds, newest first.
pub fn extracted_roots() -> Vec<PathBuf> {
    let mut roots: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(data_dir().join("game"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| data_ready(p))
        .map(|p| (fs::metadata(p.join("data/extracted/BUILD")).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH), p))
        .collect();
    roots.sort_by(|a, b| b.0.cmp(&a.0));
    roots.into_iter().map(|(_, p)| p).collect()
}

/// Folder of the mappings (`.usmap`) the player imported.
pub fn mappings_dir() -> PathBuf {
    data_dir().join("mappings")
}

/// The newest imported mappings file.
pub fn latest_mappings() -> Option<PathBuf> {
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(mappings_dir())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("usmap")))
        .map(|p| (fs::metadata(&p).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH), p))
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0));
    files.into_iter().next().map(|(_, p)| p)
}

/// Copies a `.usmap` into the mappings folder; returns the copy.
pub fn import_mappings(src: &Path) -> Result<PathBuf> {
    let data = fs::read(src).with_context(|| format!("reading {}", src.display()))?;
    // Every .usmap starts with the magic 0x30C4 (little endian).
    anyhow::ensure!(data.len() > 16 && data[0] == 0xC4 && data[1] == 0x30, "{} is not a .usmap mappings file", src.display());
    let dir = mappings_dir();
    fs::create_dir_all(&dir)?;
    let name = src.file_name().map(|n| n.to_owned()).unwrap_or_else(|| "mappings.usmap".into());
    let dst = dir.join(name);
    fs::write(&dst, data)?;
    Ok(dst)
}

/// `.usmap` files the UE4SS dumper wrote in the game folder (`ProjectAlpha/Binaries/Win64/ue4ss/`).
pub fn dumped_mappings(game: &Path) -> Vec<PathBuf> {
    let win64 = game.join("ProjectAlpha/Binaries/Win64");
    [win64.join("ue4ss"), win64]
        .iter()
        .flat_map(|d| fs::read_dir(d).into_iter().flatten().flatten())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("usmap")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_steam_manifests() {
        let acf = "\"AppState\"\n{\n\t\"appid\"\t\t\"2015240\"\n\t\"installdir\"\t\t\"Dwarven Realms\"\n\t\"buildid\"\t\t\"24115719\"\n}\n";
        assert_eq!(vdf_value(acf, "installdir").as_deref(), Some("Dwarven Realms"));
        assert_eq!(vdf_value(acf, "buildid").as_deref(), Some("24115719"));
        let lib = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t}\n}\n";
        assert_eq!(vdf_pairs(lib).into_iter().filter(|(k, _)| k == "path").map(|(_, v)| v).collect::<Vec<_>>(), vec![r"D:\SteamLibrary".to_string()]);
    }

    #[test]
    fn build_folder_names_are_safe() {
        assert!(game_data_root("epic-1.2/3:4").ends_with("game/epic-1.2_3_4"));
    }
}
