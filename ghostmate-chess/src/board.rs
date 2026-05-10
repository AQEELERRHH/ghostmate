//! Board state representation and move application.

// Piece constants
pub const EMPTY: u8 = 0;
pub const W_PAWN: u8 = 1;
pub const W_KNIGHT: u8 = 2;
pub const W_BISHOP: u8 = 3;
pub const W_ROOK: u8 = 4;
pub const W_QUEEN: u8 = 5;
pub const W_KING: u8 = 6;
pub const B_PAWN: u8 = 7;
pub const B_KNIGHT: u8 = 8;
pub const B_BISHOP: u8 = 9;
pub const B_ROOK: u8 = 10;
pub const B_QUEEN: u8 = 11;
pub const B_KING: u8 = 12;

pub const WHITE: bool = true;
pub const BLACK: bool = false;

/// A chess move: from (6 bits) | to (6 bits) | promotion (4 bits) packed in u16.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Move(pub u16);

impl Move {
    pub fn new(from: u8, to: u8, promo: u8) -> Self {
        Move((from as u16) | ((to as u16) << 6) | ((promo as u16) << 12))
    }
    pub fn from_sq(self) -> u8  { (self.0 & 0x3F) as u8 }
    pub fn to_sq(self)   -> u8  { ((self.0 >> 6) & 0x3F) as u8 }
    pub fn promo(self)   -> u8  { ((self.0 >> 12) & 0x0F) as u8 }

    pub fn null() -> Self { Move(0) }
    pub fn is_null(self) -> bool { self.0 == 0 }

    /// Pack into a u64 for Miden Felt storage.
    pub fn to_u64(self) -> u64 { self.0 as u64 }
    pub fn from_u64(v: u64) -> Self { Move(v as u16) }
}

/// Board state.
///
/// Encoding wire format (4 × u64, one "board word"):
///   Each u64 holds 16 squares × 4 bits = 64 bits.
///   Squares ordered a1=0 … h8=63 (rank-major).
///
/// Flags wire format (4 × u64, one "flags word"):
///   [0] = side (1=white, 0=black)
///   [1] = ep_sq (0-63, or 255 = none)
///   [2] = castling bitmask (WK=1, WQ=2, BK=4, BQ=8)
///   [3] = reserved (0)
#[derive(Clone, Debug)]
pub struct Board {
    pub squares: [u8; 64],
    pub side:    bool,
    pub ep_sq:   u8,
    pub castling: u8,
}

impl Board {
    /// Standard starting position.
    pub fn startpos() -> Self {
        let mut sq = [EMPTY; 64];
        sq[0] = W_ROOK;  sq[1] = W_KNIGHT; sq[2] = W_BISHOP; sq[3] = W_QUEEN;
        sq[4] = W_KING;  sq[5] = W_BISHOP; sq[6] = W_KNIGHT; sq[7] = W_ROOK;
        for i in 8..16 { sq[i] = W_PAWN; }
        sq[56] = B_ROOK; sq[57] = B_KNIGHT; sq[58] = B_BISHOP; sq[59] = B_QUEEN;
        sq[60] = B_KING; sq[61] = B_BISHOP; sq[62] = B_KNIGHT; sq[63] = B_ROOK;
        for i in 48..56 { sq[i] = B_PAWN; }
        Board { squares: sq, side: WHITE, ep_sq: 255, castling: 0b1111 }
    }

    /// Decode from wire words (each `[u64; 4]` corresponds to a Miden Word).
    pub fn from_words(board: [u64; 4], flags: [u64; 4]) -> Self {
        let mut squares = [EMPTY; 64];
        for felt_idx in 0..4usize {
            let val = board[felt_idx];
            for nibble in 0..16usize {
                squares[felt_idx * 16 + nibble] = ((val >> (nibble * 4)) & 0xF) as u8;
            }
        }
        Board {
            squares,
            side:     flags[0] == 1,
            ep_sq:    flags[1] as u8,
            castling: flags[2] as u8,
        }
    }

    /// Encode board squares into wire word.
    pub fn to_board_word(&self) -> [u64; 4] {
        let mut out = [0u64; 4];
        for sq in 0..64usize {
            out[sq / 16] |= (self.squares[sq] as u64) << ((sq % 16) * 4);
        }
        out
    }

    /// Encode flags into wire word.
    pub fn to_flags_word(&self) -> [u64; 4] {
        [
            if self.side { 1 } else { 0 },
            self.ep_sq as u64,
            self.castling as u64,
            0,
        ]
    }

    pub fn piece_at(&self, sq: u8) -> u8 { self.squares[sq as usize] }

    pub fn is_white_piece(p: u8) -> bool { p >= W_PAWN && p <= W_KING }
    pub fn is_black_piece(p: u8) -> bool { p >= B_PAWN }
    pub fn is_own(p: u8, side: bool) -> bool {
        if side == WHITE { Self::is_white_piece(p) } else { Self::is_black_piece(p) }
    }
    pub fn is_opponent(p: u8, side: bool) -> bool {
        p != EMPTY && !Self::is_own(p, side)
    }

