fn main() -> anyhow::Result<()> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let gd = dr_core::gamedata::GameData::load(&root, "fr")?;
    let cat = dr_core::catalog::Catalog::build(&gd)?;
    let mut p = dr_core::planner::Planner::new(&gd, &cat)?;
    p.open(std::env::args().nth(1).unwrap().as_ref())?;
    let s = p.compute(&Default::default())?;
    println!("CR {:.0}", s.combat_rating);
    for (a, v) in cat.abilities.iter().zip(&s.abilities).filter(|(_, v)| **v > 0.0).take(30) { println!("  ability {:<22} {:>16.0}", a.name, v); }
    for (a, v) in cat.attacks.iter().zip(&s.attacks).filter(|(a, _)| a.stance == s.stance) { println!("  attack {:<22} {:>16.0}", a.name, v); }
    let mut seen = std::collections::HashSet::new();
    let probes: Vec<(String, f64)> = cat.affixes.iter().filter(|a| a.can_roll && seen.insert(a.tag.clone())).map(|a| (a.tag.clone(), a.value + a.per_level * 80.0)).collect();
    let t = std::time::Instant::now();
    let imp = p.analyze(&Default::default(), &probes.iter().map(|(t, v)| dr_core::planner::Probe::Plain(t.clone(), *v)).collect::<Vec<_>>())?;
    println!("{} probes in {:?}", imp.len(), t.elapsed());
    let lp = cat.abilities.iter().position(|a| a.row == "LightningPlasma").unwrap();
    let mut by_cr: Vec<_> = imp.iter().collect();
    by_cr.sort_by(|a, b| b.combat_rating.partial_cmp(&a.combat_rating).unwrap());
    println!("top CR:"); for i in by_cr.iter().take(12) { println!("  {:<60} +{:.3}%  LP {:+.3}%", i.tag, i.combat_rating * 100.0, i.abilities[lp] * 100.0); }
    by_cr.sort_by(|a, b| b.ability_dps[lp].partial_cmp(&a.ability_dps[lp]).unwrap());
    println!("top Lightning Plasma DPS:"); for i in by_cr.iter().take(14) { println!("  {:<60} +{:.3}%  CR {:+.3}%", i.tag, i.ability_dps[lp] * 100.0, i.combat_rating * 100.0); }
    println!("zero-impact on CR: {}", imp.iter().filter(|i| i.combat_rating.abs() < 1e-9).count());
    Ok(())
}
