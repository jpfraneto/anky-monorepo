use sp1_sdk::{blocking::MockProver, blocking::Prover, include_elf, Elf, HashableKey, ProvingKey};

const ANKY_SP1_ELF: Elf = include_elf!("anky-sp1-program");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prover = MockProver::new();
    let pk = prover.setup(ANKY_SP1_ELF)?;
    println!("{}", pk.verifying_key().bytes32());
    Ok(())
}
