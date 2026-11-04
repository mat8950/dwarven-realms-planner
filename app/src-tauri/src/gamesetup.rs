//! First-run setup: find the game, import the mappings, run the extraction (the `dr-extract`
//! sidecar) and serve the extracted textures and fonts to the UI (`drassets://` protocol).

use std::io::{BufRead, BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::RwLock;

use dr_core::debuglog;
use dr_core::setup::{self, GameInstall, Settings};
use serde::Serialize;

/// Folder holding `data/` (extracted game data and assets) in use, if any.
static ROOT: RwLock<Option<PathBuf>> = RwLock::new(None);

pub fn data_root() -> Option<PathBuf> {
    ROOT.read().ok().and_then(|r| r.clone())
}

fn set_root(root: Option<PathBuf>) {
    if let Ok(mut r) = ROOT.write() {
        *r = root;
    }
}

/// Repository root, for `tauri dev` with data from tools/extract/extract.sh.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Picks the data to load: `DR_PLANNER_ROOT`, else (debug builds) the repository's, else the data
/// extracted for the installed game build, else the newest extraction.
pub fn init_root() {
    let root = std::env::var_os("DR_PLANNER_ROOT").map(PathBuf::from).or_else(|| {
        if cfg!(debug_assertions) && setup::data_ready(&repo_root()) {
            return Some(repo_root());
        }
        let current = Settings::load().install().map(|g| setup::game_data_root(&g.build)).filter(|r| setup::data_ready(r));
        current.or_else(|| setup::extracted_roots().into_iter().next())
    });
    set_root(root);
}

#[derive(Serialize)]
pub struct SetupStatus {
    /// Installations found on this computer.
    installs: Vec<GameInstall>,
    /// The one in use (picked by hand or the first found).
    install: Option<GameInstall>,
    /// Game folder picked by hand.
    manual_dir: Option<String>,
    save_dir: Option<String>,
    /// Data in use, and its game build.
    data_root: Option<String>,
    data_build: Option<String>,
    /// Whether the data in use matches the installed game build.
    up_to_date: bool,
    /// Imported mappings file, and the ones the UE4SS dumper left in the game folder.
    mappings: Option<String>,
    dumped: Vec<String>,
    /// Whether the extractor is available (it is bundled with the installers).
    extractor: bool,
    /// Whether the planner's UE4SS mappings dumper is installed in the game.
    dumper_installed: bool,
    /// `linux`, `windows` or `macos`: the dumper needs a launch option under Proton or Wine.
    os: String,
}

fn show(p: &Path) -> String {
    p.display().to_string()
}

#[tauri::command]
pub fn setup_status() -> SetupStatus {
    let settings = Settings::load();
    let install = settings.install();
    let root = data_root();
    let data_build = root.as_deref().and_then(setup::data_build);
    let dumper_installed = install.as_ref().is_some_and(|g| g.dir.join("ProjectAlpha/Binaries/Win64/dr-planner-ue4ss.json").is_file());
    SetupStatus {
        installs: setup::find_installs(),
        up_to_date: match (&install, &data_build) {
            (Some(g), Some(b)) => &g.build == b,
            _ => false,
        },
        dumped: install.as_ref().map(|g| setup::dumped_mappings(&g.dir).iter().map(|p| show(p)).collect()).unwrap_or_default(),
        manual_dir: settings.game_dir.as_deref().map(show),
        save_dir: settings.save_dir().as_deref().map(show),
        install,
        data_root: root.as_deref().map(show),
        data_build,
        mappings: setup::latest_mappings().as_deref().map(show),
        extractor: extractor_path().is_some(),
        dumper_installed,
        os: std::env::consts::OS.to_string(),
    }
}

/// Uses `dir` as the game folder (`None`: back to automatic detection).
#[tauri::command]
pub fn set_game_dir(dir: Option<String>) -> Result<(), String> {
    let mut s = Settings::load();
    if let Some(d) = &dir {
        if !setup::is_game_dir(Path::new(d)) {
            return Err(format!("{d} is not a Dwarven Realms folder (no ProjectAlpha/Content/Paks inside)"));
        }
    }
    s.game_dir = dir.map(PathBuf::from);
    s.save().map_err(|e| format!("{e:#}"))
}

/// Uses `dir` as the save folder (`None`: the installation's).
#[tauri::command]
pub fn set_save_dir(dir: Option<String>) -> Result<(), String> {
    let mut s = Settings::load();
    s.save_dir = dir.map(PathBuf::from);
    s.save().map_err(|e| format!("{e:#}"))
}

/// Imports a `.usmap` file (picked by hand, or written by the UE4SS dumper).
#[tauri::command]
pub fn import_mappings(path: String) -> Result<String, String> {
    let dst = setup::import_mappings(Path::new(&path)).map_err(|e| format!("{e:#}"))?;
    debuglog::info("setup", &format!("mappings imported: {}", dst.display()));
    Ok(show(&dst))
}

/// The bundled extractor: next to the app's executable (Tauri copies sidecars there), or `DR_EXTRACT`.
fn extractor_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("DR_EXTRACT") {
        return Some(PathBuf::from(p));
    }
    let name = if cfg!(windows) { "dr-extract.exe" } else { "dr-extract" };
    let p = std::env::current_exe().ok()?.with_file_name(name);
    p.is_file().then_some(p)
}

