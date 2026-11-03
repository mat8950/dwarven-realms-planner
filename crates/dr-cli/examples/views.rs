//! Prints the planner views (catalog summary, character, stats) for a save.
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let gd = dr_core::gamedata::GameData::load(&root, "fr")?;
    let cat = dr_core::catalog::Catalog::build(&gd)?;
    println!("catalog: {} attributes, {} panel rows, {} slots, {} affixes, {} pools, {} modifiers, stances {:?}", cat.attributes.len(), cat.panel.len(), cat.slots.len(), cat.affixes.len(), cat.affix_pools.len(), cat.modifiers.len(), cat.stances);
    let a = &cat.attributes["EasyRPG.Attributes.Characteristics.Luck"];
    println!("luck: {a:?}");
    let mut p = dr_core::planner::Planner::new(&gd, &cat)?;
    let path = std::env::args().nth(1).unwrap();
    let t = std::time::Instant::now();
    let ch = p.open(path.as_ref())?;
    println!("open {:?}: {} lvl {} {} stance {}", t.elapsed(), ch.name, ch.level, ch.race, ch.default_stance);
    for it in ch.items.iter().chain(ch.weapon.iter()) {
        println!("  {:?} {} [{}] r{} t{} icon={:?} {} attrs", it.slot, it.name, it.row, it.rarity, it.tier, it.icon, it.attributes.len());
    }
    let t = std::time::Instant::now();
    let s = p.compute(&Default::default())?;
    println!("compute {:?}: CR {} maxpot {}", t.elapsed(), s.combat_rating, s.max_potions);
    Ok(())
}
