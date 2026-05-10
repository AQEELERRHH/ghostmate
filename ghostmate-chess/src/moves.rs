//! Pseudo-legal and legal move generation.

#[cfg(not(test))]
extern crate alloc;

#[cfg(not(test))]
use alloc::vec::Vec;

use super::board::{
    Board, Move, WHITE,
    EMPTY, W_PAWN, W_KNIGHT, W_BISHOP, W_ROOK, W_QUEEN, W_KING,
    B_PAWN, B_KNIGHT, B_BISHOP, B_ROOK, B_QUEEN, B_KING,
};

const fn rank(sq: u8) -> u8 { sq / 8 }
const fn file(sq: u8) -> u8 { sq % 8 }

fn add_sq(sq: u8, dr: i8, df: i8) -> Option<u8> {
    let r = rank(sq) as i8 + dr;
    let f = file(sq) as i8 + df;
    if r < 0 || r > 7 || f < 0 || f > 7 { None } else { Some((r * 8 + f) as u8) }
}

static BISHOP_DIRS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
static ROOK_DIRS:   [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1),  (0, -1) ];
static KNIGHT_DELTAS: [(i8, i8); 8] = [
    (2,1),(2,-1),(-2,1),(-2,-1),(1,2),(1,-2),(-1,2),(-1,-2),
];
static KING_DELTAS: [(i8, i8); 8] = [
    (1,0),(-1,0),(0,1),(0,-1),(1,1),(1,-1),(-1,1),(-1,-1),
];

/// All pseudo-legal moves for `side` (may leave own king in check).
pub fn generate_pseudo_legal(board: &Board, side: bool) -> Vec<Move> {
    let mut moves = Vec::new();
    for sq in 0u8..64 {
        let piece = board.piece_at(sq);
        if piece == EMPTY || !Board::is_own(piece, side) { continue; }
        match piece {
            W_PAWN   => gen_wpawn(board, sq, &mut moves),
            B_PAWN   => gen_bpawn(board, sq, &mut moves),
            W_KNIGHT | B_KNIGHT => gen_jump(board, sq, side, &KNIGHT_DELTAS, &mut moves),
            W_BISHOP | B_BISHOP => gen_slider(board, sq, side, &BISHOP_DIRS,  &mut moves),
            W_ROOK   | B_ROOK   => gen_slider(board, sq, side, &ROOK_DIRS,    &mut moves),
            W_QUEEN  | B_QUEEN  => {
                gen_slider(board, sq, side, &BISHOP_DIRS, &mut moves);
                gen_slider(board, sq, side, &ROOK_DIRS,   &mut moves);
            }
            W_KING | B_KING => {
                gen_jump(board, sq, side, &KING_DELTAS, &mut moves);
                gen_castling(board, sq, side, &mut moves);
            }
            _ => {}
        }
    }
    moves
}

/// All legal moves for the side to move.
pub fn generate_legal(board: &Board) -> Vec<Move> {
    generate_pseudo_legal(board, board.side)
        .into_iter()
        .filter(|&mv| !board.apply_move(mv).in_check(board.side))
        .collect()
}

fn push_pawn(from: u8, to: u8, promote: bool, moves: &mut Vec<Move>) {
    if promote {
        for promo in [W_QUEEN, W_ROOK, W_BISHOP, W_KNIGHT] {
            moves.push(Move::new(from, to, promo));
        }
    } else {
        moves.push(Move::new(from, to, 0));
    }
}

fn gen_wpawn(board: &Board, sq: u8, moves: &mut Vec<Move>) {
    let r = rank(sq);
    if let Some(fwd) = add_sq(sq, 1, 0) {
        if board.piece_at(fwd) == EMPTY {
            push_pawn(sq, fwd, r == 6, moves);
            if r == 1 {
                if let Some(fwd2) = add_sq(sq, 2, 0) {
                    if board.piece_at(fwd2) == EMPTY { moves.push(Move::new(sq, fwd2, 0)); }
                }
            }
        }
    }
    for df in [-1i8, 1] {
        if let Some(cap) = add_sq(sq, 1, df) {
            if Board::is_black_piece(board.piece_at(cap))
                || (board.ep_sq != 255 && cap == board.ep_sq) {
                push_pawn(sq, cap, r == 6, moves);
            }
        }
    }
}

fn gen_bpawn(board: &Board, sq: u8, moves: &mut Vec<Move>) {
    let r = rank(sq);
    if let Some(fwd) = add_sq(sq, -1, 0) {
        if board.piece_at(fwd) == EMPTY {
            push_pawn(sq, fwd, r == 1, moves);
            if r == 6 {
                if let Some(fwd2) = add_sq(sq, -2, 0) {
                    if board.piece_at(fwd2) == EMPTY { moves.push(Move::new(sq, fwd2, 0)); }
                }
            }
        }
    }
    for df in [-1i8, 1] {
        if let Some(cap) = add_sq(sq, -1, df) {
            if Board::is_white_piece(board.piece_at(cap))
                || (board.ep_sq != 255 && cap == board.ep_sq) {
                push_pawn(sq, cap, r == 1, moves);
            }
        }
    }
}

