//! Pure-Rust Minimax chess engine for the GhostMate AgentAccount.
//!
//! This crate is `no_std`-compatible (uses `alloc` for Vec) so it can be
//! linked into the Miden WASM contract.  When compiled for tests, std is
//! available via `cfg_attr`.
#![cfg_attr(not(test), no_std)]

#[cfg(not(test))]
extern crate alloc;

pub mod board;
pub mod eval;
pub mod minimax;
pub mod moves;

pub use board::{Board, Move, WHITE, BLACK};
pub use eval::{decode_weight, encode_weight, evaluate, WeightAdj, NO_ADJ};
pub use minimax::best_move;
