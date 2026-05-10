//! ResultNote — emitted by GameContract when a game concludes.
//!
//! The game contract addresses this note to the MatchRegistry account.
//! When the registry consumes it, this script runs and calls
//! `match_registry::record_game_result()`.
//!
//! Note storage layout (set by GameContract):
//!   [0]  game_id               (u64 as Felt)
//!   [1]  result_status         (1=white_wins, 2=black_wins, 3=draw)
//!   [2]  game_contract_prefix  (so registry can verify sender)
//!   [3]  reserved              (0)
#![no_std]
#![feature(alloc_error_handler)]

use miden::*;
use crate::bindings::miden::match_registry::match_registry;

#[note]
struct ResultNote;

#[note]
impl ResultNote {
    /// Execute the result submission:
    ///   1. Read game_id and result from note storage.
    ///   2. Call match_registry::record_game_result() on the consuming account.
    #[note_script]
    fn run(self, _arg: Word) {
        let storage = active_note::get_storage();

        assert!(storage.len() >= 3);

        let game_id          = storage[0];
        let result_status    = storage[1];
        let gc_prefix        = storage[2];

        // Validate result status is one of the three terminal values.
        let s = result_status.as_canonical_u64();
        assert!(s == 1 || s == 2 || s == 3, "invalid result status");

        let game_info = Word::new([game_id, felt!(0), felt!(0), felt!(0)]);
        let result    = Word::new([result_status, gc_prefix, felt!(0), felt!(0)]);

        match_registry::record_game_result(&game_info, &result);
    }
}
