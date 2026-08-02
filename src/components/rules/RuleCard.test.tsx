import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DndContext } from '@dnd-kit/core';
import { SortableContext } from '@dnd-kit/sortable';
import RuleCard from './RuleCard';
import TokenInsertMenu, { insertToken } from './TokenInsertMenu';
import { ruleIcon, ruleTitle, RULE_KINDS } from './ruleMeta';
import { makeRule } from '../../test/fixtures';
import type { RenameRule } from '../../ipc/gen/RenameRule';

vi.mock('../../ipc/commands', () => ({
  openHelp: vi.fn(() => Promise.resolve()),
}));

function renderCard(rule: RenameRule, extra: Partial<Parameters<typeof RuleCard>[0]> = {}) {
  const onChange = vi.fn();
  render(
    <DndContext>
      <SortableContext items={[rule.id]}>
        <RuleCard
          rule={rule}
          impact={undefined}
          isFolders={false}
          index={0}
          count={2}
          onChange={onChange}
          onDelete={() => undefined}
          onDuplicate={() => undefined}
          onMove={() => undefined}
          {...extra}
        />
      </SortableContext>
    </DndContext>,
  );
  return onChange;
}

describe('RuleCard fields (§14.3 per-kind)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('every kind has a title and an icon mapping (§14.13)', () => {
    for (const kind of RULE_KINDS) {
      expect(ruleTitle(kind).length).toBeGreaterThan(0);
      expect(ruleIcon(kind)).toBeTruthy();
    }
    expect(ruleTitle('sanitize')).toBe('Fix Unsafe Characters');
    expect(ruleTitle('template')).toBe('New Name from Template');
  });

  it('find & replace exposes Find / Replace with / Match case', async () => {
    const user = userEvent.setup();
    const onChange = renderCard(makeRule({ kind: 'replaceText', text: 'a', replacement: 'b' }));
    expect(screen.getByText('Find')).toBeInTheDocument();
    expect(screen.getByText('Replace with')).toBeInTheDocument();
    const fields = screen.getAllByRole('textbox');
    await user.type(fields[1]!, 'x');
    expect(onChange).toHaveBeenCalled();
    await user.click(screen.getByRole('checkbox', { name: 'Match case' }));
    expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ caseSensitive: false }));
  });

  it('change case renders the segmented control', async () => {
    const user = userEvent.setup();
    const onChange = renderCard(makeRule({ kind: 'changeCase' }));
    await user.click(screen.getByRole('radio', { name: 'Title Case' }));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ caseStyle: 'titleCase' }));
  });

  it('number sequentially renders position, separator, steppers, and restart', async () => {
    const user = userEvent.setup();
    const onChange = renderCard(makeRule({ kind: 'numberSequentially', text: '-' }));
    await user.click(screen.getByRole('radio', { name: 'Before name' }));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ numberPosition: 'before' }));
    await user.click(
      screen.getByRole('checkbox', { name: 'Restart numbering in each folder' }),
    );
    expect(onChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ restartPerFolder: true }),
    );
    expect(screen.getByLabelText('Start')).toBeInTheDocument();
    expect(screen.getByLabelText('Digits')).toBeInTheDocument();
  });

  it('sanitize exposes replacement + strip accents + remove emoji', async () => {
    const user = userEvent.setup();
    const onChange = renderCard(makeRule({ kind: 'sanitize', text: '' }));
    await user.click(screen.getByRole('checkbox', { name: 'Strip accents' }));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ stripsDiacritics: true }));
    await user.click(screen.getByRole('checkbox', { name: 'Remove emoji' }));
    expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ removesEmoji: true }));
  });

  it('change extension hides Include extension; others disable it in Folders mode', () => {
    renderCard(makeRule({ kind: 'changeExtension', text: 'jpg' }));
    expect(screen.queryByRole('checkbox', { name: 'Include extension' })).not.toBeInTheDocument();
  });

  it('include extension is disabled with help in Folders mode (§14.3 CON-05)', () => {
    renderCard(makeRule({ kind: 'addPrefix', text: 'x' }), { isFolders: true });
    const checkbox = screen.getByRole('checkbox', { name: 'Include extension' });
    expect(checkbox).toBeDisabled();
    expect(screen.getByTitle('Folder names have no extension')).toBeInTheDocument();
  });

  it('the enable switch and reorder buttons are wired', async () => {
    const user = userEvent.setup();
    const onMove = vi.fn();
    const onChange = renderCard(makeRule({ kind: 'addSuffix', text: '-v2' }), { onMove });
    await user.click(screen.getByRole('switch', { name: 'Add Suffix enabled' }));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ isEnabled: false }));
    expect(screen.getByRole('button', { name: 'Move rule up' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Move rule down' }));
    expect(onMove).toHaveBeenCalledWith(1);
  });

  it('literal fields carry the exact-text hint and no token menu', () => {
    renderCard(makeRule({ kind: 'removeText', text: 'x' }));
    expect(screen.getByPlaceholderText('exact text')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Insert token' })).not.toBeInTheDocument();
  });

  it('token-capable fields get the { } menu that inserts at the cursor', async () => {
    const user = userEvent.setup();
    const onChange = renderCard(makeRule({ kind: 'addPrefix', text: 'photo-' }));
    await user.click(screen.getByRole('button', { name: 'Insert token' }));
    await user.click(screen.getByText('Sequence number (deselected files don’t count)'));
    expect(onChange).toHaveBeenCalled();
  });
});

describe('TokenInsertMenu component', () => {
  it('inserts and repositions the caret via the callback', async () => {
    const user = userEvent.setup();
    const onInsert = vi.fn();
    const ref = { current: null };
    render(<TokenInsertMenu inputRef={ref} value="abc" onInsert={onInsert} />);
    await user.click(screen.getByRole('button', { name: 'Insert token' }));
    await user.click(screen.getByRole('menuitem', { name: /\{name\}/ }));
    expect(onInsert).toHaveBeenCalledWith('abc{name}', 9);
  });

  it('insertToken clamps stale selections', () => {
    expect(insertToken('{n}', 'ab', 99, 120)).toEqual({ text: 'ab{n}', caret: 5 });
  });
});
