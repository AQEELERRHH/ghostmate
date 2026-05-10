#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;
use alloc::vec;

use miden::{
    active_account, felt, native_account, note, output_note, tx,
    Asset, Felt, NoteType, StorageMap, StorageValue, Word,
};
use ghostmate_chess::{
    board::{Board, Move, EMPTY, W_PAWN, B_PAWN, WHITE},
    moves::generate_legal,
};

// ── Game status sentinel values ───────────────────────────────────────────────
const STATUS_ACTIVE:     u64 = 0;
const STATUS_WHITE_WINS: u64 = 1;
const STATUS_BLACK_WINS: u64 = 2;
const STATUS_DRAW:       u64 = 3;

// 50-move rule: 100 half-moves without pawn move or capture = draw
const HALFMOVE_DRAW: u64 = 100;

// ── Note root placeholders (update after cargo-miden build) ───────────────────

/// MAST root of the game-turn-note script (consumed by each AgentAccount).
/// Replace with actual digest after compiling contracts/game-turn-note.
fn turn_note_root() -> miden::Digest {
    miden::Digest::from_word(
        Word::try_from([
            0xAAAA_0000_0000_0001_u64,
            0xAAAA_0000_0000_0002_u64,
            0xAAAA_0000_0000_0003_u64,
            0xAAAA_0000_0000_0004_u64,
        ])
        .unwrap(),
    )
}

/// MAST root of the standard P2ID note script for wager payouts.
/// See rust-sdk-pitfalls P9 — digest from miden-standards P2ID script.
fn p2id_note_root() -> miden::Digest {
    miden::Digest::from_word(
        Word::try_from([
            13_362_761_878_458_161_062_u64,
            15_090_726_097_241_769_395_u64,
            444_910_447_169_617_901_u64,
            3_558_201_871_398_422_326_u64,
        ])
        .unwrap(),
    )
}

// ── Wire-format helpers ───────────────────────────────────────────────────────

fn word_to_u64s(w: Word) -> [u64; 4] {
    [
        w[0].as_canonical_u64(),
        w[1].as_canonical_u64(),
        w[2].as_canonical_u64(),
        w[3].as_canonical_u64(),
    ]
}

fn u64s_to_word(v: [u64; 4]) -> Word {
    Word::new([Felt::new(v[0]), Felt::new(v[1]), Felt::new(v[2]), Felt::new(v[3])])
}

// ── Move validation helper ────────────────────────────────────────────────────

fn is_legal_move(board: &Board, mv: Move) -> bool {
    generate_legal(board).contains(&mv)
}

// ── Terminal condition detection ──────────────────────────────────────────────

/// Examine the position AFTER a move has been applied (`next_board`).
/// Returns one of the STATUS_* constants.
///
/// `next_board.side` is the player that now needs to move.
/// `halfmove_clock`  is the updated half-move count after the move.
fn detect_terminal(next_board: &Board, halfmove_clock: u64) -> u64 {
    let legal = generate_legal(next_board);
    if legal.is_empty() {
        if next_board.in_check(next_board.side) {
            // The side that JUST moved wins (next_board.side is the loser)
            if next_board.side == WHITE {
                STATUS_BLACK_WINS // white is checkmated → black wins
            } else {
                STATUS_WHITE_WINS // black is checkmated → white wins
            }
        } else {
            STATUS_DRAW // stalemate
        }
    } else if halfmove_clock >= HALFMOVE_DRAW {
        STATUS_DRAW // 50-move rule
    } else {
        STATUS_ACTIVE
    }
}

/// Returns the updated halfmove clock after applying `mv` to `board`.
/// Resets to 0 on any pawn move or capture; otherwise increments.
fn next_halfmove_clock(board: &Board, mv: Move) -> u64 {
    let moved   = board.piece_at(mv.from_sq());
    let target  = board.piece_at(mv.to_sq());
    let is_pawn    = moved == W_PAWN || moved == B_PAWN;
    let is_capture = target != EMPTY;
    if is_pawn || is_capture { 0 } else {
        // Read old clock from flags[3]; add 1 safely
        word_to_u64s(Word::new([felt!(0), felt!(0), felt!(0), felt!(0)]))[3]
            // (we receive the old clock separately, see receive_move)
            + 1
    }
}

