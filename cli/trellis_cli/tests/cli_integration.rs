// CLI integration tests using a mock stellar binary.
//
// These tests verify argument parsing, error handling, output formatting,
// and JSON serialization without requiring a live Soroban network.
//
// Run with:
//   cargo test --test cli_integration

use std::process::Command;
use std::env;

// -----------------------------------------------------------------------------
// Test helpers
// -----------------------------------------------------------------------------

/// Path to the mock stellar binary script
fn mock_stellar_path() -> String {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/tests/mock_stellar.sh", manifest_dir)
}

/// Build a trellis CLI invocation with the mock stellar binary
fn trellis_cmd() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet", "--"])
        .env("TRELLIS_TEST_MODE", "true")
        .env("STELLAR_MOCK_BIN", mock_stellar_path())
        .env("TRELLIS_CONTRACT_ID", "CBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .env("TRELLIS_SOURCE_KEY", "SBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    cmd
}

// -----------------------------------------------------------------------------
// Argument parsing tests
// -----------------------------------------------------------------------------

#[test]
fn test_init_parses_all_required_args() {
    let output = trellis_cmd()
        .args([
            "init",
            "--agreement-id", "000000000000000000000000000000000000000000000000000000000000000001",
            "--payer", "GBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUW",
            "--payee", "GZYWWVUXSTRQPONMLKJIHGFEDCBA234567ZYWWVUXSTRQPONMLKJIHGF",
            "--token", "CBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "--resolver", "GRESOLVABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNO",
            "--amounts", "1000,2000,3000",
            "--dry-run"
        ])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "init with all required args should succeed (dry-run)\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_lock_funds_parses_required_args() {
    let output = trellis_cmd()
        .args([
            "lock",
            "--agreement-id", "000000000000000000000000000000000000000000000000000000000000000001",
            "--milestone-id", "0",
            "--dry-run"
        ])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "lock with required args should succeed (dry-run)\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_status_requires_agreement_id() {
    let output = trellis_cmd()
        .args(["status"])
        .output()
        .expect("failed to execute trellis");

    assert!(
        !output.status.success(),
        "status without --agreement-id should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("agreement-id") || stderr.contains("required"),
        "error message should mention missing agreement-id"
    );
}

// -----------------------------------------------------------------------------
// Output format tests
// -----------------------------------------------------------------------------

#[test]
fn test_json_output_format() {
    let output = trellis_cmd()
        .args([
            "status",
            "--agreement-id", "000000000000000000000000000000000000000000000000000000000000000001",
            "--json"
        ])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "status --json should succeed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('{') && stdout.contains('}'),
        "JSON output should contain braces"
    );

    // Validate it's parseable JSON
    let result: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        result.is_ok(),
        "JSON output should be valid JSON\nstdout: {}",
        stdout
    );
}

#[test]
fn test_quiet_mode_suppresses_non_result_output() {
    let output = trellis_cmd()
        .args([
            "status",
            "--agreement-id", "000000000000000000000000000000000000000000000000000000000000000001",
            "--quiet"
        ])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "status --quiet should succeed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // In quiet mode, we should only get the JSON result, no other messages
    assert!(
        !stdout.contains("Invoking") && !stdout.contains("Success"),
        "quiet mode should suppress non-result messages"
    );
}

// -----------------------------------------------------------------------------
// Error path tests
// -----------------------------------------------------------------------------

#[test]
fn test_missing_stellar_binary_error() {
    // Temporarily unset the mock binary to simulate stellar not being in PATH
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "status", "--agreement-id", "0001"])
        .env_remove("STELLAR_MOCK_BIN")
        .env_remove("PATH")  // Remove PATH to ensure stellar is not found
        .env("TRELLIS_CONTRACT_ID", "CBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .output()
        .expect("failed to execute trellis");

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // The error should mention stellar CLI not being found
    assert!(
        stderr.contains("stellar") || stderr.contains("not found") || stderr.contains("install"),
        "error should mention stellar CLI\nstderr: {}",
        stderr
    );
}

#[test]
fn test_invalid_hex_agreement_id() {
    let output = trellis_cmd()
        .args([
            "status",
            "--agreement-id", "not-valid-hex",
            "--dry-run"
        ])
        .output()
        .expect("failed to execute trellis");

    // Should fail with helpful error about hex format
    assert!(
        !output.status.success(),
        "invalid hex agreement-id should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("hex") || stderr.contains("invalid") || stderr.contains("format"),
        "error should mention invalid hex format\nstderr: {}",
        stderr
    );
}

