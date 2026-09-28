//! Input size limits for user-supplied CLI arguments (#155).
//!
//! Every string that eventually becomes a Soroban contract argument is checked
//! here first. Oversized values are rejected with a uniform error *before* they
//! reach [`crate::rpc`] and the `stellar` CLI, so a caller cannot smuggle a
//! 1 MB `agreement_id` (or proof URI, or address) into a contract invocation
//! where it would inflate Soroban resource usage, trigger out-of-gas failures,
//! or be rejected by the RPC endpoint.
//!
//! These checks are purely about *length*. Character-set / format validation
//! for specific fields (hex-only agreement IDs, control-character-free proof
//! URIs) lives alongside the command handlers in [`crate::commands`].

/// Maximum length of a hex-encoded agreement ID.
///
/// A Soroban `BytesN<32>` value is exactly 64 hexadecimal characters; anything
/// longer cannot be a valid 32-byte ID.
pub const MAX_AGREEMENT_ID_LEN: usize = 64;

/// Maximum length of a proof URI.
///
/// Generous enough for an `ipfs://` CID or a long HTTPS URL with query
/// parameters, but bounded so it cannot bloat the contract payload.
pub const MAX_PROOF_URI_LEN: usize = 512;

/// Maximum length of a Stellar strkey address (`G…`, `C…`, `S…`, named
/// identities). Every strkey encodes to 56 base-32 characters.
pub const MAX_ADDRESS_LEN: usize = 56;

/// Reject `value` when it exceeds `max` characters.
///
/// `field` names the offending argument so the caller knows which input to
/// shorten. The error text always contains the phrase
/// `"input exceeds maximum length of <max> characters"`.
pub fn check_max_len(field: &str, value: &str, max: usize) -> Result<(), String> {
    let len = value.chars().count();
    if len > max {
        return Err(format!(
            "invalid {field}: input exceeds maximum length of {max} characters (got {len})"
        ));
    }
    Ok(())
}

/// Length check for an `--agreement-id` argument.
pub fn validate_agreement_id_len(value: &str) -> Result<(), String> {
    check_max_len("agreement_id", value, MAX_AGREEMENT_ID_LEN)
}

/// Length check for a `--proof-uri` argument.
pub fn validate_proof_uri_len(value: &str) -> Result<(), String> {
    check_max_len("proof_uri", value, MAX_PROOF_URI_LEN)
}

/// Length check for any address-typed argument (`--payer`, `--payee`,
/// `--token`, `--resolver`, `--caller`). `field` is the flag name without the
/// leading dashes.
pub fn validate_address_len(field: &str, value: &str) -> Result<(), String> {
    check_max_len(field, value, MAX_ADDRESS_LEN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_value_at_the_limit() {
        assert!(check_max_len("f", &"a".repeat(10), 10).is_ok());
    }

    #[test]
    fn rejects_value_one_over_the_limit() {
        let err = check_max_len("f", &"a".repeat(11), 10).unwrap_err();
        assert!(err.contains("input exceeds maximum length of 10 characters"));
    }

    #[test]
    fn agreement_id_65_chars_rejected() {
        let err = validate_agreement_id_len(&"a".repeat(65)).unwrap_err();
        assert!(err.contains("agreement_id"));
        assert!(err.contains("exceeds maximum length of 64 characters"));
    }

    #[test]
    fn agreement_id_64_chars_ok() {
        assert!(validate_agreement_id_len(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn proof_uri_oversized_rejected() {
        let err = validate_proof_uri_len(&"x".repeat(513)).unwrap_err();
        assert!(err.contains("exceeds maximum length of 512 characters"));
    }

    #[test]
    fn proof_uri_max_ok() {
        assert!(validate_proof_uri_len(&"x".repeat(512)).is_ok());
    }

    #[test]
    fn address_oversized_rejected() {
        let err = validate_address_len("payer", &"G".repeat(57)).unwrap_err();
        assert!(err.contains("payer"));
        assert!(err.contains("exceeds maximum length of 56 characters"));
    }

    #[test]
    fn address_valid_strkey_length_ok() {
        // A real Stellar public key is 56 characters.
        assert!(validate_address_len(
            "payer",
            "GBCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUVWX"
        )
        .is_ok());
    }

    #[test]
    fn counts_unicode_scalar_values_not_bytes() {
        // 10 'é' characters = 20 UTF-8 bytes but only 10 chars — must pass a
        // 10-char limit, proving the check is not byte-based.
        assert!(check_max_len("f", &"é".repeat(10), 10).is_ok());
        assert!(check_max_len("f", &"é".repeat(11), 10).is_err());
    }
}
