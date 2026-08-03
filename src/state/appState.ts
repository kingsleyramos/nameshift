// The domain mirror (§13.1): Rust owns all domain state; this store
// refreshes from get_state/get_preview on state-changed. Mutations are
// fire-and-forget command calls; the authoritative echo returns via the
// event. Only rule-field typing is optimistic (the draft below).

import { create } from 'zustand';
import type { PreviewPayload } from '../ipc/gen/PreviewPayload';
import type { Processing } from '../ipc/gen/Processing';
import type { RenameRule } from '../ipc/gen/RenameRule';
import type { RevertPreviewPayload } from '../ipc/gen/RevertPreviewPayload';
import type { SnapshotMeta } from '../ipc/gen/SnapshotMeta';
import type { StateSnapshot } from '../ipc/gen/StateSnapshot';
import type { PlatformInfo } from '../ipc/gen/PlatformInfo';
import * as commands from '../ipc/commands';
import * as events from '../ipc/events';

export interface AppStore {
  snapshot: StateSnapshot | null;
  preview: PreviewPayload | null;
  revertPreview: RevertPreviewPayload | null;
  history: SnapshotMeta[];
  platform: PlatformInfo | null;
  processing: Processing | null;
  /** Optimistic rules draft while the user types (§13.1). */
  rulesDraft: RenameRule[] | null;
  /** The rulesRevision the draft is based on; a different authoritative
   * revision (undo/preset/apply) discards the draft. */
  draftRevision: number;

  refresh: () => Promise<void>;
  setProcessing: (processing: Processing | null) => void;
  /** The rules the UI edits and renders: the draft when active. */
  editRules: (rules: RenameRule[]) => void;
  commitDraftNow: () => void;
}

let draftTimer: ReturnType<typeof setTimeout> | null = null;

export const useAppStore = create<AppStore>((set, get) => ({
  snapshot: null,
  preview: null,
  revertPreview: null,
  history: [],
  platform: null,
  processing: null,
  rulesDraft: null,
  draftRevision: -1,

  refresh: async () => {
    const [snapshot, preview] = await Promise.all([
      commands.getState(),
      commands.getPreview(get().snapshot?.version),
    ]);
    const current = get();
    // Out-of-order guard (§13.1): ignore anything older than what we have.
    if (current.snapshot && snapshot.version < current.snapshot.version) {
      return;
    }
    // Reconcile the optimistic draft: an authoritative rules change from
    // underneath (undo/redo/preset/apply) discards it.
    let { rulesDraft, draftRevision } = current;
    if (rulesDraft !== null && snapshot.rulesRevision !== draftRevision) {
      rulesDraft = null;
      draftRevision = snapshot.rulesRevision;
    }
    const [history, revertPreview] = await Promise.all([
      commands.getHistory(),
      snapshot.selectedSnapshotId !== null
        ? commands.getRevertPreview()
        : Promise.resolve(null),
    ]);
    // Preview fetches are keyed by version — a stale response is dropped,
    // never rendered (§13.1).
    const latest = get();
    if (latest.snapshot && snapshot.version < latest.snapshot.version) {
      return;
    }
    set({ snapshot, preview, history, revertPreview, rulesDraft, draftRevision });
  },

  setProcessing: (processing) => {
    set({ processing });
  },

  editRules: (rules) => {
    const snapshot = get().snapshot;
    set({ rulesDraft: rules, draftRevision: snapshot?.rulesRevision ?? -1 });
    if (draftTimer !== null) {
      clearTimeout(draftTimer);
    }
    // Debounced (~120 ms) sync to the authoritative store (§13.1).
    draftTimer = setTimeout(() => {
      draftTimer = null;
      const draft = get().rulesDraft;
      if (draft !== null) {
        void commands.setRules(draft);
      }
    }, 120);
  },

  commitDraftNow: () => {
    if (draftTimer !== null) {
      clearTimeout(draftTimer);
      draftTimer = null;
    }
    const draft = get().rulesDraft;
    if (draft !== null) {
      void commands.setRules(draft);
    }
  },
}));

/** The rules the panel renders: the optimistic draft while typing, else the
 * authoritative stack. */
export function visibleRules(store: Pick<AppStore, 'rulesDraft' | 'snapshot'>): RenameRule[] {
  return store.rulesDraft ?? store.snapshot?.rules ?? [];
}

let started = false;
let refreshTimer: ReturnType<typeof setTimeout> | null = null;

/** Wire event listeners once at boot; refreshes are debounced ~30 ms. */
export async function startAppStore(): Promise<void> {
  if (started) return;
  started = true;
  const store = useAppStore.getState();
  const scheduleRefresh = () => {
    if (refreshTimer !== null) return;
    refreshTimer = setTimeout(() => {
      refreshTimer = null;
      void useAppStore.getState().refresh();
    }, 30);
  };
  await events.onStateChanged(scheduleRefresh);
  await events.onProcessing((payload) => {
    useAppStore.getState().setProcessing(payload);
  });
  const platform = await commands.getPlatform();
  useAppStore.setState({ platform });
  await store.refresh();
}
