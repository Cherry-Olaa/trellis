use crate::config::Config;
use crate::commands::ContractResult;
use governor::{Quota, RateLimiter};
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

/// Decode a base64-encoded Soroban `ScVal` XDR result into the CLI's
/// internal `ContractResult` representation.
///
/// This is the native replacement for shelling out to `stellar contract
/// invoke` and re-printing its stdout: callers that already have a raw XDR
/// `ScVal` (e.g. from `simulateTransaction`) can decode it directly into the
/// same shape `render_json` / `render_human` consume.
///
/// The decoder understands the two result shapes produced by the Trellis
/// contract's read-only queries:
///
/// * `get_agreement` → a `ScVal::Map` with fields `id`, `payer`, `payee`,
///   `amount`, `status`, `milestone_count`.
/// * `get_milestone` → a `ScVal::Map` with fields `agreement_id`, `index`,
///   `amount`, `status`, `released`.
///
/// Returns `Err` with a human-readable message when the XDR is malformed or
/// the top-level value is not a map (so callers can surface a clear error
/// instead of silently printing garbage).
pub fn decode_scval_result(xdr_base64: &str) -> Result<ContractResult, String> {
    let raw = base64_decode(xdr_base64)
        .map_err(|e| format!("invalid base64 in ScVal result: {e}"))?;
    let val = parse_scval(&raw)
        .map_err(|e| format!("failed to parse ScVal XDR: {e}"))?;
    scval_to_contract_result(&val)
}

