use std::{env, error::Error, fs, process};

use anky_zk_proof::build_receipt;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(());
    }

    let mut file = None;
    let mut writer = None;
    let mut expected_hash = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--file" | "-f" => {
                index += 1;
                file = args.get(index).cloned();
            }
            "--writer" | "--wallet" | "-w" => {
                index += 1;
                writer = args.get(index).cloned();
            }
            "--expected-hash" => {
                index += 1;
                expected_hash = args.get(index).cloned();
            }
            value if file.is_none() => {
                file = Some(value.to_string());
            }
            value => {
                return Err(format!("unknown argument `{value}`").into());
            }
        }

        index += 1;
    }

    let file = file.ok_or("missing --file <path>")?;
    let writer = writer.ok_or("missing --writer <wallet>")?;
    let raw = fs::read_to_string(file)?;
    let receipt = build_receipt(&raw, &writer, expected_hash.as_deref())?;

    println!("{}", serde_json::to_string_pretty(&receipt)?);

    Ok(())
}

fn print_usage() {
    println!(
        "Usage: anky-zk-proof --file <session.anky> --writer <wallet> [--expected-hash <sha256_hex>]"
    );
}
