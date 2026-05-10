//! MatchRegistry — GhostMate on-chain match coordination account.
//!
//! Agents register here, post wager challenges, accept them (which triggers a
//! GameContract spawn), and report final results.  A leaderboard is derived
//! from the stored win/loss/draw statistics.
//!
//! Storage layout
//! ──────────────
//!   registry_name    StorageValue<Word>  — display name of this registry
//!   agent_count      StorageValue<Felt>  — monotone agent counter
//!   challenge_count  StorageValue<Felt>  — monotone challenge ID counter
//!   game_count       StorageValue<Felt>  — monotone game ID counter
//!   game_start_root  StorageValue<Word>  — MAST root of the GameStart note
//!   agents_name      StorageMap<Word,Word> — [prefix,0,0,0] → name (4 felts)
//!   agents_stats     StorageMap<Word,Word> — [prefix,0,0,0] → [wins,losses,draws,REGISTERED=1]
//!   challenges       StorageMap<Word,Word> — [id,0,0,0] → [challenger_prefix,wager_amount,status,0]
//!   challenge_assets StorageMap<Word,Word> — [id,0,0,0] → wager asset key word
//!   games            StorageMap<Word,Word> — [id,0,0,0] → [white_prefix,black_prefix,status,game_contract_prefix]
#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;

use miden::*;

// ── Status constants ──────────────────────────────────────────────────────────

const CHALLENGE_OPEN:      u64 = 0;
const CHALLENGE_ACCEPTED:  u64 = 1;
const CHALLENGE_CANCELLED: u64 = 2;

const GAME_ACTIVE:     u64 = 0;
const GAME_WHITE_WINS: u64 = 1;
const GAME_BLACK_WINS: u64 = 2;
const GAME_DRAW:       u64 = 3;

const REGISTERED: u64 = 1;

// ── MAST root placeholder (update after cargo-miden compiles result-note) ────

fn result_note_script_root() -> Digest {
    Digest::from_word(
        Word::try_from([0_u64, 0, 0, 1]).unwrap(),
    )
}

// ── Component definition ──────────────────────────────────────────────────────

#[component]
struct MatchRegistry {
    #[storage(description = "display name of this registry")]
    registry_name: StorageValue<Word>,

    #[storage(description = "number of registered agents")]
    agent_count: StorageValue<Felt>,

    #[storage(description = "next challenge ID")]
    challenge_count: StorageValue<Felt>,

    #[storage(description = "next game ID")]
    game_count: StorageValue<Felt>,

    #[storage(description = "MAST root of the GameStart note script")]
    game_start_root: StorageValue<Word>,

    #[storage(description = "agent prefix → name word")]
    agents_name: StorageMap<Word, Word>,

    #[storage(description = "agent prefix → [wins, losses, draws, REGISTERED]")]
    agents_stats: StorageMap<Word, Word>,

    #[storage(description = "challenge id → [challenger_prefix, wager_amount, status, 0]")]
    challenges: StorageMap<Word, Word>,

    #[storage(description = "challenge id → wager asset key word")]
    challenge_assets: StorageMap<Word, Word>,

    #[storage(description = "game id → [white_prefix, black_prefix, status, game_contract_prefix]")]
    games: StorageMap<Word, Word>,
}

// ── Helper: build a 1-element map key ────────────────────────────────────────

fn id_key(id: u64) -> Word {
    Word::try_from([id, 0_u64, 0, 0]).unwrap()
}

fn prefix_key(prefix: u64) -> Word {
    Word::try_from([prefix, 0_u64, 0, 0]).unwrap()
}

// ── Component implementation ──────────────────────────────────────────────────

#[component]
impl MatchRegistry {
    // ── Admin ─────────────────────────────────────────────────────────────────

    /// One-time setup: set the registry display name and the MAST root of the
    /// GameStart note that will be emitted when a challenge is accepted.
    ///
    /// Arguments (2 Words):
    ///   name           — 4-felt display name
    ///   game_start_root — MAST root word of the GameStart note script
    pub fn initialize(&mut self, name: Word, game_start_root: Word) {
        self.registry_name.set(name);
        self.game_start_root.set(game_start_root);
    }

    // ── Registration ──────────────────────────────────────────────────────────

    /// Register the calling account as an agent.
    ///
    /// Arguments (1 Word):
    ///   name — 4-felt display name for this agent
    ///
    /// The caller's account prefix is used as the unique agent identifier.
    /// Re-registering with a new name is allowed (stats are preserved).
    pub fn register_agent(&mut self, name: Word) {
        let caller = active_account::get_id();
        let prefix  = caller.prefix().as_canonical_u64();
        let key     = prefix_key(prefix);

        self.agents_name.set(key, name);

        // Only bump agent_count and set REGISTERED sentinel on first registration.
        let stats = self.agents_stats.get(key);
        let already_registered = stats.into_elements()[3].as_canonical_u64() == REGISTERED;
        if !already_registered {
            let current = self.agent_count.get().as_canonical_u64();
            self.agent_count.set(Felt::new(current + 1));

            let new_stats = Word::try_from([0_u64, 0, 0, REGISTERED]).unwrap();
            self.agents_stats.set(key, new_stats);
        }

        native_account::incr_nonce();
    }

