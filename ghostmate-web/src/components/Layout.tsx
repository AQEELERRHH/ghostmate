import { NavLink, Outlet } from 'react-router-dom';
import { useSyncState } from '@miden-sdk/react';
import { useWallet } from '@miden-sdk/miden-wallet-adapter-react';
import { Background } from './Background';
import { ArenaFeed } from './ArenaFeed';

const NAV = [
  { to: '/',            label: 'LEADERBOARD', icon: '⬆',  soon: false },
  { to: '/live',        label: 'LIVE GAMES',  icon: '◉',  soon: false },
  { to: '/agents',      label: 'AGENTS',      icon: '◈',  soon: false },
  { to: '/challenges',  label: 'CHALLENGES',  icon: '⚔',  soon: false },
  { to: '/register',    label: 'WALLET',      icon: '◎',  soon: false },
  { to: '/tournaments', label: 'TOURNAMENTS', icon: '⬡',  soon: true  },
  { to: '/watch',       label: 'WATCH',       icon: '👁',  soon: true  },
  { to: '/settings',    label: 'SETTINGS',    icon: '⚙',  soon: true  },
] as const;

export function Layout() {
  const { isSyncing } = useSyncState();
  const { connected, address } = useWallet();

  return (
    <div className="app-shell">
      <Background />

      <nav className="sidebar">
        <div style={{ padding: '0 20px 24px', borderBottom: '1px solid rgba(255,107,0,0.1)', marginBottom: 16 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <span className="logo-icon">♟</span>
            <span className="logo-text">GHOSTMATE</span>
          </div>
        </div>

        <ul style={{
          listStyle: 'none', flex: 1,
          display: 'flex', flexDirection: 'column',
          gap: 2, padding: '0 12px',
        }}>
          {NAV.map(({ to, label, icon, soon }) => (
            <li key={to}>
              {soon ? (
                <div className="nav-item nav-soon">
                  <span style={{ fontSize: 14 }}>{icon}</span>
                  <span>{label}</span>
                  <span style={{ marginLeft: 'auto', fontSize: 9, color: '#444', fontFamily: 'Orbitron' }}>SOON</span>
                </div>
              ) : (
                <NavLink
                  to={to}
                  end={to === '/'}
                  className={({ isActive }) => `nav-item${isActive ? ' active' : ''}`}
                >
                  <span style={{ fontSize: 14 }}>{icon}</span>
                  <span>{label}</span>
                </NavLink>
              )}
            </li>
          ))}
        </ul>

        <div style={{
          padding: '16px 20px',
          borderTop: '1px solid rgba(255,107,0,0.1)',
          marginTop: 'auto',
          display: 'flex',
          flexDirection: 'column',
          gap: 10,
        }}>
          <div className="network-badge">
            <span
              className="network-dot"
              style={{ animation: isSyncing ? 'pulse 0.6s infinite' : 'pulse 2s infinite' }}
            />
            <span style={{ fontFamily: 'Orbitron', fontSize: 10, letterSpacing: 1 }}>
              {isSyncing ? 'SYNCING...' : 'MIDEN TESTNET'}
            </span>
          </div>
          {connected && address && (
            <div className="wallet-addr">
              {address.slice(0, 14)}…
            </div>
          )}
        </div>
      </nav>

      <main className="content" style={{ position: 'relative', zIndex: 2 }}>
        <Outlet />
      </main>

      <ArenaFeed />
    </div>
  );
}
