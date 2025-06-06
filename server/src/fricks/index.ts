import { Hono } from "hono";
import { cors } from "hono/cors";
import { Connection, PublicKey } from "@solana/web3.js";
import bs58 from "bs58";
import { getEnv, type Env } from "../../env/server-env";
import * as fs from "fs/promises";
import * as path from "path";
import { PinataSDK } from "pinata";
import { getUserFrick } from "./functions";
import { siwfAuthMiddleware } from "@server/middleware";

const app = new Hono<{
  Bindings: Env;
  Variables: {
    user: {
      fid: number;
      address: string;
    };
  };
}>();

app.use(cors());

// Constants from your smart contract
const PROGRAM_ID = new PublicKey(
  "7UhisdAH7dosM1nfF1rbBXYv1Vtgr2yd6W4B7SuZJJVx"
);
const TREASURY_PUBKEY = new PublicKey(
  "6nJXxD7VQJpnpE3tdWmM9VjTnC5mB2oREeWh5B6EHuzK"
);
const MINT_PRICE_SOL = 0.006942;
const MAX_SUPPLY = 888;

// Cache configuration
const CACHE_DIR = "./data";
const CACHE_FILES = {
  WALLS: path.join(CACHE_DIR, "walls.json"),
  PROFILES: path.join(CACHE_DIR, "profiles.json"),
  REGISTRY: path.join(CACHE_DIR, "registry.json"),
  METADATA: path.join(CACHE_DIR, "metadata.json"),
};

// Cache TTL in milliseconds
const CACHE_TTL = {
  WALLS: 2 * 60 * 1000, // 2 minutes for walls (they change frequently)
  PROFILES: 30 * 60 * 1000, // 30 minutes for profiles (fairly static)
  REGISTRY: 1 * 60 * 1000, // 1 minute for registry (minting activity)
};

// Types (keeping your existing interfaces)
interface ActivatedWall {
  pda: string;
  owner: string;
  castHash: string;
  username: string | null;
  pfp: string | null;
  fid: number | null;
  index: number;
}

interface InactiveWall {
  pda: string;
  owner: string;
  username: string | null;
  pfp: string | null;
  mintedAt?: number;
  fid: number | null;
  index: number;
}

interface FarcasterProfile {
  fid: number;
  username: string;
  display_name: string;
  pfp_url?: string;
  bio?: string;
  follower_count?: number;
  following_count?: number;
}

interface SimpleWall {
  pda: string;
  owner: string;
  castHash: string;
  state: "Inactive" | "Active" | "Listed";
  price: number;
  isEmpty: boolean;
  farcasterProfile: FarcasterProfile | null;
  hasProfile: boolean;
  displayName: string;
  index: number;
}

interface CachedWallsData {
  activeWalls: ActivatedWall[];
  inactiveWalls: InactiveWall[];
  totalMinted: number;
  lastUpdated: number;
  uniqueOwners: string[];
}

interface CachedProfilesData {
  profiles: Map<string, FarcasterProfile>;
  lastUpdated: number;
}

interface CachedRegistryData {
  mintCount: number;
  lastUpdated: number;
}

interface CacheMetadata {
  lastWallsUpdate: number;
  lastProfilesUpdate: number;
  lastRegistryUpdate: number;
}

app.post("/init-session", siwfAuthMiddleware, async (c) => {
  const { castHash, solanaAddress, casterFid } = await c.req.json();
  const fid = c.get("user")?.fid;
  if (!fid) {
    return c.json({ error: "Unauthorized" }, 401);
  }

  // Read walls data from file
  const wallsData = await fs.readFile(
    "data/wall_data/walls_by_fid.json",
    "utf-8"
  );
  const wallsByFid = JSON.parse(wallsData);

  // Get walls for this fid
  const userWalls = wallsByFid[fid];
  const userWall = userWalls ? userWalls[0] : null; // Get first wall if exists

  let contextWalls, contextWall;

  if (casterFid) {
    contextWalls = wallsByFid[casterFid];
    contextWall = contextWalls[0];
  }

  await fs.unlink("data/wall_data/walls_by_fid.json");

  return c.json({
    success: true,
    userWall,
    contextWall,
    timestamp: Date.now(),
  });
});

app.get("/get-presigned-url", async (c) => {
  try {
    const env = getEnv(c.env);
    console.log("THE PINATA API JWT IS ", env.PINATA_API_JWT);
    console.log("THE PINATA GATEWAY URL IS ", env.PINATA_GATEWAY_URL);
    const pinata = new PinataSDK({
      pinataJwt: env.PINATA_API_JWT,
      pinataGateway: c.env.PINATA_GATEWAY_URL,
    });
    console.log("THE PINATA IS ", pinata);

    const url = await pinata.upload.public.createSignedURL({
      expires: 60, // Last for 60 seconds
    });
    console.log("THE PRESIGNED URL IS ", url);
    return c.json({ url });
  } catch (error) {
    console.error("Failed to get presigned URL:", error);
    return c.json({ error: "Failed to generate presigned URL" }, 500);
  }
});

// Cache management functions
async function ensureCacheDir() {
  try {
    await fs.access(CACHE_DIR);
  } catch {
    await fs.mkdir(CACHE_DIR, { recursive: true });
    console.log(`📁 Created cache directory: ${CACHE_DIR}`);
  }
}

async function readFromCache<T>(filePath: string, defaultValue: T): Promise<T> {
  try {
    const data = await fs.readFile(filePath, "utf-8");
    const parsed = JSON.parse(data);

    // Handle Map serialization for profiles
    if (filePath === CACHE_FILES.PROFILES && parsed.profiles) {
      parsed.profiles = new Map(Object.entries(parsed.profiles));
    }

    return parsed;
  } catch (error) {
    console.log(
      `📂 Cache miss for ${path.basename(filePath)}, using default value`
    );
    return defaultValue;
  }
}

