import React from "react";
import ReactDOM from "react-dom/client";
import { MidenProvider } from "@miden-sdk/react";
import { MidenWalletAdapter } from "@miden-sdk/miden-wallet-adapter-miden";
import { WalletProvider } from "@miden-sdk/miden-wallet-adapter-react";
import { App } from "./App";
import { MIDEN_CONFIG, assertRegistryConfig } from "./config";
import {
  GHOSTMATE_MIDEN_NETWORK,
  GHOSTMATE_PRIVATE_DATA_PERMISSION,
  GHOSTMATE_ALLOWED_PRIVATE_DATA,
} from "./miden";
import "./index.css";

const wallets = [new MidenWalletAdapter({ appName: "GhostMate" })];

function Loading() {
  return (
    <div className="wasm-loading">
      <span className="logo-icon spin">♟</span>
      <p>Initialising Miden WASM…</p>
    </div>
  );
}

function InitError(error: Error) {
  return (
    <div className="wasm-loading error">
      <p>Failed to load Miden WASM</p>
      <pre>{error.message}</pre>
    </div>
  );
}

// Fail fast on a mis-pinned registry id — never run against the wrong registry.
let configError: Error | null = null;
try {
  assertRegistryConfig();
} catch (err) {
  configError = err instanceof Error ? err : new Error(String(err));
  console.error("GhostMate registry config check failed:", configError);
}

const root = ReactDOM.createRoot(document.getElementById("root")!);

if (configError) {
  root.render(
    <div className="wasm-loading error">
      <p>GhostMate configuration error — refusing to start</p>
      <pre>{configError.message}</pre>
    </div>,
  );
} else root.render(
  <React.StrictMode>
    <WalletProvider
      wallets={wallets}
      network={GHOSTMATE_MIDEN_NETWORK}
      privateDataPermission={GHOSTMATE_PRIVATE_DATA_PERMISSION}
      allowedPrivateData={GHOSTMATE_ALLOWED_PRIVATE_DATA}
      autoConnect
    >
      <MidenProvider
        config={MIDEN_CONFIG}
        loadingComponent={<Loading />}
        errorComponent={InitError}
      >
        <App />
      </MidenProvider>
    </WalletProvider>
  </React.StrictMode>,
);
