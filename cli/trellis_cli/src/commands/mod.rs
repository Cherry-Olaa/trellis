use std::io::IsTerminal as _;

use clap::Subcommand;
use clap_complete::Shell;

use crate::config::Config;
use crate::rpc::{InvokeOutput, RpcClient};

// ---------------------------------------------------------------------------
// ANSI escape codes (#245)
// ---------------------------------------------------------------------------
// Hoisted to module level so they are compiled once instead of being
// re-declared on every `render_human` call. `ANSI_`-prefixed to avoid
// colliding with any other module item.

/// ANSI SGR: green foreground.
const ANSI_GREEN: &str = "\x1b[32m";
/// ANSI SGR: red foreground.
const ANSI_RED: &str = "\x1b[31m";
/// ANSI SGR: bold.
const ANSI_BOLD: &str = "\x1b[1m";
/// ANSI SGR: reset all attributes.
const ANSI_RESET: &str = "\x1b[0m";

/// Returns `true` when ANSI color output is appropriate.
///
/// Colors are suppressed when either of these conditions holds:
/// - The `NO_COLOR` environment variable is present (any value), per
///   <https://no-color.org/>.
/// - stdout is not connected to a terminal (i.e. it is a pipe or file), so
///   that piped / redirected output never contains raw escape sequences.
fn colors_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

// ---------------------------------------------------------------------------
// Output rendering options (#74, #76, #77)
// ---------------------------------------------------------------------------

/// Output rendering mode selected via the global `--json` / `--human-readable`
/// flags. `--json` takes priority when both are passed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    /// Raw stdout from the underlying `stellar contract invoke` call (default).
    Raw,
    /// Uniform, machine-parseable JSON envelope: `{status, result, tx_hash, events, error}`.
    Json,
    /// Parsed, colorized human-friendly summary; falls back to raw text if parsing fails.
    Human,
}

/// Global output/execution options threaded through every command handler.
#[derive(Clone, Copy, Debug)]
pub struct OutputOpts {
    pub format: OutputFormat,
    /// Suppress retry/progress messages so only the final JSON result is printed.
    /// Forces `format` to `Json` regardless of `--human-readable`.
    pub quiet: bool,
    /// Print the `stellar contract invoke` command instead of executing it.
    pub dry_run: bool,
}