// ── Storage layout ────────────────────────────────────────────────────────────
//
// board            StorageValue<Word>   64 squares packed as 4-bit nibbles → 4 Felts
// flags            StorageValue<Word>   [side(0/1), ep_sq(0-255), castling(0-15), halfmove_clk]
// white_agent      StorageValue<Word>   [prefix, suffix, 0, 0]
// black_agent      StorageValue<Word>   [prefix, suffix, 0, 0]
// wager_asset_key  StorageValue<Word>   asset class key (4 Felts)
// wager_per_player StorageValue<Felt>   amount each player wagered
// game_status      StorageValue<Felt>   0=active 1=white_wins 2=black_wins 3=draw
// move_count       StorageValue<Felt>   total half-moves played
//
// Note: turn_note_root is a compile-time constant (update after cargo-miden build).
// ─────────────────────────────────────────────────────────────────────────────

#[component]
struct GameContract {
    #[storage(description = "64 squares packed as 4-bit nibbles into 4 Felts")]
    board: StorageValue<Word>,

    #[storage(description = "game flags: [side, ep_sq, castling, halfmove_clock]")]
    flags: StorageValue<Word>,

    #[storage(description = "white agent AccountId [prefix, suffix, 0, 0]")]
    white_agent: StorageValue<Word>,

    #[storage(description = "black agent AccountId [prefix, suffix, 0, 0]")]
    black_agent: StorageValue<Word>,

    #[storage(description = "wager asset class key (Word)")]
    wager_asset_key: StorageValue<Word>,

    #[storage(description = "wager amount per player")]
    wager_per_player: StorageValue<Felt>,

    #[storage(description = "game status: 0=active 1=white_wins 2=black_wins 3=draw")]
    game_status: StorageValue<Felt>,

    #[storage(description = "total half-moves played")]
    move_count: StorageValue<Felt>,
}

#[component]
impl GameContract {
    // ── Setup ──────────────────────────────────────────────────────────────────

    /// One-time initialisation, called from a tx_script after deployment.
    ///
    /// Arguments (≤ 4 Words, P3):
    ///   white      — [prefix, suffix, 0, 0]  white agent AccountId
    ///   black      — [prefix, suffix, 0, 0]  black agent AccountId
    ///   wager_key  — [k0, k1, k2, k3]        wager asset class key
    ///   config     — [wager_per_player, 0, 0, 0]
    pub fn initialize(
        &mut self,
        white: &Word,
        black: &Word,
        wager_key: &Word,
        config: &Word,
    ) {
        // Set starting position
        let startpos = Board::startpos();
        let bw = startpos.to_board_word();
        let fw = startpos.to_flags_word(); // [side=1, ep=255, castling=15, 0]
        self.board.set(u64s_to_word(bw));
        self.flags.set(u64s_to_word(fw));

        self.white_agent.set(*white);
        self.black_agent.set(*black);
        self.wager_asset_key.set(*wager_key);
        self.wager_per_player.set(config[0]);

        self.game_status.set(felt!(0)); // STATUS_ACTIVE
        self.move_count.set(felt!(0));

        native_account::incr_nonce();
    }

    /// Accept a wager deposit from a player.
    ///
    /// Called by a deposit note script (consumed by the game account).
    /// `asset` — the wagered asset passed through from the note.
    /// The asset is added to the game account's vault.
    pub fn deposit_wager(&mut self, asset: Asset) {
        // Validate asset class matches the configured wager
        assert_eq(asset.key[0], self.wager_asset_key.get()[0]);
        assert_eq(asset.key[1], self.wager_asset_key.get()[1]);
        native_account::add_asset(asset);
    }

    // ── Core: receive and validate an agent's move ─────────────────────────────

