/// Network preset selectable via the CLI's `--network` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[value(rename_all = "lower")]
pub enum Network {
    Testnet,
    Mainnet,
    Futurenet,
    Custom,
}

/// Known RPC URL and network passphrase for a non-custom `Network`.
pub struct NetworkPreset {
    pub rpc_url: &'static str,
    pub network_passphrase: &'static str,
}

impl Network {
    /// Returns the known RPC URL / passphrase pair for this network, or
    /// `None` for `Custom`, which has no built-in preset.
    pub fn preset(self) -> Option<NetworkPreset> {
        match self {
            Network::Testnet => Some(NetworkPreset {
                rpc_url: "https://soroban-testnet.stellar.org",
                network_passphrase: "Test SDF Network ; September 2015",
            }),
            Network::Mainnet => Some(NetworkPreset {
                rpc_url: "https://mainnet.sorobanrpc.com",
                network_passphrase: "Public Global Stellar Network ; September 2015",
            }),
            Network::Futurenet => Some(NetworkPreset {
                rpc_url: "https://rpc-futurenet.stellar.org",
                network_passphrase: "Test SDF Future Network ; October 2022",
            }),
            Network::Custom => None,
        }
    }
}

/// Trellis CLI configuration.
///
/// Values are loaded from environment variables with Soroban Testnet defaults
/// so the tool works out-of-the-box for local development.
#[derive(Debug, Clone)]
pub struct Config {
    /// Soroban RPC endpoint.
    /// Env: `STELLAR_RPC_URL`
    /// Default: Soroban Testnet public RPC.
    pub rpc_url: String,

    /// Stellar network passphrase used to sign transactions.
    /// Env: `STELLAR_NETWORK_PASSPHRASE`
    /// Default: Testnet passphrase.
    pub network_passphrase: String,

    /// Bech32-encoded Trellis contract address (`C...`).
    /// Env: `TRELLIS_CONTRACT_ID`
    pub contract_id: String,

    /// Stellar secret key (`S...`) or named identity understood by the
    /// `stellar` CLI (e.g. `alice`).
    /// Env: `TRELLIS_SOURCE_KEY`
    pub source_key: String,
}

/// Number of characters in a Stellar strkey address (`C…` / `G…` / `S…`),
/// including the one-character type prefix.
const STRKEY_LEN: usize = 56;

/// Validate the format of a Stellar contract address (`C…`).
///
/// A contract ID is a strkey-encoded value: exactly [`STRKEY_LEN`] characters,
/// beginning with `C`, containing only RFC 4648 base32 characters (`A`–`Z`,
/// `2`–`7`). This is a cheap structural check — it does not verify the trailing
/// CRC16 checksum — but it catches the overwhelmingly common mistakes (a
/// truncated paste, a stray space, the wrong key type such as an `S…` secret)
/// before they turn into a confusing failure deep inside the Stellar CLI.
pub fn validate_contract_id(id: &str) -> Result<(), String> {
    if !id.starts_with('C') {
        return Err(format!(
            "TRELLIS_CONTRACT_ID must be a Stellar contract address starting with 'C' \
             (e.g. CA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ); got {id:?}"
        ));
    }
    if id.len() != STRKEY_LEN {
        return Err(format!(
            "TRELLIS_CONTRACT_ID must be exactly {STRKEY_LEN} characters, got {}",
            id.len()
        ));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_uppercase() || ('2'..='7').contains(&c))
    {
        return Err(
            "TRELLIS_CONTRACT_ID must contain only base32 characters (A-Z, 2-7)".to_string(),
        );
    }
    Ok(())
}