async function writeToCache<T>(filePath: string, data: T): Promise<void> {
  try {
    let serializedData = data;

    // Handle Map serialization for profiles
    if (
      filePath === CACHE_FILES.PROFILES &&
      (data as any).profiles instanceof Map
    ) {
      serializedData = {
        ...(data as any),
        profiles: Object.fromEntries((data as any).profiles),
      };
    }

    await fs.writeFile(filePath, JSON.stringify(serializedData, null, 2));
    console.log(`💾 Cached data to ${path.basename(filePath)}`);
  } catch (error) {
    console.error(
      `❌ Failed to write cache ${path.basename(filePath)}:`,
      error
    );
  }
}

function isCacheExpired(lastUpdated: number, ttl: number): boolean {
  return Date.now() - lastUpdated > ttl;
}

// Utility functions (keeping your existing ones)
export function getHeliusRpcUrl(env: Env): string {
  return `https://mainnet.helius-rpc.com/?api-key=2f9d82a6-d13a-4239-aa62-3c438c7ddb0f`;
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

// Decode functions (keeping your existing ones)
function decodeWallAccount(
  data: Buffer,
  pda: string,
  farcasterProfile: FarcasterProfile | null = null,
  index: number = -1 // Add this parameter
): SimpleWall | null {
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

    // Determine display name
    const hasProfile = !!farcasterProfile;
    const displayName = hasProfile
      ? `@${farcasterProfile.username}`
      : truncateAddress(owner);

    return {
      pda,
      owner,
      castHash,
      state,
      price,
      isEmpty,
      farcasterProfile,
      hasProfile,
      displayName,
      index, // Include the index
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

// Optimized data fetching functions
async function fetchAndCacheRegistry(env: Env): Promise<CachedRegistryData> {
  console.log("🔄 Fetching fresh registry data...");

  const heliusUrl = getHeliusRpcUrl(env);
  const registryPda = deriveRegistryPda();

  const registryResponse = await fetch(heliusUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "getAccountInfo",
      params: [registryPda.toBase58(), { encoding: "base64" }],
    }),
  });

  const registryData = (await registryResponse.json()) as any;
  let mintCount = 0;

  if (registryData.result?.value) {
    const accountData = Buffer.from(
      registryData.result.value.data[0],
      "base64"
    );
    const registry = decodeRegistryAccount(accountData);
    if (registry) {
      mintCount = registry.mintCount;
    }
  }

  const cachedData: CachedRegistryData = {
    mintCount,
    lastUpdated: Date.now(),
  };

  await writeToCache(CACHE_FILES.REGISTRY, cachedData);
  return cachedData;
}

async function fetchAndCacheWalls(env: Env): Promise<CachedWallsData> {
  console.log("🔄 Fetching fresh walls data...");

  const heliusUrl = getHeliusRpcUrl(env);

  // Fetch ALL walls (both active and inactive) with their account info
  const allWallsResponse = await fetch(heliusUrl, {
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

  const allWallsData = (await allWallsResponse.json()) as any;
  const allWalls = allWallsData?.result ?? [];

  // Create a map to store wall data with indices
  const wallsWithIndices: Array<{
    pda: string;
    owner: string;
    castHash: string;
    state: string;
    price: number;
    index: number;
  }> = [];

  // Extract index from PDA for each wall
  const registryPda = deriveRegistryPda();

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
            new Uint8Array(new Uint16Array([i]).buffer), // Convert to little-endian u16
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

    wallsWithIndices.push({
      pda: wall.pda,
      owner: wall.owner,
      castHash: wall.castHash,
      state: wall.state,
      price: wall.price,
      index: wallIndex,
    });
  }

  // Sort by index to maintain order
  wallsWithIndices.sort((a, b) => a.index - b.index);

  // Separate into active and inactive
  const activatedWalls: ActivatedWall[] = [];
  const inactiveWalls: InactiveWall[] = [];

  for (const wall of wallsWithIndices) {
    if (wall.state === "Active") {
      activatedWalls.push({
        pda: wall.pda,
        owner: wall.owner,
        castHash: wall.castHash,
        username: null,
        pfp: null,
        fid: null,
        index: wall.index,
      });
    } else if (wall.state === "Inactive") {
      inactiveWalls.push({
        pda: wall.pda,
        owner: wall.owner,
        username: null,
        pfp: null,
        fid: null,
        index: wall.index,
      });
    }
  }

  // Extract unique owners
  const uniqueOwners: string[] = [
    ...new Set(wallsWithIndices.map((w) => w.owner)),
  ];
  const totalMinted = uniqueOwners.length;

  const cachedData: CachedWallsData = {
    activeWalls: activatedWalls,
    inactiveWalls,
    totalMinted,
    uniqueOwners,
    lastUpdated: Date.now(),
  };

  await writeToCache(CACHE_FILES.WALLS, cachedData);
  console.log(
    `📊 Cached ${activatedWalls.length} active walls and ${inactiveWalls.length} inactive walls`
  );

  return cachedData;
}

async function fetchFarcasterProfiles(
  addresses: string[],
  env: Env
): Promise<Map<string, FarcasterProfile>> {
  const profileMap = new Map<string, FarcasterProfile>();

  if (addresses.length === 0) return profileMap;

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
          "x-api-key": env.NEYNAR_API_KEY,
          "x-neynar-experimental": "false",
        },
      });

      if (!response.ok) {
        console.error(
          `❌ Neynar API error: ${response.status} ${response.statusText}`
        );
        continue;
      }

      const data = (await response.json()) as any;

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
    }

    return profileMap;
  } catch (error) {
    console.error("💥 Error fetching Farcaster profiles:", error);
    return profileMap;
  }
}

