//! Runs the game data extraction the setup screen runs, without the UI.
//! usage: cargo run -p dr-app --example extract   (dr-extract next to the example binary, or DR_EXTRACT)
fn main() {
    let r = dr_app_lib::gamesetup::run_extraction(|v| println!("{v}"));
    match r {
        Ok(root) => println!("data in {}", root.display()),
        Err(e) => {
            eprintln!("extraction failed: {e}");
            std::process::exit(1);
        }
    }
}
