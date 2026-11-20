// ── Deployed contract addresses ───────────────────────────────────────────────
// CANONICAL registry id — this file is the ONLY source of truth. It is
// deliberately NOT read from env: a stale committed .env.local once silently
// pointed local dev at an abandoned registry. VITE_REGISTRY_ID is ignored.
//
// Phase 2D.1 secure MatchRegistry, deployed 2026-11-20 on testnet v0.17:
// AuthNetworkAccount, immutable allowlist (register-note + P2ID). Only the
// network operator executes its transactions. Name on-chain: GHOSTMATE-2D1.
// /deployed-addresses.json mirrors these values — update both together.
export const REGISTRY_ID = "0xe67e1b121c63d791380938be496ba1";
export const REGISTRY_BECH32 = "mtst1arn8uxcjr33a0yfcpyutujtt5yj4n82e";

/**
 * Fail-fast startup check: REGISTRY_BECH32 must decode (valid bech32, "mtst"
 * testnet prefix, valid checksum) to exactly REGISTRY_ID. Pure JS so it runs
 * before WASM loads. Throws with a readable message on any mismatch.
 * Miden addresses use bech32m (BIP-350), not classic bech32.
 */
export function assertRegistryConfig(): void {
  const CHARSET = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
  const hex = REGISTRY_ID.toLowerCase();
  if (!/^0x[0-9a-f]{30}$/.test(hex)) {
    throw new Error(`REGISTRY_ID "${REGISTRY_ID}" is not a 15-byte hex account id.`);
  }
  const addr = REGISTRY_BECH32.toLowerCase().split("_")[0];
  const sep = addr.lastIndexOf("1");
  const hrp = addr.slice(0, sep);
  if (hrp !== "mtst") {
    throw new Error(`REGISTRY_BECH32 has prefix "${hrp}", expected testnet "mtst".`);
  }
  const values = [...addr.slice(sep + 1)].map((c) => CHARSET.indexOf(c));
  if (values.length < 7 || values.includes(-1)) {
    throw new Error("REGISTRY_BECH32 is not valid bech32.");
  }
  // bech32m checksum (BIP-350): polymod of expanded hrp + data must equal
  // 0x2bc830a3 (classic bech32 would be 1 — Miden uses bech32m).
  const GEN = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
  let chk = 1;
  const expanded = [...hrp].map((c) => c.charCodeAt(0) >> 5)
    .concat([0], [...hrp].map((c) => c.charCodeAt(0) & 31), values);
  for (const v of expanded) {
    const top = chk >>> 25;
    chk = ((chk & 0x1ffffff) << 5) ^ v;
    for (let i = 0; i < 5; i++) if ((top >>> i) & 1) chk ^= GEN[i];
  }
  if (chk !== 0x2bc830a3) throw new Error("REGISTRY_BECH32 checksum is invalid.");
  // 5-bit groups -> bytes; payload = [address-type byte, 15-byte account id].
  const bytes: number[] = [];
  let acc = 0, bits = 0;
  for (const v of values.slice(0, -6)) {
    acc = (acc << 5) | v; bits += 5;
    while (bits >= 8) { bits -= 8; bytes.push((acc >> bits) & 0xff); }
  }
  const idHex = "0x" + bytes.slice(-15).map((b) => b.toString(16).padStart(2, "0")).join("");
  if (bytes.length !== 16 || idHex !== hex) {
    throw new Error(`REGISTRY_BECH32 decodes to ${idHex}, but REGISTRY_ID is ${hex}.`);
  }
}

export const WAGER_FAUCET_ID =
  import.meta.env.VITE_WAGER_FAUCET_ID ?? "0x0000000000000000";

// Compiled register-note script, served statically from public/packages.
export const REGISTER_NOTE_PKG_URL = "/packages/register-note.masp";

// ── Miden network ─────────────────────────────────────────────────────────────
export const MIDEN_CONFIG = {
  rpcUrl: (import.meta.env.VITE_RPC_URL ?? "testnet") as
    | "testnet"
    | "devnet"
    | "localhost"
    | string,
  prover: (import.meta.env.VITE_PROVER_URL ?? "testnet") as
    | "local"
    | "testnet"
    | "devnet"
    | string,
  autoSyncInterval: 15_000,
  // Phase 2C.2 needed this to execute AS the imported registry (web-sdk#222).
  // Since 2D.1 the network operator executes registry txs, so it may no longer
  // be needed — kept OFF until that is verified separately (no change here).
  useWorker: false,
} as const;

// ── Storage slot names (snake_case of package + struct + field) ───────────────
// Package: miden:match-registry → miden_match_registry
// Struct:  MatchRegistry        → match_registry
// Namespace confirmed on-chain: match_registry::match_registry::<field>
export const SLOTS = {
  agentName:       "match_registry::match_registry::agents_name",
  agentStats:      "match_registry::match_registry::agents_stats",
  agentIds:        "match_registry::match_registry::agent_ids",
  agentCount:      "match_registry::match_registry::agent_count",
  challenges:      "match_registry::match_registry::challenges",
  challengeAssets: "match_registry::match_registry::challenge_assets",
  games:           "match_registry::match_registry::games",
  challengeCount:  "match_registry::match_registry::challenge_count",
  gameCount:       "match_registry::match_registry::game_count",
} as const;

// ── Result-note storage layout ────────────────────────────────────────────────
// [0] game_id  [1] result_status  [2] gc_prefix  [3] 0
export const RESULT_NOTE_IDX = { gameId: 0, status: 1, gcPrefix: 2 } as const;

// ── TurnNote storage layout (16 felts) ───────────────────────────────────────
// [0..3] board words  [4] side  [5] ep_sq  [6] castling  [7] halfmove_clock
// [8] game_id  [9] move_from  [10] move_to  [11] move_promo  [12..15] 0
export const TURN_NOTE_IDX = {
  boardWord0: 0, boardWord1: 1, boardWord2: 2, boardWord3: 3,
  side: 4, epSq: 5, castling: 6, halfmoveClock: 7,
  gameId: 8, moveFrom: 9, moveTo: 10, movePromo: 11,
} as const;

// ── Game result status values ─────────────────────────────────────────────────
export const GAME_STATUS = {
  ACTIVE:      0n,
  WHITE_WINS:  1n,
  BLACK_WINS:  2n,
  DRAW:        3n,
} as const;

export const CHALLENGE_STATUS = {
  OPEN:      0n,
  ACCEPTED:  1n,
  CANCELLED: 2n,
} as const;