fn gen_jump(board: &Board, sq: u8, side: bool, deltas: &[(i8,i8)], moves: &mut Vec<Move>) {
    for &(dr, df) in deltas {
        if let Some(to) = add_sq(sq, dr, df) {
            let p = board.piece_at(to);
            if p == EMPTY || Board::is_opponent(p, side) { moves.push(Move::new(sq, to, 0)); }
        }
    }
}

fn gen_slider(board: &Board, sq: u8, side: bool, dirs: &[(i8,i8)], moves: &mut Vec<Move>) {
    for &(dr, df) in dirs {
        let mut cur = sq;
        loop {
            match add_sq(cur, dr, df) {
                None     => break,
                Some(to) => {
                    let p = board.piece_at(to);
                    if Board::is_own(p, side) { break; }
                    moves.push(Move::new(sq, to, 0));
                    if Board::is_opponent(p, side) { break; }
                    cur = to;
                }
            }
        }
    }
}

fn gen_castling(board: &Board, sq: u8, side: bool, moves: &mut Vec<Move>) {
    if side == WHITE && sq == 4 {
        if board.castling & 1 != 0 && board.piece_at(5) == EMPTY && board.piece_at(6) == EMPTY {
            moves.push(Move::new(4, 6, 0));
        }
        if board.castling & 2 != 0
            && board.piece_at(3) == EMPTY && board.piece_at(2) == EMPTY && board.piece_at(1) == EMPTY {
            moves.push(Move::new(4, 2, 0));
        }
    }
    if side != WHITE && sq == 60 {
        if board.castling & 4 != 0 && board.piece_at(61) == EMPTY && board.piece_at(62) == EMPTY {
            moves.push(Move::new(60, 62, 0));
        }
        if board.castling & 8 != 0
            && board.piece_at(59) == EMPTY && board.piece_at(58) == EMPTY && board.piece_at(57) == EMPTY {
            moves.push(Move::new(60, 58, 0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Board, BLACK};

    #[test]
    fn startpos_white_has_20_moves() {
        let b = Board::startpos();
        let moves = generate_legal(&b);
        // 16 pawn moves + 4 knight moves = 20
        assert_eq!(moves.len(), 20, "expected 20 opening moves, got {}", moves.len());
    }

    #[test]
    fn black_has_20_moves_after_e4() {
        let b = Board::startpos();
        let e4 = Move::new(12, 28, 0); // e2-e4
        let b2 = b.apply_move(e4);
        assert_eq!(b2.side, BLACK);
        let moves = generate_legal(&b2);
        assert_eq!(moves.len(), 20);
    }

    #[test]
    fn no_moves_in_simple_checkmate() {
        // Fool's mate position: 1.f3 e5 2.g4 Qh4#
        let mut b = Board::startpos();
        b = b.apply_move(Move::new(13, 21, 0)); // f2-f3
        b = b.apply_move(Move::new(52, 36, 0)); // e7-e5
        b = b.apply_move(Move::new(14, 30, 0)); // g2-g4
        b = b.apply_move(Move::new(59, 31, 0)); // Qd8-h4
        // White to move — should be in checkmate
        assert!(b.in_check(WHITE));
        let moves = generate_legal(&b);
        assert_eq!(moves.len(), 0);
    }

    #[test]
    fn captures_generated() {
        // Place a black pawn on e4 where white can capture from d3
        let mut b = Board::startpos();
        for sq in b.squares.iter_mut() { *sq = EMPTY; }
        b.squares[4]  = W_KING;
        b.squares[60] = B_KING;
        b.squares[19] = W_PAWN;  // d3
        b.squares[28] = B_PAWN;  // e4
        b.side = WHITE;
        let moves = generate_legal(&b);
        let captures: Vec<_> = moves.iter().filter(|m| m.to_sq() == 28).collect();
        assert!(!captures.is_empty(), "expected at least one capture of e4 pawn");
    }

    #[test]
    fn en_passant_generated() {
        let mut b = Board::startpos();
        for sq in b.squares.iter_mut() { *sq = EMPTY; }
        b.squares[4]  = W_KING;
        b.squares[60] = B_KING;
        b.squares[36] = W_PAWN;  // e5
        b.squares[37] = B_PAWN;  // f5
        b.ep_sq = 45;            // f6 (black just double-pushed f7-f5)
        b.side  = WHITE;
        let moves = generate_legal(&b);
        let ep: Vec<_> = moves.iter().filter(|m| m.to_sq() == 45).collect();
        assert_eq!(ep.len(), 1, "expected one en-passant capture");
    }
}