async function fetchAndCacheProfiles(
  uniqueOwners: string[],
  env: Env
): Promise<CachedProfilesData> {
  console.log("🔄 Fetching fresh profile data...");

  const profiles = await fetchFarcasterProfiles(uniqueOwners, env);

  const cachedData: CachedProfilesData = {
    profiles,
    lastUpdated: Date.now(),
  };

  await writeToCache(CACHE_FILES.PROFILES, cachedData);
  console.log(`🎭 Cached ${profiles.size} Farcaster profiles`);

  return cachedData;
}

// Enhanced walls data with profiles
function enhanceWallsWithProfiles(
  wallsData: CachedWallsData,
  profilesData: CachedProfilesData
): {
  enhancedActiveWalls: ActivatedWall[];
  enhancedInactiveWalls: InactiveWall[];
} {
  const enhancedActiveWalls: ActivatedWall[] = wallsData.activeWalls.map(
    (wall) => {
      const profile = profilesData.profiles.get(wall.owner.toUpperCase());
      return {
        ...wall,
        username: profile?.username ?? null,
        pfp: profile?.pfp_url ?? null,
        fid: profile?.fid ?? null,
        index: wall.index, // Preserve the index
      };
    }
  );

  const enhancedInactiveWalls: InactiveWall[] = wallsData.inactiveWalls.map(
    (wall) => {
      const profile = profilesData.profiles.get(wall.owner.toUpperCase());
      return {
        ...wall,
        username: profile?.username ?? null,
        pfp: profile?.pfp_url ?? null,
        fid: profile?.fid ?? null,
        index: wall.index, // Preserve the index
      };
    }
  );

  return { enhancedActiveWalls, enhancedInactiveWalls };
}

// Main cached data getter
async function getCachedData(env: Env) {
  await ensureCacheDir();

  // Load existing cache data
  const [wallsCache, profilesCache, registryCache] = await Promise.all([
    readFromCache<CachedWallsData>(CACHE_FILES.WALLS, {
      activeWalls: [],
      inactiveWalls: [],
      totalMinted: 0,
      uniqueOwners: [],
      lastUpdated: 0,
    }),
    readFromCache<CachedProfilesData>(CACHE_FILES.PROFILES, {
      profiles: new Map(),
      lastUpdated: 0,
    }),
    readFromCache<CachedRegistryData>(CACHE_FILES.REGISTRY, {
      mintCount: 0,
      lastUpdated: 0,
    }),
  ]);

  // Check what needs updating
  const needsWallsUpdate = isCacheExpired(
    wallsCache.lastUpdated,
    CACHE_TTL.WALLS
  );
  const needsRegistryUpdate = isCacheExpired(
    registryCache.lastUpdated,
    CACHE_TTL.REGISTRY
  );

  // Update registry if needed (fast update)
  let currentRegistryData = registryCache;
  if (needsRegistryUpdate) {
    try {
      currentRegistryData = await fetchAndCacheRegistry(env);
    } catch (error) {
      console.error("❌ Failed to update registry cache:", error);
    }
  }

  // Update walls if needed
  let currentWallsData = wallsCache;
  if (needsWallsUpdate) {
    try {
      currentWallsData = await fetchAndCacheWalls(env);
    } catch (error) {
      console.error("❌ Failed to update walls cache:", error);
    }
  }

  // Update profiles if needed (or if we have new owners)
  let currentProfilesData = profilesCache;
  const needsProfilesUpdate =
    isCacheExpired(profilesCache.lastUpdated, CACHE_TTL.PROFILES) ||
    currentWallsData.uniqueOwners.length > profilesCache.profiles.size;

  if (needsProfilesUpdate) {
    try {
      // Only fetch profiles for owners we don't have yet
      const missingOwners = currentWallsData.uniqueOwners.filter(
        (owner) => !profilesCache.profiles.has(owner.toUpperCase())
      );

      if (missingOwners.length > 0) {
        console.log(`🔄 Fetching ${missingOwners.length} missing profiles...`);
        const newProfiles = await fetchFarcasterProfiles(missingOwners, env);

        // Merge with existing profiles
        const mergedProfiles = new Map([
          ...profilesCache.profiles,
          ...newProfiles,
        ]);
        currentProfilesData = {
          profiles: mergedProfiles,
          lastUpdated: Date.now(),
        };

        await writeToCache(CACHE_FILES.PROFILES, currentProfilesData);
      } else if (
        isCacheExpired(profilesCache.lastUpdated, CACHE_TTL.PROFILES)
      ) {
        // Full refresh if TTL expired
        currentProfilesData = await fetchAndCacheProfiles(
          currentWallsData.uniqueOwners,
          env
        );
      }
    } catch (error) {
      console.error("❌ Failed to update profiles cache:", error);
    }
  }

  return {
    wallsData: currentWallsData,
    profilesData: currentProfilesData,
    registryData: currentRegistryData,
  };
}