// ---------------------------------------------------------------------------
// Commands enum — parsed by clap from argv
// ---------------------------------------------------------------------------

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a new escrow agreement on-chain.
    Init {
        /// Hex-encoded 32-byte agreement ID (64 hex chars).
        #[arg(long)]
        agreement_id: String,

        /// Stellar address of the payer (funder).
        #[arg(long)]
        payer: String,

        /// Stellar address of the payee (contractor).
        #[arg(long)]
        payee: String,

        /// SAC or token contract address used for payments.
        #[arg(long)]
        token: String,

        /// Address of the neutral dispute resolver.
        #[arg(long)]
        resolver: String,

        /// Comma-separated milestone amounts in the token's base unit.
        /// Example: --milestones "1000,2000,500"
        #[arg(long)]
        milestones: String,

        /// Skip the confirmation prompt (for scripting).
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Lock funds for a specific milestone into the escrow contract.
    LockFunds {
        /// Agreement ID (hex-encoded, 64 chars).
        #[arg(long)]
        agreement_id: String,

        /// Zero-based index of the milestone to fund.
        #[arg(long)]
        milestone_id: u32,

        /// Skip the confirmation prompt (for scripting).
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Submit proof of work for a funded milestone.
    SubmitWork {
        /// Agreement ID (hex-encoded, 64 chars).
        #[arg(long)]
        agreement_id: String,

        /// Zero-based index of the milestone being submitted.
        #[arg(long)]
        milestone_id: u32,

        /// URI pointing to delivery proof (e.g. "ipfs://...", GitHub PR URL).
        /// Omit the flag to submit without a proof link.
        #[arg(long)]
        proof_uri: Option<String>,

        /// Skip the confirmation prompt (for scripting).
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Approve submitted work and release funds to the payee.
    ApproveRelease {
        /// Agreement ID (hex-encoded, 64 chars).
        #[arg(long)]
        agreement_id: String,

        /// Zero-based index of the milestone to approve.
        #[arg(long)]
        milestone_id: u32,

        /// Skip the confirmation prompt (for scripting).
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Raise a dispute on a funded or work-submitted milestone.
    RaiseDispute {
        /// Agreement ID (hex-encoded, 64 chars).
        #[arg(long)]
        agreement_id: String,

        /// Zero-based index of the disputed milestone.
        #[arg(long)]
        milestone_id: u32,

        /// Address of the party raising the dispute (payer or payee).
        /// The contract validates the caller is one of these two roles.
        #[arg(long)]
        caller: String,

        /// Skip the confirmation prompt (for scripting).
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Resolve a disputed milestone as the designated dispute resolver.
    ResolveDispute {
        /// Agreement ID (hex-encoded, 64 chars).
        #[arg(long)]
        agreement_id: String,

        /// Zero-based index of the disputed milestone.
        #[arg(long)]
        milestone_id: u32,

        /// Pass true to refund locked funds to the payer (payer wins).
        /// Pass false to release funds to the payee (payee wins).
        #[arg(long, default_value = "false")]
        refund_to_payer: bool,

        /// Skip the confirmation prompt (for scripting).
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Cancel a milestone that was never funded (status = Pending).
    CancelMilestone {
        /// Agreement ID (hex-encoded, 64 chars).
        #[arg(long)]
        agreement_id: String,

        /// Zero-based index of the milestone to cancel.
        #[arg(long)]
        milestone_id: u32,

        /// Skip the confirmation prompt (for scripting).
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Query the current state of an agreement.
    Status {
        /// Agreement ID (hex-encoded, 64 chars).
        #[arg(long)]
        agreement_id: String,
    },

    /// Query the current status of a single milestone (cheaper than fetching the full agreement).
    MilestoneStatus {
        /// Agreement ID (hex-encoded, 64 chars).
        #[arg(long)]
        agreement_id: String,

        /// Zero-based index of the milestone to query.
        #[arg(long)]
        milestone_id: u32,
    },

    /// Generate a shell completion script for bash, zsh, fish, elvish, or PowerShell.
    ///
    /// Example installation (bash):
    ///   trellis completion bash > /etc/bash_completion.d/trellis
    Completion {
        /// Target shell to generate a completion script for.
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Manage Stellar secret keys securely using OS keychain or encrypted keystore.
    #[command(subcommand)]
    Keys(KeysSubcommand),
}

#[derive(Subcommand, Debug)]
pub enum KeysSubcommand {
    /// Store a Stellar secret key in the OS keychain for a named identity.
    Add {
        /// Identity name (e.g., "alice", "bob").
        identity: String,

        /// Stellar secret key (S...).
        key: String,
    },

    /// Remove a Stellar secret key from the OS keychain.
    Remove {
        /// Identity name to remove.
        identity: String,
    },

    /// List all Stellar secret keys stored in the keychain.
    List,
}

/// Prompts the caller to confirm a state-changing action before it runs.
///
/// Skipped entirely for `--dry-run`, since nothing is actually executed.
/// Under `--quiet` there is no terminal to read a response from, so an
/// unconfirmed action fails closed rather than silently proceeding.
fn confirm_action(summary: &str, yes: bool, opts: &OutputOpts) -> Result<(), String> {
    if yes || opts.dry_run {
        return Ok(());
    }

    if opts.quiet {
        return Err(
            "Confirmation required: pass --yes to run this non-interactively.".to_string(),
        );
    }

    use std::io::Write;

    println!("{summary}");
    print!("Continue? [y/N] ");
    std::io::stdout()
        .flush()
        .map_err(|e| format!("Failed to write prompt: {e}"))?;

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed to read confirmation: {e}"))?;

    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => Ok(()),
        _ => Err("Aborted: operation not confirmed.".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Dispatch — route each command to its handler
// ---------------------------------------------------------------------------

pub fn dispatch(cmd: Commands, config: &Config, opts: &OutputOpts) -> Result<(), String> {
    match cmd {
        Commands::Init {
            agreement_id,
            payer,
            payee,
            token,
            resolver,
            milestones,
            yes,
        } => run_init(
            config,
            agreement_id,
            payer,
            payee,
            token,
            resolver,
            milestones,
            yes,
            opts,
        ),

        Commands::LockFunds {
            agreement_id,
            milestone_id,
            yes,
        } => run_lock_funds(config, agreement_id, milestone_id, yes, opts),

        Commands::SubmitWork {
            agreement_id,
            milestone_id,
            proof_uri,
            yes,
        } => run_submit_work(config, agreement_id, milestone_id, proof_uri, yes, opts),

        Commands::ApproveRelease {
            agreement_id,
            milestone_id,
            yes,
        } => run_approve_release(config, agreement_id, milestone_id, yes, opts),

        Commands::RaiseDispute {
            agreement_id,
            milestone_id,
            caller,
            yes,
        } => run_raise_dispute(config, agreement_id, milestone_id, caller, yes, opts),

        Commands::ResolveDispute {
            agreement_id,
            milestone_id,
            refund_to_payer,
            yes,
        } => run_resolve_dispute(
            config,
            agreement_id,
            milestone_id,
            refund_to_payer,
            yes,
            opts,
        ),

        Commands::CancelMilestone {
            agreement_id,
            milestone_id,
            yes,
        } => run_cancel_milestone(config, agreement_id, milestone_id, yes, opts),

        Commands::Status { agreement_id } => run_status(config, agreement_id, opts),

        Commands::MilestoneStatus {
            agreement_id,
            milestone_id,
        } => run_milestone_status(config, agreement_id, milestone_id, opts),

        // Handled in main() before dispatch is ever reached — completions
        // need the clap `Command` object, not a `Config`.
        Commands::Completion { .. } => Ok(()),

        Commands::Keys(subcmd) => run_keys(subcmd),
    }
}

fn run_keys(subcmd: KeysSubcommand) -> Result<(), String> {
    use crate::keystore::Keystore;

    let keystore = Keystore::new();

    match subcmd {
        KeysSubcommand::Add { identity, key } => {
            keystore
                .store_in_keychain(&identity, &key)
                .map_err(|e| format!("Failed to store key: {}", e))?;
            println!("✓ Key stored for identity '{}'", identity);
            Ok(())
        }
        KeysSubcommand::Remove { identity } => {
            keystore
                .remove_from_keychain(&identity)
                .map_err(|e| format!("Failed to remove key: {}", e))?;
            println!("✓ Key removed for identity '{}'", identity);
            Ok(())
        }
        KeysSubcommand::List => {
            let identities = keystore.list_keychain_identities();
            if identities.is_empty() {
                println!("No keys stored. Use 'trellis keys add <identity> <key>' to store one.");
            } else {
                println!("Stored identities:");
                for identity in identities {
                    println!("  - {}", identity);
                }
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Input validation
// ---------------------------------------------------------------------------

/// Validate an agreement ID: must be exactly 64 lowercase or uppercase hex chars.
///
/// Rejects any value that could be used to inject additional CLI flags or
/// smuggle shell metacharacters through the argument list.
fn validate_agreement_id(id: &str) -> Result<(), String> {
    crate::sanitizer::sanitize_hex_id(id)?;
    if crate::utils::is_valid_hex(id, 64) {
        return Ok(());
    }
    if id.len() != 64 {
        return Err(format!(
            "agreement_id must be exactly 64 hex characters, got {}",
            id.len()
        ));
    }
    Err("agreement_id must contain only hexadecimal characters (0-9, a-f, A-F)".to_string())
}

/// Validate a proof URI: printable, non-empty, within a reasonable length cap.
///
/// Control characters (including newlines) are rejected so they cannot be
/// used to confuse argument parsing downstream. Unicode is normalized to NFC.
fn validate_proof_uri(uri: &str) -> Result<(), String> {
    if uri.is_empty() {
        return Err("proof_uri must not be empty".to_string());
    }
    if uri.len() > 2048 {
        return Err(format!(
            "proof_uri must not exceed 2048 characters, got {}",
            uri.len()
        ));
    }
    crate::sanitizer::sanitize_proof_uri(uri)?;
    Ok(())
}

/// Validate a user-supplied Stellar address field (`payer`, `payee`, `token`,
/// `resolver`, `caller`).
///
/// Soroban `Address` values are strkey-encoded: exactly 56 characters, starting
/// with `G` (account) or `C` (contract), containing only RFC 4648 base32
/// characters (`A`–`Z`, `2`–`7`). Anything else — a stray space, a quote, an
/// embedded `--flag`, any shell metacharacter — fails this check, so no
/// user-supplied address can smuggle extra tokens into the argument list
/// forwarded to `stellar contract invoke`.
fn validate_address(field: &str, value: &str) -> Result<(), String> {
    if value.len() != 56 {
        return Err(format!(
            "{field} must be a 56-character Stellar address, got {} characters",
            value.len()
        ));
    }
    match value.chars().next() {
        Some('G') | Some('C') => {}
        _ => {
            return Err(format!(
                "{field} must be a Stellar address starting with 'G' (account) or 'C' (contract)"
            ));
        }
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_uppercase() || ('2'..='7').contains(&c))
    {
        return Err(format!(
            "{field} must contain only base32 characters (A-Z, 2-7); \
             whitespace, quotes and other shell metacharacters are not allowed"
        ));
    }
    Ok(())
}

/// Print a validation error and terminate the process.
///
/// Returns `!` so it can be used both as a statement and as the fallback in
/// `Result::unwrap_or_else` without a type mismatch.
fn fail_validation(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

// ---------------------------------------------------------------------------
// Active command implementations
// ---------------------------------------------------------------------------

/// `stellar contract invoke … -- init …`
///
/// Final call signature:
/// ```
/// stellar contract invoke --id <C> --source <key> --rpc-url <url>
///   --network-passphrase <p> -- init
///   --agreement-id <hex> --payer <G> --payee <G>
///   --token <C> --milestones <JSON> --dispute-resolver <G>
/// ```
#[allow(clippy::too_many_arguments)]
fn run_init(
    config: &Config,
    agreement_id: String,
    payer: String,
    payee: String,
    token: String,
    resolver: String,
    milestones_csv: String,
    yes: bool,
    opts: &OutputOpts,
) -> Result<(), String> {
    validate_agreement_id(&agreement_id).unwrap_or_else(|e| fail_validation(&e));
    validate_address("payer", &payer).unwrap_or_else(|e| fail_validation(&e));
    validate_address("payee", &payee).unwrap_or_else(|e| fail_validation(&e));
    validate_address("token", &token).unwrap_or_else(|e| fail_validation(&e));
    validate_address("resolver", &resolver).unwrap_or_else(|e| fail_validation(&e));

    let milestones_json = build_milestones_json(&milestones_csv).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });

    confirm_action(
        &format!(
            "This will create agreement {agreement_id} (payer={payer}, payee={payee}, \
             token={token}, resolver={resolver}, milestones={milestones_csv})."
        ),
        yes,
        opts,
    )?;

    let args = vec![
        "--agreement-id".to_string(),
        agreement_id,
        "--payer".to_string(),
        payer,
        "--payee".to_string(),
        payee,
        "--token".to_string(),
        token,
        "--milestones".to_string(),
        milestones_json,
        "--dispute-resolver".to_string(),
        resolver,
    ];

    execute(config, "init", &args, opts)
}

/// `stellar contract invoke … -- lock_funds …`
///
/// Final call signature:
/// ```
/// stellar contract invoke … -- lock_funds
///   --agreement-id <hex> --milestone-id <u32>
/// ```
fn run_lock_funds(
    config: &Config,
    agreement_id: String,
    milestone_id: u32,
    yes: bool,
    opts: &OutputOpts,
) -> Result<(), String> {
    validate_agreement_id(&agreement_id).unwrap_or_else(|e| fail_validation(&e));

    confirm_action(
        &format!("This will lock funds for milestone {milestone_id} of agreement {agreement_id}."),
        yes,
        opts,
    )?;

    let args = vec![
        "--agreement-id".to_string(),
        agreement_id,
        "--milestone-id".to_string(),
        milestone_id.to_string(),
    ];

    execute(config, "lock_funds", &args, opts)
}

/// `stellar contract invoke … -- submit_work …`
///
/// Final call signature:
/// ```
/// stellar contract invoke … -- submit_work
///   --agreement-id <hex> --milestone-id <u32> [--proof-uri <string>]
/// ```
///
/// The contract types `proof_uri` as `Option<String>`, so omitting the flag
/// sends `None` — the canonical "no proof submitted" value. Passing an empty
/// string would create a `Some("")`, which the contract does not treat as
/// absent, so the flag is dropped entirely rather than sent empty.
fn run_submit_work(
    config: &Config,
    agreement_id: String,
    milestone_id: u32,
    proof_uri: Option<String>,
    yes: bool,
    opts: &OutputOpts,
) -> Result<(), String> {
    validate_agreement_id(&agreement_id).unwrap_or_else(|e| fail_validation(&e));

    confirm_action(
        &format!("This will submit work for milestone {milestone_id} of agreement {agreement_id}."),
        yes,
        opts,
    )?;

    let mut args = vec![
        "--agreement-id".to_string(),
        agreement_id,
        "--milestone-id".to_string(),
        milestone_id.to_string(),
    ];

    if let Some(uri) = proof_uri.filter(|u| !u.is_empty()) {
        validate_proof_uri(&uri)?;
        args.push("--proof-uri".to_string());
        // `stellar contract invoke` deserializes every non-Bytes/BytesN arg as
        // JSON, so a bare string value fails to parse — it must be JSON-quoted.
        args.push(serde_json::to_string(&uri).unwrap_or(uri));
    }

    execute(config, "submit_work", &args, opts)
}

/// `stellar contract invoke … -- approve_and_release …`
///
/// Final call signature:
/// ```
/// stellar contract invoke … -- approve_and_release
///   --agreement-id <hex> --milestone-id <u32>
/// ```
fn run_approve_release(
    config: &Config,
    agreement_id: String,
    milestone_id: u32,
    yes: bool,
    opts: &OutputOpts,
) -> Result<(), String> {
    validate_agreement_id(&agreement_id).unwrap_or_else(|e| fail_validation(&e));

    confirm_action(
        &format!(
            "This will approve milestone {milestone_id} of agreement {agreement_id} and release funds to the payee."
        ),
        yes,
        opts,
    )?;

    let args = vec![
        "--agreement-id".to_string(),
        agreement_id,
        "--milestone-id".to_string(),
        milestone_id.to_string(),
    ];

    execute(config, "approve_and_release", &args, opts)
}

/// `stellar contract invoke … -- raise_dispute …`
///
/// Final call signature:
/// ```
/// stellar contract invoke … -- raise_dispute
///   --agreement-id <hex> --milestone-id <u32> --caller <G>
/// ```
///
/// `caller` is passed explicitly because the contract checks it against
/// both `agreement.payer` and `agreement.payee` before calling
/// `caller.require_auth()`, so either party can autonomously open a dispute.
fn run_raise_dispute(
    config: &Config,
    agreement_id: String,
    milestone_id: u32,
    caller: String,
    yes: bool,
    opts: &OutputOpts,
) -> Result<(), String> {
    validate_agreement_id(&agreement_id).unwrap_or_else(|e| fail_validation(&e));
    validate_address("caller", &caller).unwrap_or_else(|e| fail_validation(&e));

    confirm_action(
        &format!("This will raise a dispute on milestone {milestone_id} of agreement {agreement_id}."),
        yes,
        opts,
    )?;

    let args = vec![
        "--agreement-id".to_string(),
        agreement_id,
        "--milestone-id".to_string(),
        milestone_id.to_string(),
        "--caller".to_string(),
        caller,
    ];

    execute(config, "raise_dispute", &args, opts)
}

/// `stellar contract invoke … -- resolve_dispute …`
///
/// Final call signature:
/// ```
/// stellar contract invoke … -- resolve_dispute
///   --agreement-id <hex> --milestone-id <u32> --refund-to-payer <true|false>
/// ```
fn run_resolve_dispute(
    config: &Config,
    agreement_id: String,
    milestone_id: u32,
    refund_to_payer: bool,
    yes: bool,
    opts: &OutputOpts,
) -> Result<(), String> {
    validate_agreement_id(&agreement_id).unwrap_or_else(|e| fail_validation(&e));

    let outcome = if refund_to_payer {
        "refund locked funds to the payer"
    } else {
        "release funds to the payee"
    };
    confirm_action(
        &format!(
            "This will resolve the dispute on milestone {milestone_id} of agreement {agreement_id} and {outcome}."
        ),
        yes,
        opts,
    )?;

    let args = vec![
        "--agreement-id".to_string(),
        agreement_id,
        "--milestone-id".to_string(),
        milestone_id.to_string(),
        "--refund-to-payer".to_string(),
        refund_to_payer.to_string(),
    ];

    execute(config, "resolve_dispute", &args, opts)
}

/// `stellar contract invoke … -- cancel_unfunded_milestone …`
///
/// Final call signature:
/// ```
/// stellar contract invoke … -- cancel_unfunded_milestone
///   --agreement-id <hex> --milestone-id <u32>
/// ```
fn run_cancel_milestone(
    config: &Config,
    agreement_id: String,
    milestone_id: u32,
    yes: bool,
    opts: &OutputOpts,
) -> Result<(), String> {
    validate_agreement_id(&agreement_id).unwrap_or_else(|e| fail_validation(&e));

    confirm_action(
        &format!("This will cancel milestone {milestone_id} of agreement {agreement_id}."),
        yes,
        opts,
    )?;

    let args = vec![
        "--agreement-id".to_string(),
        agreement_id,
        "--milestone-id".to_string(),
        milestone_id.to_string(),
    ];

    execute(config, "cancel_unfunded_milestone", &args, opts)
}

/// `stellar contract invoke … -- get_agreement …`
///
/// Final call signature:
/// ```
/// stellar contract invoke … -- get_agreement
///   --agreement-id <hex>
/// ```
///
/// The stellar CLI calls the contract's `get_agreement` view function and
/// returns the full Agreement struct as JSON, which is printed to stdout.
fn run_status(config: &Config, agreement_id: String, opts: &OutputOpts) -> Result<(), String> {
    validate_agreement_id(&agreement_id).unwrap_or_else(|e| fail_validation(&e));

    let args = vec!["--agreement-id".to_string(), agreement_id];

    execute(config, "get_agreement", &args, opts)
}

/// `stellar contract invoke … -- get_milestone …`
///
/// Final call signature:
/// ```
/// stellar contract invoke … -- get_milestone
///   --agreement-id <hex> --milestone-id <u32>
/// ```
///
/// Queries a single milestone by index without fetching the full Agreement,
/// reducing deserialization cost for agreements with many milestones.
fn run_milestone_status(
    config: &Config,
    agreement_id: String,
    milestone_id: u32,
    opts: &OutputOpts,
) -> Result<(), String> {
    validate_agreement_id(&agreement_id).unwrap_or_else(|e| fail_validation(&e));

    let args = vec![
        "--agreement-id".to_string(),
        agreement_id,
        "--milestone-id".to_string(),
        milestone_id.to_string(),
    ];

    execute(config, "get_milestone", &args, opts)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Convert a comma-separated amount string like `"1000,2000"` into the JSON
/// array format the `stellar` CLI accepts for a `Vec<Milestone>` argument.
///
/// ## Accepted CSV format
///
/// - One or more amounts separated by a single `,`.
/// - Surrounding whitespace around each amount is trimmed, so
///   `" 100 , 200 "` and `"100,200"` are equivalent.
/// - Every field between commas must be a non-empty, positive `i128`.
///
/// ## Rejected (whole command fails with a descriptive error)
///
/// - Empty / whitespace-only input.
/// - A leading comma (`",1000"`), trailing comma (`"1000,"`), or doubled
///   comma (`"1000,,2000"`) — each leaves an empty field. These are treated
///   as user typos rather than silently dropped, so a mistyped list can never
///   quietly create fewer milestones than intended (#238).
/// - Any field that is not a valid integer, is zero, or is negative.
///
/// Amounts are parsed as `i128` to match the contract's `Milestone.amount` type.
/// Values that are not valid integers, are zero, or are negative are rejected
/// with a descriptive error — the entire command fails rather than silently
/// producing a malformed milestone list.
///
/// Each milestone is given:
/// - `id`        – its 0-based position in the list
/// - `amount`    – the parsed `i128` amount (quoted, per Soroban i128 JSON encoding)
/// - `status`    – `{"Pending":null}` (XDR union tag for EscrowStatus::Pending)
/// - `proof_uri` – `null` (XDR `Void`, i.e. `None` — no proof submitted yet)
///
/// Example output for `"1000,2000"`:
/// ```json
/// [{"id":0,"amount":"1000","status":{"Pending":null},"proof_uri":null},
///  {"id":1,"amount":"2000","status":{"Pending":null},"proof_uri":null}]
/// ```
fn build_milestones_json(csv: &str) -> Result<String, String> {
    if csv.trim().is_empty() {
        return Err(
            "no milestone amounts provided — pass a comma-separated list of positive \
             integers in the token's base unit, e.g. --milestones \"1000,2000,500\""
                .to_string(),
        );
    }

    let entries: Vec<String> = csv
        .split(',')
        .enumerate()
        .map(|(idx, part)| -> Result<String, String> {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                return Err(format!(
                    "empty milestone amount at index {idx} — remove the leading, trailing, \
                     or doubled comma in \"{csv}\" (expected e.g. \"1000,2000,500\")"
                ));
            }
            let amount: i128 = trimmed.parse().map_err(|_| {
                format!(
                    "invalid milestone amount {:?} at index {} — expected a positive integer",
                    trimmed, idx
                )
            })?;
            if amount <= 0 {
                return Err(format!(
                    "milestone amount at index {} must be a positive integer, got {amount}",
                    idx
                ));
            }
            Ok(format!(
                r#"{{"id":{idx},"amount":"{amount}","status":{{"Pending":null}},"proof_uri":null}}"#,
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(format!("[{}]", entries.join(",")))
}

/// Run an RPC invocation (or preview it, under `--dry-run`) and render the
/// result according to `opts.format`.
///
/// This is the single entry point every command handler funnels through, so
/// `--dry-run`, `--json`, `--human-readable`, and `--quiet` behave
/// consistently across all commands (#74, #76, #77).
///
/// Under `--dry-run` the preview command string is wrapped in a synthetic
/// `InvokeOutput` and passed through the same `render_output` path as a
/// real result, so `--dry-run --json` produces a valid JSON envelope,
/// `--dry-run --quiet` is silent, and `--dry-run --human-readable` prints a
/// formatted preview — matching the documented guarantee above.
fn execute(
    config: &Config,
    fn_name: &str,
    args: &[String],
    opts: &OutputOpts,
) -> Result<(), String> {
    if opts.dry_run {
        let preview = RpcClient::preview(config, fn_name, args);
        let out = InvokeOutput {
            stdout: preview.clone(),
            stderr: String::new(),
            success: true,
            command_debug: preview,
        };
        return render_output(&out, opts);
    }

    let out = RpcClient::invoke(config, fn_name, args, opts.quiet);
    render_output(&out, opts)
}

/// Dispatch to the renderer selected by `opts.format`.
fn render_output(out: &InvokeOutput, opts: &OutputOpts) -> Result<(), String> {
    match opts.format {
        OutputFormat::Json => render_json(out),
        OutputFormat::Human => render_human(out),
        OutputFormat::Raw => render_raw(out),
    }
}

/// Default renderer: print the raw command output verbatim (original behavior).
///
/// On failure, prints the full verbatim command so the user can reproduce it.
/// Returns `Ok(())` on success or `Err(message)` on failure so that callers
/// (i.e. `main`) can run any cleanup before exiting with a non-zero exit code.
/// This avoids calling `std::process::exit` inside a library function, which
/// would skip destructors and flush buffers unsafely.
fn render_raw(out: &InvokeOutput) -> Result<(), String> {
    if out.success {
        println!("{}", out.stdout.trim());
        Ok(())
    } else {
        let mut msg = format!(
            "── Transaction failed ──────────────────────────────────\nCommand: {}",
            out.command_debug
        );
        if !out.stdout.is_empty() {
            msg.push_str(&format!("\nstdout:\n{}", out.stdout.trim()));
        }
        if !out.stderr.is_empty() {
            msg.push_str(&format!("\nstderr:\n{}", out.stderr.trim()));
        }
        Err(msg)
    }
}

/// `--json` renderer (#77): a single line of uniform, machine-parseable JSON.
///
/// Schema: `{"status": "success"|"error", "result", "tx_hash", "events", "error"}`.
/// `result` holds the parsed stdout payload (or the raw string if it isn't
/// valid JSON). `tx_hash`/`events` are extracted on a best-effort basis since
/// the underlying `stellar contract invoke` shell-out does not expose a
/// structured transaction envelope.
fn render_json(out: &InvokeOutput) -> Result<(), String> {
    let envelope = json_envelope(out);
    println!("{}", serde_json::to_string(&envelope).unwrap_or_default());

    if out.success {
        Ok(())
    } else {
        // Error detail is already in the JSON envelope above; returning an
        // empty message tells main() to exit(1) without printing it again.
        Err(String::new())
    }
}

fn json_envelope(out: &InvokeOutput) -> serde_json::Value {
    let trimmed_stdout = out.stdout.trim();

    if out.success {
        let result = serde_json::from_str::<serde_json::Value>(trimmed_stdout)
            .unwrap_or_else(|_| serde_json::Value::String(trimmed_stdout.to_string()));

        serde_json::json!({
            "status": "success",
            "result": result,
            "tx_hash": extract_tx_hash(&out.stdout, &out.stderr),
            "events": extract_events(&out.stderr),
            "error": null,
        })
    } else {
        let mut error = out.stderr.trim().to_string();
        if error.is_empty() {
            error = trimmed_stdout.to_string();
        }

        serde_json::json!({
            "status": "error",
            "result": null,
            "tx_hash": null,
            "events": null,
            "error": error,
        })
    }
}

/// `--human-readable` / `-H` renderer (#74): parse the stellar CLI JSON output
/// and print a colorized, formatted summary. Falls back to raw text if the
/// output isn't valid JSON.
///
/// Colors are only emitted when `colors_enabled()` returns `true`; otherwise
/// all ANSI sequences are replaced with empty strings so that piped / file
/// output is clean plain text.
fn render_human(out: &InvokeOutput) -> Result<(), String> {
    render_human_to(out, colors_enabled(), &mut std::io::stdout())
}

/// Inner implementation of `render_human` that writes to an arbitrary `Write`
/// sink and accepts an explicit `use_color` flag. Extracted so that unit tests
/// can capture output and control color behavior without touching env vars or
/// real stdout.
fn render_human_to(
    out: &InvokeOutput,
    use_color: bool,
    writer: &mut dyn std::io::Write,
) -> Result<(), String> {
    let trimmed = out.stdout.trim();

    // Resolve color codes once per call so every format site is consistent.
    let (green, red, bold, reset) = if use_color {
        (ANSI_GREEN, ANSI_RED, ANSI_BOLD, ANSI_RESET)
    } else {
        ("", "", "", "")
    };

    if out.success {
        writeln!(writer, "{green}{bold}\u{2714} Success{reset}").ok();
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(serde_json::Value::Object(map)) if !map.is_empty() => {
                for (key, value) in map {
                    writeln!(writer, "  {bold}{key}{reset}: {}", format_json_value(&value)).ok();
                }
            }
            Ok(other) if !trimmed.is_empty() => {
                writeln!(writer, "  {}", format_json_value(&other)).ok();
            }
            _ if !trimmed.is_empty() => {
                writeln!(writer, "  {trimmed}").ok();
            }
            _ => {}
        }

        if let serde_json::Value::Array(events) = extract_events(&out.stderr) {
            writeln!(writer, "  {bold}events{reset}:").ok();
            for event in events {
                writeln!(writer, "    - {}", format_json_value(&event)).ok();
            }
        }
        Ok(())
    } else {
        writeln!(writer, "{red}{bold}\u{2718} Failed{reset}").ok();
        writeln!(writer, "  {bold}command{reset}: {}", out.command_debug).ok();
        if !trimmed.is_empty() {
            writeln!(writer, "  {bold}stdout{reset}: {trimmed}").ok();
        }
        if !out.stderr.trim().is_empty() {
            writeln!(writer, "  {bold}stderr{reset}: {}", out.stderr.trim()).ok();
        }
        Err(String::new())
    }
}

fn format_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Best-effort extraction of a transaction hash from stdout/stderr: a
/// standalone 64-character hex token. The stellar CLI shell-out does not
/// expose a structured tx envelope, so this scans text output; returns
/// `null` when nothing hex-shaped of the right length is found.
fn extract_tx_hash(stdout: &str, stderr: &str) -> Option<String> {
    for text in [stdout, stderr] {
        // First try to find a hash with explicit hash-related prefix
        if let Some(hash) = extract_with_prefix(text) {
            return Some(hash);
        }
    }
    None
}

/// Extract a 64-hex-char value that follows hash-related keywords or patterns.
/// This prevents false positives from arbitrary 64-char hex strings in contract responses.
fn extract_with_prefix(text: &str) -> Option<String> {
    // Pattern: look for "tx_hash" or "hash" or similar, followed by : or =,
    // then capture the next 64-char hex token
    let hash_patterns = ["tx_hash", "tx_id", "transaction", "hash", "x_hash"];

    for pattern in &hash_patterns {
        // Case 1: pattern: followed by quoted value
        for prefix in &[": \"", ":\"", ": ", "="] {
            if let Some(pos) = text.find(&format!("{pattern}{prefix}")) {
                let start = pos + pattern.len() + prefix.len();
                if start < text.len() {
                    let rest = &text[start..];
                    for token in rest.split(|c: char| c.is_whitespace() || c == '"' || c == ',' || c == '}') {
                        if token.len() == 64 && token.chars().all(|c| c.is_ascii_hexdigit()) {
                            return Some(token.to_lowercase());
                        }
                    }
                }
            }
        }
    }

    None
}

/// Best-effort extraction of event diagnostics from stderr: any line
/// mentioning "event" (case-insensitive). Returns `null` when none are found.
fn extract_events(stderr: &str) -> serde_json::Value {
    let events: Vec<serde_json::Value> = stderr
        .lines()
        .filter(|line| line.to_lowercase().contains("event"))
        .map(|line| serde_json::Value::String(line.trim().to_string()))
        .collect();

    if events.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::Array(events)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- build_milestones_json ---

    #[test]
    fn test_build_milestones_json_happy_path() {
        let json = build_milestones_json("1000,2000,500").unwrap();
        assert_eq!(
            json,
            r#"[{"id":0,"amount":"1000","status":{"Pending":null},"proof_uri":null},{"id":1,"amount":"2000","status":{"Pending":null},"proof_uri":null},{"id":2,"amount":"500","status":{"Pending":null},"proof_uri":null}]"#
        );
    }

    #[test]
    fn test_build_milestones_json_max_i128() {
        let max = i128::MAX.to_string();
        let json = build_milestones_json(&max).unwrap();
        assert!(
            json.contains(&format!("\"amount\":\"{}\"", i128::MAX)),
            "max i128 value should be preserved verbatim"
        );
    }

    #[test]
    fn test_build_milestones_json_zero_rejects() {
        let result = build_milestones_json("0");
        assert!(result.is_err(), "zero amount must be rejected");
    }

    #[test]
    fn test_build_milestones_json_negative_rejects() {
        let result = build_milestones_json("-500");
        assert!(result.is_err(), "negative amount must be rejected");
    }

    #[test]
    fn test_build_milestones_json_non_numeric_rejects() {
        let result = build_milestones_json("abc");
        assert!(result.is_err(), "non-numeric input must be rejected");
    }

    #[test]
    fn test_build_milestones_json_whitespace_trimmed() {
        let json = build_milestones_json(" 100 , 200 ").unwrap();
        assert!(json.contains("\"amount\":\"100\""));
        assert!(json.contains("\"amount\":\"200\""));
    }

    #[test]
    fn test_build_milestones_json_single_milestone() {
        let json = build_milestones_json("42").unwrap();
        assert_eq!(
            json,
            r#"[{"id":0,"amount":"42","status":{"Pending":null},"proof_uri":null}]"#
        );
    }

    // --- build_milestones_json: empty / malformed input (#238) ---

    #[test]
    fn test_build_milestones_json_empty_string_rejected() {
        let err = build_milestones_json("").unwrap_err();
        assert!(
            err.contains("no milestone amounts") && err.contains("1000,2000,500"),
            "empty input should give a clear error with a format example, got: {err}"
        );
    }

    #[test]
    fn test_build_milestones_json_whitespace_only_rejected() {
        let err = build_milestones_json("   ").unwrap_err();
        assert!(err.contains("no milestone amounts"), "got: {err}");
    }

    #[test]
    fn test_build_milestones_json_trailing_comma_rejected() {
        let err = build_milestones_json("1000,2000,").unwrap_err();
        assert!(
            err.contains("empty milestone amount at index 2"),
            "trailing comma should be flagged clearly, got: {err}"
        );
    }

    #[test]
    fn test_build_milestones_json_leading_comma_rejected() {
        let err = build_milestones_json(",1000,2000").unwrap_err();
        assert!(
            err.contains("empty milestone amount at index 0"),
            "got: {err}"
        );
    }

    #[test]
    fn test_build_milestones_json_doubled_comma_rejected() {
        let err = build_milestones_json("1000,,2000").unwrap_err();
        assert!(
            err.contains("empty milestone amount at index 1"),
            "got: {err}"
        );
    }

    // --- build_milestones_json: comma + whitespace edge cases (#248) ---

    #[test]
    fn test_build_milestones_json_trailing_comma_with_space_rejected() {
        // A trailing ", " (comma then whitespace) still produces an empty field.
        let err = build_milestones_json("1000,2000, ").unwrap_err();
        assert!(
            err.contains("empty milestone amount at index 2"),
            "got: {err}"
        );
    }

    #[test]
    fn test_build_milestones_json_leading_comma_with_space_rejected() {
        let err = build_milestones_json(" ,1000,2000").unwrap_err();
        assert!(
            err.contains("empty milestone amount at index 0"),
            "got: {err}"
        );
    }

    #[test]
    fn test_build_milestones_json_whitespace_only_field_rejected() {
        // A field that is nothing but spaces/tabs between two commas.
        let err = build_milestones_json("1000, \t ,2000").unwrap_err();
        assert!(
            err.contains("empty milestone amount at index 1"),
            "got: {err}"
        );
    }

    #[test]
    fn test_build_milestones_json_multiple_trailing_commas_rejected() {
        let err = build_milestones_json("1000,2000,,").unwrap_err();
        assert!(
            err.contains("empty milestone amount at index 2"),
            "got: {err}"
        );
    }

    #[test]
    fn test_build_milestones_json_mixed_tabs_and_newlines_trimmed() {
        // Mixed whitespace (tabs, newlines, spaces) around otherwise valid
        // amounts is tolerated and stripped.
        let json = build_milestones_json("\t1000 ,\n 2000\t,  500\n").unwrap();
        assert_eq!(
            json,
            r#"[{"id":0,"amount":"1000","status":{"Pending":null},"proof_uri":null},{"id":1,"amount":"2000","status":{"Pending":null},"proof_uri":null},{"id":2,"amount":"500","status":{"Pending":null},"proof_uri":null}]"#
        );
    }

    #[test]
    fn test_build_milestones_json_only_commas_rejected() {
        let err = build_milestones_json(",,").unwrap_err();
        assert!(
            err.contains("empty milestone amount at index 0"),
            "got: {err}"
        );
    }

    // --- validate_address: CLI argument injection prevention (#239) ---

    fn valid_g_addr() -> String {
        format!("G{}", "A".repeat(55))
    }

    #[test]
    fn address_accepts_well_formed_g_and_c() {
        assert!(validate_address("payer", &valid_g_addr()).is_ok());
        assert!(validate_address("token", &format!("C{}", "A".repeat(55))).is_ok());
        // A mixed-alphabet 56-char strkey (G + 55 base32 chars).
        let realistic: String = std::iter::once('G')
            .chain("BCDEFGHIJKLMNOPQRSTUVWXYZ234567".chars().cycle().take(55))
            .collect();
        assert_eq!(realistic.len(), 56);
        assert!(validate_address("payee", &realistic).is_ok());
    }

    #[test]
    fn address_rejects_wrong_length() {
        assert!(validate_address("payer", "GABC").is_err());
        assert!(validate_address("payer", &format!("G{}", "A".repeat(60))).is_err());
    }

    #[test]
    fn address_rejects_wrong_prefix() {
        assert!(validate_address("payer", &format!("S{}", "A".repeat(55))).is_err());
        assert!(validate_address("payer", &format!("X{}", "A".repeat(55))).is_err());
    }

    #[test]
    fn address_rejects_injected_flag_via_spaces() {
        // Exactly 56 chars but with an embedded space — a space is the vector
        // for smuggling an extra "--flag value" token into the argument list.
        let spaced = format!("G{} {}", "A".repeat(27), "A".repeat(27));
        assert_eq!(spaced.len(), 56);
        assert!(validate_address("payer", &spaced).is_err());

        // Longer overt injection payloads are rejected too.
        assert!(validate_address("payer", "GAAAA --network mainnet --source attacker").is_err());
        assert!(validate_address("payer", &format!("{} --help", "A".repeat(56))).is_err());
    }

    #[test]
    fn address_rejects_quotes_and_shell_metacharacters() {
        for bad in [
            format!("G{}\"{}", "A".repeat(27), "A".repeat(27)),
            format!("G{};rm -rf {}", "A".repeat(20), "A".repeat(25)),
            format!("G{}$(id){}", "A".repeat(24), "A".repeat(26)),
            format!("G{}`id`{}", "A".repeat(25), "A".repeat(26)),
        ] {
            assert!(
                validate_address("payer", &bad).is_err(),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn address_rejects_lowercase_and_padding_chars() {
        assert!(validate_address("payer", &format!("G{}", "a".repeat(55))).is_err());
        // '0', '1', '8', '9' and '=' are not in the RFC 4648 base32 alphabet
        assert!(validate_address("payer", &format!("G{}0", "A".repeat(54))).is_err());
        assert!(validate_address("payer", &format!("G{}=", "A".repeat(54))).is_err());
    }

    // --- validate_agreement_id ---

    #[test]
    fn agreement_id_valid_lowercase_hex() {
        let id = "a".repeat(64);
        assert!(validate_agreement_id(&id).is_ok());
    }

    #[test]
    fn agreement_id_valid_uppercase_hex() {
        let id = "F".repeat(64);
        assert!(validate_agreement_id(&id).is_ok());
    }

    #[test]
    fn agreement_id_valid_mixed_hex() {
        let id = "0123456789abcdefABCDEF0123456789abcdefABCDEF0123456789abcdefABCD";
        assert_eq!(id.len(), 64);
        assert!(validate_agreement_id(id).is_ok());
    }

    #[test]
    fn agreement_id_rejects_wrong_length() {
        assert!(validate_agreement_id("abc").is_err());
        assert!(validate_agreement_id(&"a".repeat(63)).is_err());
        assert!(validate_agreement_id(&"a".repeat(65)).is_err());
    }

    #[test]
    fn agreement_id_rejects_non_hex_chars() {
        // space injection attempt
        let id = format!("{} --extra-flag x {}", "a".repeat(30), "b".repeat(30));
        assert!(validate_agreement_id(&id).is_err());
    }

    #[test]
    fn agreement_id_rejects_quotes_and_backslash() {
        let id = format!("{}\"{}\\{}", "a".repeat(21), "b".repeat(21), "c".repeat(20));
        assert!(validate_agreement_id(&id).is_err());
    }

    #[test]
    fn agreement_id_rejects_null_byte() {
        let mut id = "a".repeat(64);
        // Replace one char with null byte representation
        id = id.replacen('a', "\0", 1);
        assert!(validate_agreement_id(&id).is_err());
    }

    // --- validate_proof_uri ---

    #[test]
    fn proof_uri_valid_ipfs() {
        assert!(
            validate_proof_uri("ipfs://QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco").is_ok()
        );
    }

    #[test]
    fn proof_uri_valid_https() {
        assert!(validate_proof_uri("https://github.com/org/repo/pull/42").is_ok());
    }

    #[test]
    fn proof_uri_rejects_empty() {
        assert!(validate_proof_uri("").is_err());
    }

    #[test]
    fn proof_uri_rejects_control_characters() {
        assert!(validate_proof_uri("https://example.com/\nX-Injected: bad").is_err());
        assert!(validate_proof_uri("https://example.com/\x00null").is_err());
        assert!(validate_proof_uri("https://example.com/\t tab").is_err());
    }

    #[test]
    fn proof_uri_rejects_oversized() {
        let uri = "a".repeat(2049);
        assert!(validate_proof_uri(&uri).is_err());
    }

    #[test]
    fn proof_uri_accepts_max_length() {
        let uri = "a".repeat(2048);
        assert!(validate_proof_uri(&uri).is_ok());
    }

    // --- extract_tx_hash / extract_events ---

    #[test]
    fn extract_tx_hash_finds_with_prefix() {
        let hash = "a".repeat(64);
        let text = format!("tx_hash: {hash}");
        assert_eq!(extract_tx_hash(&text, ""), Some(hash));
    }

    #[test]
    fn extract_tx_hash_finds_with_quoted_prefix() {
        let hash = "a".repeat(64);
        let text = format!(r#"tx_hash: "{hash}""#);
        assert_eq!(extract_tx_hash(&text, ""), Some(hash));
    }

    #[test]
    fn extract_tx_hash_ignores_standalone_hex() {
        let hash = "a".repeat(64);
        let text = format!("some milestone amount {hash} in response");
        assert_eq!(extract_tx_hash(&text, ""), None, "should not match arbitrary 64-char hex");
    }

    #[test]
    fn extract_tx_hash_none_when_absent() {
        assert_eq!(extract_tx_hash("no hash here", ""), None);
    }

    #[test]
    fn extract_tx_hash_false_positive_contract_response() {
        let hex_amount = "b".repeat(64);
        let json = format!(r#"{{"milestone_amount": "{hex_amount}", "status": "pending"}}"#);
        assert_eq!(extract_tx_hash(&json, ""), None, "should not match hex in JSON fields");
    }

    #[test]
    fn extract_tx_hash_false_positive_random_hex() {
        let random_hex = "c".repeat(64);
        assert_eq!(extract_tx_hash(&random_hex, ""), None, "should not match standalone hex");
    }

    #[test]
    fn extract_tx_hash_with_hash_equals() {
        let hash = "d".repeat(64);
        let text = format!("hash={hash} submitted");
        assert_eq!(extract_tx_hash(&text, ""), Some(hash));
    }

    #[test]
    fn extract_events_none_when_absent() {
        assert_eq!(extract_events("plain log line"), serde_json::Value::Null);
    }

    #[test]
    fn extract_events_collects_matching_lines() {
        let stderr = "info: starting\nEvent: transfer occurred\ndone";
        let events = extract_events(stderr);
        assert!(matches!(events, serde_json::Value::Array(ref a) if a.len() == 1));
    }

    // --- output rendering functions (#241) --------------------------------
    //
    // render_raw / render_json / render_human all print to stdout and signal
    // success/failure through their `Result`. These tests pin the return
    // contract (what `main` relies on to set the exit code and decide whether
    // to print a message) and the structure of the JSON envelope.

    fn ok_output(stdout: &str) -> InvokeOutput {
        InvokeOutput {
            stdout: stdout.to_string(),
            stderr: String::new(),
            success: true,
            command_debug: "stellar contract invoke ...".to_string(),
        }
    }

    fn fail_output(stdout: &str, stderr: &str) -> InvokeOutput {
        InvokeOutput {
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            success: false,
            command_debug: "stellar contract invoke --id CAABC -- boom".to_string(),
        }
    }

    // --- json_envelope ---

    #[test]
    fn json_envelope_success_parses_json_stdout() {
        let env = json_envelope(&ok_output(r#"{"balance":"100"}"#));
        assert_eq!(env["status"], "success");
        assert_eq!(env["result"]["balance"], "100");
        assert_eq!(env["error"], serde_json::Value::Null);
    }

    #[test]
    fn json_envelope_success_wraps_non_json_stdout_as_string() {
        let env = json_envelope(&ok_output("plain text result"));
        assert_eq!(env["status"], "success");
        assert_eq!(env["result"], "plain text result");
    }

    #[test]
    fn json_envelope_error_uses_stderr_as_message() {
        let env = json_envelope(&fail_output("", "error: contract not found"));
        assert_eq!(env["status"], "error");
        assert_eq!(env["result"], serde_json::Value::Null);
        assert_eq!(env["error"], "error: contract not found");
    }

    #[test]
    fn json_envelope_error_falls_back_to_stdout_when_stderr_empty() {
        let env = json_envelope(&fail_output("stdout failure detail", ""));
        assert_eq!(env["status"], "error");
        assert_eq!(env["error"], "stdout failure detail");
    }

    #[test]
    fn json_envelope_success_extracts_tx_hash_and_events() {
        let hash = "a".repeat(64);
        let mut out = ok_output("{}");
        out.stderr = format!("submitted {hash}\nevent: Transfer");
        let env = json_envelope(&out);
        assert_eq!(env["tx_hash"], hash);
        assert!(matches!(env["events"], serde_json::Value::Array(ref a) if a.len() == 1));
    }

    // --- confirm_action (#409) ---
    //
    // These tests pin the gate behaviour that was missing from four of the
    // seven state-mutating commands before issue #409 was fixed:
    //   run_lock_funds, run_approve_release, run_raise_dispute,
    //   run_cancel_milestone.
    //
    // The tests exercise confirm_action directly because the run_* functions
    // shell out to the stellar binary (unavailable in unit-test context).
    // Integration tests for the full --yes / --quiet flow live in
    // tests/cli_integration.rs.

    fn non_interactive_opts() -> OutputOpts {
        OutputOpts {
            format: OutputFormat::Raw,
            quiet: false,
            dry_run: false,
        }
    }

    fn quiet_opts() -> OutputOpts {
        OutputOpts {
            format: OutputFormat::Json,
            quiet: true,
            dry_run: false,
        }
    }

    fn dry_run_confirm_opts() -> OutputOpts {
        OutputOpts {
            format: OutputFormat::Raw,
            quiet: false,
            dry_run: true,
        }
    }

    /// --yes bypasses the prompt unconditionally; confirm_action must return Ok.
    /// Covers the skip-confirm path for all four newly-gated commands.
    #[test]
    fn confirm_action_yes_flag_bypasses_prompt() {
        let opts = non_interactive_opts();
        assert!(
            confirm_action("This will lock funds for milestone 0 of agreement abc.", true, &opts)
                .is_ok(),
            "lock_funds: --yes should bypass prompt"
        );
        assert!(
            confirm_action(
                "This will approve milestone 0 of agreement abc and release funds to the payee.",
                true,
                &opts,
            )
            .is_ok(),
            "approve_release: --yes should bypass prompt"
        );
        assert!(
            confirm_action(
                "This will raise a dispute on milestone 0 of agreement abc.",
                true,
                &opts,
            )
            .is_ok(),
            "raise_dispute: --yes should bypass prompt"
        );
        assert!(
            confirm_action(
                "This will cancel milestone 0 of agreement abc.",
                true,
                &opts,
            )
            .is_ok(),
            "cancel_milestone: --yes should bypass prompt"
        );
    }

    /// --quiet without --yes must return an Err directing the caller to use
    /// --yes. This prevents non-interactive scripts from hanging on stdin.
    #[test]
    fn confirm_action_quiet_without_yes_returns_err() {
        let opts = quiet_opts();
        let err = confirm_action(
            "This will lock funds for milestone 0 of agreement abc.",
            false,
            &opts,
        )
        .unwrap_err();
        assert!(
            err.contains("--yes"),
            "error should mention --yes flag, got: {err:?}"
        );
    }

    /// --dry-run bypasses the prompt regardless of --yes, matching the
    /// documented guarantee that dry-run never blocks on interactive input.
    #[test]
    fn confirm_action_dry_run_bypasses_prompt() {
        let opts = dry_run_confirm_opts();
        assert!(
            confirm_action(
                "This will approve milestone 1 of agreement abc and release funds to the payee.",
                false, // yes=false; dry_run alone should be enough
                &opts,
            )
            .is_ok(),
            "dry-run should bypass prompt even without --yes"
        );
    }

    // --- render_raw ---

    #[test]
    fn render_raw_ok_on_success() {
        assert!(render_raw(&ok_output("done")).is_ok());
    }

    #[test]
    fn render_raw_err_includes_command_and_streams_on_failure() {
        let err = render_raw(&fail_output("some stdout", "some stderr")).unwrap_err();
        assert!(err.contains("Transaction failed"));
        assert!(err.contains("stellar contract invoke --id CAABC -- boom"));
        assert!(err.contains("some stdout"));
        assert!(err.contains("some stderr"));
    }

    // --- render_json ---

    #[test]
    fn render_json_ok_on_success() {
        assert!(render_json(&ok_output("{}")).is_ok());
    }

    #[test]
    fn render_json_returns_empty_err_on_failure() {
        // Empty message => main() exits non-zero without printing again
        // (the detail is already in the JSON envelope on stdout). This is
        // what "--quiet suppresses non-error output" relies on.
        let err = render_json(&fail_output("", "boom")).unwrap_err();
        assert_eq!(err, "");
    }

    // --- render_human ---

    #[test]
    fn render_human_ok_on_success() {
        assert!(render_human(&ok_output(r#"{"k":"v"}"#)).is_ok());
    }

    #[test]
    fn render_human_returns_empty_err_on_failure() {
        assert_eq!(render_human(&fail_output("", "boom")).unwrap_err(), "");
    }

    #[test]
    fn render_human_handles_non_json_stdout_without_error() {
        assert!(render_human(&ok_output("not json at all")).is_ok());
    }

    // --- render_human color suppression (#408) ---

    /// Helper: runs render_human_to with an in-memory buffer and returns the
    /// captured output as a String.
    fn capture_render_human(out: &InvokeOutput, use_color: bool) -> String {
        let mut buf: Vec<u8> = Vec::new();
        let _ = render_human_to(out, use_color, &mut buf);
        String::from_utf8(buf).expect("render_human_to wrote non-UTF-8")
    }

    #[test]
    fn render_human_no_color_suppresses_all_escape_sequences_on_success() {
        // When use_color=false every \x1b[ sequence must be absent from output.
        let rendered = capture_render_human(&ok_output(r#"{"amount":"100"}"#), false);
        assert!(
            !rendered.contains('\x1b'),
            "expected no ANSI escapes in plain-text output, got: {rendered:?}"
        );
        // The semantic content must still be present.
        assert!(rendered.contains("Success"), "success marker missing");
        assert!(rendered.contains("amount"), "field key missing");
        assert!(rendered.contains("100"), "field value missing");
    }

    #[test]
    fn render_human_no_color_suppresses_all_escape_sequences_on_failure() {
        // Adjacent case: failure path must also be escape-free when colors off.
        let rendered = capture_render_human(&fail_output("bad input", "contract error"), false);
        assert!(
            !rendered.contains('\x1b'),
            "expected no ANSI escapes in plain-text failure output, got: {rendered:?}"
        );
        assert!(rendered.contains("Failed"), "failure marker missing");
        assert!(rendered.contains("contract error"), "stderr missing from output");
    }

    #[test]
    fn render_human_with_color_emits_escape_sequences() {
        // When use_color=true the ANSI codes must be present so we know the
        // color path is not accidentally dead.
        let rendered = capture_render_human(&ok_output(r#"{"k":"v"}"#), true);
        assert!(
            rendered.contains('\x1b'),
            "expected ANSI escapes when colors enabled, got: {rendered:?}"
        );
    }

    // --- render_output dispatch ---

    fn opts(format: OutputFormat) -> OutputOpts {
        OutputOpts {
            format,
            quiet: false,
            dry_run: false,
        }
    }

    #[test]
    fn render_output_raw_failure_bubbles_detailed_message() {
        let err = render_output(&fail_output("o", "e"), &opts(OutputFormat::Raw)).unwrap_err();
        assert!(err.contains("Transaction failed"));
    }

    #[test]
    fn render_output_json_failure_is_silent_err() {
        let err = render_output(&fail_output("o", "e"), &opts(OutputFormat::Json)).unwrap_err();
        assert_eq!(err, "");
    }

    #[test]
    fn render_output_all_formats_ok_on_success() {
        for f in [OutputFormat::Raw, OutputFormat::Json, OutputFormat::Human] {
            assert!(render_output(&ok_output("{}"), &opts(f)).is_ok(), "{f:?}");
        }
    }

    // --- dry-run output routing (#407) ---

    /// Helper: an InvokeOutput shaped exactly like what execute() produces for
    /// dry-run — a preview command string as stdout, success=true, empty stderr.
    fn dry_run_output(preview: &str) -> InvokeOutput {
        InvokeOutput {
            stdout: preview.to_string(),
            stderr: String::new(),
            success: true,
            command_debug: preview.to_string(),
        }
    }

    /// Helper: OutputOpts with dry_run=true and the given format.
    fn dry_run_opts(format: OutputFormat) -> OutputOpts {
        OutputOpts {
            format,
            quiet: false,
            dry_run: true,
        }
    }

    /// `--dry-run --json` must produce a parseable JSON envelope (not a raw
    /// plain-text line), so scripts that always pass `--json` keep working.
    #[test]
    fn dry_run_json_produces_valid_json_envelope() {
        let preview = "stellar contract invoke --id CABC -- init --agreement-id deadbeef";
        let out = dry_run_output(preview);
        // Capture what render_json would write by checking the envelope directly.
        let envelope = json_envelope(&out);
        // Must be a JSON object with "status": "success".
        assert_eq!(envelope["status"], "success");
        // The result field must contain the preview string (not null).
        assert_eq!(envelope["result"], serde_json::Value::String(preview.to_string()));
        // error field must be null on success.
        assert_eq!(envelope["error"], serde_json::Value::Null);
        // The whole envelope must serialise without panic.
        let serialised = serde_json::to_string(&envelope).expect("envelope must serialise");
        // And round-trip back to a Value without error.
        let _parsed: serde_json::Value =
            serde_json::from_str(&serialised).expect("envelope must be valid JSON");
        // render_json itself must return Ok(()).
        assert!(render_output(&out, &dry_run_opts(OutputFormat::Json)).is_ok());
    }

    /// `--dry-run --human-readable` must succeed and go through the human
    /// renderer (success path), not bail out before reaching render_output.
    #[test]
    fn dry_run_human_readable_succeeds() {
        let out = dry_run_output("stellar contract invoke --id CABC -- lock_funds");
        assert!(render_output(&out, &dry_run_opts(OutputFormat::Human)).is_ok());
    }

    /// `--dry-run` (raw / default format) must still return Ok(()).
    /// This is the existing behaviour — the test guards against regression.
    #[test]
    fn dry_run_raw_succeeds() {
        let out = dry_run_output("stellar contract invoke --id CABC -- submit_work");
        assert!(render_output(&out, &dry_run_opts(OutputFormat::Raw)).is_ok());
    }

    /// `--dry-run --quiet` forces Json format; the call must still succeed and
    /// produce a valid JSON envelope rather than printing a plain string.
    #[test]
    fn dry_run_quiet_produces_valid_json_envelope() {
        let preview = "stellar contract invoke --id CABC -- cancel_milestone";
        let out = dry_run_output(preview);
        let opts = OutputOpts {
            format: OutputFormat::Json, // quiet forces Json in dispatch
            quiet: true,
            dry_run: true,
        };
        assert!(render_output(&out, &opts).is_ok());
        // Verify the envelope shape independently.
        let envelope = json_envelope(&out);
        assert_eq!(envelope["status"], "success");
        let serialised = serde_json::to_string(&envelope).expect("must serialise");
        serde_json::from_str::<serde_json::Value>(&serialised)
            .expect("--dry-run --quiet must produce parseable JSON");
    }

    /// execute() end-to-end with dry_run=true and Json format must return
    /// Ok(()) without panicking — confirms the fix wires up correctly.
    #[test]
    fn execute_dry_run_json_returns_ok() {
        let config = Config {
            rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            network_passphrase: "Test SDF Network ; September 2015".to_string(),
            contract_id: format!("C{}", "A".repeat(55)),
            source_key: "alice".to_string(),
        };
        let opts = OutputOpts {
            format: OutputFormat::Json,
            quiet: false,
            dry_run: true,
        };
        let result = execute(&config, "init", &[], &opts);
        assert!(result.is_ok(), "execute dry-run --json must return Ok, got: {result:?}");
    }
}
