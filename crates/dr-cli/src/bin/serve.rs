//! HTTP bridge exposing the Tauri commands, to run the UI in a plain browser:
//! `cargo run --release --bin serve` then `npm run dev` in app/ (Vite proxies /api).

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use dr_core::catalog::Catalog;
use dr_core::debuglog;
use dr_core::gamedata::GameData;
use dr_core::history;
use dr_core::planner::{BuildInput, Planner, default_save_dir, list_saves};
use serde_json::{Value, json};

fn main() -> Result<()> {
    let root = std::env::var_os("DR_PLANNER_ROOT").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    // One game data per language, loaded on first use (the planner borrows it).
    let mut cache: HashMap<String, &'static GameData> = HashMap::new();
    let mut load = |lang: &str| -> Result<(Catalog, Planner<'static>)> {
        let gd = match cache.get(lang) {
            Some(gd) => *gd,
            None => {
                let gd: &'static GameData = Box::leak(Box::new(GameData::load(&root, lang)?));
                cache.insert(lang.to_string(), gd);
                gd
            }
        };
        let catalog = Catalog::build(gd)?;
        let planner = Planner::new(gd, &catalog)?;
        Ok((catalog, planner))
    };
    let (mut catalog, mut planner) = load("fr")?;
    let mut opened = String::new();
    let mut seen = HashMap::new();
    let addr = std::env::var("DR_PLANNER_ADDR").unwrap_or_else(|_| "127.0.0.1:5174".into());
    let server = tiny_http::Server::http(&addr).map_err(|e| anyhow::anyhow!("{e}"))?;
    eprintln!("dr-planner api on http://{addr}");
    debuglog::install_panic_hook("bridge");
    debuglog::info("bridge", &format!("start on {addr}, data {}", root.display()));
    for mut req in server.incoming_requests() {
        let mut body = String::new();
        let _ = req.as_reader().read_to_string(&mut body);
        let args: Value = serde_json::from_str(&body).unwrap_or(json!({}));
        let cmd = req.url().trim_start_matches("/api/").to_string();
        let res: Result<Value> = (|| {
            Ok(match cmd.as_str() {
                "catalog" => serde_json::to_value(&catalog)?,
                "saves" => serde_json::to_value(list_saves(&default_save_dir())?)?,
                "open_save" => {
                    let path = args["path"].as_str().unwrap_or("").to_string();
                    let v = serde_json::to_value(planner.open(path.as_ref())?)?;
                    opened = path;
                    v
                }
                "analyze" => {
                    let save = args["save"].as_str().unwrap_or("").to_string();
                    if save != opened {
                        planner.open(save.as_ref())?;
                        opened = save;
                    }
                    let probes: Vec<dr_core::planner::Probe> = serde_json::from_value(args["probes"].clone())?;
                    serde_json::to_value(planner.analyze(&serde_json::from_value::<BuildInput>(args["build"].clone())?, &probes)?)?
                }
                "set_language" => {
                    (catalog, planner) = load(args["lang"].as_str().unwrap_or("fr"))?;
                    opened.clear();
                    serde_json::to_value(&catalog)?
                }
                "history_capture" => {
                    let mut added = Vec::new();
                    for s in list_saves(&default_save_dir())? {
                        let path = PathBuf::from(&s.path);
                        let key = history::save_key(&path);
                        if history::capture(&mut planner, &path, &key, &mut seen)? {
                            added.push(key);
                        }
                    }
                    opened.clear();
                    serde_json::to_value(added)?
                }
                "history" => serde_json::to_value(history::load(&history::save_key(args["save"].as_str().unwrap_or("").as_ref()))?)?,
                "compute" => serde_json::to_value(planner.compute(&serde_json::from_value::<BuildInput>(args["build"].clone())?)?)?,
                "log_event" => {
                    let a = |k: &str| args[k].as_str().unwrap_or("").to_string();
                    debuglog::write(&a("level").to_uppercase(), &format!("ui:{}", a("source")), &a("message"));
                    json!(null)
                }
                "debug_info" => json!({
                    "version": env!("CARGO_PKG_VERSION"),
                    "log_path": debuglog::path().display().to_string(),
                    "data_root": root.display().to_string(),
                    "save_dir": default_save_dir().display().to_string(),
                    "recent": debuglog::tail(200),
                }),
                other => anyhow::bail!("unknown command {other}"),
            })
        })();
        let (code, out) = match res {
            Ok(v) => (200, v.to_string()),
            Err(e) => {
                debuglog::error("bridge", &format!("{cmd} failed: {e:#}"));
                (500, json!(format!("{e:#}")).to_string())
            }
        };
        let header = tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap();
        let _ = req.respond(tiny_http::Response::from_string(out).with_status_code(code).with_header(header));
    }
    Ok(())
}
