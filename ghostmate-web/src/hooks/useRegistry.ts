import { useCallback, useEffect, useRef, useState } from "react";
import { useMiden, useMidenClient, useNoteStream, useSyncState } from "@miden-sdk/react";
import { REGISTRY_ID } from "../config";
import { agentScore } from "../chess";

// ── Domain types ──────────────────────────────────────────────────────────────

export interface AgentRecord {
  prefix: bigint;
  name: string;
  wins: bigint;
  losses: bigint;
  draws: bigint;
  score: bigint;
}

export interface ChallengeRecord {
  id: bigint;
  challengerPrefix: bigint;
  challengerName: string;
  wagerAmount: bigint;
  status: bigint;
}

export interface GameRecord {
  id: bigint;
  whitePrefix: bigint;
  whiteName: string;
  blackPrefix: bigint;
  blackName: string;
  status: bigint;
  gameContractPrefix: bigint;
}

// ── Persistent cache (localStorage) ──────────────────────────────────────────

const CACHE_KEY = "ghostmate:registry";

interface CacheShape {
  agents:     Record<string, { name: string; wins: string; losses: string; draws: string }>;
  challenges: Record<string, { challengerPrefix: string; wager: string; status: string }>;
  games:      Record<string, { white: string; black: string; status: string; gc: string }>;
}

const EMPTY_CACHE: CacheShape = { agents: {}, challenges: {}, games: {} };

function loadCache(): CacheShape {
  try {
    const raw = JSON.parse(localStorage.getItem(CACHE_KEY) ?? "{}") as Partial<CacheShape>;
    return {
      agents:     raw?.agents     ?? {},
      challenges: raw?.challenges ?? {},
      games:      raw?.games      ?? {},
    };
  } catch {
    return { ...EMPTY_CACHE };
  }
}

function saveCache(c: CacheShape) {
  localStorage.setItem(CACHE_KEY, JSON.stringify(c));
}

// ── Hook ──────────────────────────────────────────────────────────────────────

