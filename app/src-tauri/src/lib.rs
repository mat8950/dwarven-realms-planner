//! Tauri backend. The stat engine is single-threaded (Rc-based VM), so worker
//! threads own it and commands talk to them over channels. Every request is
//! journaled to `<cache dir>/dr-planner/app.log` (see dr_core::debuglog).
//! The game data comes from the player's installation: see `gamesetup`.

pub mod gamesetup;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};
use std::thread;
use std::time::Instant;

use dr_core::catalog::Catalog;
use dr_core::debuglog;
use dr_core::gamedata::GameData;
use dr_core::history::{self, Snapshot};
use dr_core::planner::{BuildInput, CharacterView, Impact, Planner, Probe, SaveEntry, StatsView, default_save_dir, list_saves};
use serde::Serialize;

type Reply<T> = Sender<Result<T, String>>;

enum Req {
    Catalog(Reply<Catalog>),
    Saves(Reply<Vec<SaveEntry>>),
    Open(String, Reply<CharacterView>),
    Compute(BuildInput, Reply<StatsView>),
    /// Stat impact analysis; opens `save` first if it is not the loaded one.
    Analyze(String, BuildInput, Vec<Probe>, Reply<Vec<Impact>>),
    /// Reloads the game texts in another language (locres folder name).
    SetLanguage(String, Reply<Catalog>),
    /// Snapshots every character whose save changed; returns the keys that got a new entry.
    HistoryCapture(Reply<Vec<String>>),
    /// Reloads the game data (after an extraction), in the current language.
    Reload(Reply<()>),
}

/// Main worker (interactive calls) and a second one for the slower impact analysis and the
/// history captures, so recomputing stats never waits behind them.
struct Worker(Mutex<Sender<Req>>, Mutex<Sender<Req>>);

/// Error of every call that needs game data while none is extracted (the UI shows the setup screen).
const NO_DATA: &str = "game data not extracted yet";

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// Runs a request handler; a panic is journaled and returned as an error instead of
/// killing the worker thread.
fn guarded<T>(src: &str, what: &str, f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let t = Instant::now();
    let r = catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|p| {
        let msg = p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_default();
        Err(format!("panic in {what}: {msg}"))
    });
    if let Err(e) = &r {
        debuglog::error(src, &format!("{what} failed after {:.0} ms: {e}", ms(t)));
    }
    r
}

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

/// Game data, catalog and planner for one language. Game data lives for the whole run
/// (the planner borrows it): each language is loaded at most once per worker.
fn load_language(name: &str, lang: &str, cache: &mut HashMap<String, &'static GameData>) -> Result<(Catalog, Planner<'static>), String> {
    let t = Instant::now();
    let root = gamesetup::data_root().ok_or(NO_DATA)?;
    let gd = match cache.get(lang) {
        Some(gd) => *gd,
        None => {
            let gd: &'static GameData = Box::leak(Box::new(GameData::load(&root, lang).map_err(err)?));
            cache.insert(lang.to_string(), gd);
            gd
        }
    };
    let catalog = Catalog::build(gd).map_err(err)?;
    let planner = Planner::new(gd, &catalog).map_err(err)?;
    debuglog::info(name, &format!("game data ({lang}) loaded in {:.0} ms (build {}, root {})", ms(t), catalog.build, root.display()));
    Ok((catalog, planner))
}

