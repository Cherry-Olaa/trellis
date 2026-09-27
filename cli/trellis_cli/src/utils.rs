//! Trellis CLI shared utilities.

/// Returns true if `s` is exactly `len` ASCII hex characters (0-9, a-f, A-F).
///
/// Shared by every command-line argument that maps to a Soroban `BytesN<N>`
/// value (e.g. a 32-byte agreement ID is 64 hex characters).
pub fn is_valid_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Validates a Stellar address format.
///
/// Accepts two formats:
/// - Ed25519 account address: starts with 'G', exactly 56 characters
/// - Contract address: starts with 'C', exactly 56 characters
///
/// Returns `Ok(())` if the address is valid, or `Err(message)` otherwise.
pub fn validate_stellar_address(addr: &str) -> Result<(), String> {
    if addr.len() != 56 {
        return Err(format!(
            "Stellar address must be exactly 56 characters, got {}",
            addr.len()
        ));
    }

    match addr.chars().next() {
        Some('G') => {
            if addr.chars().all(|c| c.is_ascii_alphanumeric()) {
                Ok(())
            } else {
                Err("Ed25519 account address contains invalid characters".to_string())
            }
        }
        Some('C') => {
            if addr.chars().all(|c| c.is_ascii_alphanumeric()) {
                Ok(())
            } else {
                Err("Contract address contains invalid characters".to_string())
            }
        }
        _ => Err(
            "Stellar address must start with 'G' (Ed25519 account) or 'C' (contract)"
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_hex_empty_string() {
        assert!(is_valid_hex("", 0));
        assert!(!is_valid_hex("", 1));
        assert!(!is_valid_hex("a", 0));
    }

    #[test]
    fn is_valid_hex_odd_length_is_length_checked_only() {
        // The function checks exact length, not byte alignment: an odd
        // length is accepted when it is the requested length.
        assert!(is_valid_hex("abc", 3));
        assert!(!is_valid_hex("abc", 4));
        assert!(!is_valid_hex("abcd", 3));
    }

    #[test]
    fn is_valid_hex_boundary_lengths() {
        let s = "a".repeat(32);
        assert!(is_valid_hex(&s, 32));
        assert!(!is_valid_hex(&s[..31], 32));
        assert!(!is_valid_hex(&format!("{s}a"), 32));
    }

    #[test]
    fn is_valid_hex_rejects_non_hex_characters() {
        assert!(!is_valid_hex("abcg", 4));
        assert!(!is_valid_hex("0x12", 4));
        assert!(!is_valid_hex("12 4", 4));
        assert!(!is_valid_hex("12-4", 4));
    }

    #[test]
    fn is_valid_hex_accepts_upper_lower_and_mixed_case() {
        assert!(is_valid_hex("abcdef", 6));
        assert!(is_valid_hex("ABCDEF", 6));
        assert!(is_valid_hex("aBcDeF0123456789", 16));
    }

    #[test]
    fn is_valid_hex_rejects_unicode_lookalike_digits() {
        // Fullwidth "０１" (U+FF10, U+FF11) and Arabic-Indic "٠١"
        // (U+0660, U+0661) are Unicode digits but not ASCII hex.
        assert!(!is_valid_hex("０１", "０１".len()));
        assert!(!is_valid_hex("٠١", "٠١".len()));
        // Fullwidth Latin "Ａ" (U+FF21) looks like hex A.
        assert!(!is_valid_hex("Ａ", "Ａ".len()));
    }

    #[test]
    fn is_valid_hex_length_is_in_bytes_not_chars() {
        // "é" is one char but two bytes: it must not pass as 2 hex chars,
        // nor slip through a length match on char count.
        assert!(!is_valid_hex("é", 2));
        assert!(!is_valid_hex("é", 1));
    }
}
