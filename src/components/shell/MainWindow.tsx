// The single main window (§14.1): toolbar, side panel | detail split,
// bottom action bar, overlays, dialogs, banner, and the global listeners.

import { useCallback, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import Toolbar from './Toolbar';
import ActionBar from './ActionBar';
import SplitLayout from './SplitLayout';
import DropOverlay from './DropOverlay';
import ProcessingOverlay from './ProcessingOverlay';
import Banner from './Banner';
import Dialogs from './Dialogs';
import RulesPanel from '../rules/RulesPanel';
import HistoryPanel from '../history/HistoryPanel';
import FileListPane from '../files/FileListPane';
import RevertPreviewPane from '../history/RevertPreviewPane';
import InspectorView from '../inspector/InspectorView';
import CsvModal from '../files/CsvModal';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';
import * as events from '../../ipc/events';

export default function MainWindow() {
  const snapshot = useAppStore((s) => s.snapshot);
  const processing = useAppStore((s) => s.processing);
  const sideTab = useUiStore((s) => s.sideTab);
  const inspectorOpen = useUiStore((s) => s.inspectorOpen);
  const csvModalOpen = useUiStore((s) => s.csvModalOpen);
  const setUi = useUiStore((s) => s.set);
  const revertMode = snapshot?.selectedSnapshotId != null;

  // Global listeners: banner, alerts, forwarded paths, file drops.
  useEffect(() => {
    const disposers: Promise<() => void>[] = [
      events.onApplyFinished((payload) => {
        useUiStore.getState().set({ banner: payload });
      }),
      events.onAlert((payload) => {
        useUiStore.getState().showAlert({
          kind: payload.kind === 'error' ? 'error' : 'info',
          title: payload.title,
          message: payload.message,
        });
      }),
      events.onOpenPaths((payload) => {
        void commands.importPaths(payload.paths);
      }),
      listen<string>('menu', (event) => {
        handleMenuAction(event.payload);
      }),
    ];
    return () => {
      disposers.forEach((d) => {
        void d.then((dispose) => {
          dispose();
        });
      });
    };
  }, []);

  // Whole-window drop target (§14.1): one import gate for every entry point.
  useEffect(() => {
    let dispose: (() => void) | undefined;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const ui = useUiStore.getState();
        if (event.payload.type === 'over') {
          if (!ui.confirm && !ui.alert) {
            ui.set({ dragOver: true });
          }
        } else if (event.payload.type === 'drop') {
          ui.set({ dragOver: false });
          void commands.importPaths(event.payload.paths);
        } else {
          ui.set({ dragOver: false });
        }
      })
      .then((d) => {
        dispose = d;
      });
    return () => {
      dispose?.();
    };
  }, []);

  // Esc priority (§15): cancel processing → close row edit → clear filter.
  const onKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      const ui = useUiStore.getState();
      if (useAppStore.getState().processing) {
        void commands.cancelProcessing();
      } else if (ui.editingRow !== null) {
        ui.set({ editingRow: null });
      } else if (ui.filterText !== '' || ui.filterMode !== 'all') {
        ui.set({ filterText: '', filterMode: 'all' });
      }
    },
    [],
  );

  return (
    <div
      className="flex h-full flex-col outline-none"
      onKeyDown={onKeyDown}
      aria-label={strings.appName}
    >
      <Toolbar />
      <SplitLayout
        side={
          <div className="flex h-full flex-col">
            <SideTabs />
            {sideTab === 'rules' ? <RulesPanel /> : <HistoryPanel />}
          </div>
        }
        detail={
          <div className="flex h-full min-w-0">
            <div className="min-w-0 flex-1">
              {revertMode ? <RevertPreviewPane /> : <FileListPane />}
            </div>
            {inspectorOpen && <InspectorView />}
          </div>
        }
      />
      <ActionBar />
      <Banner />
      {processing && <ProcessingOverlay processing={processing} />}
      <DropOverlay />
      {csvModalOpen && (
        <CsvModal
          onClose={() => {
            setUi({ csvModalOpen: false });
          }}
        />
      )}
      <Dialogs />
    </div>
  );
}