    // ── Challenge lifecycle ───────────────────────────────────────────────────

    /// Post an open challenge.  The wager asset (if any) must have been
    /// deposited to this account before calling.
    ///
    /// Arguments (2 Words):
    ///   wager_info — [wager_amount, 0, 0, 0]   (0 = no wager)
    ///   asset_key  — fungible asset key Word    (zeroes if no wager)
    ///
    /// Returns the new challenge ID in the nonce increment side-effect.
    pub fn post_challenge(&mut self, wager_info: &Word, asset_key: &Word) {
        let caller   = active_account::get_id();
        let prefix   = caller.prefix().as_canonical_u64();

        // Caller must be registered.
        let stats_key = prefix_key(prefix);
        let stats     = self.agents_stats.get(stats_key);
        assert!(
            stats.into_elements()[3].as_canonical_u64() == REGISTERED,
            "agent not registered"
        );

        let wager_elems  = wager_info.into_elements();
        let wager_amount = wager_elems[0].as_canonical_u64();

        let id     = self.challenge_count.get().as_canonical_u64();
        let id_key = id_key(id);

        let record = Word::try_from([prefix, wager_amount, CHALLENGE_OPEN, 0_u64]).unwrap();
        self.challenges.set(id_key, record);
        self.challenge_assets.set(id_key, *asset_key);

        self.challenge_count.set(Felt::new(id + 1));
        native_account::incr_nonce();
    }

    /// Accept an open challenge.  This marks the challenge ACCEPTED and
    /// creates a game record.  The game contract address is recorded so the
    /// result-note can later update the game status.
    ///
    /// Arguments (2 Words):
    ///   challenge_info — [challenge_id, 0, 0, 0]
    ///   game_contract  — [game_contract_prefix, 0, 0, 0]
    pub fn accept_challenge(&mut self, challenge_info: &Word, game_contract: &Word) {
        let challenge_id = challenge_info.into_elements()[0].as_canonical_u64();
        let c_key        = id_key(challenge_id);

        let record_elems = self.challenges.get(c_key).into_elements();
        let challenger_prefix = record_elems[0].as_canonical_u64();
        let wager_amount      = record_elems[1].as_canonical_u64();
        let status            = record_elems[2].as_canonical_u64();

        assert!(status == CHALLENGE_OPEN, "challenge not open");

        let acceptor    = active_account::get_id();
        let acc_prefix  = acceptor.prefix().as_canonical_u64();

        // Acceptor must be registered.
        let acc_key = prefix_key(acc_prefix);
        let acc_stats = self.agents_stats.get(acc_key);
        assert!(
            acc_stats.into_elements()[3].as_canonical_u64() == REGISTERED,
            "acceptor not registered"
        );

        // Cannot accept your own challenge.
        assert!(acc_prefix != challenger_prefix, "cannot accept own challenge");

        // Mark challenge accepted.
        let updated_challenge = Word::try_from([
            challenger_prefix,
            wager_amount,
            CHALLENGE_ACCEPTED,
            0_u64,
        ]).unwrap();
        self.challenges.set(c_key, updated_challenge);

        // Create game record (challenger = white, acceptor = black).
        let game_id     = self.game_count.get().as_canonical_u64();
        let g_key       = id_key(game_id);
        let gc_prefix   = game_contract.into_elements()[0].as_canonical_u64();

        let game_record = Word::try_from([
            challenger_prefix,
            acc_prefix,
            GAME_ACTIVE,
            gc_prefix,
        ]).unwrap();
        self.games.set(g_key, game_record);
        self.game_count.set(Felt::new(game_id + 1));

        native_account::incr_nonce();
    }

    /// Cancel an open challenge.  Only the challenger may cancel.
    ///
    /// Arguments (1 Word):
    ///   challenge_info — [challenge_id, 0, 0, 0]
    pub fn cancel_challenge(&mut self, challenge_info: &Word) {
        let challenge_id = challenge_info.into_elements()[0].as_canonical_u64();
        let c_key        = id_key(challenge_id);

        let record_elems = self.challenges.get(c_key).into_elements();
        let challenger_prefix = record_elems[0].as_canonical_u64();
        let wager_amount      = record_elems[1].as_canonical_u64();
        let status            = record_elems[2].as_canonical_u64();

        assert!(status == CHALLENGE_OPEN, "challenge not open");

        let caller  = active_account::get_id();
        let prefix  = caller.prefix().as_canonical_u64();
        assert!(prefix == challenger_prefix, "only challenger may cancel");

        let updated = Word::try_from([
            challenger_prefix,
            wager_amount,
            CHALLENGE_CANCELLED,
            0_u64,
        ]).unwrap();
        self.challenges.set(c_key, updated);

        native_account::incr_nonce();
    }

    // ── Result recording ─────────────────────────────────────────────────────

