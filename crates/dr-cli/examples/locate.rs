//! Prints the game installations, save folder and extracted data the planner finds.
fn main() {
    use dr_core::setup;
    for g in setup::find_installs() {
        println!("install: {:?}", g);
    }
    let s = setup::Settings::load();
    println!("settings: {s:?}");
    println!("save dir: {:?}", s.save_dir());
    println!("data dir: {}", setup::data_dir().display());
    println!("extracted: {:?}", setup::extracted_roots());
    println!("mappings: {:?}", setup::latest_mappings());
}