/** Whether keyboard focus sits in a text-editing element — those keep the
 * OS text-editing behavior for Undo/Redo/Select All (§15). */
function focusIsTextInput(): boolean {
  const active = document.activeElement;
  return (
    active instanceof HTMLInputElement ||
    active instanceof HTMLTextAreaElement ||
    (active instanceof HTMLElement && active.isContentEditable)
  );
}

/** Frontend halves of the §15 menu items. */
function handleMenuAction(action: string) {
  const ui = useUiStore.getState();
  switch (action) {
    case 'pick-files':
      void commands.pickAndImport();
      break;
    case 'revert': {
      // The File-menu revert mirrors the action bar's confirmed flow.
      const revertPreview = useAppStore.getState().revertPreview;
      if (!revertPreview || revertPreview.restorableRenameCount === 0) break;
      const files = revertPreview.restorableFileCount;
      const renames = revertPreview.restorableRenameCount;
      const versions = revertPreview.newerSnapshotCount + 1;
      ui.showConfirm({
        title: strings.revertConfirmTitle,
        message: strings.revertConfirmMessage(
          files,
          renames,
          versions,
          revertPreview.newerSnapshotCount,
        ),
        confirmLabel: strings.revertConfirmButton,
        destructive: true,
        onConfirm: () => {
          void commands.revertSelected();
        },
      });
      break;
    }
    case 'rename-by-csv':
      ui.set({ csvModalOpen: true });
      break;
    case 'import-presets':
      void commands.importPresets();
      break;
    case 'export-presets':
      void commands.exportPresets();
      break;
    // execCommand is deprecated but remains the only programmatic bridge to
    // the webview's NATIVE text undo/redo/select — exactly what §15 requires
    // when focus sits in a text field.
    case 'undo':
      if (focusIsTextInput()) {
        // eslint-disable-next-line @typescript-eslint/no-deprecated
        document.execCommand('undo');
      } else {
        void commands.undo();
      }
      break;
    case 'redo':
      if (focusIsTextInput()) {
        // eslint-disable-next-line @typescript-eslint/no-deprecated
        document.execCommand('redo');
      } else {
        void commands.redo();
      }
      break;
    case 'select-all': {
      if (focusIsTextInput()) {
        // eslint-disable-next-line @typescript-eslint/no-deprecated
        document.execCommand('selectAll');
        break;
      }
      // ⌘A selects ROWS, never checkboxes (§15).
      const entries = useAppStore.getState().preview?.entries ?? [];
      ui.set({
        rowSelection: new Set(entries.map((entry) => entry.id)),
        focusedRow: entries[0]?.id ?? null,
      });
      break;
    }
    case 'clear-manual-edits':
      void commands.clearAllOverrides();
      break;
    case 'find':
      document.querySelector<HTMLInputElement>('input[data-filter-field]')?.focus();
      break;
    case 'toggle-inspector':
      ui.set({ inspectorOpen: !ui.inspectorOpen });
      break;
    case 'inline-diff':
      ui.set({ inlineDiffView: !ui.inlineDiffView });
      break;
    default:
      break;
  }
}

function SideTabs() {
  const sideTab = useUiStore((s) => s.sideTab);
  const setUi = useUiStore((s) => s.set);
  const rulesCount = useAppStore((s) => s.snapshot?.rules.length ?? 0);
  const historyCount = useAppStore((s) => s.snapshot?.historyCount ?? 0);
  const tab = (id: 'rules' | 'history', label: string) => (
    <button
      role="tab"
      aria-selected={sideTab === id}
      onClick={() => {
        setUi({ sideTab: id });
      }}
      className="flex-1 rounded-md px-2 py-1 text-[12px] font-medium"
      style={
        sideTab === id
          ? { background: 'var(--pane-bg)', color: 'var(--text-primary)' }
          : { color: 'var(--text-secondary)' }
      }
    >
      {label}
    </button>
  );
  return (
    <div
      role="tablist"
      aria-label="Side panel"
      className="m-2 flex gap-1 rounded-lg p-1"
      style={{ background: 'var(--wash-hover)' }}
    >
      {tab('rules', strings.rulesTab(rulesCount))}
      {tab('history', strings.historyTab(historyCount))}
    </div>
  );
}