    /// Apply a move, returning a new board.
    pub fn apply_move(&self, mv: Move) -> Self {
        let mut next = self.clone();
        let from  = mv.from_sq() as usize;
        let to    = mv.to_sq()   as usize;
        let piece = next.squares[from];
        next.squares[from] = EMPTY;

        // En-passant capture
        if (piece == W_PAWN || piece == B_PAWN) && self.ep_sq != 255 && to == self.ep_sq as usize {
            let cap = if self.side == WHITE { to - 8 } else { to + 8 };
            next.squares[cap] = EMPTY;
        }

        // Promotion
        next.squares[to] = if mv.promo() != 0 { mv.promo() } else { piece };

        // Update en-passant square
        next.ep_sq = 255;
        if piece == W_PAWN && to == from + 16 { next.ep_sq = (from + 8) as u8; }
        if piece == B_PAWN && from == to  + 16 { next.ep_sq = (to  + 8) as u8; }

        // Castling king rook moves
        if piece == W_KING {
            next.castling &= !0b0011;
            if from == 4 && to == 6 { next.squares[5] = W_ROOK; next.squares[7] = EMPTY; }
            if from == 4 && to == 2 { next.squares[3] = W_ROOK; next.squares[0] = EMPTY; }
        }
        if piece == B_KING {
            next.castling &= !0b1100;
            if from == 60 && to == 62 { next.squares[61] = B_ROOK; next.squares[63] = EMPTY; }
            if from == 60 && to == 58 { next.squares[59] = B_ROOK; next.squares[56] = EMPTY; }
        }
        if from == 0  { next.castling &= !0b0010; }
        if from == 7  { next.castling &= !0b0001; }
        if from == 56 { next.castling &= !0b1000; }
        if from == 63 { next.castling &= !0b0100; }

        next.side = !self.side;
        next
    }

    /// True if `side`'s king is attacked by an opponent pseudo-legal move.
    pub fn in_check(&self, side: bool) -> bool {
        use super::moves::generate_pseudo_legal;
        let king = if side == WHITE { W_KING } else { B_KING };
        let mut king_sq = 64u8;
        for sq in 0..64u8 { if self.squares[sq as usize] == king { king_sq = sq; break; } }
        if king_sq == 64 { return false; } // no king (shouldn't happen in legal positions)
        generate_pseudo_legal(self, !side).iter().any(|m| m.to_sq() == king_sq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startpos_piece_count() {
        let b = Board::startpos();
        let whites: u8 = b.squares.iter().filter(|&&p| Board::is_white_piece(p)).count() as u8;
        let blacks: u8 = b.squares.iter().filter(|&&p| Board::is_black_piece(p)).count() as u8;
        assert_eq!(whites, 16);
        assert_eq!(blacks, 16);
    }

    #[test]
    fn startpos_kings_on_correct_squares() {
        let b = Board::startpos();
        assert_eq!(b.squares[4],  W_KING);
        assert_eq!(b.squares[60], B_KING);
    }

    #[test]
    fn board_word_roundtrip() {
        let b = Board::startpos();
        let bw = b.to_board_word();
        let fw = b.to_flags_word();
        let b2 = Board::from_words(bw, fw);
        assert_eq!(b.squares, b2.squares);
        assert_eq!(b.side,     b2.side);
        assert_eq!(b.castling, b2.castling);
    }

    #[test]
    fn apply_e2e4_double_pawn_push() {
        let b = Board::startpos();
        // e2 = square 12 (rank 1, file 4), e4 = square 28
        let mv = Move::new(12, 28, 0);
        let b2 = b.apply_move(mv);
        assert_eq!(b2.squares[12], EMPTY);
        assert_eq!(b2.squares[28], W_PAWN);
        assert_eq!(b2.ep_sq, 20); // e3
        assert_eq!(b2.side, BLACK);
    }

    #[test]
    fn apply_promotion() {
        let mut b = Board::startpos();
        // Manually place a white pawn on e7 (rank 6, file 4 = sq 52)
        b.squares[52] = W_PAWN;
        b.squares[12] = EMPTY;
        b.squares[60] = EMPTY; // remove black king from e8 to avoid check issues
        let mv = Move::new(52, 60, W_QUEEN);
        let b2 = b.apply_move(mv);
        assert_eq!(b2.squares[60], W_QUEEN);
        assert_eq!(b2.squares[52], EMPTY);
    }

    #[test]
    fn castling_rights_revoked_on_rook_move() {
        let b = Board::startpos();
        // Move h1-rook (sq 7) away
        let mv = Move::new(7, 15, 0); // not legal but tests castling bookkeeping
        let b2 = b.apply_move(mv);
        assert_eq!(b2.castling & 0b0001, 0); // WK castling gone
        assert_ne!(b2.castling & 0b0010, 0); // WQ still present
    }

    #[test]
    fn move_encoding_roundtrip() {
        let mv = Move::new(12, 28, W_QUEEN);
        assert_eq!(mv.from_sq(), 12);
        assert_eq!(mv.to_sq(),   28);
        assert_eq!(mv.promo(),   W_QUEEN);
        let v = mv.to_u64();
        let mv2 = Move::from_u64(v);
        assert_eq!(mv, mv2);
    }
}
