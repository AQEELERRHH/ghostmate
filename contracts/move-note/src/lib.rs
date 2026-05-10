//! MoveNote script — consumed by the game orchestrator (or any account) to
//! read the move computed by AgentAccount.
//!
//! Note storage layout (set at note creation in AgentAccount::receive_move):
//!   [0] from_square  (0-63)
//!   [1] to_square    (0-63)
//!   [2] promotion    (0 = none, otherwise piece code)
//!   [3] board_hash   (first Felt of the original board_word, for correlation)
#![no_std]
#![feature(alloc_error_handler)]

use miden::*;
use crate::bindings::miden::agent_account::agent_account;

#[note]
struct MoveNote;

#[note]
impl MoveNote {
    /// Consume the note: read the move from note storage, then call back into
    /// the AgentAccount to record any outcome update if needed.
    ///
    /// `_arg` is unused but required by the note-script signature.
    #[note_script]
    fn run(self, _arg: Word) {
        let storage = active_note::get_storage();

        // Validate that the note carries exactly the expected move fields.
        // (storage[0..3] = from, to, promo; storage[3] = board_hash)
        assert!(storage.len() >= 4);

        let from_sq  = storage[0];
        let to_sq    = storage[1];
        let promo    = storage[2];
        let _board_hash = storage[3];

        // Sanity-check squares are in range (0-63).
        assert!(from_sq.as_canonical_u64() < 64);
        assert!(to_sq.as_canonical_u64()   < 64);

        // The consuming account (game orchestrator) can now use from_sq / to_sq
        // / promo to apply the move on their side.
        //
        // Optionally call agent_account::record_win() / record_loss() after
        // the game resolves.  That would be done from the orchestrator's own
        // tx script once the game outcome is known, not from this note script.
        //
        // Suppress unused-variable warnings in the contract compilation context.
        let _ = (from_sq, to_sq, promo);
    }
}
