// Regression: the app must render the launch state — a NULL snapshot, before
// the first refresh resolves. A Zustand selector that fabricates a fresh
// array/object per call (e.g. `s.snapshot?.presets ?? []`) re-renders forever
// under React 19's external-store consistency check and crashes with error
// #185: a blank window on every launch. Component tests that pre-seed a
// snapshot can never catch that, so this one renders with none.

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import MainWindow from './MainWindow';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => undefined)),
}));
vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: vi.fn(() => Promise.resolve(() => undefined)),
  }),
}));
vi.mock('@tanstack/react-virtual', () => ({
  useVirtualizer: (options: { count: number }) => ({
    getTotalSize: () => options.count * 28,
    getVirtualItems: () => [],
    scrollToIndex: () => undefined,
  }),
}));
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
  writeText: vi.fn(() => Promise.resolve()),
}));
vi.mock('../../ipc/commands', () => ({}));

describe('MainWindow at launch (§13.1: state arrives after first paint)', () => {
  it('renders with no snapshot yet — no render loop, empty state shown', () => {
    useAppStore.setState({
      snapshot: null,
      preview: null,
      processing: null,
      history: [],
      revertPreview: null,
    });
    useUiStore.setState({
      banner: null,
      confirm: null,
      alert: null,
      csvModalOpen: false,
      sideTab: 'rules',
      filterText: '',
      filterMode: 'all',
      editingRow: null,
      inspectorOpen: false,
    });
    render(<MainWindow />);
    expect(screen.getByText('Drag files or folders here')).toBeInTheDocument();
  });
});
