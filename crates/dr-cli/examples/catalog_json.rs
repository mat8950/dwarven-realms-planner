//! Prints the UI catalog as JSON. usage: cargo run --release -p dr-cli --example catalog_json [lang] > catalog.json
fn main() -> anyhow::Result<()> {
    let root = std::env::var_os("DR_PLANNER_ROOT").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let lang = std::env::args().nth(1).unwrap_or_else(|| "fr".into());
    let gd = dr_core::gamedata::GameData::load(&root, &lang)?;
    let cat = dr_core::catalog::Catalog::build(&gd)?;
    println!("{}", serde_json::to_string(&cat)?);
    Ok(())
}
