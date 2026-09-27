//! Build script: derives the soroban-sdk compatibility range shown by
//! `trellis --version` from the contract manifest, so the two cannot drift.
//! See `sdk_compat.rs` for the parser and the coupling note.

#[path = "sdk_compat.rs"]
mod sdk_compat;

const CONTRACT_MANIFEST: &str = "../../contracts/trellis_core/Cargo.toml";

fn main() {
    println!("cargo:rerun-if-changed={CONTRACT_MANIFEST}");
    println!("cargo:rerun-if-changed=sdk_compat.rs");

    let manifest = std::fs::read_to_string(CONTRACT_MANIFEST)
        .unwrap_or_else(|e| panic!("failed to read {CONTRACT_MANIFEST}: {e}"));
    let requirement = sdk_compat::soroban_sdk_requirement(&manifest).unwrap_or_else(|| {
        panic!(
            "no `soroban-sdk` entry under [dependencies] in {CONTRACT_MANIFEST}; \
             update cli/trellis_cli/sdk_compat.rs if the manifest layout changed"
        )
    });

    println!("cargo:rustc-env=TRELLIS_SOROBAN_SDK_COMPAT={requirement}");
}
