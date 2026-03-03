#!/usr/bin/env python3
import base64
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

import requests
from dotenv import load_dotenv
from solders.keypair import Keypair
from solders.message import to_bytes_versioned
from solders.pubkey import Pubkey
from solders.system_program import TransferParams, transfer
from solders.transaction import Transaction, VersionedTransaction
from solana.rpc.api import Client
from solana.rpc.types import TxOpts

WSOL_MINT = "So11111111111111111111111111111111111111112"
DEFAULT_ANKY_MINT = "6GsRbp2Bz9QZsoAEmUSGgTpTW7s59m7R3EGtm1FPpump"
DEFAULT_DEV_WALLET = "CUJQnwHYzv2ohp4J8NgqveNJWG14Ys4Fbc2mSvgTuwd6"
QUOTE_URLS = [
    "https://quote-api.jup.ag/v6/quote",
    "https://lite-api.jup.ag/swap/v1/quote",
]
SWAP_URLS = [
    "https://quote-api.jup.ag/v6/swap",
    "https://lite-api.jup.ag/swap/v1/swap",
]


def log(msg: str) -> None:
    ts = datetime.now(timezone.utc).isoformat()
    print(f"[{ts}] {msg}")


def load_keypair(wallet_path: Path) -> Keypair:
    arr = json.loads(wallet_path.read_text())
    return Keypair.from_bytes(bytes(arr))


def parse_bool(v: str) -> bool:
    return str(v).strip().lower() in {"1", "true", "yes", "on"}


def first_successful_get(session: requests.Session, urls, params, timeout_s: int):
    last_err = None
    for url in urls:
        try:
            res = session.get(url, params=params, timeout=timeout_s)
            if res.status_code == 200:
                return url, res.json()
            last_err = f"{url} status={res.status_code} body={res.text[:300]}"
        except Exception as e:
            last_err = f"{url} err={e}"
    raise RuntimeError(last_err or "all GET endpoints failed")


def first_successful_post(session: requests.Session, urls, payload, timeout_s: int):
    last_err = None
    for url in urls:
        try:
            res = session.post(url, json=payload, timeout=timeout_s)
            if res.status_code == 200:
                return url, res.json()
            last_err = f"{url} status={res.status_code} body={res.text[:300]}"
        except Exception as e:
            last_err = f"{url} err={e}"
    raise RuntimeError(last_err or "all POST endpoints failed")


def send_dev_fee(rpc: Client, keypair: Keypair, to_wallet: str, lamports: int) -> str:
    to_pubkey = Pubkey.from_string(to_wallet)
    ix = transfer(
        TransferParams(
            from_pubkey=keypair.pubkey(),
            to_pubkey=to_pubkey,
            lamports=lamports,
        )
    )
    blockhash = rpc.get_latest_blockhash().value.blockhash
    tx = Transaction.new_signed_with_payer(
        [ix],
        keypair.pubkey(),
        [keypair],
        blockhash,
    )
    resp = rpc.send_transaction(
        tx,
        opts=TxOpts(skip_preflight=False, preflight_commitment="confirmed", max_retries=2),
    )
    return str(resp.value)


