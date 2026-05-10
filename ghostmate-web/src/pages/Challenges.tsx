import { useState } from 'react';
import {
  useAccounts,
  useTransaction,
  useWaitForCommit,
} from '@miden-sdk/react';
import { TxStatus } from '../components/TxStatus';
import { useRegistry, type ChallengeRecord } from '../hooks/useRegistry';
import { CHALLENGE_STATUS } from '../config';

export function Challenges() {
  const { wallets }                                                               = useAccounts();
  const { challenges, isLoading, error, refresh, upsertChallenge }               = useRegistry();
  const { execute, stage, isLoading: txLoading, error: txError, reset: resetTx } = useTransaction();
  const { waitForCommit }                                                         = useWaitForCommit();

  const [postWager,   setPostWager]   = useState('');
  const [pendingId,   setPendingId]   = useState<bigint | null>(null);
  const [postLoading, setPostLoading] = useState(false);
  const [postError,   setPostError]   = useState<string | null>(null);

  const connected = wallets.length > 0;
  const myWallet  = wallets[0];

  // ── Post a new challenge ──────────────────────────────────────────────────────
  const handlePost = async () => {
    if (!myWallet) return;
    setPostLoading(true);
    setPostError(null);
    const wager     = BigInt(Math.round(parseFloat(postWager || '0') * 1_000_000));
    const accountId = myWallet.id().toString();

    try {
      const result = await execute({
        accountId,
        request: async (_client) => {
          throw new Error('Connect to deployed MatchRegistry to post challenges');
        },
      });
      if (result) {
        await waitForCommit(result.transactionId, { timeoutMs: 30_000 });
        const tmpId = BigInt(Date.now());
        upsertChallenge(
          tmpId,
          BigInt(`0x${accountId.replace(/^0x/, '').slice(0, 16)}`),
          wager,
          CHALLENGE_STATUS.OPEN,
        );
        setPostWager('');
      }
    } catch (err) {
      setPostError(err instanceof Error ? err.message : String(err));
    } finally {
      setPostLoading(false);
    }
  };

  // ── Accept a challenge ────────────────────────────────────────────────────────
  const handleAccept = async (challenge: ChallengeRecord) => {
    if (!myWallet) return;
    setPendingId(challenge.id);
    const accountId = myWallet.id().toString();

    try {
      const result = await execute({
        accountId,
        request: async (_client) => {
          throw new Error('Connect to deployed MatchRegistry to accept challenges');
        },
      });
      if (result) {
        await waitForCommit(result.transactionId, { timeoutMs: 30_000 });
        upsertChallenge(challenge.id, challenge.challengerPrefix, challenge.wagerAmount, CHALLENGE_STATUS.ACCEPTED);
      }
    } finally {
      setPendingId(null);
    }
  };

  return (
    <div className="page">
      <div className="page-header">
        <div>
          <h1 className="page-title">
            <span style={{ color: '#FF6B00' }}>OPEN</span> CHALLENGES
          </h1>
          <p className="page-subtitle">Challenges looking for an opponent on Miden.</p>
        </div>
        <button className="btn btn-ghost" onClick={refresh} disabled={isLoading}>
          ↻ REFRESH
        </button>
      </div>

      {error && <div className="alert-error">{error}</div>}

      {/* Post new challenge */}
      {connected && (
        <div className="card post-challenge">
          <h2 className="card-title">POST A CHALLENGE</h2>
          <div className="post-row">
            <div className="input-group">
              <label className="input-label">Wager (MIDEN)</label>
              <input
                className="text-input"
                type="number"
                min="0"
                step="0.000001"
                placeholder="0"
                value={postWager}
                onChange={e => setPostWager(e.target.value)}
              />
            </div>
            <button
              className="btn btn-primary"
              onClick={handlePost}
              disabled={postLoading}
            >
              {postLoading ? 'POSTING…' : 'POST CHALLENGE'}
            </button>
          </div>
          {postError && <div className="alert-error" style={{ marginTop: 12 }}>{postError}</div>}
        </div>
      )}

      {!connected && (
        <div className="alert-info">
          Connect a wallet on the <strong>Wallet</strong> page to post or accept challenges.
        </div>
      )}

      {/* Challenge list */}
      <div className="card">
        {challenges.length === 0 && !isLoading ? (
          <p className="empty-state">No open challenges. Be the first to post one!</p>
        ) : (
          <table className="challenges-table">
            <thead>
              <tr>
                <th>#</th>
                <th>AGENT</th>
                <th>WAGER</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {challenges.map(ch => {
                const isMine = myWallet
                  && ch.challengerPrefix.toString(16) === myWallet.id().toString().replace(/^0x/, '').slice(0, 16);
                const accepting = pendingId === ch.id;

                return (
                  <tr key={ch.id.toString()} className="challenge-row">
                    <td className="challenge-id">#{ch.id.toString()}</td>
                    <td className="challenge-agent">{ch.challengerName}</td>
                    <td className="challenge-wager">
                      {ch.wagerAmount > 0n
                        ? `${(Number(ch.wagerAmount) / 1_000_000).toFixed(6)} MIDEN`
                        : <span className="no-wager">No wager</span>}
                    </td>
                    <td className="challenge-action">
                      {isMine ? (
                        <span className="badge-mine">Yours</span>
                      ) : (
                        <button
                          className="btn btn-orange-outline btn-sm"
                          onClick={() => handleAccept(ch)}
                          disabled={!connected || accepting || txLoading}
                        >
                          {accepting ? 'ACCEPTING…' : 'ACCEPT'}
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}

        {isLoading && (
          <div className="table-loading">
            <span className="spinner" /> LOADING…
          </div>
        )}
      </div>

      <TxStatus stage={stage} isLoading={txLoading} error={txError} onReset={resetTx} />
    </div>
  );
}
