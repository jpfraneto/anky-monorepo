import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import { BrowserRouter, Route, Routes } from "react-router-dom";
import App from "./App";
import Sessions from "./components/sessions";

import { PrivyProvider } from "@privy-io/react-auth";

createRoot(document.getElementById("root")!).render(
  <BrowserRouter>
    <StrictMode>
      <PrivyProvider
        appId={import.meta.env.VITE_PRIVY_APP_ID}
        clientId={import.meta.env.VITE_PRIVY_CLIENT_ID}
        config={{
          loginMethods: ["farcaster"],
          embeddedWallets: {
            createOnLogin: "users-without-wallets",
          },
        }}
      >
        <Routes>
          <Route path="/" element={<App />} />
          <Route path="/sessions" element={<Sessions />} />
        </Routes>
      </PrivyProvider>
    </StrictMode>
  </BrowserRouter>
);
