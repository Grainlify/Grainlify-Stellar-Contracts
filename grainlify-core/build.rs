// Build script to ensure the contract Wasm is compiled with reference-types enabled
use std::process::Command;
use std::env;

fn main() {
    // Ensure the wasm target is built before tests run
    let status = Command::new("cargo")
        .args(&["build", "--release", "--target", "wasm32-unknown-unknown"])
        .env("RUSTFLAGS", "-C target-feature=+reference-types")
        .status()
        .expect("Failed to execute cargo build for wasm");
    if !status.success() {
        panic!("Wasm build failed with status: {:?}", status);
    }
    // Re-run if any source files change
    println!("cargo:rerun-if-changed=src/");
}