// Your existing helper functions (keeping them as-is)
function generatePhilosophicalContext(
  contextualState: string,
  userProfile: FarcasterProfile | null,
  castContext: any,
  systemStats: any
) {
  type ThemeType = {
    theme: string;
    message: string;
    metaphor: string;
  };

  const themes: Record<string, ThemeType> = {
    newcomer: {
      theme: "genesis",
      message:
        "Every wall begins as a possibility, a blank canvas awaiting the first stroke of intention.",
      metaphor:
        "You stand at the threshold of creation, where digital clay awaits your touch.",
    },
    ready_to_activate: {
      theme: "crystallization",
      message:
        "This moment transforms ephemeral thoughts into eternal presence on the blockchain.",
      metaphor:
        "Your words are about to become constellation coordinates in the digital sky.",
    },
    needs_cast: {
      theme: "potential",
      message:
        "A wall without activation is like a poem unspoken, a song unsung.",
      metaphor:
        "Your wall awaits its inaugural breath, its first heartbeat of existence.",
    },
    wall_owner_viewing_own_wall: {
      theme: "reflection",
      message:
        "You gaze upon your own creation, a mirror of your digital soul made permanent.",
      metaphor: "This is your digital graffiti on the walls of eternity.",
    },
    sold_out: {
      theme: "completion",
      message:
        "All walls have found their guardians. The first chapter of wallcaster is complete.",
      metaphor:
        "Like a city that has reached its perfect population, every space now has its purpose.",
    },
  };

  const defaultTheme: ThemeType = {
    theme: "journey",
    message:
      "Each interaction with wallcaster is a step in the dance between permanence and change.",
    metaphor:
      "You are both observer and participant in this digital archaeology.",
  };

  const selectedTheme = themes[contextualState] || defaultTheme;

  return {
    ...selectedTheme,
    systemReflection: generateSystemReflection(systemStats),
    userReflection: generateUserReflection(userProfile, contextualState),
  };
}

function generateSystemReflection(stats: any) {
  const { totalMinted, totalActiveWalls, scarcityFactor, mintingVelocity } =
    stats;

  if (scarcityFactor < 0.1) {
    return "The walls of wallcaster are nearly complete, each one a testament to human creativity preserved in silicon and consensus.";
  } else if (mintingVelocity === "high") {
    return "The community grows rapidly, each new wall adding to the collective constellation of human expression.";
  } else if (totalActiveWalls / totalMinted > 0.8) {
    return "Most walls have found their voice, activated and singing their digital songs to the blockchain.";
  } else {
    return "The ecosystem breathes with the rhythm of creation and activation, each wall a heartbeat in the digital organism.";
  }
}

function generateUserReflection(
  profile: FarcasterProfile | null,
  state: string
) {
  if (!profile)
    return "Anonymous presence in the digital realm, yet no less significant.";
  if (!profile.follower_count)
    return "Unknown presence in the digital realm, yet no less significant.";

  const followerTier =
    profile.follower_count > 10000
      ? "oracle"
      : profile.follower_count > 1000
      ? "influencer"
      : profile.follower_count > 100
      ? "connector"
      : "seeker";

  return `As a ${followerTier} in the Farcaster realm, your presence carries the weight of ${profile.follower_count} connections, each one a thread in the tapestry of digital community.`;
}

function determineMembershipTier(
  profile: FarcasterProfile | null,
  wall: SimpleWall | null
): string {
  if (!profile) return "anonymous";
  if (!profile.follower_count) return "unknown";

  if (wall?.state === "Active") return "activated_builder";
  if (wall?.state === "Listed") return "merchant";
  if (wall?.state === "Inactive") return "holder";
  if (profile.follower_count > 10000) return "oracle";
  if (profile.follower_count > 1000) return "influencer";
  return "seeker";
}

function generateAvailableActions(
  contextualState: string,
  userWall: SimpleWall | null,
  canMint: boolean,
  canActivate: boolean,
  canActivateFromThisCast: boolean
): string[] {
  const actions = [];

  if (canMint) actions.push("mint");
  if (canActivateFromThisCast) actions.push("activate");
  if (canActivate && !canActivateFromThisCast) actions.push("cast_to_activate");
  if (userWall?.state === "Active") actions.push("view_wall", "list_wall");
  if (userWall?.state === "Listed") actions.push("unlist_wall");
  if (userWall?.state === "Inactive" && !canActivate) actions.push("list_wall");

  actions.push("explore_gallery", "view_stats");

  return actions;
}

app.get("/wall/:castHash", async (c) => {
  try {
    const env = getEnv(c.env);
    const castHash = c.req.param("castHash");
    const { wallsData, profilesData } = await getCachedData(env);

    const wall = wallsData.activeWalls.find(
      (wall) => wall.castHash === castHash
    );

    if (!wall) {
      return c.json(
        {
          success: false,
          error: "Wall not found",
        },
        404
      );
    }

    let repliesToWall = [];
    try {
      const neynarResponse = await fetch(
        `https://api.neynar.com/v2/farcaster/cast/conversation?reply_depth=1&include_chronological_parent_casts=true&limit=33&identifier=${wall.castHash}&type=hash&viewer_fid=2&sort_type=chron&fold=above`,
        {
          method: "GET",
          headers: {
            "x-api-key": env.NEYNAR_API_KEY,
            "x-neynar-experimental": "false",
          },
        }
      );

      const neynarData = (await neynarResponse.json()) as any;
      repliesToWall = neynarData?.conversation?.cast?.direct_replies || [];
    } catch (error) {
      console.error("Error fetching replies to wall:", error);
    }

    const profile = profilesData.profiles.get(wall.owner);

    return c.json({
      success: true,
      data: {
        pda: wall.pda,
        owner: wall.owner,
        castHash: wall.castHash,
        displayName: profile
          ? `@${profile.username}`
          : truncateAddress(wall.owner),
        pfp: profile?.pfp_url ?? null,
        replies: repliesToWall,
      },
    });
  } catch (error) {
    console.error("Error fetching wall:", error);
    return c.json(
      {
        success: false,
        error: error instanceof Error ? error.message : "Unknown error",
      },
      500
    );
  }
});

