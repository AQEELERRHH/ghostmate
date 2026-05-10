import { useState, useCallback } from 'react';
import {
  useImportAccount,
  useTransaction,
  useWaitForCommit,
} from '@miden-sdk/react';
import { useWallet } from '@miden-sdk/miden-wallet-adapter-react';
import { TxStatus } from '../components/TxStatus';
import { useRegistry } from '../hooks/useRegistry';
import { encodeName } from '../chess';
import {
  GHOSTMATE_PRIVATE_DATA_PERMISSION,
  GHOSTMATE_MIDEN_NETWORK,
  GHOSTMATE_ALLOWED_PRIVATE_DATA,
} from '../miden';

type Step = 'connect' | 'name' | 'register' | 'done';

export function Register() {
  const { connect, connected, connecting, address, select, wallets: adapterWallets } = useWallet();
  const { importAccount, isImporting, error: importError }           = useImportAccount();
  const { execute, stage, isLoading: txLoading, error: txError, reset: resetTx } = useTransaction();
  const { waitForCommit }                                             = useWaitForCommit();
  const { upsertAgent }                                               = useRegistry();

  const [step,       setStep]       = useState<Step>('connect');
  const [agentName,  setAgentName]  = useState('');
  const [importFile, setImportFile] = useState<File | null>(null);
  const [importMode, setImportMode] = useState<'new' | 'import'>('new');
  const [showGhost,  setShowGhost]  = useState(false);
  const [connectError, setConnectError] = useState<string | null>(null);

  const triggerGhost = () => {
    setShowGhost(true);
    setTimeout(() => setShowGhost(false), 2500);
  };

  const handleConnectWallet = async () => {
    setConnectError(null);
    try {
      if (adapterWallets.length > 0) {
        select(adapterWallets[0].adapter.name);
      }
      await connect(
        GHOSTMATE_PRIVATE_DATA_PERMISSION,
        GHOSTMATE_MIDEN_NETWORK,
        GHOSTMATE_ALLOWED_PRIVATE_DATA,
      );
      triggerGhost();
      setStep('name');
    } catch (err) {
      setConnectError(err instanceof Error ? err.message : 'Failed to connect wallet');
    }
  };

  const handleImportFile = async () => {
    if (!importFile) return;
    const buf   = await importFile.arrayBuffer();
    const bytes = new Uint8Array(buf);
    const acc   = await importAccount({ type: 'file', file: bytes });
    if (acc) { triggerGhost(); setStep('name'); }
  };

  const handleRegister = useCallback(async () => {
    if (!agentName.trim() || !address) return;
    const accountId = address;
    const nameWord  = encodeName(agentName.trim());
    setStep('register');
    try {
      const result = await execute({
        accountId,
        request: async (_client) => {
          throw new Error('Deploy MatchRegistry and update REGISTRY_ID in config.ts first');
        },
      });
      if (result) {
        await waitForCommit(result.transactionId, { timeoutMs: 30_000 });
        upsertAgent(BigInt(`0x${accountId.replace(/^0x/, '').slice(0, 16)}`), agentName.trim());
        void nameWord;
        setStep('done');
      }
    } catch (_err) {
      // error shown via TxStatus
    }
  }, [agentName, address, execute, waitForCommit, upsertAgent]);

  const STEPS: Step[]                       = ['connect', 'name', 'register', 'done'];
  const STEP_LABELS: Record<Step, string>   = { connect: 'CONNECT', name: 'REGISTER', register: 'CONFIRM', done: 'DONE' };

  return (
    <div style={{ maxWidth: 560, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: 32, paddingTop: 20 }}>
      {showGhost && (
        <div className="ghost-mascot" aria-hidden="true">👻</div>
      )}

      {/* Heading */}
      <div style={{ textAlign: 'center' }}>
        <h1 className="arena-heading">JOIN THE <span className="highlight">ARENA</span></h1>
        <p style={{ color: '#555555', marginTop: 8, fontFamily: 'Space Grotesk' }}>
          Deploy your AI chess agent on Miden and compete on-chain.
        </p>
      </div>

      {/* Step indicators */}
      <div className="step-indicator">
        {STEPS.slice(0, 3).map((s, i) => {
          const currentIdx = STEPS.indexOf(step);
          const isDone     = currentIdx > i;
          const isActive   = currentIdx === i;
          return (
            <div key={s} className="step-node" style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8, position: 'relative' }}>
              {i < 2 && (
                <div style={{
                  position: 'absolute', top: 17, left: '50%',
                  width: '100%', height: 1,
                  background: isDone ? '#FF6B00' : 'rgba(255,107,0,0.15)',
                  zIndex: 0,
                }} />
              )}
              <div className={`step-circle${isActive ? ' active' : ''}${isDone ? ' done' : ''}`}>
                {isDone ? '✓' : i + 1}
              </div>
              <span style={{ fontFamily: 'Orbitron', fontSize: 9, color: isActive ? '#FF6B00' : '#444444', letterSpacing: 1 }}>
                {STEP_LABELS[s]}
              </span>
            </div>
          );
        })}
      </div>

      {/* Step: connect */}
      {step === 'connect' && (
        <div style={{ background: '#111111', border: '1px solid rgba(255,107,0,0.12)', borderRadius: 10, padding: 32, display: 'flex', flexDirection: 'column', gap: 20 }}>
          {connected ? (
            <>
              <h2 style={{ fontFamily: 'Orbitron', fontSize: 16, color: '#FF6B00' }}>WALLET LINKED</h2>
              <p style={{ fontFamily: 'monospace', color: '#888888', fontSize: 12, wordBreak: 'break-all' }}>
                {address}
              </p>
              <button className="connect-wallet-btn" onClick={() => setStep('name')}>
                CONTINUE TO REGISTER →
              </button>
            </>
          ) : (
            <>
              <h2 style={{ fontFamily: 'Orbitron', fontSize: 16 }}>CONNECT YOUR AGENT</h2>
              <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
                {(['new', 'import'] as const).map(m => (
                  <button
                    key={m}
                    onClick={() => setImportMode(m)}
                    style={{
                      flex: 1, padding: '8px',
                      background: importMode === m ? 'rgba(255,107,0,0.1)' : 'transparent',
                      border: `1px solid ${importMode === m ? '#FF6B00' : 'rgba(255,107,0,0.15)'}`,
                      borderRadius: 6,
                      color: importMode === m ? '#FF6B00' : '#555555',
                      fontFamily: 'Orbitron', fontSize: 10, cursor: 'pointer', letterSpacing: 1,
                    }}
                  >
                    {m === 'new' ? 'EXTENSION' : 'IMPORT ACCOUNT'}
                  </button>
                ))}
              </div>

              {importMode === 'new' ? (
                <button className="connect-wallet-btn" onClick={handleConnectWallet} disabled={connecting}>
                  {connecting ? 'CONNECTING…' : '◎ CONNECT WALLET'}
                </button>
              ) : (
                <>
                  <input
                    type="file"
                    accept=".mac,.json"
                    className="text-input"
                    style={{ padding: '8px' }}
                    onChange={e => setImportFile(e.target.files?.[0] ?? null)}
                  />
                  <button className="connect-wallet-btn" onClick={handleImportFile} disabled={!importFile || isImporting}>
                    {isImporting ? 'IMPORTING…' : '↑ IMPORT AGENT'}
                  </button>
                </>
              )}

              {(connectError || importError) && (
                <div className="alert-error">{connectError ?? importError?.message}</div>
              )}
            </>
          )}
        </div>
      )}

      {/* Step: name */}
      {step === 'name' && (
        <div style={{ background: '#111111', border: '1px solid rgba(255,107,0,0.12)', borderRadius: 10, padding: 32, display: 'flex', flexDirection: 'column', gap: 16 }}>
          <h2 style={{ fontFamily: 'Orbitron', fontSize: 16, color: '#FF6B00' }}>NAME YOUR AGENT</h2>
          <p style={{ color: '#555555', fontSize: 14, fontFamily: 'Space Grotesk' }}>
            This name will appear on the leaderboard. Max 32 characters.
          </p>
          <input
            className="text-input"
            placeholder="e.g. NEURALREAPER"
            maxLength={32}
            value={agentName}
            onChange={e => setAgentName(e.target.value.toUpperCase())}
            style={{ fontFamily: 'Orbitron', letterSpacing: 2, fontSize: 14 }}
          />
          <button className="connect-wallet-btn" onClick={handleRegister} disabled={!agentName.trim()}>
            REGISTER AGENT →
          </button>
        </div>
      )}

      {/* Step: registering */}
      {step === 'register' && (
        <div style={{ background: '#111111', border: '1px solid rgba(255,107,0,0.15)', borderRadius: 10, padding: 32, display: 'flex', flexDirection: 'column', gap: 16 }}>
          <h2 style={{ fontFamily: 'Orbitron', fontSize: 16 }}>CONFIRMING ON-CHAIN…</h2>
          <TxStatus stage={stage} isLoading={txLoading} error={txError} onReset={resetTx} />
        </div>
      )}

      {/* Done */}
      {step === 'done' && (
        <div style={{
          background: 'rgba(255,107,0,0.06)',
          border: '1px solid rgba(255,107,0,0.35)',
          borderRadius: 10, padding: 32,
          textAlign: 'center',
          display: 'flex', flexDirection: 'column', gap: 12,
        }}>
          <div style={{ fontSize: 48 }}>👻</div>
          <h2 style={{ fontFamily: 'Orbitron', fontSize: 18, color: '#FF6B00', letterSpacing: 3 }}>AGENT LINKED</h2>
          <p style={{ color: '#888888', fontFamily: 'Space Grotesk' }}>
            Welcome to GhostMate Arena,{' '}
            <span style={{ color: 'white', fontWeight: 700 }}>{agentName}</span>
          </p>
        </div>
      )}
    </div>
  );
}
