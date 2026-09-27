//! Extracts the `soroban-sdk` version constraint from the contract manifest.
//!
//! Shared between `build.rs` (which bakes the constraint into the binary as
//! `TRELLIS_SOROBAN_SDK_COMPAT` for `trellis --version`) and the crate's unit
//! tests (via `#[path]` in `src/main.rs`). Kept dependency-free so the build
//! script needs no `[build-dependencies]`.
//!
//! COUPLING: the source of truth is the `soroban-sdk` entry under
//! `[dependencies]` in `contracts/trellis_core/Cargo.toml`. If that entry is
//! moved or renamed, the CLI build fails rather than reporting a stale range.

/// Returns the `soroban-sdk` version requirement declared under the
/// `[dependencies]` table of a Cargo manifest, or `None` if it is absent.
///
/// Supports both `soroban-sdk = "x"` and `soroban-sdk = { version = "x", .. }`.
/// Entries in other tables (e.g. `[dev-dependencies]`) are ignored.
pub fn soroban_sdk_requirement(manifest: &str) -> Option<String> {
    let mut in_dependencies = false;
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_dependencies = line == "[dependencies]";
            continue;
        }
        if !in_dependencies || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "soroban-sdk" {
            continue;
        }
        let value = value.trim();
        let quoted = if value.starts_with('{') {
            let (_, rest) = value.split_once("version")?;
            rest.trim_start().strip_prefix('=')?.trim_start()
        } else {
            value
        };
        let rest = quoted.strip_prefix('"')?;
        let (version, _) = rest.split_once('"')?;
        return Some(version.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_inline_table_version() {
        let manifest = r#"
[dependencies]
soroban-sdk = { version = ">=22.0.0, <23", features = ["alloc"] }
"#;
        assert_eq!(
            soroban_sdk_requirement(manifest).as_deref(),
            Some(">=22.0.0, <23")
        );
    }

    #[test]
    fn reads_plain_string_version() {
        let manifest = "[dependencies]\nsoroban-sdk = \"23.1.0\"\n";
        assert_eq!(soroban_sdk_requirement(manifest).as_deref(), Some("23.1.0"));
    }

    #[test]
    fn ignores_dev_dependencies_entry() {
        let manifest = r#"
[dev-dependencies]
soroban-sdk = { version = "99", features = ["testutils"] }

[dependencies]
soroban-sdk = { version = ">=22.0.0, <23", features = ["alloc"] }
"#;
        assert_eq!(
            soroban_sdk_requirement(manifest).as_deref(),
            Some(">=22.0.0, <23")
        );
    }

    #[test]
    fn missing_entry_returns_none() {
        let manifest = "[dependencies]\nserde = \"1\"\n\n[dev-dependencies]\nsoroban-sdk = \"22\"\n";
        assert_eq!(soroban_sdk_requirement(manifest), None);
    }

    #[test]
    fn ignores_commented_out_entry() {
        let manifest = "[dependencies]\n# soroban-sdk = \"21\"\nsoroban-sdk = \"22\"\n";
        assert_eq!(soroban_sdk_requirement(manifest).as_deref(), Some("22"));
    }

    /// The value baked in at build time must match the contract manifest on
    /// disk right now — guards against a stale build-script cache.
    #[test]
    fn baked_value_matches_contract_manifest() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../contracts/trellis_core/Cargo.toml"
        );
        let manifest = std::fs::read_to_string(path).expect("read contract manifest");
        assert_eq!(
            soroban_sdk_requirement(&manifest).as_deref(),
            Some(env!("TRELLIS_SOROBAN_SDK_COMPAT"))
        );
    }
}