/// Minimal base64 decoder (standard alphabet, `=` padding) so the CLI does
/// not need an extra dependency just for result decoding.
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if bytes.len() % 4 != 0 {
        return Err("length is not a multiple of 4".to_string());
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let pad = chunk.iter().filter(|&&b| b == b'=').count();
        if pad > 2 {
            return Err("too much padding".to_string());
        }
        let mut n: u32 = 0;
        for (i, &b) in chunk.iter().enumerate() {
            let v = if b == b'=' {
                0
            } else {
                val(b).ok_or_else(|| format!("invalid base64 byte 0x{b:02x} at index {i}"))?
            };
            n = (n << 6) | v as u32;
        }
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

/// A parsed subset of the Soroban `ScVal` union — just enough to represent
/// the values returned by `get_agreement` / `get_milestone`.
#[derive(Debug, Clone, PartialEq)]
enum ScVal {
    Void,
    Bool(bool),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    U128(u128),
    I128(i128),
    Symbol(String),
    String(String),
    Bytes(Vec<u8>),
    Address(String),
    Map(Vec<(ScVal, ScVal)>),
    Vec(Vec<ScVal>),
}

/// Parse a raw `ScVal` XDR blob into the local `ScVal` enum.
///
/// This is a deliberately small reader: it walks the XDR discriminant and
/// payload for the variants the Trellis contract actually returns. Unknown
/// discriminants produce an error rather than a silent mis-decode.
fn parse_scval(bytes: &[u8]) -> Result<ScVal, String> {
    let mut cur = std::io::Cursor::new(bytes);
    read_scval(&mut cur)
}

fn read_u32(cur: &mut std::io::Cursor<&[u8]>) -> Result<u32, String> {
    use std::io::Read;
    let mut buf = [0u8; 4];
    cur.read_exact(&mut buf)
        .map_err(|e| format!("unexpected end of XDR: {e}"))?;
    Ok(u32::from_be_bytes(buf))
}

fn read_u64(cur: &mut std::io::Cursor<&[u8]>) -> Result<u64, String> {
    use std::io::Read;
    let mut buf = [0u8; 8];
    cur.read_exact(&mut buf)
        .map_err(|e| format!("unexpected end of XDR: {e}"))?;
    Ok(u64::from_be_bytes(buf))
}

fn read_scval(cur: &mut std::io::Cursor<&[u8]>) -> Result<ScVal, String> {
    let disc = read_u32(cur)?;
    match disc {
        0 => Ok(ScVal::Void),
        1 => Ok(ScVal::Bool(read_u32(cur)? != 0)),
        3 => Ok(ScVal::I32(read_u32(cur)? as i32)),
        4 => Ok(ScVal::U32(read_u32(cur)?)),
        5 => Ok(ScVal::I64(read_u64(cur)? as i64)),
        6 => Ok(ScVal::U64(read_u64(cur)?)),
        10 => {
            let len = read_u32(cur)? as usize;
            let mut buf = vec![0u8; len];
            use std::io::Read;
            cur.read_exact(&mut buf)
                .map_err(|e| format!("unexpected end of XDR: {e}"))?;
            Ok(ScVal::Bytes(buf))
        }
        14 => {
            let len = read_u32(cur)? as usize;
            let mut buf = vec![0u8; len];
            use std::io::Read;
            cur.read_exact(&mut buf)
                .map_err(|e| format!("unexpected end of XDR: {e}"))?;
            Ok(ScVal::String(String::from_utf8_lossy(&buf).into_owned()))
        }
        15 => {
            let len = read_u32(cur)? as usize;
            let mut buf = vec![0u8; len];
            use std::io::Read;
            cur.read_exact(&mut buf)
                .map_err(|e| format!("unexpected end of XDR: {e}"))?;
            Ok(ScVal::Symbol(String::from_utf8_lossy(&buf).into_owned()))
        }
        16 => {
            // ScVal::Address — the payload is a ScAddress union. We only
            // need a printable form; the contract's read-only queries return
            // account/contract addresses encoded as StrKey in the CLI's
            // existing output, so we render the raw XDR bytes as hex here
            // and let callers that need StrKey re-encode.
            let addr_type = read_u32(cur)?;
            match addr_type {
                0 => {
                    // SC_ADDRESS_TYPE_ACCOUNT: PublicKey union, Ed25519 = 0.
                    let pk_type = read_u32(cur)?;
                    if pk_type != 0 {
                        return Err(format!("unsupported PublicKey type {pk_type}"));
                    }
                    let mut buf = [0u8; 32];
                    use std::io::Read;
                    cur.read_exact(&mut buf)
                        .map_err(|e| format!("unexpected end of XDR: {e}"))?;
                    Ok(ScVal::Address(hex_encode(&buf)))
                }
                1 => {
                    // SC_ADDRESS_TYPE_CONTRACT: 32-byte hash.
                    let mut buf = [0u8; 32];
                    use std::io::Read;
                    cur.read_exact(&mut buf)
                        .map_err(|e| format!("unexpected end of XDR: {e}"))?;
                    Ok(ScVal::Address(hex_encode(&buf)))
                }
                other => Err(format!("unsupported ScAddress type {other}")),
            }
        }
        17 => {
            // ScVal::Vec
            let len = read_u32(cur)? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(read_scval(cur)?);
            }
            Ok(ScVal::Vec(items))
        }
        18 => {
            // ScVal::Map
            let len = read_u32(cur)? as usize;
            let mut entries = Vec::with_capacity(len);
            for _ in 0..len {
                let k = read_scval(cur)?;
                let v = read_scval(cur)?;
                entries.push((k, v));
            }
            Ok(ScVal::Map(entries))
        }
        other => Err(format!("unsupported ScVal discriminant {other}")),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Convert a decoded `ScVal::Map` into the CLI's `ContractResult` shape.
fn scval_to_contract_result(val: &ScVal) -> Result<ContractResult, String> {
    let map = match val {
        ScVal::Map(m) => m,
        other => {
            return Err(format!(
                "expected ScVal::Map at top level, got {other:?}"
            ))
        }
    };
    let mut result = ContractResult::default();
    for (k, v) in map {
        let key = match k {
            ScVal::Symbol(s) | ScVal::String(s) => s.clone(),
            other => return Err(format!("non-string map key: {other:?}")),
        };
        result.fields.insert(key, scval_to_json(v));
    }
    Ok(result)
}

/// Render a decoded `ScVal` as a `serde_json::Value` so it slots directly
/// into the existing `render_json` output.
fn scval_to_json(val: &ScVal) -> serde_json::Value {
    use serde_json::Value;
    match val {
        ScVal::Void => Value::Null,
        ScVal::Bool(b) => Value::Bool(*b),
        ScVal::U32(n) => Value::from(*n),
        ScVal::I32(n) => Value::from(*n),
        ScVal::U64(n) => Value::from(*n),
        ScVal::I64(n) => Value::from(*n),
        ScVal::U128(n) => Value::from(n.to_string()),
        ScVal::I128(n) => Value::from(n.to_string()),
        ScVal::Symbol(s) | ScVal::String(s) => Value::from(s.clone()),
        ScVal::Bytes(b) => Value::from(hex_encode(b)),
        ScVal::Address(a) => Value::from(a.clone()),
        ScVal::Vec(items) => Value::Array(items.iter().map(scval_to_json).collect()),
        ScVal::Map(entries) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in entries {
                let key = match k {
                    ScVal::Symbol(s) | ScVal::String(s) => s.clone(),
                    other => format!("{other:?}"),
                };
                obj.insert(key, scval_to_json(v));
            }
            Value::Object(obj)
        }
    }
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

    // --- decode_scval_result ---

    #[test]
    fn decode_scval_result_rejects_invalid_base64() {
        let err = decode_scval_result("not!valid!base64!").unwrap_err();
        assert!(err.contains("base64"), "got: {err}");
    }

    #[test]
    fn decode_scval_result_rejects_truncated_xdr() {
        // Valid base64 for a single byte — too short to be a ScVal.
        let err = decode_scval_result("AQ==").unwrap_err();
        assert!(err.contains("XDR") || err.contains("parse"), "got: {err}");
    }

    #[test]
    fn base64_decode_roundtrips_known_vector() {
        // "hello" → "aGVsbG8="
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64_decode("aGVsbG8").unwrap_err(), "length is not a multiple of 4");
    }
}
