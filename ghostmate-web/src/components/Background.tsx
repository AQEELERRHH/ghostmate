import { useMemo } from 'react';

export function Background() {
  const particles = useMemo(() =>
    Array.from({ length: 25 }, (_, i) => ({
      id: i,
      left:  `${(i * 11.3 + 7) % 100}%`,
      delay: `-${(i * 0.7) % 8}s`,
      dur:   `${7 + (i * 0.9) % 5}s`,
      size:  `${2 + (i * 0.5) % 4}px`,
    })), []);

  return (
    <>
      {/* top-right glow */}
      <div style={{
        position: 'fixed', top: '-20%', right: '-10%',
        width: '60vw', height: '60vw',
        background: 'radial-gradient(circle, rgba(255,107,0,0.12) 0%, transparent 70%)',
        pointerEvents: 'none', zIndex: 0,
      }} />
      {/* bottom-left glow */}
      <div style={{
        position: 'fixed', bottom: '-20%', left: '-10%',
        width: '50vw', height: '50vw',
        background: 'radial-gradient(circle, rgba(255,107,0,0.07) 0%, transparent 70%)',
        pointerEvents: 'none', zIndex: 0,
      }} />
      {/* ember particles */}
      <div className="ember-container" aria-hidden="true">
        {particles.map(p => (
          <div
            key={p.id}
            className="ember"
            style={{
              left:   p.left,
              width:  p.size,
              height: p.size,
              '--dur':   p.dur,
              '--delay': p.delay,
            } as React.CSSProperties}
          />
        ))}
      </div>
    </>
  );
}
