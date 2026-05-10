import { useEffect, useState } from "react";
import { useNoteStream } from "@miden-sdk/react";
import type { StreamedNote } from "@miden-sdk/react";
import { decodeBoardWords, moveToUci, BoardWords } from "../chess";
import { TURN_NOTE_IDX, GAME_STATUS } from "../config";

// ── Types ─────────────────────────────────────────────────────────────────────

export interface MoveRecord {
  ply:   number;
  uci:   string;
  from:  number;
  to:    number;
  promo: number;
}

export interface GameState {
  squares:       Uint8Array;
  side:          number;
  halfmoveClock: number;
  status:        bigint;
  moves:         MoveRecord[];
  lastFrom:      number | null;
  lastTo:        number | null;
  ply:           number;
}

function emptyBoard(): GameState {
  return {
    squares:       new Uint8Array(64),
    side:          0,
    halfmoveClock: 0,
    status:        GAME_STATUS.ACTIVE,
    moves:         [],
    lastFrom:      null,
    lastTo:        null,
    ply:           0,
  };
}

// ── Note storage extraction ───────────────────────────────────────────────────
// TurnNotes are public Miden notes created by GameContract.
// Their storage (16 felts) is accessible via:
//   note.record.details().recipient().storage().items()
// Each item is a Felt whose .asInt() gives the BigInt value.

function extractTurnNoteStorage(note: StreamedNote): bigint[] | null {
  try {
    const items = note.record.details().recipient().storage().items();
    return items.map(f => f.asInt());
  } catch {
    return null;
  }
}

// ── Hook ──────────────────────────────────────────────────────────────────────

export function useGame(gameContractId: string | null) {
  const [state,  setState]  = useState<GameState>(emptyBoard);
  const [isLive, setIsLive] = useState(false);

  const { notes, markHandled } = useNoteStream({
    sender: gameContractId ?? undefined,
    status: "committed",
  });

  useEffect(() => {
    if (!notes.length || !gameContractId) return;

    // Apply notes in order of when they were first seen
    const sorted = [...notes].sort((a, b) => a.firstSeenAt - b.firstSeenAt);

    setState(prev => {
      let next = { ...prev };

      for (const note of sorted) {
        const raw = extractTurnNoteStorage(note);
        if (!raw || raw.length < 12) continue;

        const boardWords: BoardWords = [
          raw[TURN_NOTE_IDX.boardWord0],
          raw[TURN_NOTE_IDX.boardWord1],
          raw[TURN_NOTE_IDX.boardWord2],
          raw[TURN_NOTE_IDX.boardWord3],
        ];
        const side          = Number(raw[TURN_NOTE_IDX.side]);
        const halfmoveClock = Number(raw[TURN_NOTE_IDX.halfmoveClock]);
        const from          = Number(raw[TURN_NOTE_IDX.moveFrom]);
        const to            = Number(raw[TURN_NOTE_IDX.moveTo]);
        const promo         = Number(raw[TURN_NOTE_IDX.movePromo]);

        const squares = decodeBoardWords(boardWords);
        const uci     = moveToUci(from, to, promo);
        const ply     = next.ply + 1;

        next = {
          squares,
          side,
          halfmoveClock,
          status:   GAME_STATUS.ACTIVE,
          moves:    [...next.moves, { ply, uci, from, to, promo }],
          lastFrom: from,
          lastTo:   to,
          ply,
        };

        markHandled(note.id);
      }

      return next;
    });

    setIsLive(true);
  }, [notes, gameContractId, markHandled]);

  const reset = () => {
    setState(emptyBoard());
    setIsLive(false);
  };

  return { state, isLive, reset };
}
