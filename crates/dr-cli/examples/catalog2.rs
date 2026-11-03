fn main() -> anyhow::Result<()> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let gd = dr_core::gamedata::GameData::load(&root, "fr")?;
    let c = dr_core::catalog::Catalog::build(&gd)?;
    println!("{} weapons, {} abilities, {} attacks", c.weapons.len(), c.abilities.len(), c.attacks.len());
    let mut per = std::collections::BTreeMap::new();
    for w in &c.weapons { *per.entry(w.stance).or_insert(0) += 1; }
    println!("weapons per stance {per:?}");
    for w in c.weapons.iter().filter(|w| w.row.contains("GM6")).take(4) { println!("  {} {} st{} lvl{} {:?}", w.row, w.name, w.stance, w.item_level, w.attributes); }
    for a in c.abilities.iter().take(3) { println!("  {} {} {} {:?} {} {} {}", a.row, a.name, a.element, a.affinities, a.damage_tag, a.damage_base, a.bonus_tag); }
    for a in c.attacks.iter().filter(|a| a.stance == 3) { println!("  {} {} k{} x{} {}", a.row, a.name, a.kind, a.multiplier, a.damage_bonus_tag); }
    Ok(())
}