fn spawn_worker(name: &'static str) -> Sender<Req> {
    let (tx, rx) = channel::<Req>();
    thread::spawn(move || {
        let mut cache = HashMap::new();
        let mut lang = "fr".to_string();
        let (mut catalog, mut planner) = match load_language(name, &lang, &mut cache) {
            Ok((c, p)) => (Some(c), Some(p)),
            Err(e) => {
                debuglog::error(name, &format!("loading game data failed: {e}"));
                (None, None)
            }
        };
        let missing = || NO_DATA.to_string();
        let mut opened = String::new();
        let mut seen = HashMap::new();
        for req in rx {
            match req {
                Req::Catalog(r) => {
                    let _ = r.send(catalog.clone().ok_or_else(missing));
                }
                Req::Saves(r) => {
                    let _ = r.send(guarded(name, "list saves", || list_saves(&default_save_dir()).map_err(err)));
                }
                Req::Open(path, r) => {
                    let t = Instant::now();
                    let res = guarded(name, &format!("open {path}"), || match planner.as_mut() {
                        Some(p) => p.open(path.as_ref()).map_err(err),
                        None => Err(missing()),
                    });
                    if let Ok(c) = &res {
                        debuglog::info(name, &format!("opened {} lvl {} ({}) in {:.0} ms: {path}", c.name, c.level, c.race, ms(t)));
                    }
                    let _ = r.send(res);
                    opened = path;
                }
                Req::Compute(b, r) => {
                    let res = guarded(name, "compute", || match planner.as_mut() {
                        Some(p) => p.compute(&b).map_err(err),
                        None => Err(missing()),
                    });
                    if let Ok(s) = &res {
                        // Only noteworthy computes: slow or close to the step limit.
                        if s.elapsed_ms > 200.0 || s.steps * 2 > s.max_steps {
                            debuglog::info(name, &format!("compute {:.0} ms, steps {}/{}, missing {}", s.elapsed_ms, s.steps, s.max_steps, s.missing.len()));
                        }
                    }
                    let _ = r.send(res);
                }
                Req::Analyze(save, b, probes, r) => {
                    let t = Instant::now();
                    let n = probes.len();
                    let res = guarded(name, "analyze", || match planner.as_mut() {
                        Some(p) => {
                            if opened != save {
                                p.open(save.as_ref()).map_err(err)?;
                                opened = save.clone();
                            }
                            p.analyze(&b, &probes).map_err(err)
                        }
                        None => Err(missing()),
                    });
                    if res.is_ok() {
                        debuglog::info(name, &format!("analyzed {n} stats in {:.0} ms", ms(t)));
                    }
                    let _ = r.send(res);
                }
                Req::SetLanguage(l, r) => {
                    let res = guarded(name, &format!("language {l}"), || load_language(name, &l, &mut cache));
                    let reply = res.map(|(c, p)| {
                        catalog = Some(c.clone());
                        planner = Some(p);
                        lang = l;
                        // The new planner has no character: the UI reopens its save.
                        opened.clear();
                        c
                    });
                    let _ = r.send(reply);
                }
                Req::Reload(r) => {
                    // New data: the cached languages are from the previous extraction.
                    cache.clear();
                    let res = guarded(name, "reload game data", || load_language(name, &lang, &mut cache));
                    let reply = res.map(|(c, p)| {
                        catalog = Some(c);
                        planner = Some(p);
                        opened.clear();
                        seen.clear();
                    });
                    let _ = r.send(reply);
                }
                Req::HistoryCapture(r) => {
                    let t = Instant::now();
                    let res = guarded(name, "history capture", || {
                        let p = planner.as_mut().ok_or_else(missing)?;
                        let mut added = Vec::new();
                        for s in list_saves(&default_save_dir()).map_err(err)? {
                            let path = PathBuf::from(&s.path);
                            let key = history::save_key(&path);
                            match history::capture(p, &path, &key, &mut seen) {
                                Ok(true) => added.push(key),
                                Ok(false) => {}
                                Err(e) => debuglog::error(name, &format!("history of {}: {e:#}", s.path)),
                            }
                        }
                        // Captures open saves: the next analysis reopens its own.
                        opened.clear();
                        Ok(added)
                    });
                    if let Ok(a) = &res {
                        if !a.is_empty() {
                            debuglog::info(name, &format!("history: {} new snapshot(s) in {:.0} ms", a.len(), ms(t)));
                        }
                    }
                    let _ = r.send(res);
                }
            }
        }
    });
    tx
}

fn send<T>(to: &Mutex<Sender<Req>>, make: impl FnOnce(Reply<T>) -> Req) -> Result<T, String> {
    let (tx, rx) = channel();
    to.lock().map_err(|e| e.to_string())?.send(make(tx)).map_err(|e| format!("worker unavailable: {e}"))?;
    rx.recv().map_err(|e| format!("worker stopped: {e}"))?
}

#[tauri::command]
async fn catalog(w: tauri::State<'_, Worker>) -> Result<Catalog, String> {
    send(&w.0, Req::Catalog)
}

