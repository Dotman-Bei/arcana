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

    let dst = build_dir.join("deal_cards.arcis");

    // All locations where `arcium build` might write the circuit binary.
    let candidates = [
        workspace_root.join("build/deal_cards.arcis"),               // arcium build default
        workspace_root.join("arcium_compute/deal_cards.arcis"),      // arcium_compute root
        workspace_root.join("arcium_compute/build/deal_cards.arcis"),// arcium_compute/build
        workspace_root.join("encrypted-ixs/deal_cards.arcis"),       // arcium CLI convention root
        workspace_root.join("encrypted-ixs/build/deal_cards.arcis"), // arcium CLI convention build
        workspace_root.join("deal_cards.arcis"),                     // workspace root fallback
    ];

    for src in &candidates {
        if src.exists() {
            fs::copy(src, &dst).ok();
            println!("cargo:rerun-if-changed={}", src.display());
            break;
        }
    }

    if !dst.exists() {
        eprintln!(
            "\n\
            ╔═══════════════════════════════════════════════════════════════╗\n\
            ║  Missing programs/arcana_holdem/build/deal_cards.arcis        ║\n\
            ║                                                               ║\n\
            ║  Install the Arcium CLI and compile the circuit:              ║\n\
            ║    curl -sSfL https://install.arcium.com/ | bash              ║\n\
            ║    source ~/.bashrc                                           ║\n\
            ║    ln -sf arcium_compute encrypted-ixs  # CLI name convention ║\n\
            ║    arcium build                                               ║\n\
            ║    find . -name '*.arcis'               # locate the file     ║\n\
            ║    cp <path> programs/arcana_holdem/build/                    ║\n\
            ╚═══════════════════════════════════════════════════════════════╝\n"
        );
    }

    println!("cargo:rerun-if-changed=build.rs");
}
