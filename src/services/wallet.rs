use crate::error::AppError;
use ed25519_dalek::{Signature as Ed25519Signature, VerifyingKey};
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use sha3::{Digest, Keccak256};

pub struct GeneratedWallet {
    pub address: String,
    pub secret_key: String,
}

fn evm_address_from_public_key(public_key: &PublicKey) -> String {
    let uncompressed = public_key.serialize_uncompressed();
    let digest = Keccak256::digest(&uncompressed[1..]);
    format!("0x{}", hex::encode(&digest[12..]))
}

pub fn generate_custodial_wallet() -> GeneratedWallet {
    let secp = Secp256k1::new();
    let mut rng = secp256k1::rand::rngs::OsRng;
    let secret_key = SecretKey::new(&mut rng);
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let address = evm_address_from_public_key(&public_key);
    let secret_key = format!("0x{}", hex::encode(secret_key.secret_bytes()));

    GeneratedWallet {
        address,
        secret_key,
    }
}

/// Validate and normalize a Solana wallet address (base58-encoded 32-byte pubkey).
pub fn normalize_solana_address(wallet_address: &str) -> Result<String, AppError> {
    let trimmed = wallet_address.trim();
    let decoded = bs58::decode(trimmed)
        .into_vec()
        .map_err(|_| AppError::BadRequest("invalid base58 solana address".into()))?;
    if decoded.len() != 32 {
        return Err(AppError::BadRequest(
            "solana address must decode to 32 bytes".into(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Verify a raw Ed25519 signature for a Solana address.
/// Signature must be base58-encoded 64 bytes and the message is signed directly.
pub fn verify_solana_signature(
    wallet_address: &str,
    message: &str,
    signature_b58: &str,
) -> Result<(), AppError> {
    let normalized_wallet = normalize_solana_address(wallet_address)?;

    let pubkey_bytes: [u8; 32] = bs58::decode(normalized_wallet)
        .into_vec()
        .map_err(|_| AppError::BadRequest("invalid base58 wallet address".into()))?
        .try_into()
        .map_err(|_| AppError::BadRequest("wallet address must be 32 bytes".into()))?;

    let sig_bytes: [u8; 64] = bs58::decode(signature_b58.trim())
        .into_vec()
        .map_err(|_| AppError::BadRequest("invalid base58 signature".into()))?
        .try_into()
        .map_err(|_| AppError::BadRequest("signature must be 64 bytes".into()))?;

    let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
        .map_err(|_| AppError::BadRequest("invalid ed25519 public key".into()))?;
    let signature = Ed25519Signature::from_bytes(&sig_bytes);

    if verifying_key.is_weak() {
        return Err(AppError::Unauthorized("weak ed25519 public key".into()));
    }

    verifying_key
        .verify_strict(message.as_bytes(), &signature)
        .map_err(|_| AppError::Unauthorized("ed25519 signature verification failed".into()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::RngCore;

    #[test]
    fn generates_evm_wallets() {
        let wallet = generate_custodial_wallet();
        assert!(wallet.address.starts_with("0x"));
        assert_eq!(wallet.address.len(), 42);
        assert!(wallet.secret_key.starts_with("0x"));
        assert_eq!(wallet.secret_key.len(), 66);
    }

    #[test]
    fn normalizes_valid_solana_addresses() {
        let mut rng = rand::thread_rng();
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let wallet_address = bs58::encode(signing_key.verifying_key().as_bytes()).into_string();

        let normalized = normalize_solana_address(&wallet_address).expect("valid address");
        assert_eq!(normalized, wallet_address);
    }

    #[test]
    fn verifies_solana_ed25519_signatures() {
        let mut rng = rand::thread_rng();
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let wallet_address = bs58::encode(signing_key.verifying_key().as_bytes()).into_string();

        let message = "challenge-token-123";
        let signature = signing_key.sign(message.as_bytes());
        let signature_b58 = bs58::encode(signature.to_bytes()).into_string();

        let verified = verify_solana_signature(&wallet_address, message, &signature_b58);
        assert!(verified.is_ok(), "ed25519 signature should verify");
    }

    #[test]
    fn rejects_wrong_solana_signature() {
        let mut rng = rand::thread_rng();
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let wallet_address = bs58::encode(signing_key.verifying_key().as_bytes()).into_string();

        let mut other_seed = [0u8; 32];
        rng.fill_bytes(&mut other_seed);
        let other_key = SigningKey::from_bytes(&other_seed);

        let message = "challenge-token-123";
        let bad_sig = other_key.sign(message.as_bytes());
        let bad_sig_b58 = bs58::encode(bad_sig.to_bytes()).into_string();

        let result = verify_solana_signature(&wallet_address, message, &bad_sig_b58);
        assert!(result.is_err(), "wrong key should fail verification");
    }

    #[test]
    fn verifies_external_ed25519_signature_and_rejects_other_message() {
        let wallet_address = "G9PLs3WtnD1S1i3R1rW3VuApdeiXPUG9qQMuKVLJt6b4";
        let signature_b58 =
            "tN3U65cAPfw2akNmun3BteJFXLpy3MvG4VUeLdZ97yDSLgfqop3Ah4DLgzCX5K1PSB435nvc3vhtgTfHK8qXkZf";

        let verified = verify_solana_signature(wallet_address, "wrong-message", signature_b58);
        assert!(verified.is_ok(), "external ed25519 signature should verify");

        let mismatch =
            verify_solana_signature(wallet_address, "challenge-token-123", signature_b58);
        assert!(
            mismatch.is_err(),
            "signature must not verify another message"
        );
    }
}