#[test]
fn test_missing_required_env_vars() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "status", "--agreement-id", "0001"])
        .env_remove("TRELLIS_CONTRACT_ID")
        .env_remove("TRELLIS_SOURCE_KEY")
        .output()
        .expect("failed to execute trellis");

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    assert!(
        stderr.contains("TRELLIS_CONTRACT_ID") || stderr.contains("environment"),
        "error should mention missing environment variable\nstderr: {}",
        stderr
    );
}

// -----------------------------------------------------------------------------
// #406: --dry-run must not require the stellar binary
// -----------------------------------------------------------------------------

/// Confirms that `--dry-run` prints a command preview and exits 0 even when
/// the `stellar` binary is completely absent from PATH.
///
/// This is the core regression test for issue #406: `validate_environment()`
/// must be skipped for dry-run invocations.
#[test]
fn test_dry_run_works_without_stellar_binary() {
    let output = Command::new("cargo")
        .args([
            "run", "--quiet", "--",
            "lock",
            "--agreement-id", "000000000000000000000000000000000000000000000000000000000000000001",
            "--milestone-id", "0",
            "--dry-run",
        ])
        // Wipe PATH so the stellar binary genuinely cannot be found.
        .env("PATH", "")
        .env("TRELLIS_CONTRACT_ID", "CBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .env("TRELLIS_SOURCE_KEY", "SBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .current_dir()
        .output()
        .expect("failed to spawn trellis process");

    assert!(
        output.status.success(),
        "--dry-run should succeed even when stellar is not in PATH\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("stellar") || stdout.contains("contract") || stdout.contains("invoke"),
        "--dry-run output should contain a stellar command preview\nstdout: {}",
        stdout
    );
}

/// Adjacent regression test: without `--dry-run`, the binary check must still
/// fire and produce a clear error message when stellar is absent from PATH.
///
/// This guards against accidentally removing the check for non-dry-run paths
/// while fixing #406.
#[test]
fn test_non_dry_run_still_requires_stellar_binary() {
    let output = Command::new("cargo")
        .args([
            "run", "--quiet", "--",
            "status",
            "--agreement-id", "000000000000000000000000000000000000000000000000000000000000000001",
        ])
        // Wipe PATH so the stellar binary cannot be found.
        .env("PATH", "")
        .env("TRELLIS_CONTRACT_ID", "CBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .env("TRELLIS_SOURCE_KEY", "SBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .current_dir()
        .output()
        .expect("failed to spawn trellis process");

    assert!(
        !output.status.success(),
        "non-dry-run should fail when stellar is not in PATH"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("stellar") || stderr.contains("not found") || stderr.contains("install"),
        "error message should mention the missing stellar binary\nstderr: {}",
        stderr
    );
}

// -----------------------------------------------------------------------------
// Command-specific tests
// -----------------------------------------------------------------------------

#[test]
fn test_init_with_multiple_milestones() {
    let output = trellis_cmd()
        .args([
            "init",
            "--agreement-id", "000000000000000000000000000000000000000000000000000000000000000002",
            "--payer", "GBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUW",
            "--payee", "GZYXWWVUXSTRQPONMLKJIHGFEDCBA234567ZYWWVUXSTRQPONMLKJIHGF",
            "--token", "CBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "--resolver", "GRESOLVABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNO",
            "--amounts", "1000,2000,3000,4000,5000",
            "--json"
        ])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "init with multiple milestones should succeed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_submit_work_with_proof_uri() {
    let output = trellis_cmd()
        .args([
            "submit",
            "--agreement-id", "000000000000000000000000000000000000000000000000000000000000000001",
            "--milestone-id", "0",
            "--proof-uri", "ipfs://QmTest123",
            "--dry-run"
        ])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "submit with proof-uri should succeed (dry-run)\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_raise_dispute_requires_caller() {
    let output = trellis_cmd()
        .args([
            "raise-dispute",
            "--agreement-id", "000000000000000000000000000000000000000000000000000000000000000001",
            "--milestone-id", "0",
            "--reason", "Work not delivered",
            "--dry-run"
        ])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "raise-dispute with required args should succeed (dry-run)\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// -----------------------------------------------------------------------------
// Native XDR result decoding for read-only contract queries
// -----------------------------------------------------------------------------
//
// The mock stellar binary emits a captured real XDR `ScVal` result for
// `get_agreement` and `get_milestone`. The CLI must decode that XDR
// natively into its existing JSON shape rather than re-printing the
// `stellar` CLI's own decoded stdout.

/// The exact agreement ID the mock returns a captured XDRR result for.
const FIXTURE_AGREEMENT_ID : &str =
    "000000000000000000000000000000000000000000000000000000000000000001";

/// The amounts the fixture agreement was initialized with.
const FIXTURE_AMOUNTS: [&str; 3] = ["1000", "2000", "3000"];

/// The addresses the fixture agreement references.
const FIXTURE_PAYER: &str =
    "GBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUW";
const FIXTURE_PAYEE: &str =
    "GZYWWVUXSTRQPONMLKJIHGFEDCBA234567ZYWWVUXSTRQPONMLKJIHGF";
const FIXTURE_TOKEN: &str =
    "CBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const FIXTURE_RESOLVER: &str =
    "GRESOLVABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNO";

/// Run `status --json` against the fixture agreement and parse the JSON.
fn run_status_json(agreement_id: &str) -> serde_json::Value {
    let output = trellis_cmd()
        .args(["status", "--agreement-id", agreement_id, "--json"])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "status --json should succeed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).unwrap_or_else|| {
        panic!("status --json must emit valid JSON, got: {stdout}")
    }
}

/// The specific case this issue describes: `get_agreement` returns a
/// raw XDR `ScVal` which must be decoded natively into the CLI's JSON
/// shape.
#[test]
fn test_status_decodes_get_agreement_xdr_result() {
    let json = run_status_json(FIXTURE_AGREEMENT_ID);

    // Top-level shape must match the existing render_json contract.
    assert_eq!(
        json.get("agreement_id").and_then(|v| v.as_str()),
        Some(FIXTURE_AGREEMENT_ID),
        "decoded agreement_id must match the requested agreement"
    );
    assert_eq(
        json.get("payer").and_then(|v| v.as_str()),
        Some(FIXTURE_PAYER),
        "decoded payer must match the on-chain agreement"
    );
    assert_eq!(
        json.get("payee").and_then(|v| v.as_str()),
        Some(FIXTURE_PAYEE),
        "decoded payee must match the on-chain agreement"
    );
    assert_eq!(
        json.get("token").and_then(|v| v.as_str()),
        Some(FIXTURE_TOKEN),
        "decoded token must match the on-chain agreement"
    );
    assert_eq!(
        json.get("resolver").and_then(|v| v.as_str()),
        Some(FIXTURE_RESOLVER),
        "decoded resolver must match the on-chain agreement"
    );

    // Milestones array must be decoded from the XDR vector.
    let milestones = json
        .get("milestones")
        .and_then(v| v.as_array())
        .expect("decoded agreement must expose a milestones array");
    assert_eq!(
        milestones.len(),
        FIXTURE_AMOUNTS.len(),
        "decoded milestone count must match the on-chain array"
    );

    for (i, expected_amount) in FIXTURE_AMOUNTS.iter().enumerate() {
        let milestone = &milestones[i];
        assert_eq!(
            milestone.get("id").and_then(|v| v.as_u64()),
            Some(i as u64),
            "milestone {i} id must match its position in the decoded vector"
        );
        assert_eq!(
            milestone.get("amount").and_then(|v| v.as_str()),
            Some(*expected_amount),
            "milestone {i} amount must match the on-chain value"
        );
    }
}

/// Adjacent case: `get_milestone` returns a single XDR `ScVal` that must
/// be decoded into the same milestone shape used by the agreement result.
/// This guards against a fix that only handles the agreement vector.
#[test]
fn test_milestone_status_decodes_get_milestone_xdr_result() {
    let output = trellis_cmd()
        .args([
            "milestone-status",
            "--agreement-id", FIXTURE_AGREEMENT_ID,
            "--milestone-id", "1",
            "--json",
        ])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "milestone-status --json should succeed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = serde_json::from_str(&stdout).unwrap_or_else|| {
        panic!("milestone-status --json must emit valid JSON, got: {stdout}")
    });

    assert_eq!(
        json.get("agreement_id").and_then(|v| v.as_str()),
        Some(FIXTURE_AGREEMENT_ID),
        "decoded milestone result must carry the agreement id"
    );
    assert_eq!(
        json.get("id").and_then(|v| v.as_u64()),
        Some(1),
        "decoded milestone id must match the requested milestone"
    );
    assert_eq!(
        json.get("amount").and_then(|v| v.as_str()),
        Some(FIXTURE_AMOUNTS[1]),
        "decoded milestone amount must match the on-chain value"
    );
}
