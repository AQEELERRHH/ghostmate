import { useState, useEffect } from 'react';
import { useRegistry } from '../hooks/useRegistry';
import { useGame } from '../hooks/useGame';
import { ChessBoard, startposSquares } from '../components/ChessBoard';
import { MoveHistory } from '../components/MoveHistory';
import { AgentAvatar } from '../components/AgentAvatar';
import { GAME_STATUS } from '../config';
import { toBech32AccountId } from '@miden-sdk/react';

function useGameTimer(active: boolean): string {
  const [secs, setSecs] = useState(0);
  useEffect(() => {
    if (!active) return;
    const id = setInterval(() => setSecs(s => s + 1), 1000);
    return () => clearInterval(id);
  }, [active]);
  const m = String(Math.floor(secs / 60)).padStart(2, '0');
  const s = String(secs % 60).padStart(2, '0');
  return `${m}:${s}`;
}

export function LiveGame() {
  const { games } = useRegistry();
  const activeGames = games.filter(g => g.status === GAME_STATUS.ACTIVE);
  const [selectedGameId, setSelectedGameId] = useState<string | null>(null);
  const [spectators] = useState(() => 12421 + Math.floor(Math.random() * 200));

  const selectedGame = games.find(g => g.id.toString() === selectedGameId) ?? null;
  const gcId = selectedGame
    ? (() => {
        try {
          return toBech32AccountId('0x' + selectedGame.gameContractPrefix.toString(16).padStart(16, '0'));
        } catch {
          return null;
        }
      })()
    : null;

  const { state, isLive } = useGame(gcId);
  const timer = useGameTimer(!!selectedGame && state.status === GAME_STATUS.ACTIVE);
  const displaySquares = state.ply > 0 ? state.squares : startposSquares();

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <h1 style={{ fontFamily: 'Orbitron', fontSize: 24, fontWeight: 900, letterSpacing: 3, textTransform: 'uppercase' }}>
          <span style={{ color: '#FF6B00' }}>LIVE</span> ARENA
        </h1>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          {isLive && (
            <span style={{ fontFamily: 'Orbitron', fontSize: 10, color: '#FF6B00', letterSpacing: 2, animation: 'pulse 1.5s infinite' }}>
              ● LIVE
            </span>
          )}
          <span className="spectator-badge">👁 {spectators.toLocaleString()} watching</span>
        </div>
      </div>

      {/* Game selector */}
      <div style={{ background: '#111111', border: '1px solid rgba(255,107,0,0.12)', borderRadius: 8, padding: 16 }}>
        <label style={{ fontFamily: 'Orbitron', fontSize: 10, color: '#555555', letterSpacing: 2, display: 'block', marginBottom: 8 }}>
          SELECT GAME
        </label>
        <select
          style={{ width: '100%', background: '#0D0D0D', border: '1px solid rgba(255,107,0,0.2)', borderRadius: 6, color: 'white', padding: '9px 12px', fontFamily: 'Space Grotesk', outline: 'none' }}
          value={selectedGameId ?? ''}
          onChange={e => setSelectedGameId(e.target.value || null)}
        >
          <option value="">— Choose a game —</option>
          {activeGames.map(g => (
            <option key={g.id.toString()} value={g.id.toString()}>
              #{g.id.toString()} · {g.whiteName} vs {g.blackName}
            </option>
          ))}
        </select>
        {activeGames.length === 0 && (
          <p style={{ color: '#444444', fontSize: 11, marginTop: 12, fontFamily: 'Orbitron', letterSpacing: 1 }}>
            NO ACTIVE GAMES — ACCEPT A CHALLENGE TO START ONE
          </p>
        )}
      </div>

      {selectedGame ? (
        <div className="arena-layout">
          {/* Left: white agent */}
          <div className={`agent-portal${state.side === 0 ? ' active-portal' : ''}`}>
            <AgentAvatar name={selectedGame.whiteName} size={72} />
            <div style={{ fontFamily: 'Orbitron', fontWeight: 900, fontSize: 14, color: state.side === 0 ? '#FF6B00' : 'white' }}>
              {selectedGame.whiteName}
            </div>
            <div style={{ fontFamily: 'Orbitron', fontSize: 10, color: '#555555' }}>WHITE</div>
            {state.side === 0 && state.status === GAME_STATUS.ACTIVE && (
              <div className="thinking-anim">
                THINKING
                <div className="thinking-dots">
                  <span /><span /><span />
                </div>
              </div>
            )}
            <div className={`game-timer${state.side === 0 ? ' timer-active' : ''}`}>{timer}</div>
          </div>

          {/* Center: board */}
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 12 }}>
            <div style={{ position: 'relative' }}>
              <ChessBoard squares={displaySquares} lastFrom={state.lastFrom} lastTo={state.lastTo} />
              {state.status !== GAME_STATUS.ACTIVE && (
                <div style={{
                  position: 'absolute', inset: 0,
                  background: 'rgba(10,10,10,0.88)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                  fontFamily: 'Orbitron', fontSize: 22, fontWeight: 900, color: '#FF6B00',
                  borderRadius: 4,
                }}>
                  {state.status === GAME_STATUS.WHITE_WINS ? '♔ WHITE WINS'
                    : state.status === GAME_STATUS.BLACK_WINS ? '♚ BLACK WINS'
                    : '½–½  DRAW'}
                </div>
              )}
            </div>
            <div style={{ fontFamily: 'Orbitron', fontSize: 11, color: '#444444', letterSpacing: 2 }}>
              PLY {state.ply} · HM {state.halfmoveClock}/100
            </div>
          </div>

          {/* Right: black agent + move history */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
            <div className={`agent-portal${state.side === 1 ? ' active-portal' : ''}`}>
              <AgentAvatar name={selectedGame.blackName} size={72} />
              <div style={{ fontFamily: 'Orbitron', fontWeight: 900, fontSize: 14, color: state.side === 1 ? '#FF6B00' : 'white' }}>
                {selectedGame.blackName}
              </div>
              <div style={{ fontFamily: 'Orbitron', fontSize: 10, color: '#555555' }}>BLACK</div>
              {state.side === 1 && state.status === GAME_STATUS.ACTIVE && (
                <div className="thinking-anim">
                  THINKING
                  <div className="thinking-dots"><span /><span /><span /></div>
                </div>
              )}
            </div>
            <MoveHistory moves={state.moves} status={state.status} />
          </div>
        </div>
      ) : (
        <div style={{
          display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 16,
          padding: 40, background: '#111111', border: '1px solid rgba(255,107,0,0.1)', borderRadius: 10,
        }}>
          <ChessBoard squares={startposSquares()} />
          <p style={{ fontFamily: 'Orbitron', fontSize: 11, color: '#444444', letterSpacing: 2 }}>
            SELECT A GAME TO ENTER THE ARENA
          </p>
        </div>
      )}
    </div>
  );
}
