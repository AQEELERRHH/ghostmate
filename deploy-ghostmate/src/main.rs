// GhostMate testnet deployment script.
//
// Prerequisites (run from workspace root):
//   cargo miden build --manifest-path contracts/match-registry/Cargo.toml --release
//   cargo miden build --manifest-path contracts/agent-account/Cargo.toml  --release
//
// Then run the deployer:
//   cargo run -p deploy-ghostmate
//
// Optional env vars:
//   MIDEN_RPC_HOST   (default: rpc.testnet.miden.io)
//   MIDEN_RPC_PORT   (default: 443)
//   MIDEN_DATA_DIR   (default: deploy-data)
//
// Outputs:
//   deploy-data/deploy.sqlite3  — persisted client store
//   deploy-data/keys/           — one JSON file per account (FilesystemKeyStore)
//   deployed-addresses.json     — paste VITE_REGISTRY_ID from here into .env

use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use miden_client::{
    account::{
        Account, AccountBuilder, AccountCode, AccountId,
        AccountStorageMode, AccountType,
        auth::AuthFalcon512Rpo,
    },
    auth::StoreAuthenticator,
    keystore::FilesystemKeyStore,
    rpc::{Endpoint, TonicRpcClient},
    store::sqlite_store::SqliteStore,
    Client, ClientBuilder,
};
use rand::rngs::OsRng;
use serde_json::json;

// ── Compiled WASM paths (written by `cargo miden build --release`) ────────────
const REGISTRY_WASM: &str =
    "contracts/match-registry/target/wasm32-unknown-unknown/release/match_registry.wasm";
const AGENT_WASM: &str =
    "contracts/agent-account/target/wasm32-unknown-unknown/release/agent_account.wasm";

// ── Testnet endpoint ──────────────────────────────────────────────────────────
const DEFAULT_HOST: &str = "rpc.testnet.miden.io";
const DEFAULT_PORT: u16 = 443;

// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== GhostMate Testnet Deployment ===\n");

    // ── 1. Verify compiled artefacts exist before opening network connections ─
    let registry_wasm = read_wasm(REGISTRY_WASM).context(
        "MatchRegistry WASM not found. Build it first:\n  \
         cargo miden build --manifest-path contracts/match-registry/Cargo.toml --release",
    )?;
    let agent_wasm = read_wasm(AGENT_WASM).context(
        "AgentAccount WASM not found. Build it first:\n  \
         cargo miden build --manifest-path contracts/agent-account/Cargo.toml --release",
    )?;
    println!("  ✓ MatchRegistry WASM  ({} bytes)", registry_wasm.len());
    println!("  ✓ AgentAccount WASM   ({} bytes)\n", agent_wasm.len());

    // ── 2. Storage paths ──────────────────────────────────────────────────────
    let data_dir = PathBuf::from(
        env::var("MIDEN_DATA_DIR").unwrap_or_else(|_| "deploy-data".into()),
    );
    let keys_dir = data_dir.join("keys");
    let db_path = data_dir.join("deploy.sqlite3");
    fs::create_dir_all(&keys_dir).context("create deploy-data/keys/")?;

    // ── 3. RPC endpoint ───────────────────────────────────────────────────────
    let host = env::var("MIDEN_RPC_HOST").unwrap_or_else(|_| DEFAULT_HOST.into());
    let port: u16 = env::var("MIDEN_RPC_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    println!("Connecting to https://{}:{}...", host, port);

    let endpoint = Endpoint::new("https".into(), host, port);
    let rpc = TonicRpcClient::new(&endpoint);

    // ── 4. SQLite store ───────────────────────────────────────────────────────
    // The sqlite feature of miden-client enables SqliteStore.
    // `SqliteStore::new` creates the file if it doesn't exist and applies
    // migrations automatically.
    let store = SqliteStore::new(&db_path)
        .await
        .context("open SQLite store")?;

    // ── 5. Filesystem key store ───────────────────────────────────────────────
    // FilesystemKeyStore writes one JSON file per account into keys_dir.
    // Files are named by account ID so they can be looked up at signing time.
    let keystore =
        FilesystemKeyStore::new(&keys_dir).context("create FilesystemKeyStore")?;

    // ── 6. Build client ───────────────────────────────────────────────────────
    // StoreAuthenticator bridges the FilesystemKeyStore and the transaction-
    // signing pipeline.  OsRng is used for nonce generation inside the prover.
    let mut client = ClientBuilder::new()
        .with_rpc(rpc)
        .with_store(store)
        .with_authenticator(StoreAuthenticator::new_with_rng(keystore, OsRng))
        .build()
        .await
        .context("build miden client")?;

    // ── 7. Sync ───────────────────────────────────────────────────────────────
    println!("Syncing with testnet...");
    let summary = client.sync_state().await.context("sync state")?;
    println!("  Synced to block #{}\n", summary.block_num);

    // ── 8. Deploy MatchRegistry ───────────────────────────────────────────────
    println!("Deploying MatchRegistry...");
    let (registry, registry_seed) =
        deploy_account(&mut client, &registry_wasm, "MatchRegistry")
            .await
            .context("deploy MatchRegistry")?;
    let registry_id = account_id_hex(registry.id());
    println!("  ID:   {}", registry_id);
    println!("  Seed: {}\n", hex::encode(registry_seed));

    // ── 9. Deploy AgentAccount template ──────────────────────────────────────
    println!("Deploying AgentAccount template...");
    let (agent, agent_seed) =
        deploy_account(&mut client, &agent_wasm, "AgentAccount")
            .await
            .context("deploy AgentAccount")?;
    let agent_id = account_id_hex(agent.id());
    println!("  ID:   {}", agent_id);
    println!("  Seed: {}\n", hex::encode(agent_seed));

    // ── 10. Write deployed-addresses.json ─────────────────────────────────────
    let out = json!({
        "network":               "testnet",
        "VITE_REGISTRY_ID":      registry_id,
        "VITE_AGENT_ACCOUNT_ID": agent_id,
    });
    fs::write("deployed-addresses.json", serde_json::to_string_pretty(&out)?)
        .context("write deployed-addresses.json")?;

    println!("=== Done ===");
    println!("Saved  → deployed-addresses.json");
    println!("Keys   → {}", keys_dir.display());
    println!("Store  → {}", db_path.display());
    println!();
    println!("Add to ghostmate-web/.env:");
    println!("  VITE_REGISTRY_ID={}", registry_id);
    println!("  VITE_AGENT_ACCOUNT_ID={}", agent_id);

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Reads the compiled WASM produced by `cargo miden build`.
fn read_wasm(path: &str) -> Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("read {}", path))
}

