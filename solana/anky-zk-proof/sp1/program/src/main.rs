#![no_main]
sp1_zkvm::entrypoint!(main);

pub fn main() {
    let raw = sp1_zkvm::io::read::<String>();
    let writer = sp1_zkvm::io::read::<String>();
    let expected_hash = sp1_zkvm::io::read::<Option<String>>();

    let receipt = anky_zk_proof::build_receipt(&raw, &writer, expected_hash.as_deref())
        .expect("private .anky witness must satisfy the public claim");
    let public_values = serde_json::to_vec(&receipt).expect("receipt serializes");

    sp1_zkvm::io::commit_slice(&public_values);
}