/// Validate that an RPC endpoint URL is well-formed and uses an HTTP(S) scheme.
///
/// A typo like `httsp://…` or a bare `soroban-testnet.stellar.org` with no
/// scheme otherwise surfaces only much later as an opaque connection error.
/// Both `http` and `https` are accepted so a locally-run RPC
/// (`http://localhost:8000`) still works; any other scheme (`ftp`, `file`,
/// `ws`, …) and any URL with an empty host is rejected here with a clear
/// message.
///
/// Cleartext `http://` to a non-loopback host is refused by default to
/// prevent a compromised `.env` from silently redirecting traffic to an
/// unauthenticated endpoint. Pass `unsafe_rpc = true` to downgrade the hard
/// error to a printed warning (useful for development devnets). This mirrors
/// the `--unsafe-rpc` CLI flag (#156).
pub fn validate_rpc_url(raw: &str, unsafe_rpc: bool) -> Result<Option<String>, String> {
    let parsed =
        url::Url::parse(raw).map_err(|e| format!("RPC URL {raw:?} is not a valid URL: {e}"))?;

    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(format!(
                "RPC URL scheme must be http or https, got {other:?} (in {raw:?})"
            ));
        }
    }

    if parsed.host_str().map_or(true, str::is_empty) {
        return Err(format!("RPC URL {raw:?} is missing a host"));
    }

    // Guard against cleartext HTTP to non-loopback hosts (#156).
    if parsed.scheme() == "http" {
        let host = parsed.host_str().unwrap_or("");
        let is_loopback = matches!(host, "localhost" | "127.0.0.1" | "::1");
        if !is_loopback {
            let msg = format!(
                "Warning: RPC URL {raw:?} uses cleartext HTTP to a non-loopback host. \
                 Traffic may be intercepted. Use https:// or pass --unsafe-rpc to suppress this error."
            );
            if unsafe_rpc {
                return Ok(Some(msg));
            } else {
                return Err(msg);
            }
        }
    }

    Ok(None)
}

impl Config {
    /// Load configuration from environment variables with network detection.
    ///
    /// Loads network preference from `TRELLIS_NETWORK` env var (defaults to Testnet),
    /// then resolves configuration using that network as the base. All other config
    /// values are loaded from env vars with Testnet defaults.
    pub fn from_env() -> Result<Self, String> {
        let network_str = std::env::var("TRELLIS_NETWORK")
            .unwrap_or_else(|_| "testnet".to_string())
            .to_lowercase();

        let network = match network_str.as_str() {
            "mainnet" => Network::Mainnet,
            "futurenet" => Network::Futurenet,
            "custom" => Network::Custom,
            _ => Network::Testnet,
        };

        Self::resolve(network, None, None, None)
    }

    /// Resolve configuration from a `--network` preset plus optional CLI
    /// overrides for RPC URL / passphrase.
    ///
    /// Priority (highest to lowest): CLI flags > env vars > network preset.
    /// `Network::Custom` has no preset, so it requires the RPC URL and
    /// passphrase to come from a CLI flag or env var — returns `Err` naming
    /// whichever is still missing.
    ///
    /// The source key is resolved with the same priority, reading from a file
    /// first so a raw `S…` secret seed never has to be placed in an
    /// environment variable that leaks into `/proc/<pid>/environ` or shell
    /// history (see #240):
    ///   `--source-key-file` > `TRELLIS_SOURCE_KEY_FILE` > `TRELLIS_SOURCE_KEY`.
    pub fn resolve(
        network: Network,
        cli_rpc_url: Option<String>,
        cli_network_passphrase: Option<String>,
        cli_source_key_file: Option<String>,
    ) -> Result<Self, String> {
        let preset = network.preset();

        let rpc_url = cli_rpc_url
            .or_else(|| std::env::var("STELLAR_RPC_URL").ok())
            .or_else(|| preset.as_ref().map(|p| p.rpc_url.to_string()))
            .ok_or_else(|| {
                "Error: --network=custom requires --rpc-url (or STELLAR_RPC_URL)".to_string()
            })?;

        let network_passphrase = cli_network_passphrase
            .or_else(|| std::env::var("STELLAR_NETWORK_PASSPHRASE").ok())
            .or_else(|| preset.as_ref().map(|p| p.network_passphrase.to_string()))
            .ok_or_else(|| {
                "Error: --network=custom requires --network-passphrase (or STELLAR_NETWORK_PASSPHRASE)"
                    .to_string()
            })?;

        let contract_id = std::env::var("TRELLIS_CONTRACT_ID")
            .unwrap_or_else(|_| "UNSET_CONTRACT_ID".to_string());

        // Source key priority: --source-key-file CLI flag >
        // TRELLIS_SOURCE_KEY_FILE env var > TRELLIS_SOURCE_KEY env var.
        // Reading from a file keeps raw `S…` seeds out of argv and out of
        // the exported-env-var namespace where they leak into
        // `/proc/<pid>/environ` (#240).
        let source_key = if let Some(path) = cli_source_key_file {
            read_source_key_file(&path)?
        } else if let Ok(path) = std::env::var("TRELLIS_SOURCE_KEY_FILE") {
            read_source_key_file(&path)?
        } else {
            std::env::var("TRELLIS_SOURCE_KEY")
                .unwrap_or_else(|_| "UNSET_SOURCE_KEY".to_string())
        };

        Ok(Config {
            rpc_url,
            network_passphrase,
            contract_id,
            source_key,
        })
    }

