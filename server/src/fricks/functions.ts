import fs from "fs/promises";
import path from "path";

const DATA_DIR = path.join(process.cwd(), "data");
const FRICKS_FILE = path.join(DATA_DIR, "fricks.json");

// In-memory cache for fastest access
const FRICK_CACHE = new Map<number, string>();

export async function getUserFrick(fid: number, solanaAddress: string) {
  try {
    console.log(`🔍 Looking up frick for FID: ${fid}`);
    // First check in-memory cache
    const cachedFrick = FRICK_CACHE.get(fid);
    if (cachedFrick) {
      console.log(`💨 Found frick in memory cache`);
      return cachedFrick;
    }

    // Then check local storage
    await ensureDataDir();
    const fricks = await loadFricks();
    const frick = fricks[fid];

    if (frick) {
      console.log(`💾 Found frick in local storage`);
      // Update cache and return
      FRICK_CACHE.set(fid, frick);
      return frick;
    }

    return null;
  } catch (error) {
    console.error("❌ Error getting user frick:", error);
    return null;
  }
}

export async function saveUserFrick(fid: number, frick: string) {
  try {
    console.log(`💾 Saving frick for FID: ${fid}`);
    // Update in-memory cache immediately
    FRICK_CACHE.set(fid, frick);
    console.log(`✅ Updated memory cache`);

    // Update local storage in background
    await ensureDataDir();
    const fricks = await loadFricks();
    fricks[fid] = frick;
    await fs.writeFile(FRICKS_FILE, JSON.stringify(fricks, null, 2));
    console.log(`📝 Saved to local storage`);

    return true;
  } catch (error) {
    console.error("❌ Error saving user frick:", error);
    return false;
  }
}

// Helper functions
async function ensureDataDir() {
  try {
    await fs.access(DATA_DIR);
    console.log(`📁 Data directory exists`);
  } catch {
    await fs.mkdir(DATA_DIR, { recursive: true });
    console.log(`📁 Created data directory`);
  }
}

async function loadFricks(): Promise<Record<number, string>> {
  try {
    console.log(`📖 Loading fricks from file`);
    const data = await fs.readFile(FRICKS_FILE, "utf-8");
    return JSON.parse(data);
  } catch {
    console.log(`📝 No fricks file found, starting fresh`);
    // If file doesn't exist, return empty object
    return {};
  }
}
