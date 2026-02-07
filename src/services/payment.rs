use anyhow::Result;
use serde::{Deserialize, Serialize};

pub struct VerificationResult {
    pub valid: bool,
    pub reason: Option<String>,
    pub actual_amount: Option<String>,
    pub from: Option<String>,
    pub block_number: Option<u64>,
}

/// ERC20 Transfer event topic: keccak256("Transfer(address,address,uint256)")
const TRANSFER_TOPIC: &str =
    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u32,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

async fn rpc_call(
    client: &reqwest::Client,
    rpc_url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let req = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: method.into(),
        params,
        id: 1,
    };

    let resp: JsonRpcResponse = client.post(rpc_url).json(&req).send().await?.json().await?;

    if let Some(err) = resp.error {
        anyhow::bail!("RPC error: {}", err);
    }

    resp.result.ok_or_else(|| anyhow::anyhow!("No result from RPC"))
}

pub async fn verify_base_transaction(
    rpc_url: &str,
    tx_hash_hex: &str,
    expected_recipient: &str,
    token_address: &str,
    expected_amount: &str,
) -> Result<VerificationResult> {
    let client = reqwest::Client::new();

    // Get transaction receipt
    let receipt = rpc_call(
        &client,
        rpc_url,
        "eth_getTransactionReceipt",
        serde_json::json!([tx_hash_hex]),
    )
    .await?;

    let status = receipt
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("0x0");

    if status != "0x1" {
        return Ok(VerificationResult {
            valid: false,
            reason: Some("Transaction failed on-chain".into()),
            actual_amount: None,
            from: None,
            block_number: None,
        });
    }

    // Check confirmations
    let receipt_block_hex = receipt
        .get("blockNumber")
        .and_then(|b| b.as_str())
        .unwrap_or("0x0");
    let receipt_block = u64::from_str_radix(receipt_block_hex.trim_start_matches("0x"), 16).unwrap_or(0);

    let current_block_hex = rpc_call(
        &client,
        rpc_url,
        "eth_blockNumber",
        serde_json::json!([]),
    )
    .await?;
    let current_block_str = current_block_hex.as_str().unwrap_or("0x0");
    let current_block = u64::from_str_radix(current_block_str.trim_start_matches("0x"), 16).unwrap_or(0);

    if current_block.saturating_sub(receipt_block) < 2 {
        return Ok(VerificationResult {
            valid: false,
            reason: Some("Insufficient block confirmations (need >= 2)".into()),
            actual_amount: None,
            from: None,
            block_number: None,
        });
    }

    // Parse logs for matching Transfer event
    let logs = receipt
        .get("logs")
        .and_then(|l| l.as_array())
        .cloned()
        .unwrap_or_default();

    let token_addr_lower = token_address.to_lowercase();
    let expected_addr_lower = expected_recipient.to_lowercase();

    let matching_log = logs.iter().find(|log| {
        let addr = log.get("address").and_then(|a| a.as_str()).unwrap_or("").to_lowercase();
        let topics = log.get("topics").and_then(|t| t.as_array()).cloned().unwrap_or_default();

        if addr != token_addr_lower {
            return false;
        }

        // Check Transfer event topic
        let topic0 = topics.first().and_then(|t| t.as_str()).unwrap_or("").to_lowercase();
        if topic0 != TRANSFER_TOPIC {
            return false;
        }

        // Check recipient (topic[2], last 20 bytes of 32-byte topic)
        if let Some(topic2) = topics.get(2).and_then(|t| t.as_str()) {
            let to_addr = format!("0x{}", &topic2[26..]); // Last 20 bytes
            to_addr.to_lowercase() == expected_addr_lower
        } else {
            false
        }
    });

    let Some(log) = matching_log else {
        return Ok(VerificationResult {
            valid: false,
            reason: Some("No transfer to treasury address found".into()),
            actual_amount: None,
            from: None,
            block_number: None,
        });
    };

    // Parse amount from log data
    let data_hex = log.get("data").and_then(|d| d.as_str()).unwrap_or("0x0");
    let amount_hex = data_hex.trim_start_matches("0x");
    let actual_amount = u128::from_str_radix(amount_hex, 16).unwrap_or(0);
    let expected_parsed: u128 = expected_amount.parse().unwrap_or(0);

    if actual_amount < expected_parsed {
        return Ok(VerificationResult {
            valid: false,
            reason: Some(format!(
                "Insufficient amount: got {}, expected {}",
                actual_amount, expected_parsed
            )),
            actual_amount: Some(actual_amount.to_string()),
            from: None,
            block_number: None,
        });
    }

    // Extract sender from topic[1]
    let topics = log.get("topics").and_then(|t| t.as_array()).cloned().unwrap_or_default();
    let from = topics.get(1).and_then(|t| t.as_str()).map(|t| {
        format!("0x{}", &t[26..])
    });

    tracing::info!("Payment verified: {}... amount={}", &tx_hash_hex[..10], actual_amount);

    Ok(VerificationResult {
        valid: true,
        reason: None,
        actual_amount: Some(actual_amount.to_string()),
        from,
        block_number: Some(receipt_block),
    })
}
