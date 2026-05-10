import { useParams, Link } from 'react-router-dom';
import { useRegistry } from '../hooks/useRegistry';
import { GAME_STATUS } from '../config';
import { agentScore } from '../chess';
import { AgentAvatar } from '../components/AgentAvatar';
import { computeElo, getPersonality, PERSONALITY_CLASS } from '../utils/agentUtils';

export function Profile() {
  const { prefix } = useParams<{ prefix: string }>();
  const { agents, games } = useRegistry();

  const prefixBig = prefix ? BigInt(`0x${prefix}`) : null;
  const agent     = prefixBig ? agents.find(a => a.prefix === prefixBig) : null;

  const agentGames = prefixBig
    ? games.filter(g => g.whitePrefix === prefixBig || g.blackPrefix === prefixBig)
    : [];

  if (!prefix || !agent) {
    return (
      <div className="page">
        <div className="page-header">
          <h1 className="page-title">
            <span style={{ color: '#FF6B00' }}>AGENT</span> PROFILE
          </h1>
        </div>
        <div className="card">
          {agents.length === 0 ? (
            <p className="empty-state">
              No agents registered yet.{' '}
              <Link to="/register" className="link-green">Register one →</Link>
            </p>
          ) : (
            <>
              <p className="card-body muted">
                Select an agent from the leaderboard, or pick one below:
              </p>
              <ul className="agent-list">
                {agents.map(a => (
                  <li key={a.prefix.toString()}>
                    <Link to={`/profile/${a.prefix.toString(16)}`} className="agent-list-link">
                      {a.name}
                      <span className="agent-prefix">0x{a.prefix.toString(16).slice(0, 10)}…</span>
                    </Link>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      </div>
    );
  }

  const total   = agent.wins + agent.losses + agent.draws;
  const winRate = total > 0n
    ? ((Number(agent.wins) / Number(total)) * 100).toFixed(1)
    : '—';
  const elo         = computeElo(agent.wins, agent.losses, agent.draws);
  const personality = getPersonality(agent.wins, agent.losses, agent.draws, agent.name);

  return (
    <div className="page">
      <div className="page-header">
        <div style={{ display: 'flex', alignItems: 'center', gap: 20 }}>
          <AgentAvatar name={agent.name} size={72} />
          <div>
            <h1 className="page-title" style={{ color: 'white' }}>{agent.name}</h1>
            <p className="agent-id-display">0x{agent.prefix.toString(16)}</p>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginTop: 8 }}>
              <span className={`personality ${PERSONALITY_CLASS[personality]}`}>{personality}</span>
              <span style={{ fontFamily: 'Orbitron', fontSize: 14, fontWeight: 900, color: '#FF6B00' }}>{elo} ELO</span>
            </div>
          </div>
        </div>
        <Link to="/" className="btn btn-ghost">← LEADERBOARD</Link>
      </div>

      {/* Stat cards */}
      <div className="stat-grid">
        <StatCard label="Score"    value={(Number(agentScore(agent.wins, agent.draws)) / 2).toFixed(1)} accent />
        <StatCard label="Wins"     value={agent.wins.toString()}   color="green" />
        <StatCard label="Losses"   value={agent.losses.toString()} color="red" />
        <StatCard label="Draws"    value={agent.draws.toString()}  color="gray" />
        <StatCard label="Games"    value={(agent.wins + agent.losses + agent.draws).toString()} />
        <StatCard label="Win Rate" value={`${winRate}%`} />
      </div>

      {/* Game history */}
      <div className="card">
        <h2 className="card-title">GAME HISTORY</h2>
        {agentGames.length === 0 ? (
          <p className="empty-state">No games played yet.</p>
        ) : (
          <table className="history-table">
            <thead>
              <tr>
                <th>#</th>
                <th>WHITE</th>
                <th>BLACK</th>
                <th>RESULT</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {agentGames.map(g => {
                const asWhite = g.whitePrefix === prefixBig;
                let resultLabel = 'Active';
                let resultClass = 'result-active';

                if (g.status === GAME_STATUS.WHITE_WINS) {
                  resultLabel = asWhite ? 'Win' : 'Loss';
                  resultClass = asWhite ? 'result-win' : 'result-loss';
                } else if (g.status === GAME_STATUS.BLACK_WINS) {
                  resultLabel = asWhite ? 'Loss' : 'Win';
                  resultClass = asWhite ? 'result-loss' : 'result-win';
                } else if (g.status === GAME_STATUS.DRAW) {
                  resultLabel = 'Draw';
                  resultClass = 'result-draw';
                }

                return (
                  <tr key={g.id.toString()} className="history-row">
                    <td className="game-id">#{g.id.toString()}</td>
                    <td className={`player-cell ${g.whitePrefix === prefixBig ? 'is-me' : ''}`}>
                      <Link to={`/profile/${g.whitePrefix.toString(16)}`} className="player-link">
                        {g.whiteName}
                      </Link>
                    </td>
                    <td className={`player-cell ${g.blackPrefix === prefixBig ? 'is-me' : ''}`}>
                      <Link to={`/profile/${g.blackPrefix.toString(16)}`} className="player-link">
                        {g.blackName}
                      </Link>
                    </td>
                    <td>
                      <span className={`result-badge ${resultClass}`}>{resultLabel}</span>
                    </td>
                    <td>
                      {g.status === GAME_STATUS.ACTIVE && (
                        <Link to="/live" className="btn btn-ghost btn-sm">Watch →</Link>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

// ── Sub-component ─────────────────────────────────────────────────────────────

function StatCard({
  label,
  value,
  accent = false,
  color,
}: {
  label: string;
  value: string;
  accent?: boolean;
  color?: 'green' | 'red' | 'gray';
}) {
  return (
    <div className={`stat-card ${accent ? 'stat-accent' : ''} ${color ? `stat-${color}` : ''}`}>
      <span className="stat-value">{value}</span>
      <span className="stat-label">{label}</span>
    </div>
  );
}
