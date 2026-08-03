import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import InspectorView from './InspectorView';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { makeSnapshot } from '../../test/fixtures';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import * as commands from '../../ipc/commands';

vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
  writeText: vi.fn(() => Promise.resolve()),
}));
vi.mock('../../ipc/commands', () => ({
  getFileMetadata: vi.fn(),
}));

describe('InspectorView (§14.10)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      snapshot: makeSnapshot(),
      platform: { os: 'macos', channel: 'direct', hasSpotlight: true, version: '2.0.0' },
    });
  });

  it('shows the empty state with no inspected file', () => {
    useUiStore.setState({ inspectedId: null });
    render(<InspectorView />);
    expect(screen.getByText('Click a file to see its info.')).toBeInTheDocument();
  });

  it('renders general info and metadata with token-copy buttons', async () => {
    const user = userEvent.setup();
    vi.mocked(commands.getFileMetadata).mockResolvedValue({
      name: 'photo.jpg',
      path: '/d/photo.jpg',
      isDirectory: false,
      kind: 'JPEG image',
      size: '1.2 MB',
      created: '2026-07-27',
      modified: '2026-07-28',
      folder: '/d',
      attributes: [{ name: 'kMDItemPixelHeight', value: '3024' }],
    });
    useUiStore.setState({ inspectedId: 'SOME-ID' });
    render(<InspectorView />);
    expect(await screen.findByText('photo.jpg')).toBeInTheDocument();
    expect(screen.getByText('JPEG image')).toBeInTheDocument();
    expect(screen.getByText('Spotlight Metadata')).toBeInTheDocument();
    expect(screen.getByText('kMDItemPixelHeight')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Copy {kind}' }));
    expect(vi.mocked(writeText)).toHaveBeenCalledWith('{kind}');
    await user.click(screen.getByRole('button', { name: 'Copy {md:kMDItemPixelHeight}' }));
    expect(vi.mocked(writeText)).toHaveBeenCalledWith('{md:kMDItemPixelHeight}');
  });
});
