import { pieceGlyph, isWhitePiece } from '../chess';

interface Props {
  squares: Uint8Array;
  lastFrom?: number | null;
  lastTo?: number | null;
  flipped?: boolean;
}

const FILES = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
const RANKS = ['1', '2', '3', '4', '5', '6', '7', '8'];

export function ChessBoard({ squares, lastFrom, lastTo, flipped = false }: Props) {
  const ranks = flipped ? RANKS : [...RANKS].reverse();
  const files = flipped ? [...FILES].reverse() : FILES;

  return (
    <div className="chess-board-wrap">
      <div style={{ display: 'flex', gap: 6, userSelect: 'none' }}>
        {/* rank labels */}
        <div style={{ display: 'flex', flexDirection: 'column', justifyContent: 'space-around' }}>
          {ranks.map(r => (
            <span key={r} style={{ fontSize: 11, color: '#444', width: 14, textAlign: 'center' }}>{r}</span>
          ))}
        </div>
        <div>
          <div className="board-grid">
            {ranks.map((_, ri) =>
              files.map((_, fi) => {
                const rank  = flipped ? ri : 7 - ri;
                const file  = flipped ? 7 - fi : fi;
                const sq    = rank * 8 + file;
                const piece = squares[sq] ?? 0;
                const isLight = (rank + file) % 2 === 1;
                const isHL    = sq === lastFrom || sq === lastTo;
                let cls = `board-sq ${isLight ? 'sq-light' : 'sq-dark'}`;
                if (isHL) cls += ' sq-highlight';
                return (
                  <div key={sq} className={cls}>
                    {piece > 0 && (
                      <span className={`piece ${isWhitePiece(piece) ? 'piece-white' : 'piece-black'}`}>
                        {pieceGlyph(piece)}
                      </span>
                    )}
                  </div>
                );
              }),
            )}
          </div>
          {/* file labels */}
          <div style={{ display: 'flex', justifyContent: 'space-around', marginTop: 4 }}>
            {files.map(f => (
              <span key={f} style={{ fontSize: 11, color: '#444', width: 64, textAlign: 'center' }}>{f}</span>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Startpos helpers ──────────────────────────────────────────────────────────

const FEN_PIECE: Record<string, number> = {
  P: 1, N: 2, B: 3, R: 4, Q: 5, K: 6,
  p: 7, n: 8, b: 9, r: 10, q: 11, k: 12,
};

export function startposSquares(): Uint8Array {
  const STARTPOS_FEN = 'rnbqkbnrpppppppp................................PPPPPPPPRNBQKBNR';
  const sq = new Uint8Array(64);
  const rows = STARTPOS_FEN.split('');
  for (let i = 0; i < 64; i++) {
    const rank = 7 - Math.floor(i / 8);
    const file = i % 8;
    sq[rank * 8 + file] = FEN_PIECE[rows[i]] ?? 0;
  }
  return sq;
}
