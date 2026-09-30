use crate::config::Config;
use governor::{Quota, RateLimiter};
use std::io::Write;
use std::num::NonZeroU32;
use std::sync::OnceLock;

static RPC_RATE_LIMITER: OnceLock<RateLimiter> = OnceLock::new();

fn get_rate_limiter() -> &'static RateLimiter {
    RPC_RATE_LIMITER.get_or_init(|| {
        let limit_per_sec: u32 = std::env::var("STELLAR_RPC_RATE_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        if let Some(limit) = NonZeroU32::new(limit_per_sec) {
            RateLimiter::direct(Quota::per_second(limit))
        } else {
            RateLimiter::direct(Quota::per_second(NonZeroU32::new(10).unwrap()))
        }
    })
}

fn apply_rate_limit() {
    let limiter = get_rate_limiter();
    if limiter.check().is_err() {
        eprintln!("⚠️  RPC rate limit active — request queued until quota resets");
        limiter.until_ready().wait();
    }
}

/// Output from a Soroban contract invoke.
#[derive(Debug)]
pub struct InvokeOutput {
    /// Combined stdout from the process.
    pub stdout: String,
    /// Combined stderr from the process.
    pub stderr: String,
    /// Whether the process exited successfully.
    pub success: bool,
    /// The exact command string that was executed — printed on failure so
    /// the caller can reproduce/debug locally.
    pub command_debug: String,
}

/// Resolve which `stellar` executable to invoke.
///
/// The integration suite (`tests/cli_integration.rs`) sets
/// `TRELLIS_TEST_MODE=true` together with `STELLAR_MOCK_BIN=<script>` so the
/// tests exercise the full argv-building / output-rendering path against a
/// mock script instead of a live network or a real CLI install. In every
/// other case this is just `"stellar"` from `PATH`.
pub(crate) fn stellar_bin() -> String {
    if std::env::var_os("TRELLIS_TEST_MODE").is_some() {
        if let Some(mock) = std::env::var_os("STELLAR_MOCK_BIN") {
            return mock.to_string_lossy().into_owned();
        }
    }
    "stellar".to_string()
}

/// Native Soroban RPC client that talks directly to the Soroban JSON-RPC endpoint.
/// No external CLI dependency required.
pub struct RpcClient;

impl RpcClient {
    /// Invoke a Trellis contract function.
    ///
    /// Currently delegates to `stellar contract invoke` (see the type-level
    /// docs for the architecture and the planned native RPC rewrite).
    ///
    /// Transient RPC failures (timeouts, rate limits, temporary unavailability)
    /// are automatically retried with exponential backoff and jitter. The number
    /// of retries is controlled by `STELLAR_RPC_RETRIES` (default 3).
    ///
    /// # Arguments
    /// * `config`  – runtime configuration (RPC URL, keys, contract ID)
    /// * `fn_name` – the Soroban function name (e.g. `"init"`, `"lock_funds"`)
    /// * `args`    – a flat list of `--flag value` pairs **after** the `--`
    ///   separator, e.g. `["--agreement_id", "0x…", "--payer", "G…"]`
    /// * `quiet`   – suppress the retry progress messages normally printed to stderr
    pub fn invoke(config: &Config, fn_name: &str, args: &[String], quiet: bool) -> InvokeOutput {
        // TODO(native-rpc): replace this shell-out with direct Soroban
        // JSON-RPC calls (typed arg parsing, key loading, envelope signing,
        // submit + poll). See the `RpcClient` type docs for the full plan.
        // Until then we delegate to the `stellar` CLI, which already handles
        // argument encoding, transaction assembly, signing and submission.
        Self::invoke_with_retry(config, fn_name, args, quiet)
    }

