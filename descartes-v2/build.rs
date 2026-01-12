//! Build script that generates BAML client code at compile time.
//!
//! This invokes the BAML CLI to generate Rust code from .baml files,
//! eliminating the need to commit generated code to git.

use std::env;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let baml_src = Path::new(&manifest_dir).join("baml_src");
    let baml_client_dir = Path::new(&manifest_dir).join("baml_client");

    // Rerun if any .baml files change
    println!("cargo:rerun-if-changed=baml_src/");

    // Check if we need to regenerate
    let needs_regen = should_regenerate(&baml_src, &baml_client_dir);

    if needs_regen {
        eprintln!("Generating BAML client code...");

        // Try using npx first (most reliable)
        let status = Command::new("npx")
            .args([
                "@boundaryml/baml",
                "generate",
                "--from",
                baml_src.to_str().unwrap(),
            ])
            .current_dir(&manifest_dir)
            .status();

        match status {
            Ok(s) if s.success() => {
                eprintln!("BAML client generated successfully");
            }
            Ok(s) => {
                panic!("BAML generate failed with status: {}", s);
            }
            Err(e) => {
                // npx not available, try baml-cli directly
                let fallback = Command::new("baml")
                    .args([
                        "generate",
                        "--from",
                        baml_src.to_str().unwrap(),
                    ])
                    .current_dir(&manifest_dir)
                    .status();

                match fallback {
                    Ok(s) if s.success() => {
                        eprintln!("BAML client generated successfully (via baml cli)");
                    }
                    _ => {
                        panic!(
                            "Failed to run BAML generator. \
                            Install with: npm install -g @boundaryml/baml\n\
                            Original error: {}",
                            e
                        );
                    }
                }
            }
        }
    }
}

/// Check if baml_client needs regeneration by comparing timestamps
fn should_regenerate(baml_src: &Path, baml_client: &Path) -> bool {
    // If baml_client doesn't exist, definitely regenerate
    if !baml_client.exists() {
        return true;
    }

    // Get the newest .baml file timestamp
    let newest_baml = WalkDir::new(baml_src)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.extension().map(|e| e == "baml").unwrap_or(false))
        .filter_map(|p| p.metadata().ok()?.modified().ok())
        .max();

    // Get the oldest generated file timestamp
    let oldest_generated = WalkDir::new(baml_client)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
        .filter_map(|p| p.metadata().ok()?.modified().ok())
        .min();

    match (newest_baml, oldest_generated) {
        (Some(baml_time), Some(gen_time)) => baml_time > gen_time,
        (Some(_), None) => true, // No generated files
        (None, _) => false,      // No baml files (shouldn't happen)
    }
}