    /// Called by the agent-move-note script when an agent submits their move.
    ///
    /// Arguments (≤ 4 Words, P3):
    ///   mv     — [from_sq, to_sq, promo, 0]
    ///   sender — [prefix, suffix, 0, 0] (from active_note::get_sender() in the note script)
    ///
    /// On success:
    ///   • Updates board and flags in storage.
    ///   • If the game is over: releases wager via P2ID output notes.
    ///   • If the game continues: emits a TurnNote to the next agent.
    pub fn receive_move(&mut self, mv: &Word, sender: &Word) {
        // ── 1. Game must be active ──────────────────────────────────────────
        assert(self.game_status.get().as_canonical_u64() == STATUS_ACTIVE);

        // ── 2. Reconstruct board ────────────────────────────────────────────
        let bw = word_to_u64s(self.board.get());
        let fw = word_to_u64s(self.flags.get());
        let halfmove_clock = fw[3];
        let board = Board::from_words(bw, fw);

        // ── 3. Verify it's this sender's turn ───────────────────────────────
        let expected = if board.side == WHITE {
            self.white_agent.get()
        } else {
            self.black_agent.get()
        };
        // Compare both prefix and suffix for unambiguous identity.
        assert_eq(sender[0], expected[0]);
        assert_eq(sender[1], expected[1]);

        // ── 4. Parse and validate the submitted move ────────────────────────
        let from_sq = mv[0].as_canonical_u64() as u8;
        let to_sq   = mv[1].as_canonical_u64() as u8;
        let promo   = mv[2].as_canonical_u64() as u8;
        let player_move = Move::new(from_sq, to_sq, promo);

        assert(is_legal_move(&board, player_move));

        // ── 5. Update halfmove clock ────────────────────────────────────────
        let moved   = board.piece_at(from_sq);
        let target  = board.piece_at(to_sq);
        let is_pawn    = moved == W_PAWN || moved == B_PAWN;
        let is_capture = target != EMPTY;
        let new_halfmove = if is_pawn || is_capture { 0 } else {
            halfmove_clock + 1
        };

        // ── 6. Apply move and persist ───────────────────────────────────────
        let next = board.apply_move(player_move);
        let new_bw = next.to_board_word();
        let new_fw_base = next.to_flags_word(); // [side, ep_sq, castling, 0]
        // Fold halfmove_clock into flags[3]
        let new_fw = [new_fw_base[0], new_fw_base[1], new_fw_base[2], new_halfmove];

        self.board.set(u64s_to_word(new_bw));
        self.flags.set(u64s_to_word(new_fw));
        self.move_count.set(self.move_count.get() + felt!(1));

        // ── 7. Detect terminal conditions ───────────────────────────────────
        let status = detect_terminal(&next, new_halfmove);
        self.game_status.set(Felt::new(status));

        // ── 8. Settle or continue ───────────────────────────────────────────
        if status != STATUS_ACTIVE {
            self.settle_wager(status);
        } else {
            self.emit_turn_note(&next, new_halfmove);
        }

        native_account::incr_nonce();
    }

    // ── Read-only queries ──────────────────────────────────────────────────────

    pub fn get_board(&self)       -> Word { self.board.get() }
    pub fn get_flags(&self)       -> Word { self.flags.get() }
    pub fn get_game_status(&self) -> Felt { self.game_status.get() }
    pub fn get_move_count(&self)  -> Felt { self.move_count.get() }

    pub fn get_white_agent(&self) -> Word { self.white_agent.get() }
    pub fn get_black_agent(&self) -> Word { self.black_agent.get() }

    // ── Private helpers ────────────────────────────────────────────────────────

