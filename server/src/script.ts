#!/usr/bin/env node

import { Connection, PublicKey } from "@solana/web3.js";
import * as fs from "fs/promises";
import * as path from "path";
import fetch from "node-fetch"; // You might need to install this: npm install node-fetch

// Constants from your smart contract
const PROGRAM_ID = new PublicKey(
  "7UhisdAH7dosM1nfF1rbBXYv1Vtgr2yd6W4B7SuZJJVx"
);
const TREASURY_PUBKEY = new PublicKey(
  "6nJXxD7VQJpnpE3tdWmM9VjTnC5mB2oREeWh5B6EHuzK"
);
const MINT_PRICE_SOL = 0.006942;
const MAX_SUPPLY = 888;

// Configuration - UPDATE THESE WITH YOUR API KEYS
const CONFIG = {
  HELIUS_API_KEY: "2f9d82a6-d13a-4239-aa62-3c438c7ddb0f", // Your existing key
  NEYNAR_API_KEY: "C73E0EB2-5766-4FB7-BBAB-EA9C849354F8", // Replace with your actual key
  OUTPUT_DIR: "./data/wall_data",
};

// Output files
const OUTPUT_FILES = {
  WALLS_BY_FID: path.join(CONFIG.OUTPUT_DIR, "walls_by_fid.json"),
  WALLS_BY_SOLANA: path.join(CONFIG.OUTPUT_DIR, "walls_by_solana_address.json"),
  WALLS_BY_CAST_HASH: path.join(CONFIG.OUTPUT_DIR, "walls_by_cast_hash.json"),
  ACTIVE_WALLS: path.join(CONFIG.OUTPUT_DIR, "active_walls.json"),
  INACTIVE_WALLS: path.join(CONFIG.OUTPUT_DIR, "inactive_walls.json"),
  ALL_WALLS_DATA: path.join(CONFIG.OUTPUT_DIR, "all_walls_complete.json"),
};

// Types
interface FarcasterProfile {
  fid: number;
  username: string;
  display_name: string;
  pfp_url?: string;
  bio?: string;
  follower_count?: number;
  following_count?: number;
}

interface WallData {
  pda: string;
  owner: string;
  castHash: string;
  state: "Inactive" | "Active" | "Listed";
  price: number;
  isEmpty: boolean;
  index: number;
  farcasterProfile: FarcasterProfile | null;
  hasProfile: boolean;
  displayName: string;
  timestamp: number;
}

interface WallsByFid {
  [fid: string]: WallData[];
}

interface WallsBySolana {
  [solanaAddress: string]: WallData;
}

interface WallsByCastHash {
  [castHash: string]: WallData;
}

// Utility functions
function getHeliusRpcUrl(): string {
  return `https://mainnet.helius-rpc.com/?api-key=${CONFIG.HELIUS_API_KEY}`;
}

function deriveRegistryPda(): PublicKey {
  const [registryPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("registry")],
    PROGRAM_ID
  );
  return registryPda;
}

function truncateAddress(address: string, chars: number = 6): string {
  if (address.length <= chars * 2) return address;
  return `${address.slice(0, chars)}...${address.slice(-chars)}`;
}

function decodeWallAccount(
  data: Buffer,
  pda: string,
  index: number = -1
): WallData | null {
  try {
    if (data.length < 74) return null;

    let offset = 8; // Skip discriminator

    // owner: Pubkey (32 bytes)
    const ownerBytes = data.subarray(offset, offset + 32);
    const owner = new PublicKey(ownerBytes).toBase58();
    offset += 32;

    // cast_hash: [u8; 32] (32 bytes)
    const castHashBytes = data.subarray(offset, offset + 32);

    // Remove trailing zeros and convert to hex
    let trimmedBytes = castHashBytes;
    for (let i = castHashBytes.length - 1; i >= 0; i--) {
      if (castHashBytes[i] !== 0) {
        trimmedBytes = castHashBytes.subarray(0, i + 1);
        break;
      }
    }

    const castHash = "0x" + Buffer.from(trimmedBytes).toString("hex");
    const isEmpty = castHashBytes.every((byte) => byte === 0);
    offset += 32;

    // price: u64 (8 bytes)
    const priceInLamports = data.readBigUInt64LE(offset);
    const price = Number(priceInLamports) / 1_000_000_000; // Convert to SOL
    offset += 8;

    // state: u8 (1 byte)
    const stateNum = data.readUInt8(offset);
    const states = ["Inactive", "Active", "Listed"] as const;
    const state = states[stateNum] || "Inactive";

    return {
      pda,
      owner,
      castHash,
      state,
      price,
      isEmpty,
      index,
      farcasterProfile: null, // Will be filled later
      hasProfile: false, // Will be filled later
      displayName: truncateAddress(owner), // Will be updated later
      timestamp: Date.now(),
    };
  } catch (error) {
    console.error("Error decoding wall account:", error);
    return null;
  }
}