/// Creates a `RegularAccountImmutableCode` account from a cargo-miden compiled
/// WASM and registers it in the client's local SQLite store.
///
/// `AuthFalcon512Rpo` generates a fresh RpoFalcon512 key pair:
/// • The public key is embedded into the account as the auth component.
/// • The private key is stored by `FilesystemKeyStore` so the client can
///   sign future transactions for this account.
async fn deploy_account(
    client: &mut Client,
    wasm: &[u8],
    label: &str,
) -> Result<(Account, [u8; 32])> {
    let mut rng = OsRng;

    // Deterministic random seed: after hashing, this controls the account's
    // on-chain ID prefix.  Keep the seed file if you want to reproduce the
    // same ID on a fresh store.
    let init_seed: [u8; 32] = rand::random();

    // Parse the cargo-miden WASM into an AccountCode (MAST library).
    // AccountCode::from_wasm extracts the embedded MAST program that the
    // Miden VM executes when the account processes a transaction.
    let account_code = AccountCode::from_wasm(wasm)
        .with_context(|| format!("parse {} WASM as AccountCode", label))?;

    // AuthFalcon512Rpo is a component + key-pair bundled together.
    // Passing it to with_component:
    //   • embeds the public key into the account's procedure table so the VM
    //     can verify signatures
    //   • hands the private key to the FilesystemKeyStore via the
    //     StoreAuthenticator so the client can sign transactions
    let auth = AuthFalcon512Rpo::new(&mut rng);

    let (account, account_seed) = AccountBuilder::new(init_seed)
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(AccountStorageMode::Private)
        .with_component(auth)
        .code(account_code)
        .build(&mut rng)
        .with_context(|| format!("build {} account object", label))?;

    // Insert the account into the local SQLite store.
    // `overwrite = false` prevents clobbering an account from a previous run
    // that happens to share the same ID (astronomically unlikely but safe).
    client
        .add_account(&account, Some(account_seed), false)
        .await
        .with_context(|| format!("register {} in client store", label))?;

    println!("  {} registered in local store", label);
    Ok((account, init_seed))
}

/// Returns the account ID as a `0x`-prefixed 16-character hex string.
///
/// This matches the format expected by `VITE_REGISTRY_ID` in the frontend
/// and by the `config.ts` prefix lookups.  AccountId in miden is a 64-bit
/// value; `as_u64()` extracts the inner integer.
fn account_id_hex(id: AccountId) -> String {
    format!("0x{:016x}", id.as_u64())
}