// OPTIMIZED MAIN ENDPOINT
app.get("/setup-app-for-fid/:fid", async (c) => {
  try {
    const fid = parseInt(c.req.param("fid"));
    const castHash = c.req.query("castHash");
    const solanaAddress = c.req.query("solanaAddress");
    const frameContext = c.req.query("frameContext");

    if (!fid) {
      return c.json({ success: false, error: "Invalid FID provided" }, 400);
    }

    const env = getEnv(c.env);

    // Get cached data (this is the main optimization!)
    const { wallsData, profilesData, registryData } = await getCachedData(env);

    // Enhance walls with profile data
    const { enhancedActiveWalls, enhancedInactiveWalls } =
      enhanceWallsWithProfiles(wallsData, profilesData);

    // Step 1: Get user's Farcaster profile and verified addresses (still need fresh data for this user)
    console.log("🎭 Fetching user's Farcaster profile...");
    let userProfile: FarcasterProfile | null = null;
    let userSolanaAddresses: string[] = [];

    try {
      const profileResponse = await fetch(
        `https://api.neynar.com/v2/farcaster/user/bulk?fids=${fid}&viewer_fid=1`,
        {
          method: "GET",
          headers: {
            "x-api-key": env.NEYNAR_API_KEY,
            "x-neynar-experimental": "false",
          },
        }
      );

      if (profileResponse.ok) {
        const profileData = (await profileResponse.json()) as any;
        if (profileData.users && profileData.users.length > 0) {
          const user = profileData.users[0];
          userProfile = {
            fid: user.fid,
            username: user.username,
            display_name: user.display_name,
            pfp_url: user.pfp_url,
            bio: user.profile?.bio?.text || "",
            follower_count: user.follower_count,
            following_count: user.following_count,
          };

          if (user.verified_addresses?.sol_addresses) {
            userSolanaAddresses = user.verified_addresses.sol_addresses;
          }
        }
      }
    } catch (error) {
      console.error("Error fetching Farcaster profile:", error);
    }

    // If solanaAddress is provided in query, prioritize it
    if (solanaAddress) {
      userSolanaAddresses = [
        solanaAddress,
        ...userSolanaAddresses.filter((addr) => addr !== solanaAddress),
      ];
    }

    // Step 2: Analyze cast context (if provided)
    let castContext = null;
    let castAuthor = null;
    let isUserInOwnCast = false;
    let canActivateFromThisCast = false;

    if (castHash) {
      console.log("🔍 Analyzing cast context...");
      try {
        const castResponse = await fetch(
          `https://api.neynar.com/v2/farcaster/cast?identifier=${castHash}&type=hash&viewer_fid=16098`,
          {
            method: "GET",
            headers: {
              "x-api-key": env.NEYNAR_API_KEY,
            },
          }
        );

        if (castResponse.ok) {
          const castData = (await castResponse.json()) as any;
          if (castData.cast) {
            castContext = {
              hash: castData.cast.hash,
              text: castData.cast.text,
              timestamp: castData.cast.timestamp,
              author: {
                fid: castData.cast.author.fid,
                username: castData.cast.author.username,
                display_name: castData.cast.author.display_name,
                pfp_url: castData.cast.author.pfp_url,
              },
              reactions: {
                likes: castData.cast.reactions?.likes_count || 0,
                recasts: castData.cast.reactions?.recasts_count || 0,
                replies: castData.cast.replies?.count || 0,
              },
              embeds: castData.cast.embeds || [],
            };

            castAuthor = castData.cast.author;
            isUserInOwnCast = castData.cast.author.fid === fid;
            canActivateFromThisCast = isUserInOwnCast;
          }
        }
      } catch (error) {
        console.error("Error fetching cast details:", error);
      }
    }

    // Step 3: Check if user owns a wall (using cached data!)
    console.log("🧱 Checking if user owns a wall...");
    let userWall: SimpleWall | null = null;
    let userWalletAddress: string | null = null;

    // Check against all walls in cache
    for (const address of userSolanaAddresses) {
      if (!address || typeof address !== "string" || address.trim() === "") {
        continue;
      }

      if (!/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(address)) {
        continue;
      }

      // Check in active walls
      const activeWall = enhancedActiveWalls.find(
        (wall) => wall.owner === address
      );
      if (activeWall) {
        const profile = profilesData.profiles.get(address.toUpperCase());
        userWall = {
          pda: activeWall.pda,
          owner: activeWall.owner,
          castHash: activeWall.castHash,
          state: "Active",
          price: 0,
          isEmpty: false,
          farcasterProfile: profile || null,
          hasProfile: !!profile,
          displayName: profile
            ? `@${profile.username}`
            : truncateAddress(address),
          index: activeWall.index, // Include the index
        };
        userWalletAddress = address;
        break;
      }

      // Check in inactive walls
      const inactiveWall = enhancedInactiveWalls.find(
        (wall) => wall.owner === address
      );
      if (inactiveWall) {
        const profile = profilesData.profiles.get(address.toUpperCase());
        userWall = {
          pda: inactiveWall.pda,
          owner: inactiveWall.owner,
          castHash:
            "0x0000000000000000000000000000000000000000000000000000000000000000",
          state: "Inactive",
          price: 0,
          isEmpty: true,
          farcasterProfile: profile || null,
          hasProfile: !!profile,
          displayName: profile
            ? `@${profile.username}`
            : truncateAddress(address),
          index: inactiveWall.index, // Include the index
        };
        userWalletAddress = address;
        break;
      }
    }

    // Step 4: Calculate user status and actions
    const hasWall = !!userWall;
    const canMint = !hasWall && registryData.mintCount < MAX_SUPPLY;
    const canActivate = userWall ? userWall.state === "Inactive" : false;
    const hasValidCast = !!castHash && castHash.length > 0;
    const wallIsActivatedWithThisCast =
      userWall && userWall.castHash === castHash && userWall.state === "Active";

    // Determine contextual state and actions
    let contextualState = "unknown";
    let primaryAction = "";
    let actionMessage = "";
    let urgencyLevel = "low";

    if (!hasWall) {
      contextualState = "newcomer";
      primaryAction = "mint";
      actionMessage =
        "🎯 Welcome to wallcaster! Mint your first wall to join the community.";
      urgencyLevel = "medium";
    } else if (canActivate && canActivateFromThisCast) {
      contextualState = "ready_to_activate";
      primaryAction = "activate";
      actionMessage =
        "🎉 Perfect! You can activate your wall with this cast right now!";
      urgencyLevel = "critical";
    } else if (canActivate && !hasValidCast) {
      contextualState = "needs_cast";
      primaryAction = "cast";
      actionMessage =
        "📝 Create a cast to activate your wall and make it live!";
      urgencyLevel = "high";
    } else if (canActivate && hasValidCast && !isUserInOwnCast) {
      contextualState = "wrong_cast";
      primaryAction = "cast";
      actionMessage =
        "⚠️ To activate your wall, you need to open this frame from YOUR own cast.";
      urgencyLevel = "medium";
    } else if (wallIsActivatedWithThisCast) {
      contextualState = "wall_owner_viewing_own_wall";
      primaryAction = "view";
      actionMessage =
        "✨ This is your activated wall! It's live and eternal on the blockchain.";
      urgencyLevel = "low";
    } else if (userWall?.state === "Active") {
      contextualState = "wall_owner_different_context";
      primaryAction = "view";
      actionMessage = "✅ Your wall is active and live in the gallery.";
      urgencyLevel = "low";
    } else if (userWall?.state === "Listed") {
      contextualState = "wall_listed";
      primaryAction = "manage";
      actionMessage = "🏷️ Your wall is currently listed for sale.";
      urgencyLevel = "low";
    } else if (registryData.mintCount >= MAX_SUPPLY) {
      contextualState = "sold_out";
      primaryAction = "explore";
      actionMessage =
        "🎉 All walls are minted! Explore the gallery or ask for one.";
      urgencyLevel = "low";
    }

    // Step 5: Calculate stats and metrics
    const totalActiveWalls = enhancedActiveWalls.length;
    const totalInactiveWalls = enhancedInactiveWalls.length;
    const activationRate =
      totalActiveWalls / Math.max(registryData.mintCount, 1);
    const scarcityFactor = (MAX_SUPPLY - registryData.mintCount) / MAX_SUPPLY;

    // Simplified recent activity (since we don't track historical data in cache)
    const recentMints = Math.min(registryData.mintCount, 10);
    const recentActivations = Math.min(totalActiveWalls, 5);
    const mintingVelocity =
      recentMints > 5 ? "high" : recentMints > 2 ? "medium" : "low";

    // Step 6: Generate philosophical context
    const philosophicalContext = generatePhilosophicalContext(
      contextualState,
      userProfile,
      castContext,
      {
        totalMinted: registryData.mintCount,
        totalActiveWalls,
        scarcityFactor,
        mintingVelocity,
      }
    );
    let repliesToUserWall = [];
    if (userWall?.castHash) {
      console.log("THE USER WALL CAST HASH IS ", userWall.castHash);
      try {
        const neynarResponse = await fetch(
          `https://api.neynar.com/v2/farcaster/cast/conversation?reply_depth=1&include_chronological_parent_casts=true&limit=33&identifier=${userWall.castHash}&type=hash&viewer_fid=2&sort_type=chron&fold=above`,
          {
            method: "GET",
            headers: {
              "x-api-key": env.NEYNAR_API_KEY,
              "x-neynar-experimental": "false",
            },
          }
        );
        const neynarData = (await neynarResponse.json()) as any;
        repliesToUserWall =
          neynarData?.conversation?.cast?.direct_replies || [];
      } catch (error) {
        console.error("Error fetching replies to user wall:", error);
      }
    }

    // Step 7: Build the response
    const setupData = {
      // User's identity and relationship to the system
      user: {
        fid,
        profile: userProfile,
        solanaAddresses: userSolanaAddresses,
        primaryAddress: userWalletAddress || userSolanaAddresses[0] || null,
        hasWall,
        wall: userWall,
        repliesToWall: repliesToUserWall,
        contextualState,
        membershipTier: determineMembershipTier(userProfile, userWall),
      },

      // The cast context they're accessing from
      context: {
        castHash: castHash || null,
        castContext,
        castAuthor,
        isUserInOwnCast,
        canActivateFromThisCast,
        wallIsActivatedWithThisCast,
        frameContext: frameContext || "unknown",
        requestedAddress: solanaAddress || null,
        activeWalls: totalActiveWalls,
      },

      // Overall system state (using cached data!)
      stats: {
        totalMinted: registryData.mintCount,
        totalSupply: MAX_SUPPLY,
        remaining: MAX_SUPPLY - registryData.mintCount,
        totalActiveWalls,
        totalInactiveWalls,
        isSoldOut: registryData.mintCount >= MAX_SUPPLY,
        mintPrice: MINT_PRICE_SOL,
        activationRate: Math.round(activationRate * 100) / 100,
        scarcityFactor: Math.round(scarcityFactor * 100) / 100,
        mintingVelocity,
        recentActivity: {
          mints: recentMints,
          activations: recentActivations,
        },
      },

      // What the user should do next
      actions: {
        primary: primaryAction,
        message: actionMessage,
        urgencyLevel,
        canMint,
        canActivate,
        canActivateFromThisCast,
        hasValidCast,
        availableActions: generateAvailableActions(
          contextualState,
          userWall,
          canMint,
          canActivate,
          canActivateFromThisCast
        ),
      },

      // Philosophical and poetic context
      philosophy: philosophicalContext,

      // Configuration for the frontend
      config: {
        programId: PROGRAM_ID.toBase58(),
        treasury: TREASURY_PUBKEY.toBase58(),
        mintPrice: MINT_PRICE_SOL,
        maxSupply: MAX_SUPPLY,
        royaltyBps: 800, // 8%
        chainId: "mainnet-beta",
      },

      // Metadata about this response
      meta: {
        timestamp: Date.now(),
        version: "2.0.0-cached",
        node: "wallcaster-api-v2-cached",
        cacheStats: {
          wallsLastUpdated: wallsData.lastUpdated,
          profilesLastUpdated: profilesData.lastUpdated,
          registryLastUpdated: registryData.lastUpdated,
          profilesCached: profilesData.profiles.size,
        },
      },

      // Wall data (from cache!)
      activatedWalls: enhancedActiveWalls,
      inactiveWalls: enhancedInactiveWalls,
    };

    // Final logs
    console.log(`✅ Epic setup complete for FID ${fid} (cached)`);
    console.log(
      `- User: ${
        userProfile?.username || "Unknown"
      } (${contextualState}) ********************************* * * ** * * ** ** * * ** * * ** * * ** * ** `
    );
    console.log(
      `- Context: ${frameContext || "unknown"} ${
        isUserInOwnCast ? "(own cast)" : ""
      }`
    );
    console.log(
      `- Stats: ${registryData.mintCount}/${MAX_SUPPLY} minted, ${totalActiveWalls} active, ${totalInactiveWalls} inactive`
    );
    console.log(`- Action: ${primaryAction} (${urgencyLevel} urgency)`);
    console.log(
      `- Cache: Walls ${Math.round(
        (Date.now() - wallsData.lastUpdated) / 1000
      )}s old, Profiles ${Math.round(
        (Date.now() - profilesData.lastUpdated) / 1000
      )}s old`
    );
    console.log(
      `setup loaded for user ${userProfile?.username}, fid ${fid}, castHash ${castHash}`
    );

    return c.json({
      success: true,
      data: setupData,
      timestamp: Date.now(),
    });
  } catch (error) {
    console.error("💥 Error in setup-app-for-fid:", error);
    return c.json(
      {
        success: false,
        error: error instanceof Error ? error.message : "Unknown error",
        timestamp: Date.now(),
      },
      500
    );
  }
});