function decodeRegistryAccount(data: Buffer): { mintCount: number } | null {
  try {
    if (data.length < 67) return null;

    let offset = 8; // Skip discriminator
    offset += 32; // Skip authority
    offset += 32; // Skip treasury

    const mintCount = data.readUInt16LE(offset);
    return { mintCount };
  } catch (error) {
    console.error("Error decoding registry account:", error);
    return null;
  }
}

async function ensureOutputDir() {
  try {
    await fs.access(CONFIG.OUTPUT_DIR);
  } catch {
    await fs.mkdir(CONFIG.OUTPUT_DIR, { recursive: true });
    console.log(`📁 Created output directory: ${CONFIG.OUTPUT_DIR}`);
  }
}

async function fetchAllWalls(): Promise<WallData[]> {
  console.log("🔄 Fetching all walls from Solana...");

  const heliusUrl = getHeliusRpcUrl();
  const registryPda = deriveRegistryPda();

  // Fetch ALL walls with their account info
  const response = await fetch(heliusUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "getProgramAccounts",
      params: [
        PROGRAM_ID.toBase58(),
        {
          encoding: "base64",
          filters: [
            { dataSize: 82 }, // Wall account size
          ],
        },
      ],
    }),
  });

  const data = await response.json();
  const allWalls = data?.result ?? [];

  console.log(`📊 Found ${allWalls.length} walls on-chain`);

  const wallsWithIndices = [];

  // Extract index from PDA for each wall
  for (const { pubkey, account } of allWalls) {
    const wallBuf = Buffer.from(account.data[0], "base64");
    const wall = decodeWallAccount(wallBuf, pubkey);
    if (!wall) continue;

    // Find the index by trying different values
    let wallIndex = -1;
    for (let i = 0; i < MAX_SUPPLY; i++) {
      try {
        const [derivedPda] = PublicKey.findProgramAddressSync(
          [
            Buffer.from("wall"),
            registryPda.toBuffer(),
            new Uint8Array(new Uint16Array([i]).buffer),
          ],
          PROGRAM_ID
        );

        if (derivedPda.toBase58() === pubkey) {
          wallIndex = i;
          break;
        }
      } catch (e) {
        // Continue trying
      }
    }

    wall.index = wallIndex;
    wallsWithIndices.push(wall);
  }

  // Sort by index
  wallsWithIndices.sort((a, b) => a.index - b.index);

  console.log(`✅ Processed ${wallsWithIndices.length} walls with indices`);
  return wallsWithIndices;
}

