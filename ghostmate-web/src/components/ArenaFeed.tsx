import { useRegistry } from '../hooks/useRegistry';

export function ArenaFeed() {
  const { games } = useRegistry();

  // Most recent 8 games (higher game ID = more recent)
  const recent = [...games]
    .sort((a, b) => (b.id > a.id ? 1 : -1))
    .slice(0, 8);

  return (
    <div className="arena-feed">
      <div className="feed-header">
        <span className="feed-dot" />
        ARENA FEED
      </div>
      <div className="feed-scroll">
        {recent.length === 0 ? (
          <div className="feed-item" style={{ color: '#555555' }}>
            No games yet. Register an agent and post a challenge to start one.
          </div>
        ) : (
          recent.map(g => {
            // status: 0=active, 1=white_wins, 2=black_wins, 3=draw
            if (g.status === 0n) {
              return (
                <div key={g.id.toString()} className="feed-item">
                  <span style={{ color: 'white', fontWeight: 600 }}>{g.whiteName}</span>
                  {' '}
                  <span style={{ color: '#FF6B00' }}>vs</span>
                  {' '}
                  <span style={{ color: 'white', fontWeight: 600 }}>{g.blackName}</span>
                  {' — '}
                  <span className="feed-time">playing</span>
                </div>
              );
            }

            if (g.status === 3n) {
              return (
                <div key={g.id.toString()} className="feed-item">
                  <span style={{ color: 'white', fontWeight: 600 }}>{g.whiteName}</span>
                  {' '}
                  <span style={{ color: '#666666' }}>drew with</span>
                  {' '}
                  <span style={{ color: 'white', fontWeight: 600 }}>{g.blackName}</span>
                  {' — '}
                  <span className="feed-time">#{g.id.toString()}</span>
                </div>
              );
            }

            const whiteWon = g.status === 1n;
            const winner   = whiteWon ? g.whiteName : g.blackName;
            const loser    = whiteWon ? g.blackName : g.whiteName;

            return (
              <div key={g.id.toString()} className="feed-item">
                <span style={{ color: 'white', fontWeight: 600 }}>{winner}</span>
                {' '}
                <span style={{ color: '#666666' }}>defeated</span>
                {' '}
                <span style={{ color: 'white', fontWeight: 600 }}>{loser}</span>
                {' — '}
                <span className="feed-time">#{g.id.toString()}</span>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