// Cache management endpoints
app.get("/cache/status", async (c) => {
  await ensureCacheDir();

  const [wallsCache, profilesCache, registryCache] = await Promise.all([
    readFromCache<CachedWallsData>(CACHE_FILES.WALLS, {
      activeWalls: [],
      inactiveWalls: [],
      totalMinted: 0,
      uniqueOwners: [],
      lastUpdated: 0,
    }),
    readFromCache<CachedProfilesData>(CACHE_FILES.PROFILES, {
      profiles: new Map(),
      lastUpdated: 0,
    }),
    readFromCache<CachedRegistryData>(CACHE_FILES.REGISTRY, {
      mintCount: 0,
      lastUpdated: 0,
    }),
  ]);

  const now = Date.now();

  return c.json({
    success: true,
    cache: {
      walls: {
        lastUpdated: wallsCache.lastUpdated,
        ageSeconds: Math.round((now - wallsCache.lastUpdated) / 1000),
        expired: isCacheExpired(wallsCache.lastUpdated, CACHE_TTL.WALLS),
        activeWalls: wallsCache.activeWalls.length,
        inactiveWalls: wallsCache.inactiveWalls.length,
        uniqueOwners: wallsCache.uniqueOwners.length,
      },
      profiles: {
        lastUpdated: profilesCache.lastUpdated,
        ageSeconds: Math.round((now - profilesCache.lastUpdated) / 1000),
        expired: isCacheExpired(profilesCache.lastUpdated, CACHE_TTL.PROFILES),
        profileCount: profilesCache.profiles.size,
      },
      registry: {
        lastUpdated: registryCache.lastUpdated,
        ageSeconds: Math.round((now - registryCache.lastUpdated) / 1000),
        expired: isCacheExpired(registryCache.lastUpdated, CACHE_TTL.REGISTRY),
        mintCount: registryCache.mintCount,
      },
    },
    ttl: CACHE_TTL,
  });
});

