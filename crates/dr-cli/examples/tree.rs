//! Skill tree check: catalog summary, then the stats without the save's tree nodes.
fn main() -> anyhow::Result<()> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let gd = dr_core::gamedata::GameData::load(&root, "fr")?;
    let cat = dr_core::catalog::Catalog::build(&gd)?;
    let t = &cat.skill_tree;
    println!("tree: {} nodes, {} options, root {:?}", t.nodes.len(), t.nodes.iter().map(|n| n.options.len()).sum::<usize>(), t.nodes.iter().find(|n| n.root).map(|n| &n.id));
    for n in t.nodes.iter().filter(|n| n.options.len() > 1).take(2) {
        println!("  {} {:?}", n.id, n.options.iter().map(|o| format!("{} {:?}", o.name, o.attributes)).collect::<Vec<_>>());
    }
    let mut p = dr_core::planner::Planner::new(&gd, &cat)?;
    let ch = p.open(std::env::args().nth(1).expect("save").as_ref())?;
    println!("save: {} tree rows, {} unspent points", ch.tree.len(), ch.skill_points);
    let cr = |b: &dr_core::planner::BuildInput, p: &mut dr_core::planner::Planner| -> anyhow::Result<(f64, f64)> {
        let s = p.compute(b)?;
        Ok((s.combat_rating, s.elapsed_ms))
    };
    let base = cr(&Default::default(), &mut p)?;
    let same = cr(&dr_core::planner::BuildInput { tree: Some(ch.tree.clone()), ..Default::default() }, &mut p)?;
    let empty = cr(&dr_core::planner::BuildInput { tree: Some(Vec::new()), ..Default::default() }, &mut p)?;
    let mut fewer = ch.tree.clone();
    fewer.truncate(200);
    let again = cr(&dr_core::planner::BuildInput { tree: Some(fewer), ..Default::default() }, &mut p)?;
    println!("200 nodes {:.0} ({:.0} ms, cached)", again.0, again.1);
    // Other skills: the save's own levels must give the save's stats; then one card +10 levels.
    let same_levels: Vec<_> = ch.skills.clone();
    let s1 = cr(&dr_core::planner::BuildInput { skill_levels: same_levels, ..Default::default() }, &mut p)?;
    let card = ch.skills.iter().find(|s| s.table == "DT_Crystal_Cards_Skills").cloned().expect("a card");
    let up = dr_core::planner::SkillLevel { level: card.level + 10, ..card.clone() };
    let s2 = cr(&dr_core::planner::BuildInput { skill_levels: vec![up], ..Default::default() }, &mut p)?;
    println!("skills as saved {:.0} ({:.0} ms) | {} {} +10 levels {:.0} ({:.0} ms)", s1.0, s1.1, card.table, card.row, s2.0, s2.1);
    println!("CR save {:.0} ({:.0} ms) | same tree {:.0} ({:.0} ms) | empty tree {:.0}", base.0, base.1, same.0, same.1, empty.0);
    Ok(())
}
