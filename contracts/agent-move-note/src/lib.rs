//! AgentMoveNote — consumed by GameContract when an agent submits their chosen move.
//!
//! An AgentAccount (or any player account) creates this note and addresses it
//! to the game account.  When the game account includes it in a transaction,
//! this script runs and calls `game_contract::receive_move()`.
//!
//! Note storage layout (set by the creating account):
//!   [0]  from_sq   (0–63)
//!   [1]  to_sq     (0–63)
//!   [2]  promo     (0 = none; otherwise piece code matching W_QUEEN etc.)
//!   [3]  reserved  (0)
#![no_std]
#![feature(alloc_error_handler)]

use miden::*;
use crate::bindings::miden::game_contract::game_contract;

#[note]
struct AgentMoveNote;

#[note]
impl AgentMoveNote {
    /// Execute the move submission:
    ///   1. Read the move from note storage.
    ///   2. Read the authenticated sender (the submitting agent).
    ///   3. Call game_contract::receive_move() on the consuming account (game account).
    #[note_script]
    fn run(self, _arg: Word) {
        let storage = active_note::get_storage();

        // Validate the note carries at least from/to/promo fields.
        assert!(storage.len() >= 3);

        let from_sq = storage[0];
        let to_sq   = storage[1];
        let promo   = storage[2];

        // Bounds-check squares before passing to the contract (fail fast).
        assert!(from_sq.as_canonical_u64() < 64);
        assert!(to_sq.as_canonical_u64()   < 64);

        let mv_word = Word::new([from_sq, to_sq, promo, felt!(0)]);

        // active_note::get_sender() is authenticated by the Miden protocol —
        // the sender identity cannot be forged by any party.
        let sender = active_note::get_sender();
        let sender_word = Word::new([
            sender.prefix(),
            sender.suffix(),
            felt!(0),
            felt!(0),
        ]);

        game_contract::receive_move(&mv_word, &sender_word);
    }
}
