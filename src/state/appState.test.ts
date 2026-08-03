// Store logic (§18.4): the version guard drops stale payloads; the
// optimistic rules draft reconciles when the authoritative revision moves.

import { mockIPC } from '@tauri-apps/api/mocks';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore, visibleRules } from './appState';
import { makeRule, makeSnapshot } from '../test/fixtures';

describe('appState store (§13.1)', () => {
  beforeEach(() => {
    useAppStore.setState({
      snapshot: null,
      preview: null,
      revertPreview: null,
      history: [],
      rulesDraft: null,
      draftRevision: -1,
    });
  });

  it('drops payloads older than what it has (out-of-order guard)', async () => {
    useAppStore.setState({ snapshot: makeSnapshot({ version: 10 }) });
    mockIPC((cmd) => {
      if (cmd === 'get_state') return makeSnapshot({ version: 4 });
      if (cmd === 'get_preview') {
        return {
          version: 4,
          entries: [],
          ruleImpact: [],
          counts: { selectedCount: 0, changeCount: 0, conflictCount: 0, canApply: false },
          applyDisabledReason: null,
        };
      }
      if (cmd === 'get_history') return [];
      return null;
    });
    await useAppStore.getState().refresh();
    expect(useAppStore.getState().snapshot?.version).toBe(10, );
  });

  it('accepts newer payloads', async () => {
    useAppStore.setState({ snapshot: makeSnapshot({ version: 3 }) });
    mockIPC((cmd) => {
      if (cmd === 'get_state') return makeSnapshot({ version: 7 });
      if (cmd === 'get_preview') {
        return {
          version: 7,
          entries: [],
          ruleImpact: [],
          counts: { selectedCount: 0, changeCount: 0, conflictCount: 0, canApply: false },
          applyDisabledReason: null,
        };
      }
      if (cmd === 'get_history') return [];
      return null;
    });
    await useAppStore.getState().refresh();
    expect(useAppStore.getState().snapshot?.version).toBe(7);
  });

  it('keeps the optimistic draft while the revision is unchanged', async () => {
    const draft = [makeRule({ text: 'typing…' })];
    useAppStore.setState({
      snapshot: makeSnapshot({ version: 1, rulesRevision: 5 }),
      rulesDraft: draft,
      draftRevision: 5,
    });
    mockIPC((cmd) => {
      if (cmd === 'get_state') return makeSnapshot({ version: 2, rulesRevision: 5 });
      if (cmd === 'get_preview') {
        return {
          version: 2,
          entries: [],
          ruleImpact: [],
          counts: { selectedCount: 0, changeCount: 0, conflictCount: 0, canApply: false },
          applyDisabledReason: null,
        };
      }
      if (cmd === 'get_history') return [];
      return null;
    });
    await useAppStore.getState().refresh();
    expect(useAppStore.getState().rulesDraft).toEqual(draft);
  });

  it('discards the draft when rules change underneath (undo/preset/apply)', async () => {
    useAppStore.setState({
      snapshot: makeSnapshot({ version: 1, rulesRevision: 5 }),
      rulesDraft: [makeRule({ text: 'typing…' })],
      draftRevision: 5,
    });
    mockIPC((cmd) => {
      if (cmd === 'get_state') return makeSnapshot({ version: 2, rulesRevision: 6, rules: [] });
      if (cmd === 'get_preview') {
        return {
          version: 2,
          entries: [],
          ruleImpact: [],
          counts: { selectedCount: 0, changeCount: 0, conflictCount: 0, canApply: false },
          applyDisabledReason: null,
        };
      }
      if (cmd === 'get_history') return [];
      return null;
    });
    await useAppStore.getState().refresh();
    expect(useAppStore.getState().rulesDraft).toBeNull();
  });

  it('editRules debounces the authoritative sync (~120 ms)', async () => {
    vi.useFakeTimers();
    const calls: unknown[] = [];
    mockIPC((cmd, args) => {
      if (cmd === 'set_rules') calls.push(args);
      return null;
    });
    useAppStore.setState({ snapshot: makeSnapshot({ rulesRevision: 0 }) });
    const rule = makeRule({ text: 'a' });
    useAppStore.getState().editRules([rule]);
    useAppStore.getState().editRules([{ ...rule, text: 'ab' }]);
    expect(calls).toHaveLength(0);
    await vi.advanceTimersByTimeAsync(150);
    expect(calls).toHaveLength(1);
    vi.useRealTimers();
  });

  it('visibleRules prefers the draft', () => {
    const draft = [makeRule({ text: 'draft' })];
    const authoritative = makeSnapshot({ rules: [makeRule({ text: 'auth' })] });
    expect(visibleRules({ rulesDraft: draft, snapshot: authoritative })).toEqual(draft);
    expect(visibleRules({ rulesDraft: null, snapshot: authoritative })).toEqual(
      authoritative.rules,
    );
  });
});
