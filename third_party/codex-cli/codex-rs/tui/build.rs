use std::fs;
use std::path::Path;
use std::path::PathBuf;

fn main() {
    let build_number_path = build_number_path();
    println!("cargo:rerun-if-changed={}", build_number_path.display());

    let build_number = read_build_number(&build_number_path);
    println!("cargo:rustc-env=WHALE_BUILD_NUMBER={build_number}");
}

fn build_number_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("BUILD_NUMBER")
}

fn read_build_number(path: &Path) -> String {
    let value = fs::read_to_string(path).unwrap_or_else(|err| {
        panic!(
            "failed to read Whale build number from {}: {err}",
            path.display()
        )
    });
    let trimmed = value.trim();
    assert!(
        !trimmed.is_empty() && trimmed.chars().all(|ch| ch.is_ascii_digit()) && trimmed != "0",
        "Whale BUILD_NUMBER must be a positive integer"
    );
    trimmed.to_string()
}
