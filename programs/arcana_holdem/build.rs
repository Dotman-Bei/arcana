use std::{env, fs, path::Path};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let build_dir = Path::new(&manifest_dir).join("build");
    fs::create_dir_all(&build_dir).ok();

    // Primary: arcium_compute writes here after `cargo build -p arcium_compute`
    let src_conventional = workspace_root.join("arcium_compute/build/deal_cards.arcis");
    // Fallback: some arcis versions write to the workspace root
    let src_root = workspace_root.join("deal_cards.arcis");

    let dst = build_dir.join("deal_cards.arcis");

    if src_conventional.exists() {
        fs::copy(&src_conventional, &dst).ok();
        println!("cargo:rerun-if-changed={}", src_conventional.display());
    } else if src_root.exists() {
        fs::copy(&src_root, &dst).ok();
        println!("cargo:rerun-if-changed={}", src_root.display());
    }

    if !dst.exists() {
        eprintln!(
            "\n\
            ╔═══════════════════════════════════════════════════════════╗\n\
            ║  Missing build/deal_cards.arcis                           ║\n\
            ║  Run first:  cargo build -p arcium_compute                ║\n\
            ║  Then find:  find . -name '*.arcis'                       ║\n\
            ║  Then copy:  cp <path> programs/arcana_holdem/build/      ║\n\
            ╚═══════════════════════════════════════════════════════════╝\n"
        );
    }

    println!("cargo:rerun-if-changed=build.rs");
}
