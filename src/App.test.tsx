import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import App from './App';

vi.mock('./components/shell/MainWindow', () => ({
  default: () => <p>main-window</p>,
}));
vi.mock('./components/help/HelpWindow', () => ({
  default: () => <p>help-window</p>,
}));
vi.mock('./components/settings/SettingsWindow', () => ({
  default: () => <p>settings-window</p>,
}));
vi.mock('./state/appState', () => ({
  startAppStore: vi.fn(() => Promise.resolve()),
}));

describe('App routing', () => {
  it('renders the main window by default', () => {
    window.location.hash = '';
    render(<App />);
    expect(screen.getByText('main-window')).toBeInTheDocument();
  });

  it('routes #/help and #/settings to their windows', () => {
    window.location.hash = '#/help';
    render(<App />);
    expect(screen.getByText('help-window')).toBeInTheDocument();
    window.location.hash = '#/settings';
    render(<App />);
    expect(screen.getByText('settings-window')).toBeInTheDocument();
    window.location.hash = '';
  });
});