async function fetchFarcasterProfiles(
  addresses: string[]
): Promise<Map<string, FarcasterProfile>> {
  const profileMap = new Map<string, FarcasterProfile>();

  if (
    addresses.length === 0 ||
    !CONFIG.NEYNAR_API_KEY ||
    CONFIG.NEYNAR_API_KEY === "YOUR_NEYNAR_API_KEY"
  ) {
    console.log("⚠️ Skipping Farcaster profiles - no API key configured");
    return profileMap;
  }

  try {
    console.log(
      `🎭 Fetching Farcaster profiles for ${addresses.length} addresses...`
    );

    // Process addresses in chunks of 300
    const CHUNK_SIZE = 300;
    for (let i = 0; i < addresses.length; i += CHUNK_SIZE) {
      const addressChunk = addresses.slice(i, i + CHUNK_SIZE);
      const addressParams = addressChunk.join("%2C");
      const url = `https://api.neynar.com/v2/farcaster/user/bulk-by-address?addresses=${addressParams}&viewer_fid=16098`;

      const response = await fetch(url, {
        method: "GET",
        headers: {
          "x-api-key": CONFIG.NEYNAR_API_KEY,
          "x-neynar-experimental": "false",
        },
      });

      if (!response.ok) {
        console.error(
          `❌ Neynar API error: ${response.status} ${response.statusText}`
        );
        continue;
      }

      const data = await response.json();

      // Map profiles to their Solana addresses
      for (const [solAddress, users] of Object.entries(data)) {
        const userArray = users as any[];
        if (userArray?.length > 0) {
          const user = userArray[0];
          profileMap.set(solAddress.toUpperCase(), {
            fid: user.fid,
            username: user.username,
            display_name: user.display_name,
            pfp_url: user.pfp_url,
            bio: user.profile?.bio?.text || "",
            follower_count: user.follower_count,
            following_count: user.following_count,
          });
        }
      }

      // Rate limiting
      if (i + CHUNK_SIZE < addresses.length) {
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
    }

    console.log(`🎭 Fetched ${profileMap.size} Farcaster profiles`);
    return profileMap;
  } catch (error) {
    console.error("💥 Error fetching Farcaster profiles:", error);
    return profileMap;
  }
}

async function checkCastExists(castHash: string): Promise<boolean> {
  if (
    !CONFIG.NEYNAR_API_KEY ||
    CONFIG.NEYNAR_API_KEY === "YOUR_NEYNAR_API_KEY"
  ) {
    return false;
  }

  if (
    !castHash ||
    castHash ===
      "0x0000000000000000000000000000000000000000000000000000000000000000"
  ) {
    return false;
  }

  try {
    const response = await fetch(
      `https://api.neynar.com/v2/farcaster/cast?identifier=${castHash}&type=hash&viewer_fid=16098`,
      {
        method: "GET",
        headers: {
          "x-api-key": CONFIG.NEYNAR_API_KEY,
          "x-neynar-experimental": "false",
        },
      }
    );

    return response.ok;
  } catch (error) {
    console.error(`Error checking cast ${castHash}:`, error);
    return false;
  }
}

async function validateActiveCasts(
  activeWalls: WallData[]
): Promise<WallData[]> {
  if (
    !CONFIG.NEYNAR_API_KEY ||
    CONFIG.NEYNAR_API_KEY === "YOUR_NEYNAR_API_KEY"
  ) {
    console.log("⚠️ Skipping cast validation - no API key configured");
    return activeWalls.map((wall) => ({ ...wall, castExists: false }));
  }

  console.log(`🔍 Validating casts for ${activeWalls.length} active walls...`);

  const wallsWithCastStatus = [];

  for (let i = 0; i < activeWalls.length; i++) {
    const wall = activeWalls[i];
    const castExists = await checkCastExists(wall?.castHash || "");

    wallsWithCastStatus.push({
      ...wall,
      castExists,
    });

    // Progress indicator
    if ((i + 1) % 100 === 0 || i === activeWalls.length - 1) {
      console.log(`   Validated ${i + 1}/${activeWalls.length} casts`);
    }

    // Rate limiting
    await new Promise((resolve) => setTimeout(resolve, 50));
  }

  const validCasts = wallsWithCastStatus.filter((w) => w.castExists).length;
  console.log(`✅ Found ${validCasts} active walls with valid casts`);

  return wallsWithCastStatus as WallData[];
}

function enhanceWallsWithProfiles(
  walls: WallData[],
  profiles: Map<string, FarcasterProfile>
): WallData[] {
  return walls.map((wall) => {
    const profile = profiles.get(wall.owner.toUpperCase());
    return {
      ...wall,
      farcasterProfile: profile || null,
      hasProfile: !!profile,
      displayName: profile
        ? `@${profile.username}`
        : truncateAddress(wall.owner),
    };
  });
}

function organizeWallsByFid(walls: WallData[]): WallsByFid {
  const wallsByFid: WallsByFid = {};

  for (const wall of walls) {
    if (wall.farcasterProfile?.fid) {
      const fid = wall.farcasterProfile.fid.toString();
      if (!wallsByFid[fid]) {
        wallsByFid[fid] = [];
      }
      wallsByFid[fid].push(wall);
    }
  }

  // Sort walls for each FID by priority
  for (const fid in wallsByFid) {
    wallsByFid[fid] = sortWallsByPriority(wallsByFid[fid] || []);
  }

  return wallsByFid;
}

function sortWallsByPriority(walls: WallData[]): WallData[] {
  return walls.sort((a, b) => {
    // Priority 1: Active walls with existing casts (castExists: true)
    if (
      a.state === "Active" &&
      (a as any).castExists &&
      !(b.state === "Active" && (b as any).castExists)
    ) {
      return -1;
    }
    if (
      b.state === "Active" &&
      (b as any).castExists &&
      !(a.state === "Active" && (a as any).castExists)
    ) {
      return 1;
    }

    // Priority 2: Other active walls
    if (a.state === "Active" && b.state !== "Active") {
      return -1;
    }
    if (b.state === "Active" && a.state !== "Active") {
      return 1;
    }

    // Priority 3: Listed walls
    if (a.state === "Listed" && b.state === "Inactive") {
      return -1;
    }
    if (b.state === "Listed" && a.state === "Inactive") {
      return 1;
    }

    // Priority 4: Within same category, sort by index (lower index first)
    return a.index - b.index;
  });
}

function organizeWallsBySolana(walls: WallData[]): WallsBySolana {
  const wallsBySolana: WallsBySolana = {};

  for (const wall of walls) {
    wallsBySolana[wall.owner] = wall;
  }

  return wallsBySolana;
}

function organizeWallsByCastHash(walls: WallData[]): WallsByCastHash {
  const wallsByCastHash: WallsByCastHash = {};

  for (const wall of walls) {
    // Only include walls that have a valid cast hash (not empty/zero hash)
    if (
      wall.castHash &&
      wall.castHash !==
        "0x0000000000000000000000000000000000000000000000000000000000000000"
    ) {
      wallsByCastHash[wall.castHash] = wall;
    }
  }

  return wallsByCastHash;
}

async function saveJsonFile(filePath: string, data: any, description: string) {
  try {
    await fs.writeFile(filePath, JSON.stringify(data, null, 2));
    console.log(`💾 Saved ${description} to ${filePath}`);
  } catch (error) {
    console.error(`❌ Failed to save ${description}:`, error);
  }
}

async function main() {
  try {
    console.log("🚀 Starting wall data extraction...");
    console.log(`📁 Output directory: ${CONFIG.OUTPUT_DIR}`);

    await ensureOutputDir();

    // Step 1: Fetch all walls from Solana
    const allWalls = await fetchAllWalls();

    if (allWalls.length === 0) {
      console.log("❌ No walls found. Exiting.");
      return;
    }

    // Step 2: Get unique owners
    const uniqueOwners = [...new Set(allWalls.map((w) => w.owner))];
    console.log(`👥 Found ${uniqueOwners.length} unique wall owners`);

    // Step 3: Fetch Farcaster profiles
    const profiles = await fetchFarcasterProfiles(uniqueOwners);

    // Step 4: Enhance walls with profile data
    const enhancedWalls = enhanceWallsWithProfiles(allWalls, profiles);

    // Step 5: Separate into active and inactive, and validate active casts
    const activeWalls = enhancedWalls.filter((wall) => wall.state === "Active");
    const inactiveWalls = enhancedWalls.filter(
      (wall) => wall.state === "Inactive"
    );
    const listedWalls = enhancedWalls.filter((wall) => wall.state === "Listed");

    // Validate casts for active walls
    const validatedActiveWalls = await validateActiveCasts(activeWalls);

    // Update the enhanced walls array with cast validation data
    const allWallsWithCastData = enhancedWalls.map((wall) => {
      if (wall.state === "Active") {
        const validatedWall = validatedActiveWalls.find(
          (v) => v.pda === wall.pda
        );
        return validatedWall || wall;
      }
      return wall;
    });

    console.log(`📊 Wall distribution:`);
    console.log(
      `  - Active: ${activeWalls.length} (${
        validatedActiveWalls.filter((w) => (w as any).castExists).length
      } with valid casts)`
    );
    console.log(`  - Inactive: ${inactiveWalls.length}`);
    console.log(`  - Listed: ${listedWalls.length}`);
    console.log(`  - Total: ${enhancedWalls.length}`);

    // Step 6: Organize data (now with prioritized sorting)
    const wallsByFid = organizeWallsByFid(allWallsWithCastData);
    const wallsBySolana = organizeWallsBySolana(allWallsWithCastData);
    const wallsByCastHash = organizeWallsByCastHash(allWallsWithCastData);

    console.log(`📋 Organization complete:`);
    console.log(`  - FIDs with walls: ${Object.keys(wallsByFid).length}`);
    console.log(`  - Solana addresses: ${Object.keys(wallsBySolana).length}`);
    console.log(
      `  - Valid cast hashes: ${Object.keys(wallsByCastHash).length}`
    );

    // Step 7: Save all files
    await Promise.all([
      saveJsonFile(
        OUTPUT_FILES.WALLS_BY_FID,
        wallsByFid,
        "walls organized by FID (prioritized)"
      ),
      saveJsonFile(
        OUTPUT_FILES.WALLS_BY_SOLANA,
        wallsBySolana,
        "walls organized by Solana address"
      ),
      saveJsonFile(
        OUTPUT_FILES.WALLS_BY_CAST_HASH,
        wallsByCastHash,
        "walls organized by cast hash"
      ),
      saveJsonFile(
        OUTPUT_FILES.ACTIVE_WALLS,
        validatedActiveWalls,
        "active walls (with cast validation)"
      ),
      saveJsonFile(
        OUTPUT_FILES.INACTIVE_WALLS,
        inactiveWalls,
        "inactive walls"
      ),
      saveJsonFile(
        OUTPUT_FILES.ALL_WALLS_DATA,
        {
          metadata: {
            timestamp: Date.now(),
            totalWalls: enhancedWalls.length,
            activeWalls: activeWalls.length,
            activeWallsWithValidCasts: validatedActiveWalls.filter(
              (w) => (w as any).castExists
            ).length,
            inactiveWalls: inactiveWalls.length,
            listedWalls: listedWalls.length,
            uniqueOwners: uniqueOwners.length,
            profilesFetched: profiles.size,
            wallsWithCastHashes: Object.keys(wallsByCastHash).length,
            generatedBy: "wall-data-extractor-script",
            sortingLogic:
              "Active walls with valid casts first, then other active walls, then listed, then inactive",
          },
          allWalls: allWallsWithCastData,
          wallsByFid,
          wallsBySolana,
          wallsByCastHash,
          activeWalls: validatedActiveWalls,
          inactiveWalls,
          listedWalls,
        },
        "complete wall dataset (with cast validation and prioritized sorting)"
      ),
    ]);

    // Step 8: Generate summary
    const summary = {
      extraction_completed: new Date().toISOString(),
      total_walls: enhancedWalls.length,
      wall_distribution: {
        active: activeWalls.length,
        active_with_valid_casts: validatedActiveWalls.filter(
          (w) => (w as any).castExists
        ).length,
        inactive: inactiveWalls.length,
        listed: listedWalls.length,
      },
      unique_owners: uniqueOwners.length,
      profiles_found: profiles.size,
      files_generated: Object.keys(OUTPUT_FILES).length + 1, // +1 for summary
      output_files: OUTPUT_FILES,
      sorting_logic: {
        description: "Walls for each FID are sorted by priority",
        priority_order: [
          "1. Active walls with existing/valid casts",
          "2. Other active walls",
          "3. Listed walls",
          "4. Inactive walls",
          "5. Within same category: sorted by index (lower first)",
        ],
      },
    };

    await saveJsonFile(
      path.join(CONFIG.OUTPUT_DIR, "extraction_summary.json"),
      summary,
      "extraction summary"
    );

    console.log("\n✅ Wall data extraction completed successfully!");
    console.log(`📁 All files saved to: ${CONFIG.OUTPUT_DIR}`);
    console.log(
      `🎯 You can now access wall data offline using the generated JSON files`
    );

    // Usage examples
    console.log("\n📖 Usage examples:");
    console.log(`
// Get walls for a specific FID:
const wallsByFid = JSON.parse(fs.readFileSync('${OUTPUT_FILES.WALLS_BY_FID}'));
const userWalls = wallsByFid['123']; // Replace 123 with actual FID

// Get wall by Solana address:
const wallsBySolana = JSON.parse(fs.readFileSync('${OUTPUT_FILES.WALLS_BY_SOLANA}'));
const userWall = wallsBySolana['SolanaAddressHere'];

// Get wall by cast hash:
const wallsByCastHash = JSON.parse(fs.readFileSync('${OUTPUT_FILES.WALLS_BY_CAST_HASH}'));
const wall = wallsByCastHash['0xabcdef123456...']; // Replace with actual cast hash

// Get all active walls:
const activeWalls = JSON.parse(fs.readFileSync('${OUTPUT_FILES.ACTIVE_WALLS}'));

// Get all inactive walls:
const inactiveWalls = JSON.parse(fs.readFileSync('${OUTPUT_FILES.INACTIVE_WALLS}'));
    `);
  } catch (error) {
    console.error("💥 Fatal error:", error);
    process.exit(1);
  }
}

main();
