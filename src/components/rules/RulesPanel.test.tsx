import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import RulesPanel from './RulesPanel';
import { useAppStore } from '../../state/appState';
import { makeRule, makeSnapshot } from '../../test/fixtures';
import * as commands from '../../ipc/commands';

vi.mock('../../ipc/commands', () => ({
  setRules: vi.fn(() => Promise.resolve()),
  replaceRulesUndoable: vi.fn(() => Promise.resolve()),
  setTrimsWhitespace: vi.fn(() => Promise.resolve()),
  openHelp: vi.fn(() => Promise.resolve()),
  presetNameExists: vi.fn(() => Promise.resolve(false)),
  savePreset: vi.fn(() => Promise.resolve()),
  applyPreset: vi.fn(() => Promise.resolve()),
  deletePreset: vi.fn(() => Promise.resolve()),
  importPresets: vi.fn(() => Promise.resolve()),
  exportPresets: vi.fn(() => Promise.resolve()),
}));

describe('RulesPanel (§14.3)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({ rulesDraft: null, draftRevision: -1, preview: null });
  });

  it('shows the empty state with the recipes link and the ghost card', () => {
    useAppStore.setState({ snapshot: makeSnapshot() });
    render(<RulesPanel />);
    expect(screen.getByText('No rules yet')).toBeInTheDocument();
    expect(screen.getByText('See Example Recipes')).toBeInTheDocument();
    expect(screen.getByText('＋ Add Rule…')).toBeInTheDocument();
    expect(screen.getByText('AFTER ALL RULES')).toBeInTheDocument();
  });

  it('shows the post-apply state only when rules were cleared by apply', () => {
    useAppStore.setState({ snapshot: makeSnapshot({ rulesClearedByApply: true }) });
    render(<RulesPanel />);
    expect(screen.getByText('Rules applied and cleared')).toBeInTheDocument();
  });

  it('adds a rule from the ghost card into the optimistic draft', async () => {
    const user = userEvent.setup();
    useAppStore.setState({ snapshot: makeSnapshot() });
    render(<RulesPanel />);
    await user.click(screen.getByText('＋ Add Rule…'));
    await user.click(screen.getByRole('menuitem', { name: /Add Prefix/ }));
    const draft = useAppStore.getState().rulesDraft;
    expect(draft).toHaveLength(1);
    expect(draft?.[0]?.kind).toBe('addPrefix');
  });

  it('REGRESSION: clearing the rules from underneath an open card must not crash (id-keyed)', () => {
    const rule = makeRule({ kind: 'addPrefix', text: 'x-' });
    useAppStore.setState({ snapshot: makeSnapshot({ rules: [rule] }) });
    const { rerender } = render(<RulesPanel />);
    expect(screen.getByTestId('rule-card-addPrefix')).toBeInTheDocument();
    // Apply clears the array from underneath (rulesRevision bumps).
    useAppStore.setState({
      snapshot: makeSnapshot({ rules: [], rulesRevision: 1, rulesClearedByApply: true }),
    });
    expect(() => {
      rerender(<RulesPanel />);
    }).not.toThrow();
    expect(screen.queryByTestId('rule-card-addPrefix')).not.toBeInTheDocument();
    expect(screen.getByText('Rules applied and cleared')).toBeInTheDocument();
  });

  it('marks an invalid regex pattern and keeps the rule inert', () => {
    const rule = makeRule({ kind: 'regexReplace', text: '(' });
    useAppStore.setState({ snapshot: makeSnapshot({ rules: [rule] }) });
    render(<RulesPanel />);
    expect(screen.getByText('This pattern isn’t a valid regular expression.')).toBeInTheDocument();
    expect(screen.getByText('Syntax reference')).toBeInTheDocument();
  });

  it('shows the no-input hint for empty primary fields', () => {
    const rule = makeRule({ kind: 'removeText', text: '' });
    useAppStore.setState({ snapshot: makeSnapshot({ rules: [rule] }) });
    render(<RulesPanel />);
    expect(screen.getByText('No input yet — rule is skipped')).toBeInTheDocument();
  });

  it('badges Change Extension cards in Folders mode', () => {
    const rule = makeRule({ kind: 'changeExtension', text: 'jpg' });
    useAppStore.setState({
      snapshot: makeSnapshot({ rules: [rule], listMode: 'Folders' }),
    });
    render(<RulesPanel />);
    expect(screen.getByText('Skipped for folders')).toBeInTheDocument();
  });

  it('deleting a rule goes through the undoable funnel', async () => {
    const user = userEvent.setup();
    const rule = makeRule({ kind: 'addSuffix', text: '-v2' });
    useAppStore.setState({ snapshot: makeSnapshot({ rules: [rule] }) });
    render(<RulesPanel />);
    await user.click(screen.getByRole('button', { name: 'Delete Add Suffix' }));
    expect(vi.mocked(commands.replaceRulesUndoable)).toHaveBeenCalledWith(
      [],
      false,
      'Delete Rule',
    );
  });

  it('shows affects counts from rule impact', () => {
    const rule = makeRule({ kind: 'addPrefix', text: 'x-' });
    useAppStore.setState({
      snapshot: makeSnapshot({ rules: [rule] }),
      preview: {
        version: 1,
        entries: [],
        ruleImpact: [{ ruleId: rule.id, affected: 14, eligible: 20 }],
        counts: { selectedCount: 20, changeCount: 14, conflictCount: 0, canApply: true },
        applyDisabledReason: null,
      },
    });
    render(<RulesPanel />);
    expect(screen.getByText('affects 14 of 20')).toBeInTheDocument();
  });
});
