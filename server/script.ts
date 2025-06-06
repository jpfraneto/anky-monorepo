import fs from "fs";
import csv from "csv-parser";
import fetch from "node-fetch";

interface BrandEntry {
  name: string;
  url: string;
  iconLogoUrl: string;
  category: string;
  profile: string;
  channel: string;
  description: string;
}

const NEYNAR_API_KEY = "C73E0EB2-5766-4FB7-BBAB-EA9C849354F8";

async function getFarcasterPfp(username: string): Promise<string | null> {
  try {
    const options = {
      method: "GET",
      headers: {
        "x-api-key": NEYNAR_API_KEY,
        "x-neynar-experimental": "false",
      },
    };

    console.log(`🔍 Fetching PFP for user: ${username}`);

    const response = await fetch(
      `https://api.neynar.com/v2/farcaster/user/search?limit=2&viewer_fid=16098&q=${username}`,
      options
    );
    const data = await response.json();

    if (data?.result?.users?.[0]?.pfp_url) {
      console.log(`✅ Found PFP for ${username}`);
      return data.result.users[0].pfp_url;
    }
    console.log(`❌ No PFP found for ${username}`);
    return null;
  } catch (err) {
    console.error(`❌ Error fetching PFP for ${username}:`, err);
    return null;
  }
}

async function processCsv() {
  const results: BrandEntry[] = [];
  const processedRows: any[] = [];

  console.log("📊 Starting CSV processing...");

  return new Promise((resolve, reject) => {
    // Define explicit headers based on your data structure
    const headers = [
      "name", // _0
      "url", // _1
      "iconUrl", // _2
      "category", // _3
      "profile", // _4
      "channel", // _5
      "type", // _6
      "description", // _7
      "col8", // _8
      "col9", // _9
      "col10", // _10
      "col11", // _11
      "col12", // _12
    ];

    fs.createReadStream("brnd.csv")
      .pipe(
        csv({
          headers: headers,
        })
      )
      .on("data", (row) => {
        console.log("ROW", row);

        // Store all rows first for batch processing
        processedRows.push(row);
      })
      .on("end", async () => {
        console.log(`📋 Processing ${processedRows.length} rows...`);

        // Process rows sequentially to avoid rate limiting
        for (const row of processedRows) {
          // Skip entries without profile
          if (!row.profile || row.profile.trim() === "") {
            console.log(
              `⏭️ Skipping row - no profile found for: ${row.name || "unnamed"}`
            );
            continue;
          }

          // Extract username from profile (removing @ if present)
          const username = row.profile.startsWith("@")
            ? row.profile.slice(1)
            : row.profile;

          console.log(`🔄 Processing: ${row.name} (${username})`);

          const pfpUrl = await getFarcasterPfp(username);

          // Skip if no Farcaster profile found
          if (!pfpUrl) {
            console.log(`⏭️ Skipping ${username} - no Farcaster profile found`);
            continue;
          }

          console.log(`➕ Adding entry for: ${row.name}`);
          results.push({
            name: row.name || "",
            url: row.url || "",
            iconLogoUrl: pfpUrl,
            category: row.category || "",
            profile: row.profile || "",
            channel: row.channel || "",
            description: row.description || "",
          });

          // Add delay to avoid rate limiting
          await new Promise((resolve) => setTimeout(resolve, 100));
        }

        // Create data directory if it doesn't exist
        if (!fs.existsSync("./data")) {
          console.log("📁 Creating data directory");
          fs.mkdirSync("./data");
        }

        console.log(`💾 Writing ${results.length} entries to JSON file`);
        // Write results to JSON file
        fs.writeFileSync("./data/brnd.json", JSON.stringify(results, null, 2));

        console.log(
          `✨ Processing complete! Found ${results.length} valid entries out of ${processedRows.length} total rows.`
        );
        resolve(results);
      })
      .on("error", (error) => {
        console.error("❌ Error processing CSV:", error);
        reject(error);
      });
  });
}

processCsv()
  .then(() => console.log("🎉 All done!"))
  .catch((err) => console.error("❌ Error processing CSV:", err));
