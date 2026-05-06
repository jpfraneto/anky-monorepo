const CREDENTIAL_QUERY_RE = /([?&](?:api-key|apikey|token|access_token|auth)=)[^&\s"'`)>]+/gi;
const BEARER_RE = /\b(Bearer\s+)[A-Za-z0-9._~+/=-]+/gi;
const CONNECTION_URL_RE = /\b((?:postgres|postgresql|mongodb(?:\+srv)?|redis):\/\/)[^\s"'`]+/gi;
const KEYPAIR_PATH_RE =
  /((?:~?\/|\.{1,2}\/|[A-Za-z]:[\\/])(?:[^\s"'`]*[\\/])?[^\s"'`]*(?:keypair|wallet|deployer|id\.json)[^\s"'`]*)/gi;

export function redactSecretValues(value) {
  return String(value ?? "")
    .replace(CREDENTIAL_QUERY_RE, "$1<redacted>")
    .replace(BEARER_RE, "$1<redacted>")
    .replace(CONNECTION_URL_RE, "$1<redacted>")
    .replace(KEYPAIR_PATH_RE, "<redacted-path>");
}
