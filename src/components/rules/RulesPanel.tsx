// The rules panel (§14.3): the stack (cards → ghost add card → pinned trim
// pseudo-card), plus the persistent footer. Cards are keyed by rule.id —
// never array position (§13.4).

import { DndContext, PointerSensor, useSensor, useSensors, type DragEndEvent } from '@dnd-kit/core';
import { SortableContext, arrayMove, verticalListSortingStrategy } from '@dnd-kit/sortable';
import { Braces, Trash2 } from 'lucide-react';
import type { RenameRule } from '../../ipc/gen/RenameRule';
import type { RuleKind } from '../../ipc/gen/RuleKind';
import Menu, { MenuItem, MenuSeparator } from '../shell/Menu';
import RuleCard from './RuleCard';
import PresetsMenu from './PresetsMenu';
import { newRule, ruleTitle, RULE_KINDS, TOKENS } from './ruleMeta';
import { useAppStore, visibleRules } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';

export default function RulesPanel() {
  const snapshot = useAppStore((s) => s.snapshot);
  const preview = useAppStore((s) => s.preview);
  const rulesDraft = useAppStore((s) => s.rulesDraft);
  const editRules = useAppStore((s) => s.editRules);
  const showConfirm = useUiStore((s) => s.showConfirm);
  const rules = visibleRules({ rulesDraft, snapshot });
  const isFolders = snapshot?.listMode === 'Folders';
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));

  const impacts = new Map((preview?.ruleImpact ?? []).map((impact) => [impact.ruleId, impact]));

  const addRule = (kind: RuleKind) => {
    editRules([...rules, newRule(kind)]);
  };

  const changeRule = (updated: RenameRule) => {
    editRules(rules.map((rule) => (rule.id === updated.id ? updated : rule)));
  };

  const deleteRule = (id: string) => {
    // Deletion goes through the undoable funnel (§13.3).
    void commands.replaceRulesUndoable(
      rules.filter((rule) => rule.id !== id),
      snapshot?.trimsWhitespace ?? false,
      'Delete Rule',
    );
  };

  const duplicateRule = (id: string) => {
    const index = rules.findIndex((rule) => rule.id === id);
    const source = rules[index];
    if (!source) return;
    const copy = { ...source, id: crypto.randomUUID().toUpperCase() };
    const next = [...rules];
    next.splice(index + 1, 0, copy);
    editRules(next);
  };

  const moveRule = (id: string, direction: -1 | 1) => {
    const index = rules.findIndex((rule) => rule.id === id);
    const target = index + direction;
    if (index === -1 || target < 0 || target >= rules.length) return;
    editRules(arrayMove(rules, index, target));
  };

  const onDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    const from = rules.findIndex((rule) => rule.id === active.id);
    const to = rules.findIndex((rule) => rule.id === over.id);
    if (from !== -1 && to !== -1) {
      editRules(arrayMove(rules, from, to));
    }
  };

  const addMenuItems = (close: () => void) =>
    RULE_KINDS.map((kind) => (
      <MenuItem
        key={kind}
        label={ruleTitle(kind)}
        onSelect={() => {
          close();
          addRule(kind);
        }}
      />
    ));

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="min-h-0 flex-1 overflow-auto px-2 pb-2" data-testid="rules-stack">
        {rules.length === 0 && (
          <div className="px-2 py-6 text-center text-[13px]" style={{ color: 'var(--text-secondary)' }}>
            {snapshot?.rulesClearedByApply ? (
              <>
                <p className="font-medium" style={{ color: 'var(--text-primary)' }}>
                  {strings.rulesAppliedAndCleared}
                </p>
                <p className="mt-1">{strings.rulesAppliedHint}</p>
              </>
            ) : (
              <>
                <p className="font-medium" style={{ color: 'var(--text-primary)' }}>
                  {strings.noRulesYet}
                </p>
                <p className="mt-1">{strings.noRulesHint}</p>
                <button
                  className="mt-1 underline"
                  style={{ color: 'var(--link-text)' }}
                  onClick={() => {
                    void commands.openHelp('recipes');
                  }}
                >
                  {strings.seeExampleRecipes}
                </button>
              </>
            )}
          </div>
        )}

        <DndContext sensors={sensors} onDragEnd={onDragEnd}>
          <SortableContext items={rules.map((r) => r.id)} strategy={verticalListSortingStrategy}>
            {rules.map((rule, index) => (
              <RuleCard
                key={rule.id}
                rule={rule}
                impact={impacts.get(rule.id)}
                isFolders={isFolders}
                index={index}
                count={rules.length}
                onChange={changeRule}
                onDelete={() => {
                  deleteRule(rule.id);
                }}
                onDuplicate={() => {
                  duplicateRule(rule.id);
                }}
                onMove={(direction) => {
                  moveRule(rule.id, direction);
                }}
              />
            ))}
          </SortableContext>
        </DndContext>

        {/* The ghost card sits exactly where the next rule will land. */}
        <Menu
          ariaLabel="Add rule"
          trigger={(_, toggle) => (
            <button
              onClick={toggle}
              className="mb-2 w-full rounded-[10px] border border-dashed px-3 py-2 text-left text-[13px] hover:bg-[var(--wash-hover)]"
              style={{ borderColor: 'var(--separator)', color: 'var(--text-secondary)' }}
            >
              {strings.addRuleGhost}
            </button>
          )}
        >
          {addMenuItems}
        </Menu>

        {/* Pinned pseudo-card: placement teaches execution order (§14.3). */}
        <div
          className="rounded-[10px] border px-3 py-2"
          style={{ borderColor: 'var(--separator)', background: 'var(--window-bg)' }}
        >
          <p className="text-[10.5px] font-semibold tracking-wide" style={{ color: 'var(--text-secondary)' }}>
            {strings.afterAllRules}
          </p>
          <label className="mt-1 flex items-center gap-1.5 text-[12.5px]" style={{ color: 'var(--text-primary)' }}>
            <input
              type="checkbox"
              checked={snapshot?.trimsWhitespace ?? false}
              onChange={(event) => {
                void commands.setTrimsWhitespace(event.target.checked);
              }}
              className="h-3.5 w-3.5 accent-[var(--accent)]"
            />
            {strings.trimSpacesFromNames}
          </label>
        </div>
      </div>

      {/* Persistent footer — the constant affordance for long stacks. */}
      <footer
        className="flex items-center gap-1.5 px-2 py-2"
        style={{ borderTop: '1px solid var(--separator)' }}
      >
        <Menu
          ariaLabel="Add rule"
          trigger={(_, toggle) => (
            <button
              onClick={toggle}
              className="h-6 rounded-md border px-2 text-[12px] hover:bg-[var(--wash-hover)]"
              style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
            >
              {strings.addRule}
            </button>
          )}
        >
          {addMenuItems}
        </Menu>
        <PresetsMenu />
        <Menu
          ariaLabel="Token reference"
          trigger={(_, toggle) => (
            <button
              onClick={toggle}
              className="flex h-6 items-center gap-1 rounded-md border px-2 text-[12px] hover:bg-[var(--wash-hover)]"
              style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
            >
              <Braces size={12} aria-hidden />
              Tokens
            </button>
          )}
        >
          {(close) => (
            <div className="max-h-72 w-72 overflow-auto p-1">
              {TOKENS.map((token) => (
                <div key={token.insert} className="px-2 py-1">
                  <span className="font-mono text-[12px]" style={{ color: 'var(--text-primary)' }}>
                    {token.insert}
                  </span>
                  <span className="ml-2 text-[11.5px]" style={{ color: 'var(--text-secondary)' }}>
                    {token.description}
                  </span>
                </div>
              ))}
              <MenuSeparator />
              <MenuItem
                label="Open the full token reference"
                onSelect={() => {
                  close();
                  void commands.openHelp('tokens');
                }}
              />
            </div>
          )}
        </Menu>
        <div className="flex-1" />
        <button
          aria-label={strings.clearAllRules}
          title={strings.clearAllRules}
          disabled={rules.length === 0}
          onClick={() => {
            showConfirm({
              title: strings.clearRulesTitle,
              message: strings.clearRulesMessage,
              confirmLabel: strings.clearRulesButton,
              destructive: true,
              onConfirm: () => {
                void commands.replaceRulesUndoable(
                  [],
                  snapshot?.trimsWhitespace ?? false,
                  'Clear Rules',
                );
              },
            });
          }}
          className="flex h-[22px] w-[22px] items-center justify-center rounded hover:enabled:bg-[var(--wash-hover)] disabled:opacity-30"
          style={{ color: 'var(--text-secondary)' }}
        >
          <Trash2 size={13} aria-hidden />
        </button>
      </footer>
    </div>
  );
}