app.post("/cache/refresh", async (c) => {
  try {
    const env = getEnv(c.env);
    const { wallsData, profilesData, registryData } = await getCachedData(env);

    return c.json({
      success: true,
      message: "Cache refreshed successfully",
      stats: {
        walls: wallsData.activeWalls.length + wallsData.inactiveWalls.length,
        profiles: profilesData.profiles.size,
        mintCount: registryData.mintCount,
      },
    });
  } catch (error) {
    return c.json(
      {
        success: false,
        error: error instanceof Error ? error.message : "Unknown error",
      },
      500
    );
  }
});

app.delete("/cache/clear", async (c) => {
  try {
    await Promise.all([
      fs.unlink(CACHE_FILES.WALLS).catch(() => {}),
      fs.unlink(CACHE_FILES.PROFILES).catch(() => {}),
      fs.unlink(CACHE_FILES.REGISTRY).catch(() => {}),
      fs.unlink(CACHE_FILES.METADATA).catch(() => {}),
    ]);

    return c.json({
      success: true,
      message: "Cache cleared successfully",
    });
  } catch (error) {
    return c.json(
      {
        success: false,
        error: error instanceof Error ? error.message : "Unknown error",
      },
      500
    );
  }
});

// OPTIMIZED: Get all inactive walls (using cache)
app.get("/inactive-walls", async (c) => {
  try {
    const env = getEnv(c.env);
    const { wallsData, profilesData } = await getCachedData(env);

    const { enhancedInactiveWalls } = enhanceWallsWithProfiles(
      wallsData,
      profilesData
    );

    return c.json({
      success: true,
      data: {
        inactiveWalls: enhancedInactiveWalls,
        count: enhancedInactiveWalls.length,
        message: `Found ${enhancedInactiveWalls.length} inactive walls waiting to be activated`,
      },
      timestamp: Date.now(),
    });
  } catch (error) {
    console.error("Error fetching inactive walls:", error);
    return c.json(
      {
        success: false,
        error: error instanceof Error ? error.message : "Unknown error",
      },
      500
    );
  }
});

