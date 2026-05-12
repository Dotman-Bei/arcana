use std::{env, fs};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    // Create build/ so the arcis proc macro can write deal_cards.arcis there.
    fs::create_dir_all(format!("{}/build", manifest_dir)).ok();
    println!("cargo:rerun-if-changed=src/lib.rs");
}