    /// Validate that all required env vars are set to real values.
    ///
    /// Returns `Err` listing the names of every missing variable. Call this
    /// at startup before dispatching any command so users see a clear error
    /// instead of a cryptic RPC failure deep in the pipeline.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.contract_id == "UNSET_CONTRACT_ID" {
            errors.push("TRELLIS_CONTRACT_ID".to_string());
        } else if let Err(e) = validate_contract_id(&self.contract_id) {
            errors.push(e);
        }

        if self.source_key == "UNSET_SOURCE_KEY" {
            errors.push("TRELLIS_SOURCE_KEY".to_string());
        }

        if let Err(e) = validate_rpc_url(&self.rpc_url, false) {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Try to resolve the source key from keystore, then fall back to env var.
    pub fn resolve_source_key(&self) -> Result<String, String> {
        use crate::keystore::Keystore;
        let keystore = Keystore::new();
        keystore
            .resolve_source_key(&self.source_key)
            .map_err(|e| format!("Failed to resolve source key: {}", e))
    }
}

/// Read a source key from `path`, trimming surrounding whitespace / newlines.
///
/// Keeping the secret in a file (ideally mode `0600`) instead of an argv
/// entry or exported env var is the mitigation for #240: file contents are
/// never visible in `ps` output.
fn read_source_key_file(path: &str) -> Result<String, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("Error: cannot read --source-key-file {path:?}: {e}"))?;
    let key = raw.trim().to_string();
    if key.is_empty() {
        return Err(format!("Error: --source-key-file {path:?} is empty"));
    }
    Ok(key)
}