#[tauri::command]
async fn saves(w: tauri::State<'_, Worker>) -> Result<Vec<SaveEntry>, String> {
    send(&w.0, Req::Saves)
}

#[tauri::command]
async fn open_save(w: tauri::State<'_, Worker>, path: String) -> Result<CharacterView, String> {
    send(&w.0, |r| Req::Open(path, r))
}

#[tauri::command]
async fn compute(w: tauri::State<'_, Worker>, build: BuildInput) -> Result<StatsView, String> {
    send(&w.0, |r| Req::Compute(build, r))
}

#[tauri::command]
async fn analyze(w: tauri::State<'_, Worker>, save: String, build: BuildInput, probes: Vec<Probe>) -> Result<Vec<Impact>, String> {
    send(&w.1, |r| Req::Analyze(save, build, probes, r))
}

/// Both workers switch language; the main one's catalog is returned.
#[tauri::command]
async fn set_language(w: tauri::State<'_, Worker>, lang: String) -> Result<Catalog, String> {
    // The analysis worker switches when it is free (it may be analysing): its results do not
    // depend on the language, so the reply does not wait for it.
    let (tx, _rx) = channel();
    w.1.lock().map_err(|e| e.to_string())?.send(Req::SetLanguage(lang.clone(), tx)).map_err(|e| format!("worker unavailable: {e}"))?;
    send(&w.0, |r| Req::SetLanguage(lang, r))
}

#[tauri::command]
async fn history_capture(w: tauri::State<'_, Worker>) -> Result<Vec<String>, String> {
    send(&w.1, Req::HistoryCapture)
}

/// Snapshots of the character whose save is `save`, oldest first.
#[tauri::command]
fn history(save: String) -> Result<Vec<Snapshot>, String> {
    history::load(&history::save_key(save.as_ref())).map_err(err)
}

/// Extracts the game data from the installation in use, then reloads both workers on it.
#[tauri::command]
async fn extract(app: tauri::AppHandle, w: tauri::State<'_, Worker>) -> Result<String, String> {
    use tauri::Emitter;
    let run = move || gamesetup::run_extraction(|v| {
        let _ = app.emit("extract-progress", v);
    });
    let root = tauri::async_runtime::spawn_blocking(run).await.map_err(|e| e.to_string())??;
    send(&w.1, Req::Reload)?;
    send(&w.0, Req::Reload)?;
    Ok(root.display().to_string())
}

/// Frontend errors and notable events go to the same journal.
#[tauri::command]
fn log_event(level: String, source: String, message: String) {
    debuglog::write(&level.to_uppercase(), &format!("ui:{source}"), &message);
}

#[derive(Serialize)]
struct DebugInfo {
    version: String,
    log_path: String,
    data_root: String,
    save_dir: String,
    recent: Vec<String>,
}

#[tauri::command]
fn debug_info() -> DebugInfo {
    DebugInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        log_path: debuglog::path().display().to_string(),
        data_root: gamesetup::data_root().map(|p| p.display().to_string()).unwrap_or_default(),
        save_dir: default_save_dir().display().to_string(),
        recent: debuglog::tail(200),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    debuglog::install_panic_hook("app");
    gamesetup::init_root();
    let root = gamesetup::data_root().map(|p| p.display().to_string()).unwrap_or_else(|| "none".into());
    debuglog::info("app", &format!("start v{} ({}), data {root}", env!("CARGO_PKG_VERSION"), if cfg!(debug_assertions) { "debug" } else { "release" }));
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("drassets", |_ctx, request| gamesetup::serve_asset(request))
        .manage(Worker(Mutex::new(spawn_worker("main")), Mutex::new(spawn_worker("analysis"))))
        .invoke_handler(tauri::generate_handler![
            catalog,
            saves,
            open_save,
            compute,
            analyze,
            set_language,
            history_capture,
            history,
            log_event,
            debug_info,
            extract,
            gamesetup::setup_status,
            gamesetup::set_game_dir,
            gamesetup::set_save_dir,
            gamesetup::import_mappings,
            gamesetup::dumper_install,
            gamesetup::dumper_remove,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
