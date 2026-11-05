//! Watches the newest character save and prints what changed each time the game writes it: since the previous
//! write and since the start (the session). The terminal version of the planner's session view.
//!
//!   cargo run --release -p dr-cli --example session [save.sav] [--all] [--keep <dir>]
//!   cargo run --release -p dr-cli --example session import <dir> <save key>
//!
//! `--all` also lists every other save field that changed (to find what the game records);
//! `--keep <dir>` copies each version of the save (and the world save) there.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use dr_core::session::{self, Snapshot};
use dr_core::value::Value;

fn human(v: f64) -> String {
    let a = v.abs();
    if a >= 1e12 {
        format!("{:.2}T", v / 1e12)
    } else if a >= 1e9 {
        format!("{:.2}B", v / 1e9)
    } else if a >= 1e6 {
        format!("{:.2}M", v / 1e6)
    } else if a >= 1e4 {
        format!("{:.1}k", v / 1e3)
    } else if v.fract() != 0.0 {
        format!("{v:.2}")
    } else {
        format!("{v:.0}")
    }
}

fn signed(d: f64) -> String {
    if d > 0.0 { format!("+{}", human(d)) } else { human(d) }
}

/// Every scalar of the save, by path (arrays by index, maps by key).
fn flatten(v: &Value, path: String, out: &mut BTreeMap<String, String>) {
    match v {
        Value::Struct(s) => {
            for (k, f) in &s.fields {
                flatten(f, format!("{path}.{}", k.split('_').next().unwrap_or(k)), out);
            }
        }
        Value::Array(a) => {
            out.insert(format!("{path}.len"), a.len().to_string());
            for (i, e) in a.iter().enumerate() {
                flatten(e, format!("{path}[{i}]"), out);
            }
        }
        Value::Map(m) => {
            out.insert(format!("{path}.len"), m.len().to_string());
            for (k, e) in m {
                flatten(e, format!("{path}[{}]", format!("{k:?}").chars().take(60).collect::<String>()), out);
            }
        }
        other => {
            out.insert(path, format!("{other:?}").chars().take(80).collect());
        }
    }
}

fn flat(path: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Ok(sav) = dr_core::save::load(path) {
        for (k, v) in &sav.fields {
            flatten(v, k.to_string(), &mut out);
        }
    }
    out
}

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

fn newest_save(dir: &Path) -> Option<PathBuf> {
    dr_core::planner::list_saves(dir).ok()?.into_iter().next().map(|s| PathBuf::from(s.path))
}

