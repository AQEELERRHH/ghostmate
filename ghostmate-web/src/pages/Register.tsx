import { useState, useCallback, useEffect } from 'react';
import {
  useCreateWallet,
  useAccount,
  useWaitForNotes,
  useConsume,
  useTransaction,
  useWaitForCommit,
  useSyncState,
  useMiden,
} from '@miden-sdk/react';
import { AccountId, NoteArray } from '@miden-sdk/miden-sdk';
import { TxStatus } from '../components/TxStatus';
import { SecureRebuildNotice } from '../components/SecureRebuildNotice';
import { useRegistry, readAgentName } from '../hooks/useRegistry';
import { buildRegisterNote } from '../lib/registerNote';
import { REGISTRY_ID } from '../config';
import { decodeName, encodeName } from '../chess';

// Namespaced so future multi-account UI can coexist cleanly.
const AGENT_KEY = 'ghostmate:agent_account_id';
const FAUCET_URL = 'https://faucet.testnet.miden.io/';
// The network operator usually consumes the register-note ~10s after commit.
const OPERATOR_TIMEOUT_MS = 180_000;
const OPERATOR_POLL_MS = 5_000;

function humanError(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === 'string') return err;
  return 'Unexpected error';
}

export function Register() {
  const { client, runExclusive }                         = useMiden();
  const { createWallet, isCreating }                     = useCreateWallet();
  const { waitForConsumableNotes }                       = useWaitForNotes();
  const { consume }                                      = useConsume();
  const { execute, stage, isLoading: txLoading, error: txError, reset: resetTx } = useTransaction();
  const { waitForCommit }                                = useWaitForCommit();
  const { sync }                                         = useSyncState();
  const { refresh }                                      = useRegistry();

  const [agentId,   setAgentId]   = useState<string | null>(null);
  const [agentName, setAgentName] = useState('');
  const [status,    setStatus]    = useState('');
  const [busy,      setBusy]      = useState(false);
  const [done,      setDone]      = useState(false);
  const [error,     setError]     = useState<string | null>(null);
  const [showGhost, setShowGhost] = useState(false);

  // The agent account is a LOCAL account, distinct from any browser wallet.
  const { account } = useAccount(agentId ?? undefined);

  useEffect(() => {
    const saved = localStorage.getItem(AGENT_KEY);
    if (saved) setAgentId(saved);
  }, []);

  const triggerGhost = () => {
    setShowGhost(true);
    setTimeout(() => setShowGhost(false), 2200);
  };

  // ── Create the local agent account ──────────────────────────────────────────
  const createAgent = useCallback(async () => {
    setError(null);
    try {
      const wallet = await createWallet({ storageMode: 'public' });
      if (!wallet) throw new Error('Account creation returned no account.');
      const id = wallet.id().toString();
      localStorage.setItem(AGENT_KEY, id);
      setAgentId(id);
      triggerGhost();
    } catch (err) {
      setError(`Could not create your agent account: ${humanError(err)}. You can try again.`);
    }
  }, [createWallet]);

  // ── Export the local agent account file (portability / backup) ──────────────
  const exportAgent = useCallback(async () => {
    if (!client || !agentId) return;
    setError(null);
    try {
      const file = await client.exportAccountFile(AccountId.fromHex(agentId));
      const src = file.serialize();
      const bytes = new Uint8Array(src.length);
      bytes.set(src);
      const blob = new Blob([bytes], { type: 'application/octet-stream' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `ghostmate-agent-${agentId.replace(/^0x/, '').slice(0, 10)}.mac`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      setError(`Could not export your agent account: ${humanError(err)}`);
    }
  }, [client, agentId]);

  // ── Full registration flow ──────────────────────────────────────────────────
  const handleRegister = useCallback(async () => {
    if (!client || !agentId || !agentName.trim()) return;
    setError(null);
    setBusy(true);
    try {
      setStatus('Syncing with the network…');
      await sync();

      // 1. Fund + commit the agent account by consuming its faucet note(s).
      //    Skipped for an agent that is already on-chain (nonce > 0) — e.g.
      //    re-registering an existing agent on the new secure registry.
      const local = await client.getAccount(AccountId.fromHex(agentId));
      const onChain = !!local && local.nonce().asInt() > 0n;
      if (!onChain) {
        setStatus('Waiting for faucet funds to arrive at your agent account…');
        const faucetNotes = await waitForConsumableNotes({
          accountId: agentId,
          minCount: 1,
          timeoutMs: 600_000,
        });
        setStatus('Claiming funds into your agent account…');
        await consume({
          accountId: agentId,
          notes: faucetNotes.map((n) => n.inputNoteRecord()),
        });
        await sync();
      }

      // 2. Emit the register-note (a network note) from the agent account.
      setStatus('Step 1 of 2 — emitting your registration note…');
      const note = await buildRegisterNote(
        AccountId.fromHex(agentId),
        agentName.trim(),
        AccountId.fromHex(REGISTRY_ID),
      );
      const noteId = note.id().toString();
      const emitted = await execute({
        accountId: agentId,
        request: async (c) => {
          const outs = new NoteArray();
          outs.push(note);
          const builder = await c.feeAwareTransactionRequestBuilder(AccountId.fromHex(agentId));
          return builder.withOwnOutputNotes(outs).build();
        },
      });
      if (!emitted?.transactionId) throw new Error('Registration note was not emitted.');
      await waitForCommit(emitted.transactionId, { timeoutMs: 60_000 });
      await sync();

      // 3. The Miden network operator consumes the note AS the registry (which
      //    keys the agent by its authenticated id). Nothing for us to submit —
      //    poll the registry's storage until the agent shows up.
      setStatus('Step 2 of 2 — the Miden network is processing your registration (usually ~10–30s)…');
      const expected = decodeName([...encodeName(agentName.trim())]);
      const deadline = Date.now() + OPERATOR_TIMEOUT_MS;
      let registered = false;
      while (Date.now() < deadline) {
        const onChainName = await runExclusive(() => readAgentName(client, agentId));
        if (onChainName === expected) { registered = true; break; }
        await new Promise((r) => setTimeout(r, OPERATOR_POLL_MS));
      }
      if (!registered) {
        throw new Error(
          `Your registration note was sent (note ${noteId.slice(0, 12)}…), but the network ` +
          `hasn't processed it yet. Check the leaderboard in a few minutes — ` +
          `there's no need to register again.`,
        );
      }

      setStatus('');
      setDone(true);
      triggerGhost();
      void refresh();
    } catch (err) {
      setError(humanError(err));
    } finally {
      setBusy(false);
    }
  }, [
    client, runExclusive, agentId, agentName, sync, waitForConsumableNotes, consume,
    execute, waitForCommit, refresh,
  ]);

  const shortId = agentId ? `${agentId.slice(0, 10)}…${agentId.slice(-6)}` : '';

  return (
    <div style={{ maxWidth: 560, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 28, paddingTop: 20 }}>
      {showGhost && <div className="ghost-mascot" aria-hidden="true">👻</div>}

      <div style={{ textAlign: 'center' }}>
        <h1 className="arena-heading">JOIN THE <span className="highlight">ARENA</span></h1>
        <p style={{ color: '#555555', marginTop: 8, fontFamily: 'Space Grotesk' }}>
          Deploy your AI chess agent on Miden testnet and compete on-chain.
        </p>
      </div>

      <SecureRebuildNotice />

      {error && <div className="alert-error">{error}</div>}

      {/* Step 1: create the agent account */}
      {!agentId ? (
        <div style={{ background: '#111111', border: '1px solid rgba(255,107,0,0.12)', borderRadius: 10, padding: 32, display: 'flex', flexDirection: 'column', gap: 16 }}>
          <h2 style={{ fontFamily: 'Orbitron', fontSize: 16, color: '#FF6B00' }}>CREATE YOUR AGENT ACCOUNT</h2>
          <p style={{ color: '#888888', fontSize: 14, fontFamily: 'Space Grotesk' }}>
            GhostMate creates a dedicated <strong>agent account</strong> on Miden testnet for you.
            This is a GhostMate account — <strong>it is not your browser/Bread wallet</strong>.
            You'll fund it with free testnet tokens, then register.
          </p>
          <button className="connect-wallet-btn" onClick={createAgent} disabled={isCreating}>
            {isCreating ? 'CREATING…' : '✦ CREATE AGENT ACCOUNT'}
          </button>
        </div>
      ) : (
        <>
          {/* Agent account identity + export (always visible once created) */}
          <div style={{ background: '#111111', border: '1px solid rgba(255,107,0,0.18)', borderRadius: 10, padding: 24, display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: 12 }}>
              <h2 style={{ fontFamily: 'Orbitron', fontSize: 14, color: '#FF6B00', letterSpacing: 1 }}>YOUR AGENT ACCOUNT</h2>
              <button
                onClick={exportAgent}
                title="Download a backup of your agent account"
                style={{ background: 'transparent', border: '1px solid rgba(255,107,0,0.4)', borderRadius: 6, color: '#FF8533', fontFamily: 'Orbitron', fontSize: 10, letterSpacing: 1, padding: '6px 10px', cursor: 'pointer' }}
              >
                ↓ EXPORT
              </button>
            </div>
            <p style={{ fontFamily: 'monospace', color: '#cccccc', fontSize: 12, wordBreak: 'break-all' }}>
              {account?.bech32id?.() ?? agentId}
            </p>
            <div className="alert-info" style={{ fontSize: 12 }}>
              ⚠️ This is your GhostMate <strong>agent account</strong>, not your Bread wallet.{' '}
              <strong>Export it</strong> to back it up — clearing browser storage will otherwise lose this agent.
            </div>
          </div>

          {!done ? (
            <>
              {/* Fund panel */}
              <div style={{ background: '#111111', border: '1px solid rgba(255,107,0,0.12)', borderRadius: 10, padding: 24, display: 'flex', flexDirection: 'column', gap: 12 }}>
                <h2 style={{ fontFamily: 'Orbitron', fontSize: 14, color: '#FF6B00', letterSpacing: 1 }}>STEP 1 — FUND YOUR AGENT</h2>
                <p style={{ color: '#888888', fontSize: 14, fontFamily: 'Space Grotesk' }}>
                  Your agent needs a little testnet USDCx to pay its own transaction fee.
                  Open the faucet, paste the address below (<span style={{ fontFamily: 'monospace' }}>{shortId}</span>),
                  request funds, then come back and register. Re-registering an agent you funded
                  before? Skip this step — it still has funds.
                </p>
                <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                  <a className="connect-wallet-btn" href={FAUCET_URL} target="_blank" rel="noreferrer" style={{ textAlign: 'center', textDecoration: 'none' }}>
                    ⛲ OPEN TESTNET FAUCET
                  </a>
                  <button
                    onClick={() => { navigator.clipboard?.writeText(agentId); }}
                    style={{ background: 'transparent', border: '1px solid rgba(255,107,0,0.3)', borderRadius: 8, color: '#FF8533', fontFamily: 'Orbitron', fontSize: 11, cursor: 'pointer', padding: '0 16px' }}
                  >
                    ⧉ COPY ADDRESS
                  </button>
                </div>
              </div>

              {/* Register panel */}
              <div style={{ background: '#111111', border: '1px solid rgba(255,107,0,0.12)', borderRadius: 10, padding: 24, display: 'flex', flexDirection: 'column', gap: 16 }}>
                <h2 style={{ fontFamily: 'Orbitron', fontSize: 14, color: '#FF6B00', letterSpacing: 1 }}>STEP 2 — NAME & REGISTER</h2>
                <p style={{ color: '#555555', fontSize: 13, fontFamily: 'Space Grotesk' }}>
                  This name appears on the leaderboard. Max 32 characters. Your agent sends one
                  registration note; the Miden network then processes it automatically on the registry.
                </p>
                <input
                  className="text-input"
                  placeholder="Choose an agent name"
                  maxLength={32}
                  value={agentName}
                  onChange={(e) => setAgentName(e.target.value.toUpperCase())}
                  disabled={busy}
                  style={{ fontFamily: 'Orbitron', letterSpacing: 2, fontSize: 14 }}
                />
                <button className="connect-wallet-btn" onClick={handleRegister} disabled={busy || !agentName.trim()}>
                  {busy ? 'REGISTERING…' : 'REGISTER AGENT →'}
                </button>
                {busy && status && (
                  <p style={{ color: '#FF8533', fontSize: 13, fontFamily: 'Space Grotesk' }}>{status}</p>
                )}
                {busy && <TxStatus stage={stage} isLoading={txLoading} error={txError} onReset={resetTx} />}
              </div>
            </>
          ) : (
            <div style={{ background: 'rgba(255,107,0,0.06)', border: '1px solid rgba(255,107,0,0.35)', borderRadius: 10, padding: 32, textAlign: 'center', display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div style={{ fontSize: 48 }}>👻</div>
              <h2 style={{ fontFamily: 'Orbitron', fontSize: 18, color: '#FF6B00', letterSpacing: 3 }}>AGENT REGISTERED</h2>
              <p style={{ color: '#888888', fontFamily: 'Space Grotesk' }}>
                <span style={{ color: 'white', fontWeight: 700 }}>{agentName}</span> is on the GhostMate leaderboard.
              </p>
              <p style={{ color: '#666666', fontSize: 12 }}>Remember to keep your exported agent-account file safe.</p>
            </div>
          )}
        </>
      )}
    </div>
  );
}
