import { useEffect, useRef } from 'react';
import type { MoveRecord } from '../hooks/useGame';

interface Props {
  moves: MoveRecord[];
  status: bigint;
}

const STATUS_LABEL: Record<string, string> = {
  '0': 'ACTIVE',
  '1': 'WHITE WINS',
  '2': 'BLACK WINS',
  '3': 'DRAW',
};

export function MoveHistory({ moves, status }: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [moves.length]);

  const pairs: [MoveRecord, MoveRecord | null][] = [];
  for (let i = 0; i < moves.length; i += 2) {
    pairs.push([moves[i], moves[i + 1] ?? null]);
  }

  const statusStr = status.toString();

  return (
    <div className="move-history">
      <div className="mh-header">
        <span>MOVE HISTORY</span>
        <span style={{
          color: status === 0n ? '#FF6B00' : status === 3n ? '#888888' : '#ffffff',
          fontSize: 9,
        }}>
          ● {STATUS_LABEL[statusStr] ?? 'UNKNOWN'}
        </span>
      </div>
      <div style={{ overflowY: 'auto', flex: 1 }}>
        {pairs.length === 0 && (
          <div style={{
            padding: '20px 16px',
            color: '#444444',
            fontSize: 13,
            textAlign: 'center',
            fontFamily: 'Orbitron',
            letterSpacing: 1,
          }}>
            AWAITING FIRST MOVE…
          </div>
        )}
        {pairs.map(([w, b], idx) => (
          <div key={idx} className="move-pair">
            <span className="move-num">{idx + 1}.</span>
            <span className="white-move">{w.uci}</span>
            {b && <span className="black-move">{b.uci}</span>}
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
