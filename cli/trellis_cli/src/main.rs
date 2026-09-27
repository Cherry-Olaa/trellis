mod commands;
mod config;
mod input;
mod rpc;
mod sanitizer;
mod utils;

use clap::{CommandFactory, Parser};
use commands::{Commands, OutputFormat, OutputOpts};
use config::Network;
use std::process;

// ---------------------------------------------------------------------------
// Top-level CLI definition
// ---------------------------------------------------------------------------

/// Trellis Protocol CLI — milestone-based escrow on Stellar Soroban.
///
/// Configuration can be loaded from a `.env` file in the working directory or
/// set as environment variables.  CLI flags take highest priority, then env
/// vars, then the `.env` file.
///
/// Required variables:
///   TRELLIS_CONTRACT_ID        — Bech32 contract address (`C…`)
///   TRELLIS_SOURCE_KEY         — Stellar secret key or named identity
///
/// Optional variables (Soroban Testnet used as default):
///   STELLAR_RPC_URL            — Soroban JSON-RPC endpoint
///   STELLAR_NETWORK_PASSPHRASE — Network passphrase for transaction signing
///   STELLAR_RPC_RETRIES        — Retries for transient RPC failures (default 3; 0 disables)
#[derive(Parser, Debug)]
#[command(
    name = "trellis",
    version = env!("CARGO_PKG_VERSION"),
    // `--version` (long form) shows this extended string; `-V` (short form)
    // still shows the bare crate version above. Includes the on-chain
    // contract's soroban-sdk compatibility range so bug reports carry
    // enough environment context without a separate lookup.
    long_version = concat!(
        env!("CARGO_PKG_VERSION"),
        "\nsoroban-sdk compat: >=22.0.0, <23 (see contracts/trellis_core/Cargo.toml)",
    ),
    author,
    about,
    long_about = None,
    propagate_version = true,
)]
struct Cli {
    /// Network preset to connect to. `custom` requires `--rpc-url` and
    /// `--network-passphrase`.
    #[arg(long, global = true, value_enum, default_value_t = Network::Testnet)]
    network: Network,

    /// Custom Soroban RPC endpoint. Required when `--network=custom`;
    /// overrides the preset / env var for any other network.
    #[arg(long, global = true)]
    rpc_url: Option<String>,

    /// Custom network passphrase. Required when `--network=custom`;
    /// overrides the preset / env var for any other network.
    #[arg(long, global = true)]
    network_passphrase: Option<String>,

    /// Read the source key from this file instead of the `TRELLIS_SOURCE_KEY`
    /// environment variable. Preferred for raw `S…` secret seeds: file
    /// contents never appear in `ps` / `/proc` the way an argv or exported
    /// env var can (#240). Overrides `TRELLIS_SOURCE_KEY_FILE`.
    #[arg(long, global = true, value_name = "PATH")]
    source_key_file: Option<String>,

    #[command(subcommand)]
    command: Commands,

    /// Output a uniform, machine-parseable JSON envelope instead of raw
    /// command output. Takes priority over `--human-readable`.
    #[arg(long, global = true)]
    json: bool,

    /// Parse the underlying stellar CLI output into a human-friendly,
    /// colorized summary. Falls back to raw output if parsing fails.
    #[arg(short = 'H', long = "human-readable", global = true)]
    human_readable: bool,

    /// Suppress all output except the final JSON result. Implies `--json`.
    #[arg(long, global = true)]
    quiet: bool,

    /// Print the `stellar contract invoke` command that would run, without
    /// executing it or submitting anything on-chain.
    #[arg(long, global = true)]
    dry_run: bool,

    /// Allow a cleartext `http://` RPC endpoint for a non-localhost host.
    ///
    /// By default the CLI refuses any RPC URL that is not `https://` unless
    /// the host is loopback (`localhost` / `127.0.0.1` / `::1`), so a planted
    /// `.env` file cannot silently redirect traffic to an unauthenticated
    /// endpoint. Pass this flag for development against a self-hosted devnet.
    #[arg(long, global = true)]
    unsafe_rpc: bool,
}

// ---------------------------------------------------------------------------
// Environment validation (#68)
// ---------------------------------------------------------------------------

/// The minimum `stellar` CLI major version required by this tool.
///
/// DEPLOYMENT.md documents `stellar CLI 26.x+` as a prerequisite.
const STELLAR_MIN_MAJOR: u64 = 26;

