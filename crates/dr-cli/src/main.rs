//! dr-cli <character.sav> [--stance N] [--potions N] [--compare fixture.json] [--all]

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use dr_core::engine::{Engine, Situation};
use dr_core::gamedata::GameData;
use dr_core::save;
use dr_core::value::Value;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut save_path = None;
    let mut situation = Situation::default();
    let mut compare = None;
    let mut all = false;
    let mut dump = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--stance" => situation.stance = Some(args.get(i + 1).context("--stance N")?.parse()?),
            "--potions" => situation.available_potions = Some(args.get(i + 1).context("--potions N")?.parse()?),
            "--compare" => compare = Some(PathBuf::from(args.get(i + 1).context("--compare file")?)),
            "--dump" => dump = Some(PathBuf::from(args.get(i + 1).context("--dump file")?)),
            "--effects-from" => {
                // Reuse the buffs a DRLiveDump captured in game.
                let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(args.get(i + 1).context("--effects-from file")?)?)?;
                for e in j["components"][0]["EffectAttributes"].as_array().into_iter().flatten() {
                    situation.effect_attributes.push((e["tag"].as_str().unwrap_or("").to_string(), e["value"].as_f64().unwrap_or(0.0)));
                }
            }
            "--all" => {
                all = true;
                i += 1;
                continue;
            }
            p => {
                save_path = Some(PathBuf::from(p));
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    let Some(save_path) = save_path else { bail!("usage: dr-cli <character.sav> [--stance N] [--potions N] [--compare fixture.json] [--all]") };
    let root = std::env::var_os("DR_PLANNER_ROOT").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));

    let t = std::time::Instant::now();
    let gd = GameData::load(&root, "fr")?;
    let sav = save::load(&save_path)?;
    let Some(Value::Struct(player)) = sav.get("HostPlayerData").cloned() else { bail!("no HostPlayerData in save") };
    eprintln!("loaded data + save in {:?}", t.elapsed());

    let t = std::time::Instant::now();
    let mut eng = Engine::new(&gd)?;
    eng.load_player(&player)?;
    let stats = eng.compute(&situation)?;
    eprintln!("computed in {:?}", t.elapsed());
    let get = |tag: &str| if tag == "CombatRating" { Some(stats.combat_rating) } else { stats.attributes.get(tag).copied() };

    if let Some(path) = dump {
        // Same shape as the DRLiveDump mod output, for diffing against the game.
        let arr = |prop: &str| -> serde_json::Value {
            let v = eng.vm.get(eng.attr, prop);
            let items: Vec<Value> = match v {
                Value::Array(a) => a,
                Value::Map(m) => m.into_iter().map(|(_, v)| v).collect(),
                _ => Vec::new(),
            };
            items
                .iter()
                .filter_map(|it| match it {
                    Value::Struct(s) => {
                        let tag = s.fields.iter().find(|(k, _)| k.starts_with("GameplayTag"))?.1.name().to_string();
                        let val = s.fields.iter().find(|(k, _)| k.starts_with("Value"))?.1.f64();
                        Some(serde_json::json!({"tag": tag, "value": val}))
                    }
                    _ => None,
                })
                .collect()
        };
        let mut j = serde_json::json!({});
        for p in ["BaseAttributes", "PlayerAttributes", "EquipmentAttributes", "EffectAttributes", "SelectedItemAttributes", "AttributesNamed"] {
            j[p] = arr(p);
        }
        j["StanceDamageMultiplier"] = eng.vm.get(eng.attr, "Stance Damage Multiplier").f64().into();
        j["StanceCritMult"] = eng.vm.get(eng.attr, "StanceCritMult").f64().into();
        j["MaxPotions"] = eng.vm.get(eng.potions, "Max Potions").f64().into();
        std::fs::write(&path, serde_json::to_string_pretty(&j)?)?;
    }

    if let Some(fx) = compare {
        let f: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&fx)?)?;
        let (mut ok, mut total) = (0, 0);
        println!("{:<30} {:>16} {:>16} {:>9}", "stat", "game", "engine", "diff");
        for r in f["rows"].as_array().into_iter().flatten() {
            let scale = r["scale"].as_f64().unwrap_or(1.0);
            let game = r["game"].as_f64().unwrap_or(0.0);
            let ours = get(r["tag"].as_str().unwrap_or("")).unwrap_or(0.0) * scale;
            let diff = if game == 0.0 { ours.abs() } else { (ours - game) / game.abs() };
            let tol = 0.5 / 100f64.max(game.abs()); // panel rounds to 2 decimals
            let good = (ours - game).abs() <= 0.006 * scale.max(1.0) || diff.abs() < tol.max(1e-4);
            total += 1;
            if good {
                ok += 1;
            }
            println!("{:<30} {:>16.2} {:>16.2} {:>8.2}% {}", r["label"].as_str().unwrap_or(""), game, ours, diff * 100.0, if good { "ok" } else { "<<" });
        }
        println!("\n{ok}/{total} match");
    } else {
        println!("CombatRating = {:.2}", stats.combat_rating);
        for (k, v) in &stats.attributes {
            if all || *v != 0.0 {
                println!("{} = {v}", k.trim_start_matches("EasyRPG."));
            }
        }
    }
    println!("potions {} / {}", eng.vm.get(eng.potions, "Available Potions"), eng.vm.get(eng.potions, "Max Potions"));
    for m in &stats.missing {
        eprintln!("missing: {m}");
    }
    Ok(())
}