/// True when `s` looks like a Stellar strkey secret seed (`S…`, 56 base32
/// chars). Such values must never be passed on a command line — see
/// `crate::rpc` and #240.
pub fn is_secret_seed(s: &str) -> bool {
    s.len() == 56
        && s.starts_with('S')
        && s.bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A syntactically valid contract address (`C` + 55 base32 chars).
    fn valid_contract_id() -> String {
        format!("C{}", "A".repeat(STRKEY_LEN - 1))
    }

    fn config_with(contract_id: &str, source_key: &str) -> Config {
        Config {
            rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            network_passphrase: "Test SDF Network ; September 2015".to_string(),
            contract_id: contract_id.to_string(),
            source_key: source_key.to_string(),
        }
    }

    #[test]
    fn validate_passes_when_all_vars_set() {
        let cfg = config_with(&valid_contract_id(), "SABC123");
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn validate_fails_on_unset_contract_id() {
        let cfg = config_with("UNSET_CONTRACT_ID", "SABC123");
        let err = cfg.validate().unwrap_err();
        assert!(err.contains(&"TRELLIS_CONTRACT_ID".to_string()));
        assert!(!err.contains(&"TRELLIS_SOURCE_KEY".to_string()));
    }

    #[test]
    fn validate_fails_on_unset_source_key() {
        let cfg = config_with(&valid_contract_id(), "UNSET_SOURCE_KEY");
        let err = cfg.validate().unwrap_err();
        assert!(err.contains(&"TRELLIS_SOURCE_KEY".to_string()));
        assert!(!err.contains(&"TRELLIS_CONTRACT_ID".to_string()));
    }

    #[test]
    fn validate_reports_all_missing_vars() {
        let cfg = config_with("UNSET_CONTRACT_ID", "UNSET_SOURCE_KEY");
        let err = cfg.validate().unwrap_err();
        assert_eq!(err.len(), 2);
        assert!(err.contains(&"TRELLIS_CONTRACT_ID".to_string()));
        assert!(err.contains(&"TRELLIS_SOURCE_KEY".to_string()));
    }

    // --- validate_contract_id (#236) ---

    #[test]
    fn contract_id_accepts_well_formed_address() {
        assert!(validate_contract_id(&valid_contract_id()).is_ok());
        assert!(
            validate_contract_id("CA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ")
                .is_ok()
        );
    }

    #[test]
    fn contract_id_rejects_missing_c_prefix() {
        let err = validate_contract_id(&format!("G{}", "A".repeat(STRKEY_LEN - 1))).unwrap_err();
        assert!(err.contains("starting with 'C'"), "got: {err}");
    }

    #[test]
    fn contract_id_rejects_wrong_length() {
        assert!(validate_contract_id("CABC123").is_err());
        assert!(validate_contract_id(&format!("C{}", "A".repeat(STRKEY_LEN))).is_err());
    }

    #[test]
    fn contract_id_rejects_non_base32_chars() {
        // lowercase, `1`, `0`, `8`, `9` and punctuation are outside the alphabet
        let bad = format!("C{}1", "A".repeat(STRKEY_LEN - 2));
        assert!(validate_contract_id(&bad).is_err());
        let spaced = format!("C{} x{}", "A".repeat(26), "A".repeat(STRKEY_LEN - 29));
        assert!(validate_contract_id(&spaced).is_err());
    }

    #[test]
    fn validate_reports_malformed_contract_id() {
        let cfg = config_with("CBOGUS", "SABC123");
        let err = cfg.validate().unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err[0].contains("TRELLIS_CONTRACT_ID"), "got: {:?}", err);
    }

    // --- validate_rpc_url (#237) ---

    #[test]
    fn rpc_url_accepts_https_and_http_localhost() {
        assert!(validate_rpc_url("https://soroban-testnet.stellar.org", false).is_ok());
        assert!(validate_rpc_url("http://localhost:8000/soroban/rpc", false).is_ok());
        assert!(validate_rpc_url("http://127.0.0.1:8000", false).is_ok());
    }

    #[test]
    fn rpc_url_rejects_scheme_typo() {
        let err = validate_rpc_url("httsp://soroban-testnet.stellar.org", false).unwrap_err();
        assert!(
            err.contains("scheme") || err.contains("valid URL"),
            "got: {err}"
        );
    }

    #[test]
    fn rpc_url_rejects_missing_scheme() {
        assert!(validate_rpc_url("soroban-testnet.stellar.org", false).is_err());
    }

    #[test]
    fn rpc_url_rejects_non_http_scheme() {
        assert!(validate_rpc_url("ftp://example.com", false).is_err());
        assert!(validate_rpc_url("ws://example.com", false).is_err());
        assert!(validate_rpc_url("file:///etc/passwd", false).is_err());
    }

    #[test]
    fn rpc_url_rejects_empty_host() {
        assert!(validate_rpc_url("https://", false).is_err());
        assert!(validate_rpc_url("http://", false).is_err());
    }

    #[test]
    fn validate_reports_malformed_rpc_url() {
        let mut cfg = config_with(&valid_contract_id(), "SABC123");
        cfg.rpc_url = "not-a-url".to_string();
        let err = cfg.validate().unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err[0].contains("RPC URL"), "got: {:?}", err);
    }

    // --- validate_rpc_url: cleartext HTTP guard (#156) ---

    /// Cleartext http:// to a non-loopback host is rejected by default.
    #[test]
    fn rpc_url_rejects_cleartext_http_to_non_loopback() {
        let err = validate_rpc_url("http://rpc.example.com/soroban", false).unwrap_err();
        assert!(
            err.contains("cleartext HTTP") || err.contains("https"),
            "expected cleartext-HTTP error, got: {err}"
        );
    }

    /// With unsafe_rpc=true the same URL returns Ok(Some(warning)) instead of Err.
    #[test]
    fn rpc_url_unsafe_rpc_downgrades_cleartext_to_warning() {
        let result = validate_rpc_url("http://rpc.example.com/soroban", true);
        assert!(result.is_ok(), "unsafe_rpc should not error; got: {result:?}");
        let warning = result.unwrap();
        assert!(
            warning.is_some(),
            "unsafe_rpc should return Some(warning) for cleartext non-loopback URL"
        );
    }

    /// https:// to any host is always clean regardless of unsafe_rpc.
    #[test]
    fn rpc_url_https_always_ok_no_warning() {
        let result = validate_rpc_url("https://rpc.example.com/soroban", false);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none(), "https:// should produce no warning");
    }

    /// Loopback http:// is not subject to the cleartext guard.
    #[test]
    fn rpc_url_loopback_http_always_ok() {
        for loopback in ["http://localhost:8000", "http://127.0.0.1:8000", "http://[::1]:8000"] {
            let result = validate_rpc_url(loopback, false);
            assert!(result.is_ok(), "loopback http should be ok: {loopback} → {result:?}");
            assert!(result.unwrap().is_none(), "loopback http should produce no warning: {loopback}");
        }
    }