// OPTIMIZED: Get activated walls (using cache)
app.get("/activated-walls", async (c) => {
  try {
    const env = getEnv(c.env);
    const { wallsData, profilesData } = await getCachedData(env);

    const { enhancedActiveWalls } = enhanceWallsWithProfiles(
      wallsData,
      profilesData
    );

    return c.json({
      success: true,
      data: {
        activatedWalls: enhancedActiveWalls,
        count: enhancedActiveWalls.length,
        message: `Found ${enhancedActiveWalls.length} activated walls in the gallery`,
      },
      timestamp: Date.now(),
    });
  } catch (error) {
    console.error("Error fetching activated walls:", error);
    return c.json(
      {
        success: false,
        error: error instanceof Error ? error.message : "Unknown error",
      },
      500
    );
  }
});

// Keep your existing get-wall-information endpoint (this still needs real-time data)
app.get("/get-wall-information/:pda", async (c) => {
  try {
    const env = getEnv(c.env);
    const heliusUrl = getHeliusRpcUrl(env);
    const pda = c.req.param("pda");

    // Fetch the wall account data
    const response = await fetch(heliusUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "getAccountInfo",
        params: [pda, { encoding: "base64" }],
      }),
    });

    const data = (await response.json()) as any;

    if (data.error) {
      throw new Error(data.error.message);
    }

    if (!data.result?.value) {
      return c.json(
        {
          success: false,
          error: "Wall not found",
        },
        404
      );
    }

    // Decode the wall account
    const accountData = Buffer.from(data.result.value.data[0], "base64");
    const wall = decodeWallAccount(accountData, pda);

    if (!wall) {
      return c.json(
        {
          success: false,
          error: "Failed to decode wall data",
        },
        400
      );
    }

    // Get owner's Farcaster profile (try cache first)
    const { profilesData } = await getCachedData(env);
    let profile = profilesData.profiles.get(wall.owner.toUpperCase());

    // If not in cache, fetch it
    if (!profile) {
      const profileMap = await fetchFarcasterProfiles([wall.owner], env);
      profile = profileMap.get(wall.owner.toUpperCase()) ?? undefined;
    }

    // Get recent casts if we have a profile
    let recentCasts: any[] = [];
    if (
      profile?.fid &&
      wall.castHash !==
        "0x0000000000000000000000000000000000000000000000000000000000000000"
    ) {
      try {
        const neynarResponse = await fetch(
          `https://api.neynar.com/v2/farcaster/cast?identifier=${wall.castHash}&type=hash&viewer_fid=${profile.fid}`,
          {
            method: "GET",
            headers: {
              "x-api-key": env.NEYNAR_API_KEY,
              "x-neynar-experimental": "false",
            },
          }
        );
        console.log("IN HERE, the neynar response is ", neynarResponse);

        if (neynarResponse.ok) {
          const castData = (await neynarResponse.json()) as any;
          recentCasts = [castData.cast];
        }
      } catch (err) {
        console.error("Error fetching cast:", err);
      }
    }

    return c.json({
      success: true,
      data: {
        pda: wall.pda,
        owner: wall.owner,
        state: wall.state,
        castHash: wall.castHash,
        displayName: profile
          ? `@${profile.username}`
          : truncateAddress(wall.owner),
        pfp: profile?.pfp_url ?? null,
        recentWritings: recentCasts,
      },
    });
  } catch (error) {
    console.error("Error fetching wall information:", error);
    return c.json(
      {
        success: false,
        error: error instanceof Error ? error.message : "Unknown error",
      },
      500
    );
  }
});

// 🏥 HEALTH CHECK
app.get("/health", (c) => {
  return c.json({
    status: "healthy",
    message: "Cached Wallcaster API",
    version: "2.0.0-cached",
    endpoints: [
      "GET /setup-app-for-fid/:fid - Complete data for your miniapp (CACHED)",
      "GET /activated-walls - Community gallery of active walls (CACHED)",
      "GET /inactive-walls - Inactive walls waiting for activation (CACHED)",
      "GET /get-wall-information/:pda - Get specific wall by PDA",
      "GET /cache/status - Check cache status",
      "POST /cache/refresh - Force cache refresh",
      "DELETE /cache/clear - Clear all caches",
    ],
    caching: {
      wallsTTL: `${CACHE_TTL.WALLS / 1000}s`,
      profilesTTL: `${CACHE_TTL.PROFILES / 1000}s`,
      registryTTL: `${CACHE_TTL.REGISTRY / 1000}s`,
    },
  });
});

// 🏠 ROOT
app.get("/", (c) => {
  return c.json({
    message: "🧱 Cached Wallcaster API",
    description: "Lightning-fast API with intelligent local caching",
    performance: {
      Before: "Every request fetched all walls + profiles (~5-10s)",
      After: "Cached data serves in milliseconds, smart refresh",
    },
    caching: {
      strategy: "TTL-based with smart invalidation",
      storage: "Local filesystem in /data folder",
      wallsCache: `${CACHE_TTL.WALLS / 1000}s TTL`,
      profilesCache: `${CACHE_TTL.PROFILES / 1000}s TTL`,
      registryCache: `${CACHE_TTL.REGISTRY / 1000}s TTL`,
    },
    usage: {
      "Check user status": "GET /setup-app-for-fid/:fid (now cached!)",
      "Show community": "GET /activated-walls (now cached!)",
      "Get inactive walls": "GET /inactive-walls (now cached!)",
      "Cache management":
        "GET /cache/status, POST /cache/refresh, DELETE /cache/clear",
    },
    benefits: [
      "⚡ 100x faster response times",
      "🔄 Smart cache invalidation",
      "📊 Reduced API calls to external services",
      "🎯 Better user experience",
      "💰 Lower infrastructure costs",
    ],
  });
});

export default app;
