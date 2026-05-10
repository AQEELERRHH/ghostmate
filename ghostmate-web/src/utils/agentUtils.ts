export type PersonalityTag =
  | 'DOMINATING'
  | 'AGGRESSIVE'
  | 'TACTICAL'
  | 'DEFENSIVE'
  | 'CHAOTIC'
  | 'CALCULATIVE'
  | 'ELUSIVE';

export type StatusBadge = 'DOMINATING' | 'HOT' | 'IN GAME' | 'ACTIVE' | 'NEW';

/** djb2 hash, always positive */
export function hashStr(s: string): number {
  let h = 5381;
  for (let i = 0; i < s.length; i++) {
    h = ((h << 5) + h) ^ s.charCodeAt(i);
  }
  return h >>> 0; // unsigned
}

export function computeElo(wins: bigint, losses: bigint, draws: bigint): number {
  return 1200 + Number(wins) * 18 - Number(losses) * 14 + Number(draws) * 4;
}

export function getPersonality(
  wins: bigint,
  losses: bigint,
  draws: bigint,
  name: string,
): PersonalityTag {
  const w = Number(wins), l = Number(losses), d = Number(draws);
  const total = w + l + d;
  if (total === 0) {
    return (['CALCULATIVE', 'TACTICAL', 'ELUSIVE'] as PersonalityTag[])[hashStr(name) % 3];
  }
  const wr = w / total;
  const dr = d / total;
  if (wr > 0.75) return 'DOMINATING';
  if (wr > 0.6)  return 'AGGRESSIVE';
  if (dr > 0.4)  return 'DEFENSIVE';
  if (l > w && l > d) return 'CHAOTIC';
  if (Math.abs(w - l) <= 1) return 'TACTICAL';
  return 'ELUSIVE';
}

export function getStatusBadge(
  rank: number,
  wins: bigint,
  losses: bigint,
  hasActiveGame: boolean,
): StatusBadge {
  if (hasActiveGame) return 'IN GAME';
  if (rank === 1 && wins > 0n) return 'DOMINATING';
  const w = Number(wins), l = Number(losses);
  if (w + l > 0 && w / (w + l) > 0.7 && w > 2) return 'HOT';
  if (w + l === 0) return 'NEW';
  return 'ACTIVE';
}

export const PERSONALITY_CLASS: Record<PersonalityTag, string> = {
  DOMINATING:  'p-dominating',
  AGGRESSIVE:  'p-aggressive',
  TACTICAL:    'p-tactical',
  DEFENSIVE:   'p-defensive',
  CHAOTIC:     'p-chaotic',
  CALCULATIVE: 'p-calculative',
  ELUSIVE:     'p-elusive',
};

export const STATUS_CLASS: Record<StatusBadge, string> = {
  DOMINATING: 'badge-dominating',
  HOT:        'badge-hot',
  'IN GAME':  'badge-ingame',
  ACTIVE:     'badge-active',
  NEW:        'badge-active',
};
