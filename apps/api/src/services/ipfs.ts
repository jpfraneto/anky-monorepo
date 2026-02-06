import { Logger } from "../lib/logger.js";

const logger = Logger("IPFS");

const PINATA_API_URL = "https://api.pinata.cloud/pinning";

function getPinataJwt(): string | null {
  return process.env.PINATA_JWT || null;
}

export function getPinataGatewayUrl(hash: string): string {
  const gatewayDomain = process.env.PINATA_GATEWAY || "gateway.pinata.cloud";
  return `https://${gatewayDomain}/ipfs/${hash}`;
}

export async function uploadImageToIPFS(
  base64: string
): Promise<{ ipfsHash: string; gatewayUrl: string }> {
  const pinataJwt = getPinataJwt();
  if (!pinataJwt) throw new Error("PINATA_JWT not configured");

  const imageBuffer = Buffer.from(base64, "base64");
  const imageBlob = new Blob([imageBuffer], { type: "image/png" });
  const formData = new FormData();
  formData.append("file", imageBlob, `anky-image-${Date.now()}.png`);
  formData.append(
    "pinataMetadata",
    JSON.stringify({ name: `anky-image-${Date.now()}` })
  );

  const response = await fetch(`${PINATA_API_URL}/pinFileToIPFS`, {
    method: "POST",
    headers: { Authorization: `Bearer ${pinataJwt}` },
    body: formData,
  });

  if (!response.ok) {
    const text = await response.text();
    logger.error("Image upload failed", { status: response.status, body: text });
    throw new Error(`Pinata image upload failed: ${response.status}`);
  }

  const data = (await response.json()) as { IpfsHash: string };
  const ipfsHash = data.IpfsHash;

  logger.info(`Image uploaded to IPFS: ${ipfsHash}`);
  return { ipfsHash, gatewayUrl: getPinataGatewayUrl(ipfsHash) };
}

export async function uploadWritingToIPFS(
  content: string
): Promise<{ ipfsHash: string }> {
  const pinataJwt = getPinataJwt();
  if (!pinataJwt) throw new Error("PINATA_JWT not configured");

  const response = await fetch(`${PINATA_API_URL}/pinJSONToIPFS`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${pinataJwt}`,
    },
    body: JSON.stringify({
      pinataMetadata: { name: `anky-writing-${Date.now()}` },
      pinataContent: {
        writingSession: content,
        createdAt: new Date().toISOString(),
      },
    }),
  });

  if (!response.ok) {
    const text = await response.text();
    logger.error("Writing upload failed", { status: response.status, body: text });
    throw new Error(`Pinata writing upload failed: ${response.status}`);
  }

  const data = (await response.json()) as { IpfsHash: string };
  logger.info(`Writing uploaded to IPFS: ${data.IpfsHash}`);
  return { ipfsHash: data.IpfsHash };
}

export async function uploadMetadataToIPFS(params: {
  title: string;
  reflection: string;
  imageIpfsHash: string;
  writingIpfsHash: string;
  imagePrompt: string;
}): Promise<{ ipfsHash: string }> {
  const pinataJwt = getPinataJwt();
  if (!pinataJwt) throw new Error("PINATA_JWT not configured");

  const response = await fetch(`${PINATA_API_URL}/pinJSONToIPFS`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${pinataJwt}`,
    },
    body: JSON.stringify({
      pinataMetadata: { name: `anky-metadata-${Date.now()}` },
      pinataContent: {
        name: params.title,
        description: params.reflection,
        image: `ipfs://${params.imageIpfsHash}`,
        external_url: "https://anky.app",
        attributes: [
          { trait_type: "image_prompt", value: params.imagePrompt },
        ],
        properties: {
          writing_session: `ipfs://${params.writingIpfsHash}`,
        },
      },
    }),
  });

  if (!response.ok) {
    const text = await response.text();
    logger.error("Metadata upload failed", { status: response.status, body: text });
    throw new Error(`Pinata metadata upload failed: ${response.status}`);
  }

  const data = (await response.json()) as { IpfsHash: string };
  logger.info(`Metadata uploaded to IPFS: ${data.IpfsHash}`);
  return { ipfsHash: data.IpfsHash };
}
