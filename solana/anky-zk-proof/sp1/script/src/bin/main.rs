use std::{fs, path::PathBuf};

use anky_zk_proof::ProofReceipt;
use clap::Parser;
use sp1_sdk::{
    blocking::{ProveRequest, Prover, ProverClient},
    include_elf, Elf, HashableKey, ProvingKey, SP1Stdin,
};

const ANKY_SP1_ELF: Elf = include_elf!("anky-sp1-program");

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Execute or prove the Anky .anky verifier inside SP1."
)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,

    #[arg(long)]
    file: PathBuf,

    #[arg(long, alias = "wallet")]
    writer: String,

    #[arg(long)]
    expected_hash: Option<String>,

    #[arg(long)]
    receipt_out: Option<PathBuf>,

    #[arg(long)]
    proof_out: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    sp1_sdk::utils::setup_logger();
    dotenv::dotenv().ok();

    let args = Args::parse();
    if args.execute == args.prove {
        return Err("specify exactly one of --execute or --prove".into());
    }

    let raw = fs::read_to_string(&args.file)?;
    let mut stdin = SP1Stdin::new();
    stdin.write(&raw);
    stdin.write(&args.writer);
    stdin.write(&args.expected_hash);

    let client = ProverClient::from_env();

    if args.execute {
        let (output, report) = client.execute(ANKY_SP1_ELF, stdin).run()?;
        let receipt = decode_receipt(output.as_slice())?;
        write_receipt(&receipt, args.receipt_out.as_ref())?;
        println!("{}", serde_json::to_string_pretty(&receipt)?);
        println!("cycles: {}", report.total_instruction_count());
        return Ok(());
    }

    let pk = client.setup(ANKY_SP1_ELF)?;
    println!("vkey: {}", pk.verifying_key().bytes32());

    let proof = client.prove(&pk, stdin).run()?;
    client.verify(&proof, pk.verifying_key(), None)?;

    let receipt = decode_receipt(proof.public_values.as_slice())?;
    write_receipt(&receipt, args.receipt_out.as_ref())?;

    if let Some(path) = args.proof_out.as_ref() {
        proof.save(path)?;
        println!("proof: {}", path.display());
    }

    println!("{}", serde_json::to_string_pretty(&receipt)?);
    println!("proof verified");

    Ok(())
}

fn decode_receipt(bytes: &[u8]) -> Result<ProofReceipt, serde_json::Error> {
    if bytes.is_empty() {
        eprintln!("SP1 guest emitted empty public values; check that the private .anky witness is valid and has no trailing newline after terminal 8000.");
    }

    serde_json::from_slice(bytes)
}

fn write_receipt(
    receipt: &ProofReceipt,
    path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = path {
        fs::write(path, serde_json::to_string_pretty(receipt)?)?;
        println!("receipt: {}", path.display());
    }

    Ok(())
}