export function useRegistry() {
  const { runExclusive } = useMiden();
  const { sync } = useSyncState();
  const [agents,     setAgents]     = useState<AgentRecord[]>([]);
  const [challenges, setChallenges] = useState<ChallengeRecord[]>([]);
  const [games,      setGames]      = useState<GameRecord[]>([]);
  const [isLoading,  setIsLoading]  = useState(false);
  const [error,      setError]      = useState<string | null>(null);
  const cache = useRef<CacheShape>(loadCache());

  // Build typed views from cache -----------------------------------------------
  const rebuildFromCache = useCallback(() => {
    const c = cache.current;
    const nameOf = (prefix: string) => c.agents[prefix]?.name ?? `Agent ${prefix.slice(0, 6)}`;

    const agentList: AgentRecord[] = Object.entries(c.agents ?? {}).map(([prefix, a]) => {
      const wins   = BigInt(a.wins   ?? 0);
      const losses = BigInt(a.losses ?? 0);
      const draws  = BigInt(a.draws  ?? 0);
      return { prefix: BigInt(prefix), name: a.name, wins, losses, draws, score: agentScore(wins, draws) };
    });
    agentList.sort((a, b) => (b.score > a.score ? 1 : b.score < a.score ? -1 : 0));
    setAgents(agentList);

    const challengeList: ChallengeRecord[] = Object.entries(c.challenges ?? {}).map(([id, ch]) => ({
      id:               BigInt(id),
      challengerPrefix: BigInt(ch.challengerPrefix),
      challengerName:   nameOf(ch.challengerPrefix),
      wagerAmount:      BigInt(ch.wager),
      status:           BigInt(ch.status),
    }));
    setChallenges(challengeList.filter(c => c.status === 0n)); // CHALLENGE_OPEN = 0

    const gameList: GameRecord[] = Object.entries(c.games ?? {}).map(([id, g]) => ({
      id:                 BigInt(id),
      whitePrefix:        BigInt(g.white),
      whiteName:          nameOf(g.white),
      blackPrefix:        BigInt(g.black),
      blackName:          nameOf(g.black),
      status:             BigInt(g.status),
      gameContractPrefix: BigInt(g.gc),
    }));
    setGames(gameList);
  }, []);

  // Fetch fresh state from the chain --------------------------------------------
  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      await sync();
      await runExclusive(async () => {
        // TODO: When the Miden web client exposes a storage-map query API,
        // replace this section with real chain reads using:
        //   const client = useMidenClient();
        //   const stats = await client.getAccountStorageItem(REGISTRY_ID, SLOTS.agentStats, key);
        //
        // For now we rely on note streaming (below) to discover agents/games.
      });
      rebuildFromCache();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [sync, runExclusive, rebuildFromCache]);

  // Initial load ----------------------------------------------------------------
  useEffect(() => {
    rebuildFromCache();
    refresh();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Live note streaming — discover new agents, results, challenges --------------
  const { notes } = useNoteStream({ sender: REGISTRY_ID });

  useEffect(() => {
    if (!notes.length) return;
    let changed = false;
    const c = cache.current;

    for (const note of notes) {
      const storage = note.assets; // use note metadata when the full storage API is available
      // TODO: parse note.storage once the SDK exposes InputNoteRecord.storage
      // For now this block is scaffolded — real parsing happens after deployment.
      void storage;
    }

    if (changed) {
      saveCache(c);
      rebuildFromCache();
    }
  }, [notes, rebuildFromCache]);

  // Helpers for manual cache updates (used by mutation hooks) ------------------
  const upsertAgent = useCallback((
    prefix: bigint,
    name: string,
    wins = 0n, losses = 0n, draws = 0n,
  ) => {
    const c = cache.current;
    const key = prefix.toString();
    const existing = c.agents[key];
    c.agents[key] = {
      name,
      wins:   (existing?.wins   != null ? BigInt(existing.wins)   : wins).toString(),
      losses: (existing?.losses != null ? BigInt(existing.losses) : losses).toString(),
      draws:  (existing?.draws  != null ? BigInt(existing.draws)  : draws).toString(),
    };
    saveCache(c);
    rebuildFromCache();
  }, [rebuildFromCache]);

  const upsertChallenge = useCallback((
    id: bigint, challengerPrefix: bigint, wager: bigint, status: bigint,
  ) => {
    const c = cache.current;
    c.challenges[id.toString()] = {
      challengerPrefix: challengerPrefix.toString(),
      wager: wager.toString(),
      status: status.toString(),
    };
    saveCache(c);
    rebuildFromCache();
  }, [rebuildFromCache]);

  const upsertGame = useCallback((
    id: bigint, white: bigint, black: bigint, status: bigint, gc: bigint,
  ) => {
    const c = cache.current;
    c.games[id.toString()] = {
      white: white.toString(), black: black.toString(),
      status: status.toString(), gc: gc.toString(),
    };
    saveCache(c);
    rebuildFromCache();
  }, [rebuildFromCache]);

  return {
    agents, challenges, games,
    isLoading, error,
    refresh,
    upsertAgent, upsertChallenge, upsertGame,
  };
}

// ── Standalone account stats query ───────────────────────────────────────────

export function useAgentProfile(agentId: string | null) {
  const client = useMidenClient();
  const [isLoading, setIsLoading] = useState(false);
  const { runExclusive } = useMiden();

  useEffect(() => {
    if (!agentId) return;
    let cancelled = false;
    setIsLoading(true);
    runExclusive(async () => {
      // TODO: query REGISTRY_ID storage for this agent's stats and name once
      // the Miden web client exposes a storage-map read API, e.g.:
      //   const stats = await client.getAccountStorageItem(REGISTRY_ID, SLOTS.agentStats, key);
      void client;
      if (!cancelled) setIsLoading(false);
    }).catch(() => { if (!cancelled) setIsLoading(false); });
    return () => { cancelled = true; };
  }, [agentId, client, runExclusive]);

  return { profile: null as AgentRecord | null, isLoading };
}