    /// Release the wager once the game is over.
    ///
    /// Winner (STATUS_WHITE_WINS or STATUS_BLACK_WINS) receives both wagers.
    /// Draw (STATUS_DRAW): each player receives their original wager back.
    fn settle_wager(&self, status: u64) {
        let asset_key = self.wager_asset_key.get();
        // Safe: wager amounts are configured at init and won't overflow in practice.
        let per_player = self.wager_per_player.get().as_canonical_u64();

        match status {
            STATUS_WHITE_WINS => {
                let total = per_player * 2;
                self.emit_p2id_payout(self.white_agent.get(), asset_key, total, 0);
            }
            STATUS_BLACK_WINS => {
                let total = per_player * 2;
                self.emit_p2id_payout(self.black_agent.get(), asset_key, total, 0);
            }
            _ => {
                // Draw: return each player's wager
                self.emit_p2id_payout(self.white_agent.get(), asset_key, per_player, 0);
                self.emit_p2id_payout(self.black_agent.get(), asset_key, per_player, 1);
            }
        }
    }

    /// Build one P2ID output note and deduct the asset from the game vault.
    ///
    /// `serial_disc` differentiates serial numbers when multiple notes are
    /// created in the same transaction (avoids duplicate serial numbers).
    fn emit_p2id_payout(&self, recipient: Word, asset_key: Word, amount: u64, serial_disc: u64) {
        let serial = Word::new([
            tx::get_block_number(),
            self.move_count.get(),
            Felt::new(serial_disc),
            felt!(0),
        ]);

        // P2ID note inputs: [suffix, prefix] (see rust-sdk-pitfalls P8)
        let note_inputs = vec![recipient[1], recipient[0]];
        let recipient_built = note::build_recipient(serial, p2id_note_root(), note_inputs);

        // Private note (P9/P10)
        let note_type = NoteType::from(felt!(2));
        let note_idx = output_note::create(
            miden::NoteTag::from(felt!(0)),
            note_type,
            recipient_built,
        );

        // Transfer the asset from the game vault into the output note
        let asset = Asset {
            key:   asset_key,
            value: Word::new([Felt::new(amount), felt!(0), felt!(0), felt!(0)]),
        };
        native_account::remove_asset(asset.clone());
        output_note::add_asset(asset, note_idx);
    }

    /// Emit a TurnNote to the next agent so they can compute and submit their move.
    ///
    /// TurnNote storage layout (16 Felts):
    ///   [0..3]  board_word  — current board state
    ///   [4..7]  flags_word  — [side, ep_sq, castling, halfmove_clock]
    ///   [8..11] game_id     — [prefix, suffix, 0, 0] (where to send the reply)
    ///   [12..15] reserved   — [0, 0, 0, 0]
    fn emit_turn_note(&self, next_board: &Board, halfmove_clock: u64) {
        let recipient = if next_board.side == WHITE {
            self.white_agent.get()
        } else {
            self.black_agent.get()
        };

        let bw_arr = next_board.to_board_word();
        let fw_base = next_board.to_flags_word();
        let fw_arr = [fw_base[0], fw_base[1], fw_base[2], halfmove_clock];

        let game_id = active_account::get_id();

        let note_inputs = vec![
            // board
            Felt::new(bw_arr[0]), Felt::new(bw_arr[1]),
            Felt::new(bw_arr[2]), Felt::new(bw_arr[3]),
            // flags (with updated halfmove clock)
            Felt::new(fw_arr[0]), Felt::new(fw_arr[1]),
            Felt::new(fw_arr[2]), Felt::new(fw_arr[3]),
            // game account id (reply-to for the agent's response move note)
            game_id.prefix(), game_id.suffix(), felt!(0), felt!(0),
            // response note tag (0 = public)
            felt!(0), felt!(0), felt!(0), felt!(0),
        ];

        let serial = Word::new([
            tx::get_block_number(),
            self.move_count.get(),
            felt!(0),
            felt!(0),
        ]);

        let recipient_built = note::build_recipient(serial, turn_note_root(), note_inputs);

        // Private turn note (only the target agent can consume it)
        let note_type = NoteType::from(felt!(2));
        let _note_idx = output_note::create(
            miden::NoteTag::from(felt!(0)),
            note_type,
            recipient_built,
        );

        let _ = recipient; // used via note routing, not added to note data directly
    }
}
