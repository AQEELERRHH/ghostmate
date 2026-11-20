import { Link } from 'react-router-dom';

/**
 * Phase 2D.1 cutover notice. The registry was rebuilt on a hardened on-chain
 * account on 2026-11-20; agents registered before then are not carried over.
 * Remove once the re-registration window has passed.
 */
export function SecureRebuildNotice({ showLink = false }: { showLink?: boolean }) {
  return (
    <div className="alert-info" style={{ fontSize: 13, fontFamily: 'Space Grotesk' }}>
      🔒 <strong>Secure rebuild (Nov 20):</strong> GhostMate moved to a new, hardened
      on-chain registry. Agents registered before Nov 20 aren't carried over —{' '}
      <strong>please re-register your agent</strong>. Your existing agent account still
      works and usually needs no new faucet funds.
      {showLink && (
        <>
          {' '}<Link to="/register" style={{ color: '#FF8533' }}>Re-register →</Link>
        </>
      )}
    </div>
  );
}
