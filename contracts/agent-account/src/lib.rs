#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;
use alloc::vec;

use miden::{
    active_account, felt, native_account, note, output_note, tx,
    Felt, NoteType, StorageMap, StorageValue, Word,
};
use ghostmate_chess::{Board, Move, WeightAdj, best_move, decode_weight};

// MoveNote MAST root — update after compiling contracts/move-note with cargo-miden.
fn move_note_script_root() -> miden::Digest {
    miden::Digest::from_word(
        Word::try_from([
            0x0000_0000_0000_0001_u64,
            0x0000_0000_0000_0002_u64,
            0x0000_0000_0000_0003_u64,
            0x0000_0000_0000_0004_u64,
        ])
        .unwrap(),
    )
}

fn word_to_u64s(w: Word) -> [u64; 4] {
    [
        w[0].as_canonical_u64(),
        w[1].as_canonical_u64(),
        w[2].as_canonical_u64(),
        w[3].as_canonical_u64(),
    ]
}

/// AgentAccount storage layout:
///
/// | Field          | Type                  | Visibility | Description                               |
/// |----------------|-----------------------|------------|-------------------------------------------|
/// | agent_name     | StorageValue<Word>    | public     | Name packed as 4 Felts (UTF-8, null-pad)  |
/// | owner          | StorageValue<Word>    | public     | Owner AccountId [prefix, suffix, 0, 0]    |
/// | win_count      | StorageValue<Felt>    | public     | Games won                                 |
/// | loss_count     | StorageValue<Felt>    | public     | Games lost                                |
/// | opening_book   | StorageMap<Word,Word> | private    | board_word → [move_u64, 0, 0, 0]         |
/// | engine_weights | StorageMap<Word,Word> | private    | [slot,0,0,0] → [encoded_offset,0,0,0]    |
#[component]
struct AgentAccount {
    #[storage(description = "agent name packed into one Word (4 felts, UTF-8 bytes)")]
    agent_name: StorageValue<Word>,

    #[storage(description = "owner wallet AccountId: [prefix, suffix, 0, 0]")]
    owner: StorageValue<Word>,

    #[storage(description = "number of wins")]
    win_count: StorageValue<Felt>,

    #[storage(description = "number of losses")]
    loss_count: StorageValue<Felt>,

    /// Private: maps board_word → [move_u64, 0, 0, 0].
    /// A zero move_u64 means no book entry.
    #[storage(description = "private opening book: board word hash -> packed move")]
    opening_book: StorageMap<Word, Word>,

    /// Private: maps [slot_index, 0, 0, 0] → [encoded_weight, 0, 0, 0].
    /// 12 slots (0-11), one per piece type; encoded as (offset + 128).
    #[storage(description = "private per-piece material adjustment weights (12 slots)")]
    engine_weights: StorageMap<Word, Word>,
}

#[component]
impl AgentAccount {
    // ── Initialisation ──────────────────────────────────────────────────────

    /// One-time setup called from a tx_script after deployment.
    pub fn initialize(&mut self, name: Word, owner: Word) {
        self.agent_name.set(name);
        self.owner.set(owner);
        self.win_count.set(felt!(0));
        self.loss_count.set(felt!(0));
        native_account::incr_nonce();
    }

    // ── Owner-only admin ────────────────────────────────────────────────────

    /// Add or overwrite one opening-book entry.
    pub fn set_opening_entry(&mut self, board_key: Word, move_word: Word) {
        self.assert_caller_is_owner();
        self.opening_book.set(board_key, move_word);
    }

    /// Set a weight-adjustment slot (0-11) as a raw encoded byte (offset + 128).
    pub fn set_weight(&mut self, slot: Felt, encoded: Felt) {
        self.assert_caller_is_owner();
        let key = Word::new([slot, felt!(0), felt!(0), felt!(0)]);
        let val = Word::new([encoded, felt!(0), felt!(0), felt!(0)]);
        self.engine_weights.set(key, val);
    }

    // ── Game-state bookkeeping ───────────────────────────────────────────────

    pub fn record_win(&mut self) {
        let c = self.win_count.get();
        self.win_count.set(c + felt!(1));
        native_account::incr_nonce();
    }

    pub fn record_loss(&mut self) {
        let c = self.loss_count.get();
        self.loss_count.set(c + felt!(1));
        native_account::incr_nonce();
    }

    // ── Read-only queries ────────────────────────────────────────────────────

    pub fn get_agent_name(&self) -> Word { self.agent_name.get() }
    pub fn get_owner(&self)      -> Word { self.owner.get() }
    pub fn get_win_count(&self)  -> Felt { self.win_count.get() }
    pub fn get_loss_count(&self) -> Felt { self.loss_count.get() }

    // ── Core: receive a board state and emit a MoveNote ─────────────────────

    /// Called from the MoveRequestNote script when the note is consumed.
    ///
    /// Arguments (≤ 4 Words, see P3):
    ///   board_word   — 64 squares packed as 4-bit nibbles across 4 Felts
    ///   flags        — [side(0/1), ep_sq(0-63|255), castling(0-15), 0]
    ///   recipient    — [prefix, suffix, 0, 0] of the account awaiting the move
    ///   tag_word     — [note_tag, 0, 0, 0] for the output MoveNote
    ///
    /// Output: one private MoveNote carrying [from_sq, to_sq, promo, board_hash].
    pub fn receive_move(
        &mut self,
        board_word: &Word,
        flags: &Word,
        recipient: &Word,
        tag_word: &Word,
    ) {
        let board = Board::from_words(word_to_u64s(*board_word), word_to_u64s(*flags));
        let adj   = self.load_weight_adj();

        // Opening book lookup (keyed by the raw board word).
        let book_val  = self.opening_book.get(*board_word);
        let book_move = book_val[0].as_canonical_u64();
        let computed  = if book_move != 0 {
            Move::from_u64(book_move)
        } else {
            best_move(&board, &adj)
        };

        // Board fingerprint for the recipient to correlate the response.
        let board_hash = board_word[0];

        let serial_num = Word::new([
            tx::get_block_number(),
            active_account::get_id().prefix(),
            Felt::new(computed.to_u64()),
            felt!(0),
        ]);

        let note_inputs = vec![
            Felt::new(computed.from_sq() as u64),
            Felt::new(computed.to_sq()   as u64),
            Felt::new(computed.promo()   as u64),
            board_hash,
        ];

        let recipient_built = note::build_recipient(
            serial_num,
            move_note_script_root(),
            note_inputs,
        );

        let tag       = miden::NoteTag::from(tag_word[0]);
        let note_type = NoteType::from(felt!(2)); // private
        let _note_idx = output_note::create(tag, note_type, recipient_built);

        // Silence the recipient word — used only to build the serial number context
        let _ = recipient;

        native_account::incr_nonce();
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    fn assert_caller_is_owner(&self) {
        let caller = active_account::get_id();
        let stored = self.owner.get();
        assert_eq(caller.prefix(), stored[0]);
    }

    fn load_weight_adj(&self) -> WeightAdj {
        let mut adj = [0i32; 12];
        for i in 0u64..12 {
            let key = Word::new([Felt::new(i), felt!(0), felt!(0), felt!(0)]);
            let val = self.engine_weights.get(key);
            adj[i as usize] = decode_weight(val[0].as_canonical_u64());
        }
        adj
    }
}
