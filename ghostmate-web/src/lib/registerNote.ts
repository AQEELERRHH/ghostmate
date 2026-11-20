import {
  Package,
  NoteScript,
  NoteStorage,
  NoteRecipient,
  NoteMetadata,
  NoteTag,
  Note,
  NoteAssets,
  FeltArray,
  Felt,
  NoteType,
  NetworkAccountTarget,
  type AccountId,
} from "@miden-sdk/miden-sdk";
import { REGISTER_NOTE_PKG_URL } from "../config";
import { encodeName } from "../chess";

// The compiled register-note script is immutable, so fetch + decode it once.
let scriptPromise: Promise<NoteScript> | null = null;

/** Fetches and caches the compiled register-note script from /packages. */
export async function loadRegisterNoteScript(): Promise<NoteScript> {
  if (!scriptPromise) {
    scriptPromise = (async () => {
      const res = await fetch(REGISTER_NOTE_PKG_URL);
      if (!res.ok) {
        throw new Error(
          `Could not load the registration script (HTTP ${res.status}).`,
        );
      }
      const bytes = new Uint8Array(await res.arrayBuffer());
      return NoteScript.fromPackage(Package.deserialize(bytes));
    })().catch((err) => {
      scriptPromise = null; // allow a retry on the next call
      throw err;
    });
  }
  return scriptPromise;
}

/**
 * Builds the asset-less register-note emitted by the local agent account.
 *
 * The note's sender is `senderId` (the agent account). The transaction kernel
 * stamps that sender and it cannot be forged, so when the registry consumes the
 * note the script reads it via `get_sender()` and keys the agent by it. The
 * note storage carries the 4-felt encoded display name.
 *
 * Since Phase 2D.1 the registry is a network account: only the Miden network
 * operator may execute its transactions. The note therefore carries a
 * `NetworkAccountTarget` attachment, which makes it a network note that the
 * operator consumes automatically (typically within ~10s of commit).
 */
export async function buildRegisterNote(
  senderId: AccountId,
  name: string,
  registryId: AccountId,
): Promise<Note> {
  const script = await loadRegisterNoteScript();

  const felts = new FeltArray();
  for (const limb of encodeName(name)) felts.push(new Felt(limb));

  const recipient = NoteRecipient.fromScript(script, new NoteStorage(felts));
  const metadata = new NoteMetadata(
    senderId,
    NoteType.Public,
    NoteTag.withAccountTarget(registryId),
  );
  const target = new NetworkAccountTarget(registryId).toAttachment();
  const note = Note.withAttachments(new NoteAssets([]), metadata, recipient, [target]);
  if (!note.isNetworkNote()) {
    // Without the attachment the operator ignores the note and it would sit
    // unconsumed forever — refuse to emit rather than strand the user.
    throw new Error("Internal error: registration note is not a network note.");
  }
  return note;
}
