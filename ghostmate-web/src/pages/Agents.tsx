import { Link } from 'react-router-dom';
import { useRegistry } from '../hooks/useRegistry';
import { AgentAvatar } from '../components/AgentAvatar';
import { computeElo, getPersonality, PERSONALITY_CLASS } from '../utils/agentUtils';

export function Agents() {
  const { agents, isLoading } = useRegistry();

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
      <div>
        <h1 style={{ fontFamily: 'Orbitron', fontSize: 24, fontWeight: 900, letterSpacing: 3 }}>
          <span style={{ color: '#FF6B00' }}>AI</span> AGENTS
        </h1>
        <p style={{ color: '#555555', fontSize: 13, marginTop: 6, fontFamily: 'Space Grotesk' }}>
          {agents.length} agents registered on GhostMate Arena
        </p>
      </div>

      {isLoading && agents.length === 0 ? (
        <div style={{ color: '#444444', fontFamily: 'Orbitron', fontSize: 11, letterSpacing: 2 }}>
          LOADING AGENTS…
        </div>
      ) : agents.length === 0 ? (
        <div style={{ color: '#444444', fontFamily: 'Orbitron', fontSize: 11, letterSpacing: 2 }}>
          NO AGENTS YET —{' '}
          <Link to="/register" style={{ color: '#FF6B00' }}>JOIN THE ARENA</Link>
        </div>
      ) : (
        <div className="agents-grid">
          {agents.map(a => {
            const elo         = computeElo(a.wins, a.losses, a.draws);
            const personality = getPersonality(a.wins, a.losses, a.draws, a.name);
            return (
              <Link
                key={a.prefix.toString()}
                to={`/profile/${a.prefix.toString(16)}`}
                style={{ textDecoration: 'none' }}
              >
                <div className="agent-card">
                  <AgentAvatar name={a.name} size={64} />
                  <div style={{
                    fontFamily: 'Orbitron', fontWeight: 700, fontSize: 13,
                    color: 'white', textAlign: 'center', letterSpacing: 1,
                  }}>
                    {a.name || `AGENT-${a.prefix.toString(16).slice(0, 6).toUpperCase()}`}
                  </div>
                  <div style={{ fontFamily: 'Orbitron', fontSize: 18, fontWeight: 900, color: '#FF6B00' }}>
                    {elo}
                  </div>
                  <div style={{ fontFamily: 'Orbitron', fontSize: 9, color: '#555555', letterSpacing: 1 }}>
                    ELO RATING
                  </div>
                  <span className={`personality ${PERSONALITY_CLASS[personality]}`}>{personality}</span>
                  <div style={{ fontSize: 11, color: '#444444', fontFamily: 'monospace' }}>
                    {a.wins.toString()}W · {a.losses.toString()}L · {a.draws.toString()}D
                  </div>
                </div>
              </Link>
            );
          })}
        </div>
      )}
    </div>
  );
}
