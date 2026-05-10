import { useState, useEffect } from 'react';

const SEED_EVENTS = [
  { a: 'NEURALREAPER', b: 'ROOK-X',      result: 'defeated',  ago: '1m'  },
  { a: 'BISHOP-7',     b: 'ENDGAME',     result: 'drew with', ago: '3m'  },
  { a: 'DEEPTHOUGHT',  b: 'PAWN-STORM',  result: 'defeated',  ago: '7m'  },
  { a: 'QUEEN-ZERO',   b: 'GHOST-V2',    result: 'defeated',  ago: '12m' },
  { a: 'ROOK-X',       b: 'CALCULUS',    result: 'drew with', ago: '15m' },
  { a: 'ENDGAME',      b: 'NEURALREAPER',result: 'defeated',  ago: '22m' },
];

interface FeedEvent {
  a: string;
  b: string;
  result: string;
  ago: string;
}

export function ArenaFeed() {
  const [events, setEvents] = useState<FeedEvent[]>(SEED_EVENTS);

  useEffect(() => {
    const names = [
      'NEURALREAPER', 'ROOK-X', 'BISHOP-7', 'DEEPTHOUGHT',
      'PAWN-STORM', 'QUEEN-ZERO', 'GHOST-V2', 'CALCULUS',
      'ENDGAME', 'PHANTOM-9',
    ];
    const interval = setInterval(() => {
      const a = names[Math.floor(Math.random() * names.length)];
      let b = names[Math.floor(Math.random() * names.length)];
      while (b === a) b = names[Math.floor(Math.random() * names.length)];
      const result = Math.random() > 0.3 ? 'defeated' : 'drew with';
      setEvents(prev => [{ a, b, result, ago: 'just now' }, ...prev.slice(0, 7)]);
    }, 12000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="arena-feed">
      <div className="feed-header">
        <span className="feed-dot" />
        ARENA FEED
      </div>
      <div className="feed-scroll">
        {events.map((e, i) => (
          <div key={i} className="feed-item">
            <span style={{ color: 'white', fontWeight: 600 }}>{e.a}</span>
            {' '}
            <span style={{ color: '#666666' }}>{e.result}</span>
            {' '}
            <span style={{ color: 'white', fontWeight: 600 }}>{e.b}</span>
            {' — '}
            <span className="feed-time">{e.ago}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
