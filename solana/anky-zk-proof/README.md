# anky-zk-proof

This crate is the proof core for private `.anky` validation.

It runs as a normal local verifier/CLI:

```bash
cargo run -- --file path/to/session.anky --writer <wallet> --expected-hash <sha256_hex>
```

It reads private `.anky` plaintext locally, validates the exact mobile protocol rules, checks that the rite lasted a full 8 minutes including the terminal 8000ms silence, and prints a public receipt:

- `writer`
- `session_hash`
- `utc_day`
- timing metrics
- `proof_hash`

It never prints or sends the writing text.

The same `src/lib.rs` logic is used by `sp1/program`, an SP1 guest that proves the receipt inside the zkVM.

## SP1

Install SP1 with `sp1up`, then build the guest:

```bash
cd solana/anky-zk-proof/sp1/program
cargo prove build
```

Execute the guest without generating a proof:

```bash
cd ../script
PROTOC=/home/kithkui/.local/protoc-34.1/bin/protoc \
RUST_LOG=info \
cargo run --release -- \
  --execute \
  --file ../../fixtures/full.anky \
  --writer <wallet> \
  --receipt-out receipt.json
```

Generate and verify an SP1 Core proof:

```bash
PROTOC=/home/kithkui/.local/protoc-34.1/bin/protoc \
RUST_LOG=info \
cargo run --release -- \
  --prove \
  --file ../../fixtures/full.anky \
  --writer <wallet> \
  --receipt-out receipt.json \
  --proof-out proof-with-public-values.bin
```

The fixture intentionally has no final newline after terminal `8000`, matching the canonical mobile `.anky` format.

The current Solana program records an authority-gated verified receipt after off-chain SP1 verification. The next hardening step is direct on-chain verification of an SP1 Groth16 proof with a Solana verifier program.
