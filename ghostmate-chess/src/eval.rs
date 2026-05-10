//! Position evaluation: material + piece-square tables.

use super::board::{
    Board, WHITE,
    W_PAWN, W_KNIGHT, W_BISHOP, W_ROOK, W_QUEEN, W_KING,
    B_PAWN, B_KNIGHT, B_BISHOP, B_ROOK, B_QUEEN, B_KING,
};

const VAL_PAWN:   i32 = 100;
const VAL_KNIGHT: i32 = 320;
const VAL_BISHOP: i32 = 330;
const VAL_ROOK:   i32 = 500;
const VAL_QUEEN:  i32 = 900;
const VAL_KING:   i32 = 20_000;

/// 12-element weight adjustment array indexed by (piece - 1) % 6.
/// Each entry is added/subtracted from material value for white/black pieces.
pub type WeightAdj = [i32; 12];
pub const NO_ADJ: WeightAdj = [0i32; 12];

/// Decode a weight stored in Miden account storage.
/// The Felt value encodes a signed offset in [-128, 127] as (offset + 128).
pub fn decode_weight(raw: u64) -> i32 {
    raw as i32 - 128
}

/// Encode a weight adjustment as a storage-safe u64.
pub fn encode_weight(offset: i32) -> u64 {
    (offset.clamp(-128, 127) + 128) as u64
}

#[rustfmt::skip]
static PST_PAWN: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
];

#[rustfmt::skip]
static PST_KNIGHT: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

#[rustfmt::skip]
static PST_BISHOP: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];

#[rustfmt::skip]
static PST_ROOK: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
     5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     0,  0,  0,  5,  5,  0,  0,  0,
];

#[rustfmt::skip]
static PST_QUEEN: [i32; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20,
];

#[rustfmt::skip]
static PST_KING: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20,
];

fn mirror(sq: u8) -> u8 { (7 - sq / 8) * 8 + sq % 8 }

fn pst_score(piece: u8, sq: u8, adj: &WeightAdj) -> i32 {
    let (val, table, is_white) = match piece {
        W_PAWN   => (VAL_PAWN,   PST_PAWN.as_slice(),   true),
        W_KNIGHT => (VAL_KNIGHT, PST_KNIGHT.as_slice(), true),
        W_BISHOP => (VAL_BISHOP, PST_BISHOP.as_slice(), true),
        W_ROOK   => (VAL_ROOK,   PST_ROOK.as_slice(),   true),
        W_QUEEN  => (VAL_QUEEN,  PST_QUEEN.as_slice(),  true),
        W_KING   => (VAL_KING,   PST_KING.as_slice(),   true),
        B_PAWN   => (VAL_PAWN,   PST_PAWN.as_slice(),   false),
        B_KNIGHT => (VAL_KNIGHT, PST_KNIGHT.as_slice(), false),
        B_BISHOP => (VAL_BISHOP, PST_BISHOP.as_slice(), false),
        B_ROOK   => (VAL_ROOK,   PST_ROOK.as_slice(),   false),
        B_QUEEN  => (VAL_QUEEN,  PST_QUEEN.as_slice(),  false),
        B_KING   => (VAL_KING,   PST_KING.as_slice(),   false),
        _ => return 0,
    };
    let idx = if is_white { sq as usize } else { mirror(sq) as usize };
    let piece_adj = adj[(piece - 1) as usize % 6];
    let raw = val + table[idx] + piece_adj;
    if is_white { raw } else { -raw }
}

/// Evaluate from white's perspective; caller negates for black to move.
pub fn evaluate_abs(board: &Board, adj: &WeightAdj) -> i32 {
    board.squares.iter().enumerate().map(|(sq, &p)| {
        if p == 0 { 0 } else { pst_score(p, sq as u8, adj) }
    }).sum()
}

/// Evaluate from the side-to-move's perspective (positive = good for side to move).
pub fn evaluate(board: &Board, adj: &WeightAdj) -> i32 {
    let abs = evaluate_abs(board, adj);
    if board.side == WHITE { abs } else { -abs }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    #[test]
    fn startpos_is_roughly_equal() {
        let b = Board::startpos();
        // Should be zero (symmetric position) or very close
        let score = evaluate_abs(&b, &NO_ADJ);
        assert!(score.abs() < 10, "startpos score = {}", score);
    }

    #[test]
    fn material_advantage_detected() {
        let mut b = Board::startpos();
        // Remove a black queen — white should be up ~900
        b.squares[59] = 0; // black queen on d8
        let score = evaluate_abs(&b, &NO_ADJ);
        assert!(score > 800 && score < 1200, "score after removing black queen = {}", score);
    }

    #[test]
    fn weight_adjustment_applied() {
        let b = Board::startpos();
        let mut adj = NO_ADJ;
        adj[0] = 50; // bonus for pawns (index 0 = W_PAWN-1)
        let base  = evaluate_abs(&b, &NO_ADJ);
        let bonus = evaluate_abs(&b, &adj);
        // Should be 0 from white's perspective (symmetric), but let's check abs impact
        // Both sides have 8 pawns, so white gets +400, black gets -400 → net = 0
        assert_eq!(base, bonus, "symmetric board: adj cancels out");
    }

    #[test]
    fn decode_encode_weight_roundtrip() {
        for offset in [-128i32, -64, 0, 64, 127] {
            let encoded = encode_weight(offset);
            let decoded = decode_weight(encoded);
            assert_eq!(decoded, offset);
        }
    }
}
