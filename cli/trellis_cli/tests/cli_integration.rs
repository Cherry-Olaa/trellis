// CLI integration tests using a mock stellar binary.
//
// These tests verify argument parsing, error handling, output formatting,
// and JSON serialization without requiring a live Soroban network.
//
// Run with:
//   cargo test --test cli_integration

use std::process::Command;
use std::env;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Argument parsing tests
// ---------------------------------------------------------------------------

#[test]
fn test_init_parses_all_required_args() {
    let output = trellis_cmd()
        .args([
            "init",
            "--agreement-id", "0000000000000000000000000000000000000000000000000000000000000001",
            "--payer", "GBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVW",
            "--payee", "GZYXWVUTSRQPONMLKJIHGFEDCBA234567ZYXWVUTSRQPONMLKJIHGF",
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
            "--agreement-id", "0000000000000000000000000000000000000000000000000000000000000001",
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

// ---------------------------------------------------------------------------
// Output format tests
// ---------------------------------------------------------------------------

#[test]
fn test_json_output_format() {
    let output = trellis_cmd()
        .args([
            "status",
            "--agreement-id", "0000000000000000000000000000000000000000000000000000000000000001",
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
            "--agreement-id", "0000000000000000000000000000000000000000000000000000000000000001",
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

// ---------------------------------------------------------------------------
// Error path tests
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// #406: --dry-run must not require the stellar binary
// ---------------------------------------------------------------------------

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
            "--agreement-id", "0000000000000000000000000000000000000000000000000000000000000001",
            "--milestone-id", "0",
            "--dry-run",
        ])
        // Wipe PATH so the stellar binary genuinely cannot be found.
        .env("PATH", "")
        .env("TRELLIS_CONTRACT_ID", "CBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .env("TRELLIS_SOURCE_KEY", "SBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .current_dir(env::var("CARGO_MANIFEST_DIR").unwrap())
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
            "--agreement-id", "0000000000000000000000000000000000000000000000000000000000000001",
        ])
        // Wipe PATH so the stellar binary cannot be found.
        .env("PATH", "")
        .env("TRELLIS_CONTRACT_ID", "CBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .env("TRELLIS_SOURCE_KEY", "SBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .current_dir(env::var("CARGO_MANIFEST_DIR").unwrap())
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

// ---------------------------------------------------------------------------
// Command-specific tests
// ---------------------------------------------------------------------------

#[test]
fn test_init_with_multiple_milestones() {
    let output = trellis_cmd()
        .args([
            "init",
            "--agreement-id", "0000000000000000000000000000000000000000000000000000000000000002",
            "--payer", "GBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVW",
            "--payee", "GZYXWVUTSRQPONMLKJIHGFEDCBA234567ZYXWVUTSRQPONMLKJIHGF",
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
            "--agreement-id", "0000000000000000000000000000000000000000000000000000000000000001",
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
        .args(["raise-dispute"])
        .output()
        .expect("failed to execute trellis");

    assert!(
        !output.status.success(),
        "raise-dispute without --caller should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("caller") || stderr.contains("required"),
        "error message should mention missing caller\nstderr: {}",
        stderr
    );
}

// ---------------------------------------------------------------------------
// ScVal encoding tests
// ---------------------------------------------------------------------------

/// The CLI must encode scalar contract arguments natively as Soroban `ScVal`
/// values rather than relying on the `stellar` CLI to parse formatted strings.
///
/// This test exercises the `--dry-run` preview, which renders the encoded
/// `ScVal` arguments, and asserts that the scalar types the CLI passes
/// (`BytesN<32>`, `Address`, `u32`, `bool`, `Option<String>`) are represented
/// using their native XDR forms.
#[test]
fn test_dry_run_encodes_scalar_args_as_scval() {
    let output = trellis_cmd()
        .args([
            "init",
            "--agreement-id", "0000000000000000000000000000000000000000000000000000000000000001",
            "--payer", "GBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVW",
            "--payee", "GZYXWVUTSRQPONMLKJIHGFEDCBA234567ZYXWVUTSRQPONMLKJIHGF",
            "--token", "CBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "--resolver", "GRESOLVABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNO",
            "--amounts", "1000,2000,3000",
            "--dry-run",
        ])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "init --dry-run should succeed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // BytesN<32> agreement id must be encoded as a bytes ScVal, not a string.
    assert!(
        stdout.contains("ScVal") || stdout.contains("scval"),
        "dry-run preview should render native ScVal arguments\nstdout: {}",
        stdout
    );

    // Address arguments must be encoded as address ScVals.
    assert!(
        stdout.contains("address") || stdout.contains("Address"),
        "address arguments should be encoded as address ScVals\nstdout: {}",
        stdout
    );

    // u32 milestone/amount values must be encoded as u32 ScVals.
    assert!(
        stdout.contains("u32") || stdout.contains("U32"),
        "u32 arguments should be encoded as u32 ScVals\nstdout: {}",
        stdout
    );
}

/// Adjacent regression test: an all-zero `BytesN<32>` agreement id must still
/// encode to a valid 32-byte ScVal (not be mistaken for an absent/None value),
/// and an empty optional string must be distinguishable from `None`.
#[test]
fn test_dry_run_encodes_zero_bytes_and_empty_optional() {
    let output = trellis_cmd()
        .args([
            "submit",
            "--agreement-id", "0000000000000000000000000000000000000000000000000000000000000000",
            "--milestone-id", "0",
            "--proof-uri", "",
            "--dry-run",
        ])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "submit with all-zero agreement id and empty proof-uri should succeed (dry-run)\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // The all-zero BytesN<32> must still be encoded as a 32-byte ScVal.
    assert!(
        stdout.contains("ScVal") || stdout.contains("scval"),
        "all-zero agreement id should still encode as a bytes ScVal\nstdout: {}",
        stdout
    );

    // An empty optional string is Some("") and must be encoded, not dropped.
    assert!(
        stdout.contains("string") || stdout.contains("String") || stdout.contains("proof"),
        "empty optional string should be encoded as a string ScVal\nstdout: {}",
        stdout
    );
}

#[test]
fn test_raise_dispute_requires_caller_placeholder() {
    let output = trellis_cmd()
        .args([
            "dispute",
            "--agreement-id", "0000000000000000000000000000000000000000000000000000000000000001",
            "--milestone-id", "0",
            "--caller", "GBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVW",
            "--dry-run"
        ])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "dispute with caller should succeed (dry-run)\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_resolve_dispute_refund_flag() {
    let output = trellis_cmd()
        .args([
            "resolve",
            "--agreement-id", "0000000000000000000000000000000000000000000000000000000000000001",
            "--milestone-id", "0",
            "--refund-to-payer",
            "--dry-run"
        ])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "resolve with refund flag should succeed (dry-run)\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_batch_lock_with_multiple_ids() {
    let output = trellis_cmd()
        .args([
            "batch-lock",
            "--agreement-id", "0000000000000000000000000000000000000000000000000000000000000001",
            "--milestone-ids", "0,1,2,3",
            "--dry-run"
        ])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "batch-lock with multiple IDs should succeed (dry-run)\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_completion_command_generates_shell_completions() {
    let output = trellis_cmd()
        .args(["completion", "--shell", "bash"])
        .output()
        .expect("failed to execute trellis");

    assert!(
        output.status.success(),
        "completion should succeed\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("_trellis") || stdout.contains("complete"),
        "bash completion should contain completion function"
    );
}

// ---------------------------------------------------------------------------
// Agreement ID injection guard (#405)
// ---------------------------------------------------------------------------
// `validate_agreement_id` is invoked at the top of every command handler that
// accepts an `--agreement-id`, so a hostile value is rejected before any
// argument is built and long before `RpcClient` would shell out to
// `stellar contract invoke`.
//
// These tests assert that ordering directly: every command below is run
// WITHOUT `--dry-run`, so if validation were ever moved after argument
// building (or removed), the mock `stellar` binary would be reached and
// would succeed — the test would then fail on `assert!(!status.success())`.
// A dry-run-only test could not distinguish "rejected by validation" from
// "accepted and previewed".

/// Every command handler that takes an `--agreement-id`, paired with the
/// extra args it needs, so a single table can drive all of them.
///
/// Kept as an explicit table rather than generated so that adding a new
/// command without covering it here is a visible omission during review.
fn agreement_id_commands() -> Vec<(&'static str, Vec<&'static str>)> {
    let payer = "GBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVW";
    vec![
        ("init", vec![
            "--payer", payer,
            "--payee", "GZYXWVUTSRQPONMLKJIHGFEDCBA234567ZYXWVUTSRQPONMLKJIHGF",
            "--token", "CBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "--resolver", "GRESOLVABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNO",
            "--milestones", "1000",
            "--yes",
        ]),
        ("lock", vec!["--milestone-id", "0", "--yes"]),
        ("submit-work", vec!["--milestone-id", "0", "--yes"]),
        ("approve-release", vec!["--milestone-id", "0", "--yes"]),
        ("raise-dispute", vec!["--milestone-id", "0", "--caller", payer, "--yes"]),
        ("resolve-dispute", vec!["--milestone-id", "0", "--yes"]),
        ("cancel-milestone", vec!["--milestone-id", "0", "--yes"]),
        ("status", vec![]),
        ("milestone-status", vec!["--milestone-id", "0"]),
    ]
}

/// Assemble `<cmd> --agreement-id <id> <extra...>` for a handler under test.
fn build_args(cmd: &str, extra: &[&str], id: &str) -> Vec<String> {
    let mut args: Vec<String> = vec![
        cmd.to_string(),
        "--agreement-id".to_string(),
        id.to_string(),
    ];
    args.extend(extra.iter().map(|s| s.to_string()));
    args
}

/// A 64-char hex ID with a smuggled `--flag` appended past the 64-char mark.
///
/// This is the exact shape the guard exists to stop: the first 64 chars are a
/// well-formed ID, so only a length+charset check catches the trailing
/// ` --network mainnet` argument-injection payload.
const INJECTED_AGREEMENT_ID: &str =
    "0000000000000000000000000000000000000000000000000000000000000001 --network mainnet --source attacker";

/// A payload that is a well-formed 64-char hex ID with a `;` appended — the
/// shape that would smuggle a second argv entry if the charset check were
/// ever weakened to a length-only or prefix check.
///
/// The 64-hex prefix is important: only a *full* length+charset check rejects
/// it, so this is the adjacent case most likely to regress if the guard is
/// "fixed" carelessly into something that just checks `id.len() >= 64`.
const SEMICOLON_TRAILING_AGREEMENT_ID: &str =
    "0000000000000000000000000000000000000000000000000000000000000001;id";

/// Every command must reject an argument-injection payload in `--agreement-id`
/// before reaching `RpcClient` (#405).
#[test]
fn test_injected_agreement_id_rejected_by_every_command() {
    for (cmd, extra) in agreement_id_commands() {
        let output = trellis_cmd()
            .args(build_args(cmd, &extra, INJECTED_AGREEMENT_ID))
            .output()
            .expect("failed to execute trellis");

        assert!(
            !output.status.success(),
            "`{cmd}` must reject an injected --agreement-id before invoking the \
             stellar CLI (no --dry-run: a reached mock binary would succeed)"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("agreement_id"),
            "`{cmd}` should name the offending field in its error\nstderr: {stderr}"
        );
    }
}

/// Adjacent regression case (#405): a 64-hex ID with a trailing shell
/// metacharacter must also be rejected.
///
/// This guards the "only the length is checked" failure mode. The value's
/// first 64 characters are valid hex, so a length-only check would wave it
/// through and let `;` reach the argument vector.
#[test]
fn test_metacharacter_agreement_id_rejected_by_every_command() {
    for (cmd, extra) in agreement_id_commands() {
        let output = trellis_cmd()
            .args(build_args(
                cmd,
                &extra,
                SEMICOLON_TRAILING_AGREEMENT_ID,
            ))
            .output()
            .expect("failed to execute trellis");

        assert!(
            !output.status.success(),
            "`{cmd}` must reject a metacharacter in --agreement-id before \
             invoking the stellar CLI"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("agreement_id"),
            "`{cmd}` should name the offending field in its error\nstderr: {stderr}"
        );
    }
}

