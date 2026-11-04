//! Lists a character's inventory and stash, and what the first equipment items would bring.
//! usage: stash <save.sav> [n items to evaluate]
use dr_core::planner::{BuildInput, Candidate};

fn main() -> anyhow::Result<()> {
    let root = std::env::var_os("DR_PLANNER_ROOT").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let path = std::path::PathBuf::from(std::env::args().nth(1).expect("save path"));
    let n: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(5);
    let gd = dr_core::gamedata::GameData::load(&root, "fr")?;
    let cat = dr_core::catalog::Catalog::build(&gd)?;
    let mut p = dr_core::planner::Planner::new(&gd, &cat)?;
    p.open(&path)?;
    let t = std::time::Instant::now();
    let tabs = p.inventory(&path)?;
    println!("read in {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
    let mut cands = Vec::new();
    for c in &tabs {
        println!("{} #{}: {}/{} used", c.kind, c.number, c.items.len(), c.size);
        for it in c.items.iter().take(3) {
            println!("  [{}] {} {} r{} t{} h{} slots {:?} weapon {} attrs {} pools {:?}", it.index, it.row, it.name, it.rarity, it.tier, it.hearts, it.slots, it.weapon, it.attributes.len(), it.pools.iter().map(|p| p.len()).collect::<Vec<_>>());
        }
        for it in &c.items {
            if it.weapon || !it.slots.is_empty() {
                cands.push(Candidate { row: it.row.clone(), attributes: it.attributes.clone(), rarity: it.rarity, tier: it.tier, hearts: it.hearts, slot: if it.weapon { None } else { it.slots.first().copied() } });
            }
        }
    }
    cands.truncate(n);
    let t = std::time::Instant::now();
    let res = p.evaluate_items(&BuildInput::default(), &cands)?;
    println!("evaluated {} items in {:.0} ms", cands.len(), t.elapsed().as_secs_f64() * 1000.0);
    for (c, i) in cands.iter().zip(&res) {
        println!("  {} slot {:?}: CR {:+.2}% phys {:+.2}% ele {:+.2}%", c.row, c.slot, i.combat_rating * 100.0, i.physical * 100.0, i.elemental * 100.0);
    }
    Ok(())
}
