//! Claim token generation and hashing

use rand::RngExt;
use sha2::{Digest, Sha256};

/// Generate a cryptographically secure claim token
///
/// Returns a 64-character alphanumeric string with high entropy (384 bits)
pub fn generate_claim_token() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();

    (0..64)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Hash a claim token using SHA-256
///
/// Returns the hex-encoded hash of the token
pub fn hash_claim_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_claim_token_length() {
        let token = generate_claim_token();
        assert_eq!(token.len(), 64, "Token should be 64 characters long");
    }

    #[test]
    fn test_generate_claim_token_uniqueness() {
        let token1 = generate_claim_token();
        let token2 = generate_claim_token();
        let token3 = generate_claim_token();

        assert_ne!(token1, token2, "Tokens should be unique");
        assert_ne!(token2, token3, "Tokens should be unique");
        assert_ne!(token1, token3, "Tokens should be unique");
    }

    #[test]
    fn test_generate_claim_token_alphanumeric() {
        let token = generate_claim_token();
        assert!(
            token.chars().all(|c| c.is_ascii_alphanumeric()),
            "Token should only contain alphanumeric characters"
        );
    }

    #[test]
    fn test_hash_claim_token_format() {
        let token = "test_token_123";
        let hash = hash_claim_token(token);

        assert_eq!(hash.len(), 64, "SHA-256 hash should be 64 hex characters");
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "Hash should only contain hex digits"
        );
    }

    #[test]
    fn test_hash_claim_token_deterministic() {
        let token = "test_token_123";
        let hash1 = hash_claim_token(token);
        let hash2 = hash_claim_token(token);

        assert_eq!(
            hash1, hash2,
            "Hashing the same token should produce the same result"
        );
    }

    #[test]
    fn test_hash_claim_token_different_inputs() {
        let hash1 = hash_claim_token("token1");
        let hash2 = hash_claim_token("token2");

        assert_ne!(
            hash1, hash2,
            "Different tokens should produce different hashes"
        );
    }

    #[test]
    fn test_known_hash_value() {
        // SHA-256("password") = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8"
        let hash = hash_claim_token("password");
        assert_eq!(
            hash,
            "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8"
        );
    }

    #[test]
    fn test_token_generation_properties() {
        // Generate multiple tokens and verify properties
        for _ in 0..10 {
            let token = generate_claim_token();

            // Should be 64 characters
            assert_eq!(token.len(), 64);

            // Should be alphanumeric
            assert!(token.chars().all(|c| c.is_ascii_alphanumeric()));

            // Hash should be deterministic
            let hash1 = hash_claim_token(&token);
            let hash2 = hash_claim_token(&token);
            assert_eq!(hash1, hash2);

            // Hash should be 64 hex characters
            assert_eq!(hash1.len(), 64);
            assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}