/// Verify that the `stellar` CLI binary is available in `PATH` **and** that
/// its reported major version meets the documented minimum (`26.x+`).
///
/// This is called once at startup so users get a clear, actionable error
/// message instead of a cryptic OS-level "program not found" or an obscure
/// RPC/argument error deep inside the invoke path.
///
/// Returns `Ok(())` if the binary is found and version-compatible, or
/// `Err(message)` with installation/upgrade instructions otherwise.
fn validate_environment() -> Result<(), String> {
    use std::process::Command;

    let output = Command::new(rpc::stellar_bin())
        .arg("--version")
        .output()
        .map_err(|_| {
            "Error: `stellar` CLI not found in PATH.\n\
             \n\
             Install it with:\n\
             \n\
             \tcargo install --locked stellar-cli --features opt\n\
             \n\
             Or follow the official guide:\n\
             \thttps://developers.stellar.org/docs/tools/cli/install-cli\n\
             \n\
             After installing, run `stellar --version` to confirm the installation."
                .to_string()
        })?;

    // `stellar --version` prints something like "stellar 26.0.0\n" to stdout.
    // Parse the first version-looking token (digits separated by dots) from
    // the combined stdout + stderr so we are robust against minor format
    // changes across releases.
    let version_output = String::from_utf8_lossy(&output.stdout);
    let version_output = version_output.trim();

    let detected_major = version_output
        .split_whitespace()
        .find_map(|token| {
            // Take the first token that looks like a semver / version number
            // (starts with a digit, contains at least one dot).
            if token.starts_with(|c: char| c.is_ascii_digit()) && token.contains('.') {
                token
                    .split('.')
                    .next()
                    .and_then(|major| major.parse::<u64>().ok())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            format!(
                "Error: could not determine the `stellar` CLI version.\n\
                 \n\
                 `stellar --version` produced unexpected output: {version_output:?}\n\
                 \n\
                 Expected output like `stellar 26.0.0`.  Please ensure you have\n\
                 stellar CLI {STELLAR_MIN_MAJOR}.x or newer installed:\n\
                 \n\
                 \tcargo install --locked stellar-cli --features opt\n\
                 \thttps://developers.stellar.org/docs/tools/cli/install-cli"
            )
        })?;

    if detected_major < STELLAR_MIN_MAJOR {
        return Err(format!(
            "Error: `stellar` CLI version {detected_major}.x detected, \
             but version {STELLAR_MIN_MAJOR}.x or newer is required.\n\
             \n\
             Upgrade with:\n\
             \n\
             \tcargo install --locked stellar-cli --features opt\n\
             \n\
             Or follow the official guide:\n\
             \thttps://developers.stellar.org/docs/tools/cli/install-cli"
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    // ── #66: Load .env file before reading any env vars ───────────────────
    // dotenvy::dotenv() looks for a `.env` file in the current directory (and
    // walks up to the workspace root).  `.ok()` silently ignores a missing
    // file — users who rely purely on exported env vars are unaffected.
    // Priority: CLI args > env vars > .env file values.
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // ── #75: Shell completions never touch the network or the stellar
    // binary, so handle them before environment validation / dispatch. ────
    // Matched by reference (not by value) so `cli.command` is still whole
    // and movable into `dispatch` below when this arm doesn't match.
    if let Commands::Completion { shell } = &cli.command {
        let mut cmd = Cli::command();
        let bin_name = cmd.get_name().to_string();
        clap_complete::generate(*shell, &mut cmd, bin_name, &mut std::io::stdout());
        return;
    }

    // ── #68: Validate stellar binary at startup ────────────────────────────
    if let Err(msg) = validate_environment() {
        eprintln!("{msg}");
        process::exit(1);
    }

    // ── #80: Resolve config from --network preset + CLI / env overrides ───
    let config = match config::Config::resolve(
        cli.network,
        cli.rpc_url.clone(),
        cli.network_passphrase.clone(),
        cli.source_key_file.clone(),
    ) {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("{msg}");
            process::exit(1);
        }
    };

    // ── #236/#237: Reject a malformed contract ID or RPC URL up front with
    // a clear message instead of a cryptic failure deep in the Stellar CLI. ─
    if let Err(errors) = config.validate() {
        eprintln!("Error: invalid configuration:");
        for e in &errors {
            eprintln!("  - {e}");
        }
        eprintln!(
            "\nSet the required environment variables (TRELLIS_CONTRACT_ID, \
             TRELLIS_SOURCE_KEY) or pass the matching CLI flags."
        );
        process::exit(1);
    }

    // ── #156: Validate the resolved RPC URL before any network use ────────
    // Rejects malformed URLs and cleartext HTTP to non-localhost hosts so a
    // compromised `.env` cannot point the CLI at a forged RPC endpoint.
    // `--unsafe-rpc` downgrades the hard error to a printed warning.
    match config::validate_rpc_url(&config.rpc_url, cli.unsafe_rpc) {
        Ok(Some(warning)) => eprintln!("{warning}"),
        Ok(None) => {}
        Err(msg) => {
            eprintln!("{msg}");
            process::exit(1);
        }
    }

    // ── #77/#74: --json takes priority over --human-readable; --quiet
    // forces the JSON envelope so the only stdout line is the result. ──────
    let format = if cli.json || cli.quiet {
        OutputFormat::Json
    } else if cli.human_readable {
        OutputFormat::Human
    } else {
        OutputFormat::Raw
    };
    let opts = OutputOpts {
        format,
        quiet: cli.quiet,
        dry_run: cli.dry_run,
    };

    // ── #67: Propagate errors from dispatch; exit(1) only in main ─────────
    // All cleanup (destructors, buffer flushes) runs before the exit call
    // because process::exit is only called here, never inside library code.
    if let Err(msg) = commands::dispatch(cli.command, &config, &opts) {
        if !msg.is_empty() {
            eprintln!("{msg}");
        }
        process::exit(1);
    }
}
