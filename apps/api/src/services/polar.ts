import { Polar } from "@polar-sh/sdk";
import { Logger } from "../lib/logger.js";

const logger = Logger("Polar");

const POLAR_ACCESS_TOKEN = process.env.POLAR_ACCESS_TOKEN;
const POLAR_PRODUCT_ID = process.env.POLAR_PRODUCT_ID;

if (!POLAR_ACCESS_TOKEN) {
  logger.warn("POLAR_ACCESS_TOKEN not set - Polar.sh integration disabled");
}

const polar = POLAR_ACCESS_TOKEN
  ? new Polar({ accessToken: POLAR_ACCESS_TOKEN })
  : null;

export { polar, POLAR_PRODUCT_ID };

export async function createCheckout(
  userId: string,
  successUrl: string,
): Promise<string | null> {
  if (!polar || !POLAR_PRODUCT_ID) {
    logger.error("Polar.sh not configured");
    return null;
  }

  try {
    const checkout = await polar.checkouts.create({
      products: [POLAR_PRODUCT_ID],
      successUrl,
      metadata: { userId },
    });

    logger.info(`Checkout created for user ${userId.slice(0, 8)}...`);
    return checkout.url;
  } catch (error) {
    logger.error("Polar checkout error:", error);
    return null;
  }
}
