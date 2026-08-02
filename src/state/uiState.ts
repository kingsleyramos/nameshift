// Ephemeral UI state (§13.1): row selection, filters, dialogs, banner.
// Row *selection* is frontend-only and never crosses IPC (§24 Q28) —
// distinct from checkbox *inclusion*, which is domain state.

import { create } from 'zustand';
import type { ApplyFinished } from '../ipc/gen/ApplyFinished';

export type ListFilter = 'all' | 'willChange' | 'conflicts' | 'edited';
export type SideTab = 'rules' | 'history';

export interface ConfirmRequest {
  title: string;
  message: string;
  confirmLabel: string;
  destructive: boolean;
  onConfirm: () => void;
}

export interface AlertRequest {
  kind: 'error' | 'info';
  title: string;
  message: string;
}

interface UiStore {
  sideTab: SideTab;
  filterText: string;
  filterMode: ListFilter;
  inspectorOpen: boolean;
  inspectedId: string | null;
  /** Highlighted rows (ephemeral, §14.2). */
  rowSelection: Set<string>;
  /** Roving focus row for keyboard operations. */
  focusedRow: string | null;
  /** Anchor for ⇧-click row ranges. */
  selectionAnchor: string | null;
  /** Anchor for ⇧-click checkbox ranges (reset when the visible set
   * changes — §14.2). */
  checkboxAnchor: string | null;
  inlineDiffView: boolean;
  dragOver: boolean;
  banner: ApplyFinished | null;
  confirm: ConfirmRequest | null;
  alert: AlertRequest | null;
  csvModalOpen: boolean;
  /** The row being renamed inline, if any. */
  editingRow: string | null;
  splitRatio: number;
  columnSplit: number;

  set: (partial: Partial<UiStore>) => void;
  toggleFilter: (mode: ListFilter) => void;
  clearSelection: () => void;
  selectSingle: (id: string) => void;
  showConfirm: (request: ConfirmRequest) => void;
  showAlert: (request: AlertRequest) => void;
}

export const useUiStore = create<UiStore>((set, get) => ({
  sideTab: 'rules',
  filterText: '',
  filterMode: 'all',
  inspectorOpen: false,
  inspectedId: null,
  rowSelection: new Set<string>(),
  focusedRow: null,
  selectionAnchor: null,
  checkboxAnchor: null,
  inlineDiffView: false,
  dragOver: false,
  banner: null,
  confirm: null,
  alert: null,
  csvModalOpen: false,
  editingRow: null,
  splitRatio: 0.34,
  columnSplit: 0.5,

  set: (partial) => {
    set(partial);
  },
  toggleFilter: (mode) => {
    set({ filterMode: get().filterMode === mode ? 'all' : mode, checkboxAnchor: null });
  },
  clearSelection: () => {
    set({ rowSelection: new Set(), focusedRow: null, selectionAnchor: null });
  },
  selectSingle: (id) => {
    set({ rowSelection: new Set([id]), focusedRow: id, selectionAnchor: id, inspectedId: id });
  },
  showConfirm: (request) => {
    set({ confirm: request });
  },
  showAlert: (request) => {
    set({ alert: request });
  },
}));
