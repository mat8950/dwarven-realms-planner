//! Many recomputes on one planner: must not hit the VM step limit.
fn main() -> anyhow::Result<()> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let gd = dr_core::gamedata::GameData::load(&root, "fr")?;
    let cat = dr_core::catalog::Catalog::build(&gd)?;
    let mut p = dr_core::planner::Planner::new(&gd, &cat)?;
    p.open(std::env::args().nth(1).unwrap().as_ref())?;
    let probes: Vec<(String, f64)> = cat.affixes.iter().filter(|a| a.can_roll).take(60).map(|a| (a.tag.clone(), a.value + a.per_level * 80.0)).collect();
    for i in 0..40 {
        let s = p.compute(&Default::default())?;
        if i % 10 == 0 { p.analyze(&Default::default(), &probes.iter().map(|(t, v)| dr_core::planner::Probe::Plain(t.clone(), *v)).collect::<Vec<_>>())?; println!("round {i}: CR {:.0}", s.combat_rating); }
    }
    println!("ok");
    Ok(())
}