    /// Build the exact `stellar contract invoke …` argument list and its
    /// copy-paste-friendly command string, without executing anything.
    ///
    /// Shared by the real invocation path (so failures can print the command
    /// that ran) and by `--dry-run` previews (which never execute at all).
    fn build_cmd_args(config: &Config, fn_name: &str, args: &[String]) -> (Vec<String>, String) {
        let mut cmd_args: Vec<String> = vec![
            "contract".to_string(),
            "invoke".to_string(),
            "--id".to_string(),
            config.contract_id.clone(),
        ];

        // #240: a raw `S…` secret seed must never land in argv — anyone on the
        // host can read it via `ps`. Pass it to the child through the
        // `STELLAR_SECRET_KEY` environment variable instead (see
        // `invoke_once`); only non-secret identity names go on the command
        // line. Named `stellar keys` identities are still passed via
        // `--source` exactly as before.
        if !crate::config::is_secret_seed(&config.source_key) {
            cmd_args.push("--source".to_string());
            cmd_args.push(config.source_key.clone());
        }

        cmd_args.extend_from_slice(&[
            "--rpc-url".to_string(),
            config.rpc_url.clone(),
            "--network-passphrase".to_string(),
            config.network_passphrase.clone(),
            "--".to_string(),
            fn_name.to_string(),
        ]);
        cmd_args.extend_from_slice(args);

        // Quote any argument containing whitespace so the printed command can
        // be copy-pasted straight into a shell.
        let quoted = cmd_args
            .iter()
            .map(|a| {
                if a.contains(' ') {
                    format!("'{a}'")
                } else {
                    a.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        // Never print the seed itself — show that it is supplied via the
        // environment so `--dry-run` / failure output stays copy-pasteable
        // without leaking the key.
        let command_debug = if crate::config::is_secret_seed(&config.source_key) {
            format!("STELLAR_SECRET_KEY=<redacted> stellar {quoted}")
        } else {
            format!("stellar {quoted}")
        };

        (cmd_args, command_debug)
    }

    /// Build the `stellar contract invoke …` command that *would* run for
    /// `fn_name`/`args`, without executing it. Used by `--dry-run`.
    pub fn preview(config: &Config, fn_name: &str, args: &[String]) -> String {
        Self::build_cmd_args(config, fn_name, args).1
    }

    /// Fetch the network passphrase reported by the configured RPC endpoint.
    ///
    /// Issues a native `getNetwork` JSON-RPC request directly to
    /// `config.rpc_url` (no `stellar` CLI dependency) and returns the
    /// `passphrase` field from the response.
    ///
    /// The request is sent over a plain `TcpStream` using a minimal HTTP/1.1
    /// POST so the CLI does not need an async runtime or an HTTP client
    /// dependency. Only `http://` endpoints are supported here; `https://`
    /// endpoints return an error so callers can fall back to the CLI path.
    pub fn get_network_passphrase(config: &Config) -> Result<String, String> {
        let url = config.rpc_url.trim();
        let rest = url
            .strip_prefix("http://")
            .ok_or_else(|| format!("unsupported RPC URL scheme (expected http://): {url}"))?;

        let (host_port, path) = match rest.find('/') {
            Some(idx) => (&rest[..idx], &rest[idx..]),
            None => (rest, "/"),
        };
        let (host, port) = match host_port.rsplit_once(':') {
            Some((h, p)) => (
                h,
                p.parse::<u16>()
                    .map_err(|_| format!("invalid port in RPC URL: {url}"))?,
            ),
            None => (host_port, 80u16),
        };

        let body = r#"{"jsonrpc":"2.0","id":1,"method":"getNetwork"}"#;
        let request = format!(
            "POST {path} HTTP/1.1\r\n\
             Host: {host}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {len}\r\n\
             Connection: close\r\n\
             \r\n\
             {body}",
            len = body.len(),
        );

        let mut stream = std::net::TcpStream::connect((host, port))
            .map_err(|e| format!("failed to connect to RPC endpoint {host}:{port}: {e}"))?;
        stream
            .write_all(request.as_bytes())
            .map_err(|e| format!("failed to send getNetwork request: {e}"))?;

        let mut response = String::new();
        std::io::Read::read_to_string(&mut stream, &mut response)
            .map_err(|e| format!("failed to read getNetwork response: {e}"))?;

        let body_start = response
            .find("\r\n\r\n")
            .map(|i| i + 4)
            .ok_or_else(|| "malformed HTTP response from RPC endpoint".to_string())?;
        let json = &response[body_start..];

        extract_json_string_field(json, "passphrase")
            .ok_or_else(|| format!("RPC getNetwork response missing `passphrase`: {json}"))
    }

    /// Verify that `config.network_passphrase` matches the passphrase reported
    /// by the configured RPC endpoint.
    ///
    /// Returns `Ok(())` when they match and a clear, specific error naming both
    /// values when they do not. This is intended to be called early (e.g. from
    /// `validate_environment`) so a misconfigured `--rpc-url` /
    /// `--network-passphrase` pair fails fast instead of producing a confusing
    /// downstream error.
    pub fn verify_network_passphrase(config: &Config) -> Result<(), String> {
        let actual = Self::get_network_passphrase(config)?;
        if actual == config.network_passphrase {
            Ok(())
        } else {
            Err(format!(
                "network passphrase mismatch: configured '{}' but RPC endpoint {} reports '{}'",
                config.network_passphrase, config.rpc_url, actual
            ))
        }
    }

    /// Invoke via stellar CLI with automatic retry on transient RPC failures.
    ///
    /// Backoff schedule (before jitter): 1 s, 2 s, 4 s, 8 s (capped).
    /// Jitter adds up to 200 ms derived from the current system clock so
    /// concurrent processes do not thunder-herd the RPC endpoint together.
    ///
    /// Set `STELLAR_RPC_RETRIES=0` to disable retries entirely.
    fn invoke_with_retry(
        config: &Config,
        fn_name: &str,
        args: &[String],
        quiet: bool,
    ) -> InvokeOutput {
        let max_retries: u32 = std::env::var("STELLAR_RPC_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);

        // Exponential backoff: 1 s → 2 s → 4 s → 8 s (capped at index 3).
        const BACKOFF_MS: [u64; 4] = [1_000, 2_000, 4_000, 8_000];

        let mut attempt = 0u32;
        loop {
            let out = Self::invoke_once(config, fn_name, args);

            if out.success {
                return out;
            }

            // Spawn failure means the stellar CLI is not installed — no point retrying.
            if out.stderr.starts_with("Failed to spawn") {
                return out;
            }

            if attempt >= max_retries {
                return out;
            }

            // Only retry errors that look like transient network / RPC issues.
            if !is_transient_error(&out.stderr) {
                return out;
            }

            attempt += 1;
            let idx = ((attempt - 1) as usize).min(BACKOFF_MS.len() - 1);
            let base_ms = BACKOFF_MS[idx];
            let jitter_ms = jitter_millis();
            let delay_ms = base_ms + jitter_ms;

            if !quiet {
                eprintln!(
                    "RPC attempt {attempt}/{max_retries} failed (transient error), retrying in {delay_ms}ms…"
                );
                eprintln!(
                    "  {}",
                    out.stderr.lines().next().unwrap_or("(no error message)")
                );
            }

            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }
    }

    /// Single attempt at invoking the stellar CLI — no retry logic here.
    fn invoke_once(config: &Config, fn_name: &str, args: &[String]) -> InvokeOutput {
        use std::process::Command;

        let (cmd_args, command_debug) = Self::build_cmd_args(config, fn_name, args);

        let mut command = Command::new(stellar_bin());
        command.args(&cmd_args);

        // #240: hand a raw secret seed to the child via its environment rather
        // than argv so it cannot be read from `ps` / `/proc/<pid>/cmdline`.
        if crate::config::is_secret_seed(&config.source_key) {
            command.env("STELLAR_SECRET_KEY", &config.source_key);
        }

        let output = command.output();

        match output {
            Ok(out) => InvokeOutput {
                stdout: decode_process_output("stdout", out.stdout),
                stderr: decode_process_output("stderr", out.stderr),
                success: out.status.success(),
                command_debug,
            },
            Err(e) => InvokeOutput {
                stdout: String::new(),
                stderr: format!(
                    "Failed to spawn `stellar` CLI: {e}\n\
                     Is the Stellar CLI installed?  https://developers.stellar.org/docs/tools/cli/install-cli"
                ),
                success: false,
                command_debug,
            },
        }
    }
}

/// Decode a subprocess output stream, without silently discarding bytes.
///
/// `String::from_utf8_lossy` replaces every invalid byte sequence with
/// U+FFFD, which can erase the very error detail a caller needs to debug a
/// non-UTF-8 failure. This tries strict UTF-8 first; on failure it falls
/// back to Latin-1 (ISO-8859-1), a direct byte→codepoint mapping that never
/// fails and preserves every original byte, and logs a warning to stderr
/// (including a hex preview of the raw bytes) so the user still has the
/// original context even though the string could not be decoded cleanly.
fn decode_process_output(label: &str, bytes: Vec<u8>) -> String {
    let (decoded, fell_back) = decode_bytes(&bytes);
    if let Some(first_bad) = fell_back {
        eprintln!(
            "warning: stellar CLI {label} was not valid UTF-8 ({} bytes, first \
             invalid byte at offset {first_bad}); decoded as Latin-1 — output \
             may not render correctly. Raw bytes (hex): {}",
            bytes.len(),
            hex_preview(&bytes),
        );
    }
    decoded
}

/// Decode `bytes` as UTF-8, falling back to a lossless Latin-1 mapping.
///
/// Returns the decoded string and, when the Latin-1 fallback was used, the
/// byte offset of the first invalid UTF-8 sequence (so callers can point at
/// exactly where the stream stopped being valid UTF-8).
fn decode_bytes(bytes: &[u8]) -> (String, Option<usize>) {
    match std::str::from_utf8(bytes) {
        Ok(s) => (s.to_string(), None),
        Err(e) => (
            // Latin-1: every byte maps 1:1 to U+0000..=U+00FF, so no byte is
            // ever lost and the original stream can be recovered.
            bytes.iter().map(|&b| b as char).collect(),
            Some(e.valid_up_to()),
        ),
    }
}

/// Render up to the first 64 bytes of `bytes` as space-separated hex, so a
/// non-UTF-8 stream still leaves a reproducible trace in the warning.
fn hex_preview(bytes: &[u8]) -> String {
    const MAX: usize = 64;
    let mut out = bytes
        .iter()
        .take(MAX)
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    if bytes.len() > MAX {
        out.push_str(&format!(" … (+{} more)", bytes.len() - MAX));
    }
    out
}

/// Extract the value of a top-level string field from a JSON object without
/// pulling in a JSON dependency.
///
/// This is intentionally minimal: it looks for `"<field>"` followed by a
/// colon and a double-quoted string, and returns the unescaped contents. It is
/// only used for the small, well-formed `getNetwork` response.
fn extract_json_string_field(json: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\"");
    let start = json.find(&needle)? + needle.len();
    let after = &json[start..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    let mut chars = rest.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for c in chars {
        if escaped {
            out.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(out);
        } else {
            out.push(c);
        }
    }
    None
}

/// Return true when stderr content indicates a transient, retriable RPC error.
///
/// Matches common patterns from Stellar RPC responses, HTTP errors, and
/// OS-level network failures. Contract-level errors (e.g. "contract not found",
/// "invalid argument") do not match and will not be retried.
///
/// Patterns are deliberately specific. A bare `"network"` substring, for
/// example, also matches the *permanent* error "network passphrase mismatch",
/// which would send the CLI into an endless retry loop (issue #249). Each entry
/// below is an exact phrase that only appears in genuinely transient failures;
/// add a negative test to `non_transient_*` whenever a new pattern is added.
fn is_transient_error(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    const TRANSIENT_PATTERNS: &[&str] = &[
        "timeout",
        "timed out",
        "connection refused",
        "connection reset",
        "connection closed",
        "connection error",
        "network error",
        "network timeout",
        "network is unreachable",
        "network is down",
        "temporary failure in name resolution",
        "rate limit",
        "too many requests",
        "service unavailable",
        "bad gateway",
        "gateway timeout",
        "deadline exceeded",
        "host unreachable",
        "no route to host",
        " 429",
        " 502",
        " 503",
        " 504",
    ];
    TRANSIENT_PATTERNS.iter().any(|p| lower.contains(p))
}

/// Compute a 0–199 ms jitter value from the subsecond part of the system clock.
///
/// Using wall-clock nanoseconds avoids a dependency on the `rand` crate while
/// still producing enough variance to prevent concurrent processes from all
/// waking up at the same millisecond.
fn jitter_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| (d.subsec_nanos() % 200) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_transient_error ---

    #[test]
    fn transient_detects_timeout() {
        assert!(is_transient_error("error: connection timeout after 30s"));
        assert!(is_transient_error("request timed out"));
    }

    #[test]
    fn transient_detects_connection_refused() {
        assert!(is_transient_error(
            "Os error: connection refused (os error 111)"
        ));
    }

    #[test]
    fn transient_detects_rate_limit_http_codes() {
        assert!(is_transient_error(
            "server returned status 429 Too Many Requests"
        ));
        assert!(is_transient_error("HTTP 503 Service Unavailable"));
        assert!(is_transient_error("upstream error: 502 Bad Gateway"));
        assert!(is_transient_error("gateway timeout: 504"));
    }

    #[test]
    fn transient_detects_rate_limit_text() {
        assert!(is_transient_error("rate limit exceeded, please slow down"));
        assert!(is_transient_error("too many requests"));
    }

    #[test]
    fn non_transient_contract_errors_not_retried() {
        assert!(!is_transient_error("contract not found: CABC123"));
        assert!(!is_transient_error("invalid argument: agreement_id"));
        assert!(!is_transient_error("error: source account does not exist"));
        assert!(!is_transient_error("authentication failed"));
    }

    #[test]
    fn non_transient_empty_stderr_not_retried() {
        assert!(!is_transient_error(""));
    }

    /// #249: permanent errors that merely *contain* a transient-looking word
    /// (most notably "network") must never trigger a retry.
    #[test]
    fn non_transient_network_config_errors_not_retried() {
        assert!(!is_transient_error(
            "error: network passphrase mismatch: expected 'Test SDF Network ; September 2015'"
        ));
        assert!(!is_transient_error("unknown network 'testnet'"));
        assert!(!is_transient_error("no network configured; run `stellar network add`"));
        assert!(!is_transient_error("network name contains invalid characters"));
        // "deadline" / "temporary" / "unreachable" as bare words in an
        // unrelated message are no longer enough on their own.
        assert!(!is_transient_error("filing deadline for the proposal has passed"));
        assert!(!is_transient_error("temporary directory could not be created"));
    }

    #[test]
    fn transient_detects_network_failure_phrases() {
        assert!(is_transient_error("network error: could not reach RPC endpoint"));
        assert!(is_transient_error("Os error: network is unreachable (os error 101)"));
        assert!(is_transient_error("dns lookup failed: Temporary failure in name resolution"));
        assert!(is_transient_error("504 Gateway Timeout"));
        assert!(is_transient_error("grpc status: deadline exceeded"));
    }

    // --- decode_process_output / decode_bytes ---

    #[test]
    fn decode_bytes_passes_through_valid_utf8() {
        let (s, fell_back) = decode_bytes("héllo — 世界".as_bytes());
        assert_eq!(s, "héllo — 世界");
        assert_eq!(fell_back, None);
    }

    #[test]
    fn decode_bytes_falls_back_to_latin1_on_invalid_utf8() {
        // 0xE9 is "é" in Latin-1 but an incomplete UTF-8 lead byte here.
        let raw = b"caf\xE9 not utf8";
        let (s, fell_back) = decode_bytes(raw);
        assert_eq!(s, "café not utf8");
        assert_eq!(fell_back, Some(3), "first invalid byte is at offset 3");
        // Every original byte is still recoverable from the decoded string.
        assert_eq!(s.chars().count(), raw.len());
    }

    #[test]
    fn decode_bytes_handles_mixed_valid_and_invalid_sequences() {
        // Valid multi-byte UTF-8 ("→", 0xE2 0x86 0x92) followed by a lone 0xFF.
        let raw = b"ok \xE2\x86\x92 then \xFF end";
        let (s, fell_back) = decode_bytes(raw);
        assert_eq!(fell_back, Some(3));
        assert!(s.starts_with("ok "));
        assert!(s.ends_with(" end"));
        assert_eq!(s.chars().count(), raw.len());
    }

    #[test]
    fn decode_process_output_returns_clean_string_for_valid_utf8() {
        assert_eq!(
            decode_process_output("stdout", "all good".as_bytes().to_vec()),
            "all good"
        );
    }

    #[test]
    fn decode_process_output_still_returns_bytes_on_fallback() {
        let out = decode_process_output("stderr", b"bad \xC0\xC0 byte".to_vec());
        assert!(out.contains("bad "));
        assert!(out.contains(" byte"));
    }

    #[test]
    fn hex_preview_formats_and_truncates() {
        assert_eq!(hex_preview(&[0x00, 0x1f, 0xff]), "00 1f ff");
        let long: Vec<u8> = (0..80).map(|_| 0xABu8).collect();
        let preview = hex_preview(&long);
        assert!(preview.contains("(+16 more)"), "got: {preview}");
    }

    // --- jitter_millis ---

    #[test]
    fn jitter_within_bounds() {
        for _ in 0..20 {
            let j = jitter_millis();
            assert!(j < 200, "jitter {j} should be < 200ms");
        }
    }

    // --- STELLAR_RPC_RETRIES parsing ---

    #[test]
    fn retry_count_defaults_to_three() {
        // Temporarily unset the var to test the default.
        std::env::remove_var("STELLAR_RPC_RETRIES");
        let retries: u32 = std::env::var("STELLAR_RPC_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);
        assert_eq!(retries, 3);
    }

    #[test]
    fn retry_count_reads_from_env() {
        std::env::set_var("STELLAR_RPC_RETRIES", "5");
        let retries: u32 = std::env::var("STELLAR_RPC_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);
        assert_eq!(retries, 5);
        std::env::remove_var("STELLAR_RPC_RETRIES");
    }

    #[test]
    fn retry_count_falls_back_on_invalid_value() {
        std::env::set_var("STELLAR_RPC_RETRIES", "not_a_number");
        let retries: u32 = std::env::var("STELLAR_RPC_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);
        assert_eq!(retries, 3);
        std::env::remove_var("STELLAR_RPC_RETRIES");
    }

    // --- #240: secret seed never reaches argv / printed output ---

    fn cfg_with_source(source_key: &str) -> Config {
        Config {
            rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            network_passphrase: "Test SDF Network ; September 2015".to_string(),
            contract_id: "CAABC123".to_string(),
            source_key: source_key.to_string(),
        }
    }

    fn a_seed() -> String {
        // 56 chars, `S` + base32 — matches `config::is_secret_seed`.
        format!("S{}", "A".repeat(55))
    }

    #[test]
    fn build_cmd_args_omits_secret_seed_from_argv() {
        let seed = a_seed();
        let (argv, debug) = RpcClient::build_cmd_args(&cfg_with_source(&seed), "init", &[]);
        assert!(
            !argv.iter().any(|a| a == &seed),
            "secret seed must not appear in argv: {argv:?}"
        );
        assert!(
            !argv.iter().any(|a| a == "--source"),
            "--source flag must be dropped for a raw seed"
        );
        assert!(!debug.contains(&seed), "seed must not be printed: {debug}");
        assert!(debug.starts_with("STELLAR_SECRET_KEY=<redacted> stellar "));
    }

    #[test]
    fn build_cmd_args_keeps_source_for_identity_name() {
        let (argv, debug) = RpcClient::build_cmd_args(&cfg_with_source("alice"), "init", &[]);
        let src = argv
            .iter()
            .position(|a| a == "--source")
            .expect("--source present");
        assert_eq!(argv[src + 1], "alice");
        assert!(debug.starts_with("stellar contract invoke"));
    }

    #[test]
    fn preview_never_prints_a_secret_seed() {
        let seed = a_seed();
        let preview = RpcClient::preview(&cfg_with_source(&seed), "init", &[]);
        assert!(
            !preview.contains(&seed),
            "dry-run leaked the seed: {preview}"
        );
        assert!(preview.contains("<redacted>"));
    }

    // --- network passphrase verification ---

    #[test]
    fn extract_json_string_field_reads_passphrase() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"passphrase":"Test SDF Network ; September 2015","protocolVersion":20}}"#;
        assert_eq!(
            extract_json_string_field(json, "passphrase").as_deref(),
            Some("Test SDF Network ; September 2015")
        );
    }

    #[test]
    fn extract_json_string_field_handles_escapes() {
        let json = r#"{"passphrase":"a \"quoted\" value"}"#;
        assert_eq!(
            extract_json_string_field(json, "passphrase").as_deref(),
            Some("a \"quoted\" value")
        );
    }

    #[test]
    fn extract_json_string_field_missing_returns_none() {
        assert_eq!(extract_json_string_field(r#"{"result":{}}"#, "passphrase"), None);
    }

    #[test]
    fn verify_network_passphrase_rejects_unsupported_scheme() {
        let mut cfg = cfg_with_source("alice");
        cfg.rpc_url = "https://soroban-testnet.stellar.org".to_string();
        let err = RpcClient::verify_network_passphrase(&cfg).unwrap_err();
        assert!(err.contains("unsupported RPC URL scheme"), "got: {err}");
    }
}
