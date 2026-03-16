use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

pub struct GeneratedWallet {
    pub address: String,
    pub secret_key: String,
}

pub fn generate_custodial_wallet() -> GeneratedWallet {
    let mut rng = OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let address = bs58::encode(signing_key.verifying_key().to_bytes()).into_string();
    let secret_key = STANDARD.encode(signing_key.to_keypair_bytes());

    GeneratedWallet {
        address,
        secret_key,
    }
}
