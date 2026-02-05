/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_URL: string;
  readonly VITE_PRIVY_APP_ID: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

interface Window {
  ethereum?: {
    request: (args: { method: string; params?: unknown[] }) => Promise<unknown>;
    on: (event: string, callback: (accounts: string[]) => void) => void;
    selectedAddress?: string;
  };
}