/// `import <dir> <save key>`: adds the copies `--keep` made (`<unix time>.sav`) to the character's journal.
fn import(dir: &Path, key: &str) -> anyhow::Result<()> {
    let mut files: Vec<(f64, PathBuf)> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter_map(|p| {
            let stem = p.file_stem()?.to_str()?.to_string();
            (p.extension()? == "sav").then_some(())?;
            // Copied about half a second after the game wrote the save.
            Some((stem.parse::<f64>().ok()? - 0.5, p))
        })
        .collect();
    files.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (t, p) in files {
        println!("{} {}", p.display(), if session::import(key, &p, t)? { "added" } else { "already there" });
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("import") {
        return import(Path::new(args.get(1).map(String::as_str).unwrap_or(".")), args.get(2).map(String::as_str).unwrap_or(""));
    }
    let all = args.iter().any(|a| a == "--all");
    let keep = args.iter().position(|a| a == "--keep").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let explicit = args.iter().find(|a| a.ends_with(".sav")).map(PathBuf::from);
    let dir = dr_core::planner::default_save_dir();
    let save = match explicit.or_else(|| newest_save(&dir)) {
        Some(p) => p,
        None => anyhow::bail!("no character save in {}", dir.display()),
    };
    let world = dr_core::inventory::world_save(&save);
    if let Some(k) = &keep {
        std::fs::create_dir_all(k)?;
    }
    println!("watching {}", save.display());

    let (mut first, mut prev): (Option<Snapshot>, Option<Snapshot>) = (None, None);
    let mut prev_flat = BTreeMap::new();
    let (mut seen, mut seen_world) = (None, None);
    loop {
        let (m, mw) = (mtime(&save), mtime(&world));
        if mw != seen_world {
            if seen_world.is_some() {
                println!("  (world save written {})", chrono_now());
            }
            seen_world = mw;
            if let (Some(k), true) = (&keep, world.is_file()) {
                let _ = std::fs::copy(&world, k.join(format!("{}_WRL.sav", stamp())));
            }
        }
        if m != seen {
            // Let the game finish writing.
            std::thread::sleep(Duration::from_millis(500));
            seen = mtime(&save);
            match session::read(&save) {
                Ok(s) => {
                    if let Some(k) = &keep {
                        let _ = std::fs::copy(&save, k.join(format!("{}.sav", stamp())));
                    }
                    report(first.as_ref(), prev.as_ref(), &s);
                    if all {
                        let f = flat(&save);
                        if !prev_flat.is_empty() {
                            let mut n = 0;
                            for (k, v) in &f {
                                if prev_flat.get(k) != Some(v) && !k.contains("Transform") && !k.contains(".Resources[") {
                                    n += 1;
                                    if n <= 80 {
                                        println!("    {k}: {} -> {v}", prev_flat.get(k).map(String::as_str).unwrap_or("-"));
                                    }
                                }
                            }
                            for k in prev_flat.keys().filter(|k| !f.contains_key(*k)).take(20) {
                                println!("    {k}: removed");
                            }
                            if n > 80 {
                                println!("    ... {} more changed fields", n - 80);
                            }
                        }
                        prev_flat = f;
                    }
                    if first.is_none() {
                        first = Some(s.clone());
                    }
                    prev = Some(s);
                }
                Err(e) => println!("{}: {e:#}", chrono_now()),
            }
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

fn report(first: Option<&Snapshot>, prev: Option<&Snapshot>, s: &Snapshot) {
    println!("\n== {} · lvl {} · {} · save of {} ==", s.name, s.level, s.zone, clock(s.time));
    let (Some(first), Some(prev)) = (first, prev) else {
        let res = |k: &str| s.resources.get(k).copied().unwrap_or(0.0);
        println!("  start: ElvenFragments {}  CasinoKey {}  Coins {}", human(res("ElvenFragments")), human(res("CasinoKey")), human(res("Coins")));
        return;
    };
    let since = s.time - prev.time;
    println!("  since previous save ({}):", duration(since));
    for (group, key, a, b) in session::changes(prev, s) {
        println!("    {group:<10} {key:<24} {:>12} {}", human(b), signed(b - a));
    }
    let elapsed = s.time - first.time;
    let hours = elapsed / 3600.0;
    println!("  session ({}):", duration(elapsed));
    for (group, key, a, b) in session::changes(first, s) {
        let rate = if hours > 0.01 { format!("  {}/h", signed((b - a) / hours)) } else { String::new() };
        println!("    {group:<10} {key:<24} {:>12} {}{rate}", human(b), signed(b - a));
    }
}

fn duration(s: f64) -> String {
    let s = s.max(0.0) as u64;
    if s >= 3600 { format!("{}h{:02}m", s / 3600, s / 60 % 60) } else { format!("{}m{:02}s", s / 60, s % 60) }
}

fn clock(unix: f64) -> String {
    let t = unix as i64 + local_offset();
    format!("{:02}:{:02}:{:02}", t / 3600 % 24, t / 60 % 60, t % 60)
}

fn chrono_now() -> String {
    clock(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0))
}

fn stamp() -> String {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0).to_string()
}

/// Local time offset in seconds, from `date +%z` (good enough for a terminal clock).
fn local_offset() -> i64 {
    std::process::Command::new("date")
        .arg("+%z")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|z| {
            let z = z.trim();
            let sign = if z.starts_with('-') { -1 } else { 1 };
            let (h, m) = (z.get(1..3)?.parse::<i64>().ok()?, z.get(3..5)?.parse::<i64>().ok()?);
            Some(sign * (h * 3600 + m * 60))
        })
        .unwrap_or(0)
}
