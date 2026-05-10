//! Negamax with alpha-beta pruning.

use super::board::{Board, Move};
use super::eval::{evaluate, WeightAdj};
use super::moves::generate_legal;

pub const SEARCH_DEPTH: u8 = 3;
pub const INF: i32 = 1_000_000;

/// Negamax alpha-beta. Returns `(best_move, score_from_side_to_move)`.
pub fn search(board: &Board, depth: u8, mut alpha: i32, beta: i32, adj: &WeightAdj) -> (Move, i32) {
    if depth == 0 {
        return (Move::null(), evaluate(board, adj));
    }

    let moves = generate_legal(board);

    if moves.is_empty() {
        return if board.in_check(board.side) {
            // Checkmate — losing; preferring faster mates
            (Move::null(), -INF + (SEARCH_DEPTH - depth) as i32)
        } else {
            (Move::null(), 0) // stalemate
        };
    }

    let mut best_move  = moves[0];
    let mut best_score = -INF;

    for mv in moves {
        let next = board.apply_move(mv);
        let (_, child_score) = search(&next, depth - 1, -beta, -alpha, adj);
        let score = -child_score;

        if score > best_score {
            best_score = score;
            best_move  = mv;
        }
        if score > alpha { alpha = score; }
        if alpha >= beta { break; }
    }

    (best_move, best_score)
}

/// Convenience: return best move at default depth.
pub fn best_move(board: &Board, adj: &WeightAdj) -> Move {
    search(board, SEARCH_DEPTH, -INF, INF, adj).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Board, W_KING, B_KING, W_QUEEN, B_QUEEN, EMPTY, WHITE, BLACK};
    use crate::eval::NO_ADJ;

    #[test]
    fn captures_hanging_queen() {
        // White queen on e4 can take an undefended black queen on e5.
        // No better move exists — the engine must capture.
        let mut b = Board::startpos();
        for sq in b.squares.iter_mut() { *sq = EMPTY; }
        b.squares[4]  = W_KING;   // e1
        b.squares[60] = B_KING;   // e8
        b.squares[28] = W_QUEEN;  // e4
        b.squares[36] = B_QUEEN;  // e5 (undefended)
        b.side    = WHITE;
        b.castling = 0;

        let mv = best_move(&b, &NO_ADJ);
        assert_eq!(mv.to_sq(), 36, "expected capture of hanging black queen on e5, got to={}", mv.to_sq());
    }

    #[test]
    fn stalemate_detected() {
        // Minimalist stalemate: black king cornered, white queen blocks all moves
        let mut b = Board::startpos();
        for sq in b.squares.iter_mut() { *sq = EMPTY; }
        b.squares[63] = B_KING;   // h8 (black king)
        b.squares[4]  = W_KING;   // e1 (white king)
        b.squares[53] = W_QUEEN;  // f7 (blocks all black king moves)
        b.side = BLACK;
        b.castling = 0;

        let (mv, score) = search(&b, SEARCH_DEPTH, -INF, INF, &NO_ADJ);
        assert!(mv.is_null());
        assert_eq!(score, 0, "stalemate should score 0, got {}", score);
    }

    #[test]
    fn best_move_not_null_from_startpos() {
        let b = Board::startpos();
        let mv = best_move(&b, &NO_ADJ);
        assert!(!mv.is_null());
        assert!(mv.from_sq() < 64);
        assert!(mv.to_sq()   < 64);
    }

    #[test]
    fn depth_zero_returns_null_move() {
        let b = Board::startpos();
        let (mv, _score) = search(&b, 0, -INF, INF, &NO_ADJ);
        assert!(mv.is_null());
    }

    #[test]
    fn takes_free_queen() {
        // White rook can capture a hanging black queen
        let mut b = Board::startpos();
        for sq in b.squares.iter_mut() { *sq = EMPTY; }
        b.squares[4]  = W_KING;
        b.squares[60] = B_KING;
        b.squares[0]  = crate::board::W_ROOK;   // a1
        b.squares[56] = crate::board::B_QUEEN;  // a8 (undefended)
        b.side = WHITE;
        b.castling = 0;

        let mv = best_move(&b, &NO_ADJ);
        assert_eq!(mv.to_sq(), 56, "expected rook captures queen on a8");
    }
}