/// Runs the extractor with `args`. `progress` gets its JSON lines; an `{"error"}` line or a failed exit
/// is the error.
fn run_sidecar(args: &[&std::ffi::OsStr], mut progress: impl FnMut(serde_json::Value)) -> Result<(), String> {
    let exe = extractor_path().ok_or("the extractor (dr-extract) is missing from this installation")?;
    let mut cmd = Command::new(&exe);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().map_err(|e| format!("cannot start {}: {e}", exe.display()))?;
    let mut stderr = child.stderr.take();
    let mut error = None;
    for line in BufReader::new(child.stdout.take().expect("piped stdout")).lines().map_while(Result::ok) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
        if let Some(e) = v["error"].as_str() {
            error = Some(e.to_string());
        }
        if let Some(w) = v["warn"].as_str() {
            debuglog::write("WARN", "extract", w);
        }
        progress(v);
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if let Some(e) = error {
        return Err(e);
    }
    if !status.success() {
        let mut tail = String::new();
        if let Some(s) = stderr.as_mut() {
            let _ = s.read_to_string(&mut tail);
        }
        return Err(format!("extractor exited with {status}: {}", tail.lines().rev().take(5).collect::<Vec<_>>().join(" | ")));
    }
    Ok(())
}

/// Extracts the game data for the installation in use, then switches to it. `progress` gets the
/// extractor's JSON lines (the app forwards them as `extract-progress` events). Returns the data folder.
pub fn run_extraction(progress: impl FnMut(serde_json::Value)) -> Result<PathBuf, String> {
    let install = Settings::load().install().ok_or("Dwarven Realms installation not found")?;
    let usmap = setup::latest_mappings().ok_or("no mappings file (.usmap) imported")?;
    let out = setup::game_data_root(&install.build);
    let cache = setup::cache_dir().join("oodle");
    debuglog::info("setup", &format!("extracting build {} from {} to {}", install.build, install.dir.display(), out.display()));
    let args: [&std::ffi::OsStr; 9] = [
        "game".as_ref(),
        install.dir.as_os_str(),
        out.as_os_str(),
        "--usmap".as_ref(),
        usmap.as_os_str(),
        "--build".as_ref(),
        install.build.as_ref(),
        "--cache".as_ref(),
        cache.as_os_str(),
    ];
    let res = run_sidecar(&args, progress).and_then(|()| if setup::data_ready(&out) { Ok(()) } else { Err("the extraction left incomplete data".into()) });
    if let Err(e) = res {
        debuglog::error("setup", &format!("extraction failed: {e}"));
        return Err(e);
    }
    debuglog::info("setup", &format!("extraction done: {}", out.display()));
    set_root(Some(out.clone()));
    Ok(out)
}

/// Installs (`install`) or removes (`remove`) the UE4SS mappings dumper in the game folder in use.
fn dumper(action: &str) -> Result<(), String> {
    let install = Settings::load().install().ok_or("Dwarven Realms installation not found")?;
    let res = run_sidecar(&["dumper".as_ref(), action.as_ref(), install.dir.as_os_str()], |_| {});
    match &res {
        Ok(()) => debuglog::info("setup", &format!("mappings dumper {action}: {}", install.dir.display())),
        Err(e) => debuglog::error("setup", &format!("mappings dumper {action} failed: {e}")),
    }
    res
}

/// Downloads RE-UE4SS and installs it with the DRDump mod in the game: once a character is loaded,
/// DRDump writes the mappings file.
#[tauri::command]
pub async fn dumper_install() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| dumper("install")).await.map_err(|e| e.to_string())?
}

/// Removes what `dumper_install` put in the game folder.
#[tauri::command]
pub async fn dumper_remove() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| dumper("remove")).await.map_err(|e| e.to_string())?
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or(""), 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `drassets://localhost/<path>` (`http://drassets.localhost/<path>` on Windows): a file of
/// `<data root>/data/assets`.
pub fn serve_asset(request: tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    let not_found = || tauri::http::Response::builder().status(404).body(Vec::new()).expect("response");
    let rel = decode(request.uri().path().trim_start_matches('/'));
    let rel = Path::new(&rel);
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return not_found();
    }
    let Some(root) = data_root() else { return not_found() };
    let Ok(bytes) = std::fs::read(root.join("data/assets").join(rel)) else { return not_found() };
    let mime = match rel.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "webp" => "image/webp",
        "png" => "image/png",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    };
    tauri::http::Response::builder()
        .header("Content-Type", mime)
        // Another extraction can change a file behind the same URL.
        .header("Cache-Control", "no-cache")
        .header("Access-Control-Allow-Origin", "*")
        .body(bytes)
        .expect("response")
}

#[cfg(test)]
mod tests {
    #[test]
    fn decodes_asset_paths() {
        assert_eq!(super::decode("EasySurvivalRPG%2FTextures%2FUI%2Fa%20b.webp"), "EasySurvivalRPG/Textures/UI/a b.webp");
        assert_eq!(super::decode("x%2"), "x%2");
    }
}
