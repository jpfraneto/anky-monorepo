const fs = require("fs");
const path = require("path");

const PUBLIC_ENV_KEYS = [
  "EXPO_PUBLIC_PRIVY_APP_ID",
  "EXPO_PUBLIC_PRIVY_CLIENT_ID",
  "EXPO_PUBLIC_ANKY_API_URL",
  "EXPO_PUBLIC_APP_URL",
  "EXPO_PUBLIC_SOLANA_RPC_URL",
  "EXPO_PUBLIC_SOLANA_CLUSTER",
  "EXPO_PUBLIC_SOLANA_SEAL_ADAPTER",
  "EXPO_PUBLIC_ANKY_CORE_PROGRAM_ID",
  "EXPO_PUBLIC_ANKY_CORE_COLLECTION",
  "EXPO_PUBLIC_ANKY_SEAL_PROGRAM_ID",
  "EXPO_PUBLIC_PRIVY_WALLET_EXPORT_URL",
  "EXPO_PUBLIC_IAP_CREDITS_8_ID",
  "EXPO_PUBLIC_IAP_CREDITS_24_ID",
  "EXPO_PUBLIC_IAP_CREDITS_88_ID",
  "EXPO_PUBLIC_IAP_PREMIUM_MONTHLY_ID",
];

function parseDotenvValue(rawValue) {
  const value = rawValue.trim();
  const quote = value[0];

  if ((quote === "\"" || quote === "'") && value[value.length - 1] === quote) {
    return value.slice(1, -1);
  }

  const commentIndex = value.search(/\s+#/);
  return (commentIndex === -1 ? value : value.slice(0, commentIndex)).trim();
}

function readLocalDotenv() {
  const dotenvPath = path.join(__dirname, ".env");

  if (!fs.existsSync(dotenvPath)) {
    return {};
  }

  return fs
    .readFileSync(dotenvPath, "utf8")
    .split(/\r?\n/)
    .reduce((env, rawLine) => {
      const line = rawLine.trim();

      if (line.length === 0 || line.startsWith("#")) {
        return env;
      }

      const match = line.match(/^([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$/);

      if (match == null) {
        return env;
      }

      env[match[1]] = parseDotenvValue(match[2] ?? "");
      return env;
    }, {});
}

const localEnv = readLocalDotenv();

for (const key of PUBLIC_ENV_KEYS) {
  if (
    process.env[key] == null &&
    localEnv[key] != null &&
    localEnv[key].trim().length > 0
  ) {
    process.env[key] = localEnv[key];
  }
}

function getPublicEnv() {
  return PUBLIC_ENV_KEYS.reduce((env, key) => {
    const value = process.env[key] ?? localEnv[key];

    if (value != null && value.trim().length > 0) {
      env[key] = value;
    }

    return env;
  }, {});
}

module.exports = ({ config }) => ({
  ...config,
  extra: {
    ...config.extra,
    publicEnv: getPublicEnv(),
  },
});
