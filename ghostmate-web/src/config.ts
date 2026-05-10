// ── Deployed contract addresses ───────────────────────────────────────────────
// Fill these in after running `cargo miden run` / `cargo miden deploy`.

export const REGISTRY_ID =
  import.meta.env.VITE_REGISTRY_ID ?? "0x0000000000000000";

export const WAGER_FAUCET_ID =
  import.meta.env.VITE_WAGER_FAUCET_ID ?? "0x0000000000000000";

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
} as const;

// ── Storage slot names (snake_case of package + struct + field) ───────────────
// Package: miden:match-registry → miden_match_registry
// Struct:  MatchRegistry        → match_registry
export const SLOTS = {
  agentName:       "miden_match_registry::match_registry::agents_name",
  agentStats:      "miden_match_registry::match_registry::agents_stats",
  challenges:      "miden_match_registry::match_registry::challenges",
  challengeAssets: "miden_match_registry::match_registry::challenge_assets",
  games:           "miden_match_registry::match_registry::games",
  agentCount:      "miden_match_registry::match_registry::agent_count",
  challengeCount:  "miden_match_registry::match_registry::challenge_count",
  gameCount:       "miden_match_registry::match_registry::game_count",
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
