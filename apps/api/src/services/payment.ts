import { createPublicClient, http, parseAbiItem } from "viem";
import { base } from "viem/chains";
import { Logger } from "../lib/logger.js";

const logger = Logger("PaymentVerification");

const client = createPublicClient({
  chain: base,
  transport: http(process.env.BASE_RPC_URL || "https://mainnet.base.org"),
});

const ERC20_TRANSFER_EVENT = parseAbiItem(
  "event Transfer(address indexed from, address indexed to, uint256 value)"
);

export interface VerificationResult {
  valid: boolean;
  reason?: string;
  actualAmount?: string;
  from?: string;
  blockNumber?: bigint;
}

export async function verifyBaseTransaction(params: {
  txHash: `0x${string}`;
  expectedRecipient: `0x${string}`;
  method: "usdc" | "anky_token";
  expectedAmount: string;
  tokenAddress: `0x${string}`;
}): Promise<VerificationResult> {
  try {
    // Get transaction receipt
    const receipt = await client.getTransactionReceipt({
      hash: params.txHash,
    });

    if (!receipt) {
      return { valid: false, reason: "Transaction not found" };
    }

    if (receipt.status !== "success") {
      return { valid: false, reason: "Transaction failed on-chain" };
    }

    // Check block confirmations
    const currentBlock = await client.getBlockNumber();
    const confirmations = currentBlock - receipt.blockNumber;

    if (confirmations < 2n) {
      return { valid: false, reason: "Insufficient block confirmations (need >= 2)" };
    }

    // Parse Transfer events from the receipt
    const transferLogs = receipt.logs.filter((log) => {
      return (
        log.address.toLowerCase() === params.tokenAddress.toLowerCase() &&
        log.topics[0] === "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef" // Transfer event topic
      );
    });

    if (transferLogs.length === 0) {
      return { valid: false, reason: "No token transfer found in transaction" };
    }

    // Find transfer to the expected recipient
    const matchingTransfer = transferLogs.find((log) => {
      const toAddress = log.topics[2];
      if (!toAddress) return false;
      const to = ("0x" + toAddress.slice(26)).toLowerCase();
      return to === params.expectedRecipient.toLowerCase();
    });

    if (!matchingTransfer) {
      return { valid: false, reason: "No transfer to treasury address found" };
    }

    // Check amount
    const actualAmount = matchingTransfer.data;
    const actualBigInt = BigInt(actualAmount);
    const expectedBigInt = BigInt(params.expectedAmount);

    if (actualBigInt < expectedBigInt) {
      return {
        valid: false,
        reason: `Insufficient amount: got ${actualBigInt.toString()}, expected ${expectedBigInt.toString()}`,
        actualAmount: actualBigInt.toString(),
      };
    }

    // Extract sender
    const fromTopic = matchingTransfer.topics[1];
    const from = fromTopic ? ("0x" + fromTopic.slice(26)) as `0x${string}` : undefined;

    logger.info(`Payment verified: ${params.txHash.slice(0, 10)}... amount=${actualBigInt.toString()}`);

    return {
      valid: true,
      actualAmount: actualBigInt.toString(),
      from,
      blockNumber: receipt.blockNumber,
    };
  } catch (error) {
    logger.error("Payment verification error:", error);
    return {
      valid: false,
      reason: error instanceof Error ? error.message : "Verification failed",
    };
  }
}
