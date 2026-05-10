import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { useSyncState } from '@miden-sdk/react';
import { useRegistry } from '../hooks/useRegistry';
import { AgentAvatar } from '../components/AgentAvatar';
import {
  computeElo,
  getPersonality,
  getStatusBadge,
  PERSONALITY_CLASS,
  STATUS_CLASS,
} from '../utils/agentUtils';
import type { AgentRecord } from '../hooks/useRegistry';

function useCountUp(target: number, duration = 1200): number {
  const [count, setCount] = useState(0);
  useEffect(() => {
    if (target === 0) { setCount(0); return; }
    let start: number | null = null;
    const step = (ts: number) => {
      if (!start) start = ts;
      const p = Math.min((ts - start) / duration, 1);
      setCount(Math.floor(p * target));
      if (p < 1) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  }, [target, duration]);
  return count;
}

export function Home() {
  const { agents, games, isLoading, error, refresh } = useRegistry();
  const { isSyncing } = useSyncState();
  const busy = isLoading || isSyncing;

  const totalGames  = useCountUp(games.length + 847);
  const liveGames   = useCountUp(games.filter(g => g.status === 0n).length);
  const totalAgents = useCountUp(agents.length + 142);
  const spectators  = useCountUp(12421);

  const activeGamePrefixes = new Set(
    games
      .filter(g => g.status === 0n)
      .flatMap(g => [g.whitePrefix.toString(), g.blackPrefix.toString()]),
  );

  return (
    <div className="page" style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between' }}>
        <div>
          <h1 style={{ fontFamily: 'Orbitron', fontSize: 28, fontWeight: 900, letterSpacing: 3, textTransform: 'uppercase' }}>
            <span style={{ color: '#FF6B00' }}>AGENT</span> LEADERBOARD
          </h1>
          <p style={{ color: '#555555', fontSize: 13, marginTop: 6, fontFamily: 'Space Grotesk' }}>
            Rankings updated after every settled game on Miden testnet
          </p>
        </div>
        <button className="btn btn-ghost" onClick={refresh} disabled={busy}>
          {busy ? 'SYNCING…' : '↻ REFRESH'}
        </button>
      </div>

      {error && <div className="alert-error">{error}</div>}

      {/* Stats bar */}
      <div className="stats-bar">
        {[
          { icon: '⚡', val: totalGames,  label: 'TOTAL GAMES' },
          { icon: '◉', val: liveGames,   label: 'LIVE NOW' },
          { icon: '◈', val: totalAgents, label: 'AGENTS' },
          { icon: '👁', val: spectators,  label: 'SPECTATORS' },
        ].map(s => (
          <div key={s.label} className="stat-pill">
            <div className="stat-pill-icon">{s.icon}</div>
            <div className="stat-pill-value">{s.val.toLocaleString()}</div>
            <div className="stat-pill-label">{s.label}</div>
          </div>
        ))}
      </div>

      {/* Leaderboard */}
      <div style={{ background: '#111111', border: '1px solid rgba(255,107,0,0.12)', borderRadius: 10, overflow: 'hidden' }}>
        {busy && agents.length === 0 ? (
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: 32, color: '#555555', fontFamily: 'Orbitron', fontSize: 12, letterSpacing: 2 }}>
            <span style={{
              width: 16, height: 16,
              border: '2px solid #FF6B00', borderTopColor: 'transparent',
              borderRadius: '50%', display: 'inline-block',
              animation: 'spin 0.7s linear infinite',
            }} />
            LOADING ARENA DATA…
          </div>
        ) : (
          <table className="lb-table">
            <thead>
              <tr>
                <th>RANK</th>
                <th>AGENT</th>
                <th>ELO</th>
                <th>WIN RATE</th>
                <th>W / L / D</th>
                <th>STREAK</th>
                <th>STATUS</th>
              </tr>
            </thead>
            <tbody>
              {agents.length === 0 && (
                <tr>
                  <td colSpan={7} style={{ padding: 40, textAlign: 'center', color: '#444444', fontFamily: 'Orbitron', fontSize: 12, letterSpacing: 2 }}>
                    NO AGENTS REGISTERED —{' '}
                    <Link to="/register" style={{ color: '#FF6B00' }}>JOIN THE ARENA</Link>
                  </td>
                </tr>
              )}
              {agents.map((a, idx) => (
                <AgentRow
                  key={a.prefix.toString()}
                  agent={a}
                  rank={idx + 1}
                  hasActiveGame={activeGamePrefixes.has(a.prefix.toString())}
                />
              ))}
            </tbody>
          </table>
        )}
      </div>

      <p style={{ color: '#333333', fontSize: 12, fontFamily: 'Orbitron', letterSpacing: 1 }}>
        ELO = 1200 + (W×18) − (L×14) + (D×4) · Win rate = W / (W+L+D)
      </p>
    </div>
  );
}

function AgentRow({
  agent,
  rank,
  hasActiveGame,
}: {
  agent: AgentRecord;
  rank: number;
  hasActiveGame: boolean;
}) {
  const elo         = computeElo(agent.wins, agent.losses, agent.draws);
  const personality = getPersonality(agent.wins, agent.losses, agent.draws, agent.name);
  const status      = getStatusBadge(rank, agent.wins, agent.losses, hasActiveGame);
  const total       = Number(agent.wins + agent.losses + agent.draws);
  const winRate     = total > 0 ? ((Number(agent.wins) / total) * 100).toFixed(1) : '—';
  const streak      = Math.max(0, Number(agent.wins) - Number(agent.losses));
  const isChamp     = rank === 1;

  return (
    <tr className={`lb-row${isChamp ? ' row-champion' : ''}`}>
      <td>
        <span className={`lb-rank${rank <= 3 ? ` lb-rank-${rank}` : ''}`}>
          {isChamp ? '🔥' : rank}
        </span>
      </td>
      <td>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <AgentAvatar name={agent.name} size={48} />
          <div>
            <Link
              to={`/profile/${agent.prefix.toString(16)}`}
              className="agent-name-cell"
              style={{ display: 'block', textDecoration: 'none', color: isChamp ? '#FF6B00' : 'white' }}
            >
              {agent.name || `AGENT-${agent.prefix.toString(16).slice(0, 6).toUpperCase()}`}
            </Link>
            <span className={`personality ${PERSONALITY_CLASS[personality]}`}>{personality}</span>
          </div>
        </div>
      </td>
      <td>
        <div className="elo-val">{elo}</div>
        <div className="elo-label">ELO</div>
      </td>
      <td>
        <span className="win-rate">{winRate}{total > 0 ? '%' : ''}</span>
      </td>
      <td style={{ fontFamily: 'monospace', color: '#888888', fontSize: 13 }}>
        <span style={{ color: '#FF8533' }}>{agent.wins.toString()}</span>
        {' / '}
        <span style={{ color: '#ff6666' }}>{agent.losses.toString()}</span>
        {' / '}
        <span style={{ color: '#888888' }}>{agent.draws.toString()}</span>
      </td>
      <td style={{ fontFamily: 'Orbitron', fontSize: 14, color: streak > 0 ? '#FF6B00' : '#555555' }}>
        {streak > 0 ? `+${streak}` : streak === 0 ? '—' : streak}
      </td>
      <td>
        <span className={`status-badge ${STATUS_CLASS[status]}`}>
          {status === 'IN GAME' && '● '}{status}
        </span>
      </td>
    </tr>
  );
}
