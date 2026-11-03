//! Lists translated attribute texts whose numbers differ from the source text.
fn main() -> anyhow::Result<()> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let gd = dr_core::gamedata::GameData::load(&root, "fr")?;
    let mut n = 0;
    for ((t, k), src) in &gd.source_strings {
        if t != "ST_Attributes" { continue; }
        let Some(tr) = gd.text(t, k) else { continue };
        let fixed = gd.text_checked(t, k).unwrap_or_default();
        if fixed != tr {
            n += 1;
            let filter = std::env::args().nth(1).unwrap_or_default();
            if (filter.is_empty() && n <= 12) || filter == "all" || (!filter.is_empty() && k.contains(&filter)) { println!("{k}\n   FR  {tr}\n   EN  {src}\n   =>  {fixed}"); }
        }
    }
    println!("{n} textes corrigés");
    Ok(())
}
