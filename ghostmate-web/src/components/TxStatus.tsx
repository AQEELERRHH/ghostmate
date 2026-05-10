import type { TransactionStage } from '@miden-sdk/react';

interface Props {
  stage?:     TransactionStage | null;
  isLoading?: boolean;
  error?:     Error | null | string;
  onReset?:   () => void;
}

const STAGE_LABELS: Record<string, string> = {
  idle:       '',
  executing:  'Executing transaction…',
  proving:    'Generating proof…',
  submitting: 'Submitting to network…',
  complete:   'Confirmed ✓',
};

export function TxStatus({ stage, isLoading, error, onReset }: Props) {
  if (!isLoading && !error && (!stage || stage === 'idle')) return null;

  const errMsg = error instanceof Error ? error.message : (error ?? null);

  return (
    <div className={`tx-status ${errMsg ? 'tx-error' : stage === 'complete' ? 'tx-complete' : 'tx-pending'}`}>
      {isLoading && <span className="tx-spinner" />}
      <span className="tx-label">
        {errMsg ? `Error: ${errMsg}` : (STAGE_LABELS[stage ?? ''] ?? stage)}
      </span>
      {(errMsg || stage === 'complete') && onReset && (
        <button className="tx-reset" onClick={onReset}>✕</button>
      )}
    </div>
  );
}
