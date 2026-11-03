//! Progression history from the command line.
//!   history capture             snapshot every character whose save changed
//!   history import <sav> <key>  add an older save (e.g. a validation copy) to a character's history
//!   history show <key>          list a character's snapshots
use std::collections::HashMap;

use dr_core::history;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let gd = dr_core::gamedata::GameData::load(&root, "fr")?;
    let cat = dr_core::catalog::Catalog::build(&gd)?;
    let mut p = dr_core::planner::Planner::new(&gd, &cat)?;
    match args.first().map(String::as_str) {
        Some("capture") => {
            let mut seen = HashMap::new();
            for s in dr_core::planner::list_saves(&dr_core::planner::default_save_dir())? {
                let path = std::path::PathBuf::from(&s.path);
                let key = history::save_key(&path);
                println!("{key} {} lvl {}: {}", s.name, s.level, if history::capture(&mut p, &path, &key, &mut seen)? { "new snapshot" } else { "unchanged" });
            }
        }
        Some("import") => {
            let (sav, key) = (args.get(1).expect("save"), args.get(2).expect("key"));
            let snap = history::snapshot(&mut p, sav.as_ref())?;
            let mut all = history::load(key)?;
            if all.iter().any(|s| s.save_modified == snap.save_modified) {
                println!("already in the history");
                return Ok(());
            }
            all.push(snap);
            all.sort_by_key(|s| s.save_modified);
            let dir = history::history_dir();
            std::fs::create_dir_all(&dir)?;
            let lines: Vec<String> = all.iter().map(serde_json::to_string).collect::<Result<_, _>>()?;
            std::fs::write(dir.join(format!("{key}.jsonl")), lines.join("\n") + "\n")?;
            println!("{} snapshots", all.len());
        }
        Some("show") => {
            for s in history::load(args.get(1).expect("key"))? {
                println!("{} lvl {} CR {:.4} DPS boss {:.3e} items {} tree {}", s.save_modified, s.level, s.combat_rating, s.dps_boss, s.items.len(), s.tree.len());
            }
        }
        _ => anyhow::bail!("usage: history capture | import <sav> <key> | show <key>"),
    }
    Ok(())
}