    /// Record a settled game result and update agent leaderboard stats.
    ///
    /// Called by the result-note emitted by GameContract when the game ends.
    /// The note's sender must be the game contract whose prefix is stored in
    /// the game record.
    ///
    /// Arguments (2 Words):
    ///   game_info — [game_id, 0, 0, 0]
    ///   result    — [status(1=white_wins,2=black_wins,3=draw), game_contract_prefix, 0, 0]
    pub fn record_game_result(&mut self, game_info: &Word, result: &Word) {
        let game_id     = game_info.into_elements()[0].as_canonical_u64();
        let g_key       = id_key(game_id);

        let game_elems  = self.games.get(g_key).into_elements();
        let white_prefix = game_elems[0].as_canonical_u64();
        let black_prefix = game_elems[1].as_canonical_u64();
        let cur_status   = game_elems[2].as_canonical_u64();
        let gc_prefix    = game_elems[3].as_canonical_u64();

        assert!(cur_status == GAME_ACTIVE, "game already settled");

        let result_elems = result.into_elements();
        let new_status   = result_elems[0].as_canonical_u64();
        let caller_gc    = result_elems[1].as_canonical_u64();

        // Only the recorded game contract may report the result.
        assert!(caller_gc == gc_prefix, "unauthorized result reporter");
        assert!(
            new_status == GAME_WHITE_WINS
                || new_status == GAME_BLACK_WINS
                || new_status == GAME_DRAW,
            "invalid result status"
        );

        // Update game record.
        let updated_game = Word::try_from([
            white_prefix,
            black_prefix,
            new_status,
            gc_prefix,
        ]).unwrap();
        self.games.set(g_key, updated_game);

        // Update white agent stats.
        let w_key   = prefix_key(white_prefix);
        let w_stats = self.agents_stats.get(w_key).into_elements();
        let (ww, wl, wd) = (
            w_stats[0].as_canonical_u64(),
            w_stats[1].as_canonical_u64(),
            w_stats[2].as_canonical_u64(),
        );
        let (ww2, wl2, wd2) = apply_result_counts(ww, wl, wd, true, new_status);
        self.agents_stats.set(
            w_key,
            Word::try_from([ww2, wl2, wd2, REGISTERED]).unwrap(),
        );

        // Update black agent stats.
        let b_key   = prefix_key(black_prefix);
        let b_stats = self.agents_stats.get(b_key).into_elements();
        let (bw, bl, bd) = (
            b_stats[0].as_canonical_u64(),
            b_stats[1].as_canonical_u64(),
            b_stats[2].as_canonical_u64(),
        );
        let (bw2, bl2, bd2) = apply_result_counts(bw, bl, bd, false, new_status);
        self.agents_stats.set(
            b_key,
            Word::try_from([bw2, bl2, bd2, REGISTERED]).unwrap(),
        );

        native_account::incr_nonce();
    }

    // ── Read-only getters ─────────────────────────────────────────────────────

    /// Return the stored name for the given agent prefix.
    ///
    /// Arguments (1 Word):
    ///   agent_key — [prefix, 0, 0, 0]
    pub fn get_agent_name(&self, agent_key: &Word) -> Word {
        self.agents_name.get(*agent_key)
    }

    /// Return [wins, losses, draws, REGISTERED] for the given agent prefix.
    ///
    /// Arguments (1 Word):
    ///   agent_key — [prefix, 0, 0, 0]
    pub fn get_agent_stats(&self, agent_key: &Word) -> Word {
        self.agents_stats.get(*agent_key)
    }

    /// Return [challenger_prefix, wager_amount, status, 0] for a challenge.
    ///
    /// Arguments (1 Word):
    ///   challenge_key — [challenge_id, 0, 0, 0]
    pub fn get_challenge(&self, challenge_key: &Word) -> Word {
        self.challenges.get(*challenge_key)
    }

    /// Return [white_prefix, black_prefix, status, game_contract_prefix] for a game.
    ///
    /// Arguments (1 Word):
    ///   game_key — [game_id, 0, 0, 0]
    pub fn get_game(&self, game_key: &Word) -> Word {
        self.games.get(*game_key)
    }

    /// Return [agent_count, challenge_count, game_count, 0].
    pub fn get_counts(&self) -> Word {
        Word::try_from([
            self.agent_count.get().as_canonical_u64(),
            self.challenge_count.get().as_canonical_u64(),
            self.game_count.get().as_canonical_u64(),
            0_u64,
        ]).unwrap()
    }
}

// ── Pure helper (no self) — update win/loss/draw counts ──────────────────────

fn apply_result_counts(
    wins:    u64,
    losses:  u64,
    draws:   u64,
    is_white: bool,
    status:  u64,
) -> (u64, u64, u64) {
    match status {
        GAME_WHITE_WINS => if is_white { (wins + 1, losses, draws) }
                           else        { (wins, losses + 1, draws) },
        GAME_BLACK_WINS => if !is_white { (wins + 1, losses, draws) }
                           else         { (wins, losses + 1, draws) },
        GAME_DRAW       => (wins, losses, draws + 1),
        _               => (wins, losses, draws),
    }
}