def main() -> int:
    env_file = os.getenv("ANKY_DCA_ENV", "/home/kithkui/anky/.secrets/anky_dca.env")
    if Path(env_file).exists():
        load_dotenv(env_file)

    wallet_path = Path(os.getenv("DCA_WALLET_PATH", "/home/kithkui/anky/.secrets/anky_dca_wallet.json"))
    if not wallet_path.exists():
        log(f"wallet file not found: {wallet_path}")
        return 1

    rpc_url = os.getenv("SOLANA_RPC_URL", "https://api.mainnet-beta.solana.com")
    output_mint = os.getenv("ANKY_TOKEN_MINT", DEFAULT_ANKY_MINT)
    buy_sol = float(os.getenv("ANKY_BUY_SOL_PER_RUN", "0.0005"))
    slippage_bps = int(os.getenv("ANKY_SLIPPAGE_BPS", "300"))
    min_sol_reserve = float(os.getenv("ANKY_MIN_SOL_RESERVE", "0.02"))
    dev_fee_enabled = parse_bool(os.getenv("ANKY_DEV_FEE_ENABLED", "false"))
    dev_fee_bps = int(os.getenv("ANKY_DEV_FEE_BPS", "100"))
    dev_wallet = os.getenv("ANKY_DEV_WALLET", DEFAULT_DEV_WALLET)
    dry_run = parse_bool(os.getenv("ANKY_DRY_RUN", "false"))
    timeout_s = int(os.getenv("ANKY_HTTP_TIMEOUT", "20"))

    lamports = int(buy_sol * 1_000_000_000)
    reserve_lamports = int(min_sol_reserve * 1_000_000_000)
    if lamports <= 0:
        log("ANKY_BUY_SOL_PER_RUN must be > 0")
        return 1

    keypair = load_keypair(wallet_path)
    pubkey = str(keypair.pubkey())
    log(
        f"wallet={pubkey} buy_sol={buy_sol} slippage_bps={slippage_bps} dry_run={dry_run} "
        f"dev_fee_enabled={dev_fee_enabled} dev_fee_bps={dev_fee_bps}"
    )

    rpc = Client(rpc_url)
    bal_resp = rpc.get_balance(keypair.pubkey())
    balance = int(bal_resp.value)
    log(f"current_balance_sol={balance/1_000_000_000:.6f}")

    if balance < lamports + reserve_lamports:
        log("skip: insufficient balance after reserve guard")
        return 0

    fee_lamports = 0
    swap_lamports = lamports
    if dev_fee_enabled and dev_fee_bps > 0 and dev_wallet:
        fee_lamports = (lamports * dev_fee_bps) // 10_000
        if fee_lamports > 0:
            swap_lamports = lamports - fee_lamports
            if swap_lamports <= 0:
                log("skip: buy size too small after dev fee deduction")
                return 0

    session = requests.Session()
    q_params = {
        "inputMint": WSOL_MINT,
        "outputMint": output_mint,
        "amount": str(swap_lamports),
        "slippageBps": str(slippage_bps),
        "swapMode": "ExactIn",
    }
    try:
        quote_url, quote_json = first_successful_get(session, QUOTE_URLS, q_params, timeout_s)
    except Exception as e:
        log(f"quote_failed={e}")
        return 0
    log(f"quote_endpoint={quote_url}")
    out_amt = int(quote_json.get("outAmount", "0"))
    log(f"quoted_out_anky_raw={out_amt}")

    if dry_run:
        log("dry-run enabled; not sending swap transaction")
        return 0

    if fee_lamports > 0:
        try:
            fee_sig = send_dev_fee(rpc, keypair, dev_wallet, fee_lamports)
            log(
                f"dev_fee_sent_lamports={fee_lamports} "
                f"dev_fee_sol={fee_lamports/1_000_000_000:.9f} to={dev_wallet} signature={fee_sig}"
            )
        except Exception as e:
            log(f"dev_fee_transfer_failed={e}")
            return 0

    payload = {
        "quoteResponse": quote_json,
        "userPublicKey": pubkey,
        "wrapAndUnwrapSol": True,
        "dynamicComputeUnitLimit": True,
        "prioritizationFeeLamports": "auto",
    }
    try:
        swap_url, swap_json = first_successful_post(session, SWAP_URLS, payload, timeout_s)
    except Exception as e:
        log(f"swap_build_failed={e}")
        return 0
    log(f"swap_endpoint={swap_url}")
    swap_tx_b64 = swap_json.get("swapTransaction")
    if not swap_tx_b64:
        log(f"swap response missing transaction: {swap_json}")
        return 1

    raw_tx = VersionedTransaction.from_bytes(base64.b64decode(swap_tx_b64))
    sig = keypair.sign_message(to_bytes_versioned(raw_tx.message))
    signed_tx = VersionedTransaction.populate(raw_tx.message, [sig])

    send_res = rpc.send_raw_transaction(
        bytes(signed_tx),
        opts=TxOpts(skip_preflight=False, preflight_commitment="confirmed", max_retries=2),
    )
    tx_sig = send_res.value
    log(f"swap_submitted signature={tx_sig}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as e:
        log(f"fatal_error={e}")
        raise SystemExit(1)
