import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { PrivyProvider } from "@privy-io/react-auth";
import { BrowserRouter } from "react-router-dom";
import "./index.css";
import App from "./App";

const privyAppId = import.meta.env.VITE_PRIVY_APP_ID || "";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <PrivyProvider
      appId={privyAppId}
      config={{
        appearance: {
          theme: "dark",
          accentColor: "#fbbf24",
        },
        loginMethods: ["wallet"],
      }}
    >
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </PrivyProvider>
  </StrictMode>,
);
