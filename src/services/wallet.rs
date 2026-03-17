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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_evm_wallets() {
        let wallet = generate_custodial_wallet();
        assert!(wallet.address.starts_with("0x"));
        assert_eq!(wallet.address.len(), 42);
        assert!(wallet.secret_key.starts_with("0x"));
        assert_eq!(wallet.secret_key.len(), 66);
    }
}
