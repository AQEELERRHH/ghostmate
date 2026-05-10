// ── Piece constants (mirror ghostmate-chess/src/board.rs) ─────────────────────

export const EMPTY   = 0;
export const W_PAWN  = 1; export const W_KNIGHT = 2; export const W_BISHOP = 3;
export const W_ROOK  = 4; export const W_QUEEN  = 5; export const W_KING   = 6;
export const B_PAWN  = 7; export const B_KNIGHT = 8; export const B_BISHOP = 9;
export const B_ROOK  = 10; export const B_QUEEN  = 11; export const B_KING  = 12;

export const WHITE = 0;
export const BLACK = 1;

// Unicode glyphs; index = piece code
const GLYPHS = ['', '♙', '♘', '♗', '♖', '♕', '♔', '♟', '♞', '♝', '♜', '♛', '♚'];
export function pieceGlyph(piece: number): string { return GLYPHS[piece] ?? ''; }
export function isWhitePiece(piece: number): boolean { return piece >= 1 && piece <= 6; }
export function isBlackPiece(piece: number): boolean { return piece >= 7 && piece <= 12; }

// ── Board wire format ─────────────────────────────────────────────────────────

export type BoardWords = [bigint, bigint, bigint, bigint];

/**
 * Decode a 64-square board from 4 packed u64s.
 * Square i lives at boardWords[i>>4], bits [(i&0xF)*4 .. (i&0xF)*4+3].
 */
export function decodeBoardWords(words: BoardWords): Uint8Array {
  const squares = new Uint8Array(64);
  for (let sq = 0; sq < 64; sq++) {
    const wi = sq >> 4;
    const shift = BigInt((sq & 0xf) * 4);
    squares[sq] = Number((words[wi] >> shift) & 0xfn);
  }
  return squares;
}

// ── Square / move notation ────────────────────────────────────────────────────

export function sqFile(sq: number): number { return sq % 8; }
export function sqRank(sq: number): number { return Math.floor(sq / 8); }

export function sqToAlg(sq: number): string {
  return String.fromCharCode(0x61 + sqFile(sq)) + (sqRank(sq) + 1);
}

const PROMO_CHARS: Record<number, string> = {
  [W_QUEEN]: 'q', [W_ROOK]: 'r', [W_BISHOP]: 'b', [W_KNIGHT]: 'n',
  [B_QUEEN]: 'q', [B_ROOK]: 'r', [B_BISHOP]: 'b', [B_KNIGHT]: 'n',
};

export function moveToUci(from: number, to: number, promo: number): string {
  return sqToAlg(from) + sqToAlg(to) + (promo ? (PROMO_CHARS[promo] ?? '') : '');
}

// ── Name encoding ─────────────────────────────────────────────────────────────

/**
 * Pack a display name (up to 32 chars) into 4 BigInt felts.
 * Each felt holds 8 UTF-8 bytes in big-endian order.
 */
export function encodeName(name: string): [bigint, bigint, bigint, bigint] {
  const buf = new Uint8Array(32);
  const encoded = new TextEncoder().encode(name.slice(0, 32));
  buf.set(encoded);
  const felts: bigint[] = [];
  for (let i = 0; i < 4; i++) {
    let v = 0n;
    for (let j = 0; j < 8; j++) v = (v << 8n) | BigInt(buf[i * 8 + j]);
    felts.push(v);
  }
  return felts as [bigint, bigint, bigint, bigint];
}

/** Reverse of encodeName. Strips null padding. */
export function decodeName(felts: bigint[]): string {
  const buf = new Uint8Array(32);
  for (let i = 0; i < 4; i++) {
    let v = felts[i] ?? 0n;
    for (let j = 7; j >= 0; j--) {
      buf[i * 8 + j] = Number(v & 0xffn);
      v >>= 8n;
    }
  }
  let end = buf.indexOf(0);
  if (end === -1) end = 32;
  return new TextDecoder().decode(buf.slice(0, end));
}

// ── Leaderboard score (mirror ghostmate-registry) ─────────────────────────────

export function agentScore(wins: bigint, draws: bigint): bigint {
  return wins * 2n + draws;
}
