# 👻♟️ GhostMate: Agent Chess on Miden

> **Deploy your AI agent. Watch it play. You never touch a piece.**

[![MIT License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Built on Miden](https://img.shields.io/badge/built%20on-Miden-blueviolet)](https://polygon.technology/polygon-miden)

---

## What is GhostMate?

GhostMate is an on-chain chess arena where players deploy autonomous AI agents instead of playing moves themselves. Each agent is a Miden smart account running a minimax engine with a private opening book your strategy is yours alone, sealed in a zero-knowledge proof. Opponents challenge your agent, the GameContract validates every move, and the MatchRegistry tracks your ranking on a public leaderboard.

---

## Why Miden?

| Property | What it means for GhostMate |
|---|---|
| **Private account state** | Your agent's opening book and engine weights are stored privately opponents see your moves, never your strategy |
| **ZK proofs** | Every move is proven client-side and verified on-chain; no trust in a central server |
| **Client-side proving** | The WASM prover runs in the browser, so games are fully self-sovereign no backend required |
| **Account-native logic** | The chess engine lives inside an AccountComponent, not a separate contract; state transitions are atomic |

---

## How It Works

```
1. Register Agent      →   Deploy your AgentAccount with a name and opening book
                           Your minimax engine (depth 3) is sealed as private state

2. Challenge Opponent  →   Pick an agent from the leaderboard and post a wager
                           The GameContract mints a TurnNote and starts the clock

3. Watch Your Ghost    →   Your agent receives TurnNotes, proves each move,
                           and emits MoveNotes autonomously, until checkmate
```

No wallet interaction needed after setup. The ghost plays while you sleep.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Smart contracts | [Miden](https://polygon.technology/polygon-miden) (MASM + `cargo-miden`) |
| Contract logic | Rust (`no_std`) chess engine, registry, game rules |
| Frontend | React 18 + TypeScript + Vite |
| Miden SDK | `@miden-sdk/react` · `@miden-sdk/miden-sdk` · `@miden-sdk/miden-wallet-adapter-*` v0.14 |
| Wallet | Miden browser extension via `MidenWalletAdapter` |
| Hosting | Vercel |

---

## Project Structure

```
ghostmate/
│
├── ghostmate-chess/          # no_std chess engine (board, move gen, minimax eval)
├── ghostmate-registry/       # no_std registry logic (leaderboard sort, game results)
│
├── contracts/
│   ├── agent-account/        # AgentAccount opens/receives moves, runs engine
│   ├── move-note/            # Note script delivering computed moves to the agent
│   ├── agent-move-note/      # Note script consumed by GameContract (from/to/promo)
│   ├── game-contract/        # GameContract 64-sq board, move validation, wager
│   ├── match-registry/       # MatchRegistry registration, challenges, leaderboard
│   └── result-note/          # ResultNote consumed by MatchRegistry on game end
│
├── integration/              # 59 integration tests (agent, game, registry)
├── deploy-ghostmate/         # Deployment helpers and transaction scripts
│
└── ghostmate-web/            # React frontend (Vite + TypeScript)
    └── src/
        ├── components/       # Layout, Background, ArenaFeed, TxStatus, …
        ├── pages/            # Home, Register, Challenges, LiveGame, Agents, Profile
        ├── hooks/            # useRegistry, useNoteStream, …
        ├── config.ts         # Contract addresses, network config
        ├── miden.ts          # Wallet adapter constants (network, permissions)
        └── chess.ts          # Board helpers shared between UI and contract encoding
```

---

## Run Locally

### Prerequisites

- [Rust](https://rustup.rs) + `cargo-miden` `cargo install cargo-miden`
- [Node.js](https://nodejs.org) ≥ 20
- [Miden wallet extension](https://chrome.google.com/webstore) installed in your browser

### 1 Run the test suite

```bash
cargo test
# 100 tests: 21 chess-engine, 20 registry-logic, 59 integration
```

### 2 Start the frontend dev server

```bash
cd ghostmate-web
npm install
npm run dev
# → http://localhost:5173
```

### 3 Build for production

```bash
cd ghostmate-web
npm run build   # tsc + vite build
npm run preview # preview the production bundle locally
```

### Environment variables (optional)

Create `ghostmate-web/.env.local` to point at deployed contracts:

```env
VITE_REGISTRY_ID=0x<your-registry-account-id>
VITE_WAGER_FAUCET_ID=0x<your-faucet-account-id>
VITE_RPC_URL=testnet
VITE_PROVER_URL=testnet
```

Defaults (`testnet`) are used when unset.

---

## Live App

**[ghostmate.vercel.app](https://ghostmate.vercel.app)**

> Miden testnet connect the Miden browser extension, register your agent, and challenge opponents on the leaderboard.

---

## Status

**Testnet contracts deployment in progress.**

| Component | Status |
|---|---|
| Chess engine + registry logic | All 100 tests passing |
| `AgentAccount` / `GameContract` / `MatchRegistry` MASM | Written, pending `cargo-miden` deploy |
| Frontend (wallet connect, leaderboard, live game view) | Live on Vercel |
| Wager / faucet integration | Pending contract addresses |

---

## Built On

Built on **[@0xMiden](https://twitter.com/0xMiden)** the privacy-first ZK rollup.

---

## Links

- **Live app:** [ghostmate.vercel.app](https://ghostmate.vercel.app)
- **GitHub:** [github.com/AQEELERRHH/ghostmate](https://github.com/AQEELERRHH/ghostmate)
- **Miden docs:** [docs.polygon.technology/miden](https://docs.polygon.technology/miden)

---

## License

[MIT](LICENSE)
