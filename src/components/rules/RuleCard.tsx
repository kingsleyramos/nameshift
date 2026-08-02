// One rule card (§14.3): kind icon + title + impact + enable switch + grip
// + ↑/↓ + ✕, with per-kind fields. KEYED BY rule.id at the call site —
// positional identity crashed the legacy app when Apply cleared the array
// mid-interaction (§13.4 invariant; regression-tested).

import { useRef } from 'react';
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { ArrowDown, ArrowUp, GripVertical, TriangleAlert } from 'lucide-react';
import type { RenameRule } from '../../ipc/gen/RenameRule';
import type { RuleImpact } from '../../ipc/gen/RuleImpact';
import TokenInsertMenu from './TokenInsertMenu';
import { ruleIcon, ruleTitle } from './ruleMeta';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';

interface RuleCardProps {
  rule: RenameRule;
  impact: RuleImpact | undefined;
  isFolders: boolean;
  index: number;
  count: number;
  onChange: (rule: RenameRule) => void;
  onDelete: () => void;
  onDuplicate: () => void;
  onMove: (direction: -1 | 1) => void;
}

export default function RuleCard({
  rule,
  impact,
  isFolders,
  index,
  count,
  onChange,
  onDelete,
  onDuplicate,
  onMove,
}: RuleCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: rule.id,
  });
  const Icon = ruleIcon(rule.kind);
  const patch = (partial: Partial<RenameRule>) => {
    onChange({ ...rule, ...partial });
  };

  const invalidRegex =
    rule.kind === 'regexReplace' && rule.text.length > 0 && !regexCompiles(rule.text);
  const needsInput =
    ['removeText', 'replaceText', 'regexReplace', 'addPrefix', 'addSuffix', 'template', 'changeExtension'].includes(
      rule.kind,
    ) && rule.text.length === 0;
  const skippedForFolders = isFolders && rule.kind === 'changeExtension';

  return (
    <div
      ref={setNodeRef}
      data-testid={`rule-card-${rule.kind}`}
      className="mb-2 p-2.5"
      style={{
        background: 'var(--pane-bg)',
        border: '1px solid var(--separator)',
        borderRadius: 'var(--radius-card)',
        transform: CSS.Transform.toString(transform),
        transition,
        opacity: isDragging ? 0.6 : 1,
      }}
      onContextMenu={(event) => {
        event.preventDefault();
      }}
    >
      <div className="flex items-center gap-1.5">
        <Icon size={14} aria-hidden style={{ color: 'var(--text-secondary)' }} />
        <span className="text-[13px] font-medium" style={{ color: 'var(--text-primary)' }}>
          {ruleTitle(rule.kind)}
        </span>
        <div className="flex-1" />
        {impact && rule.isEnabled && !needsInput && !invalidRegex && impact.affected > 0 && (
          <span className="text-[11.5px]" style={{ color: 'var(--text-secondary)' }}>
            {strings.affectsCount(impact.affected, impact.eligible)}
          </span>
        )}
        {impact && rule.isEnabled && !needsInput && !invalidRegex && impact.affected === 0 && (
          <span
            className="flex items-center gap-1 text-[11.5px]"
            style={{ color: 'var(--warning-text)' }}
          >
            <TriangleAlert size={11} aria-hidden />
            {strings.affectsCount(0, impact.eligible)}
          </span>
        )}
        <label className="flex items-center" title={rule.isEnabled ? 'Disable rule' : 'Enable rule'}>
          <input
            type="checkbox"
            role="switch"
            aria-label={`${ruleTitle(rule.kind)} enabled`}
            checked={rule.isEnabled}
            onChange={(event) => {
              patch({ isEnabled: event.target.checked });
            }}
            className="h-3.5 w-6 accent-[var(--accent)]"
          />
        </label>
        <button
          {...attributes}
          {...listeners}
          aria-label="Reorder rule"
          className="flex h-[22px] w-[22px] cursor-grab items-center justify-center rounded hover:bg-[var(--wash-hover)]"
          style={{ color: 'var(--text-secondary)' }}
        >
          <GripVertical size={13} aria-hidden />
        </button>
        <button
          aria-label="Move rule up"
          disabled={index === 0}
          onClick={() => {
            onMove(-1);
          }}
          className="flex h-[22px] w-[22px] items-center justify-center rounded hover:enabled:bg-[var(--wash-hover)] disabled:opacity-30"
          style={{ color: 'var(--text-secondary)' }}
        >
          <ArrowUp size={13} aria-hidden />
        </button>
        <button
          aria-label="Move rule down"
          disabled={index === count - 1}
          onClick={() => {
            onMove(1);
          }}
          className="flex h-[22px] w-[22px] items-center justify-center rounded hover:enabled:bg-[var(--wash-hover)] disabled:opacity-30"
          style={{ color: 'var(--text-secondary)' }}
        >
          <ArrowDown size={13} aria-hidden />
        </button>
        <button
          aria-label={`Delete ${ruleTitle(rule.kind)}`}
          onClick={onDelete}
          className="flex h-[22px] w-[22px] items-center justify-center rounded hover:bg-[var(--wash-hover)]"
          style={{ color: 'var(--text-secondary)' }}
        >
          ✕
        </button>
      </div>

      <RuleFields rule={rule} patch={patch} invalidRegex={invalidRegex} />

      {needsInput && !skippedForFolders && (
        <p className="mt-1.5 flex items-center gap-1 text-[12px]" style={{ color: 'var(--text-secondary)' }}>
          <TriangleAlert size={11} aria-hidden />
          {strings.noInputYet}
        </p>
      )}
      {skippedForFolders && (
        <p className="mt-1.5 flex items-center gap-1 text-[12px]" style={{ color: 'var(--warning-text)' }}>
          <TriangleAlert size={11} aria-hidden />
          {strings.skippedForFolders}
        </p>
      )}
      {invalidRegex && (
        <p className="mt-1.5 text-[12px]" style={{ color: 'var(--warning-text)' }}>
          {strings.invalidRegex}{' '}
          <button
            className="underline"
            onClick={() => {
              void commands.openHelp('rename-rules');
            }}
          >
            {strings.syntaxReference}
          </button>
        </p>
      )}

      {rule.kind !== 'changeExtension' && (
        <label
          className="mt-1.5 flex items-center gap-1.5 text-[12px]"
          style={{ color: 'var(--text-secondary)' }}
          title={isFolders ? strings.folderNamesHaveNoExtension : undefined}
        >
          <input
            type="checkbox"
            checked={rule.includesExtension}
            disabled={isFolders}
            onChange={(event) => {
              patch({ includesExtension: event.target.checked });
            }}
            className="h-3.5 w-3.5 accent-[var(--accent)]"
          />
          {strings.includeExtension}
        </label>
      )}
      <button className="sr-only" onClick={onDuplicate}>
        Duplicate Rule
      </button>
    </div>
  );
}

function regexCompiles(pattern: string): boolean {
  try {
    // A cheap client-side validity hint only — the engine's fancy-regex is
    // authoritative and simply skips invalid rules (§5.3-c).
    new RegExp(pattern);
    return true;
  } catch {
    return false;
  }
}

function RuleFields({
  rule,
  patch,
  invalidRegex,
}: {
  rule: RenameRule;
  patch: (partial: Partial<RenameRule>) => void;
  invalidRegex: boolean;
}) {
  switch (rule.kind) {
    case 'removeText':
      return (
        <>
          <Field label="Text" value={rule.text} onChange={(text) => { patch({ text }); }} literal />
          <Toggle label={strings.matchCase} checked={rule.caseSensitive} onChange={(caseSensitive) => { patch({ caseSensitive }); }} />
        </>
      );
    case 'replaceText':
      return (
        <>
          <Field label="Find" value={rule.text} onChange={(text) => { patch({ text }); }} literal />
          <Field label="Replace with" value={rule.replacement} onChange={(replacement) => { patch({ replacement }); }} />
          <Toggle label={strings.matchCase} checked={rule.caseSensitive} onChange={(caseSensitive) => { patch({ caseSensitive }); }} />
        </>
      );
    case 'regexReplace':
      return (
        <>
          <Field label="Pattern" value={rule.text} onChange={(text) => { patch({ text }); }} literal invalid={invalidRegex} />
          <Field label="Replacement" value={rule.replacement} onChange={(replacement) => { patch({ replacement }); }} />
          <Toggle label={strings.matchCase} checked={rule.caseSensitive} onChange={(caseSensitive) => { patch({ caseSensitive }); }} />
        </>
      );
    case 'addPrefix':
    case 'addSuffix':
      return <Field label="Text" value={rule.text} onChange={(text) => { patch({ text }); }} />;
    case 'changeCase':
      return (
        <div role="radiogroup" aria-label="Case style" className="mt-1.5 flex gap-1">
          {(['lowercase', 'uppercase', 'titleCase'] as const).map((style) => (
            <button
              key={style}
              role="radio"
              aria-checked={rule.caseStyle === style}
              onClick={() => {
                patch({ caseStyle: style });
              }}
              className="rounded-md border px-2 py-0.5 text-[12px]"
              style={
                rule.caseStyle === style
                  ? { background: 'var(--accent)', color: '#fff', borderColor: 'var(--accent)' }
                  : { borderColor: 'var(--separator)', color: 'var(--text-primary)' }
              }
            >
              {style === 'lowercase' ? 'lowercase' : style === 'uppercase' ? 'UPPERCASE' : 'Title Case'}
            </button>
          ))}
        </div>
      );
    case 'changeExtension':
      return (
        <Field label="New extension" placeholder="jpg" value={rule.text} onChange={(text) => { patch({ text }); }} />
      );
    case 'sanitize':
      return (
        <>
          <Field label="Replace unsafe characters with" value={rule.text} onChange={(text) => { patch({ text }); }} literal />
          <Toggle label="Strip accents" checked={rule.stripsDiacritics} onChange={(stripsDiacritics) => { patch({ stripsDiacritics }); }} />
          <Toggle label="Remove emoji" checked={rule.removesEmoji} onChange={(removesEmoji) => { patch({ removesEmoji }); }} />
        </>
      );
    case 'numberSequentially':
      return (
        <>
          <div role="radiogroup" aria-label="Position" className="mt-1.5 flex gap-1">
            {(
              [
                ['before', 'Before name'],
                ['after', 'After name'],
                ['replaceName', 'Replace name'],
              ] as const
            ).map(([value, label]) => (
              <button
                key={value}
                role="radio"
                aria-checked={rule.numberPosition === value}
                onClick={() => {
                  patch({ numberPosition: value });
                }}
                className="rounded-md border px-2 py-0.5 text-[12px]"
                style={
                  rule.numberPosition === value
                    ? { background: 'var(--accent)', color: '#fff', borderColor: 'var(--accent)' }
                    : { borderColor: 'var(--separator)', color: 'var(--text-primary)' }
                }
              >
                {label}
              </button>
            ))}
          </div>
          <Field label="Separator" value={rule.text} onChange={(text) => { patch({ text }); }} />
          <div className="mt-1.5 flex gap-3">
            <Stepper label="Start" value={rule.numberStart} min={0} max={99999} onChange={(numberStart) => { patch({ numberStart }); }} />
            <Stepper label="Digits" value={rule.numberPadding} min={1} max={10} onChange={(numberPadding) => { patch({ numberPadding }); }} />
          </div>
          <Toggle label="Restart numbering in each folder" checked={rule.restartPerFolder} onChange={(restartPerFolder) => { patch({ restartPerFolder }); }} />
        </>
      );
    case 'template':
      return (
        <Field
          label="Template"
          placeholder="{created} {name} {n:3}"
          value={rule.text}
          onChange={(text) => {
            patch({ text });
          }}
        />
      );
  }
}

function Field({
  label,
  value,
  onChange,
  placeholder,
  literal,
  invalid,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  /** Literal fields never expand tokens (§14.3) — no { } menu. */
  literal?: boolean;
  invalid?: boolean;
}) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  return (
    <label className="mt-1.5 block text-[12px]" style={{ color: 'var(--text-secondary)' }}>
      {label}
      <span className="mt-0.5 flex items-center gap-1">
        <input
          ref={inputRef}
          value={value}
          placeholder={placeholder ?? (literal ? 'exact text' : undefined)}
          title={literal ? 'exact text' : undefined}
          onChange={(event) => {
            onChange(event.target.value);
          }}
          className="w-full rounded-md border px-2 py-1 text-[13px] outline-none focus:border-[var(--accent)]"
          style={{
            borderColor: invalid ? 'var(--warning-text)' : 'var(--separator)',
            background: 'var(--pane-bg)',
            color: 'var(--text-primary)',
          }}
        />
        {!literal && (
          <TokenInsertMenu
            inputRef={inputRef}
            value={value}
            onInsert={(next) => {
              onChange(next);
            }}
          />
        )}
      </span>
    </label>
  );
}

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="mt-1.5 flex items-center gap-1.5 text-[12px]" style={{ color: 'var(--text-secondary)' }}>
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => {
          onChange(event.target.checked);
        }}
        className="h-3.5 w-3.5 accent-[var(--accent)]"
      />
      {label}
    </label>
  );
}

function Stepper({
  label,
  value,
  min,
  max,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="flex items-center gap-1.5 text-[12px]" style={{ color: 'var(--text-secondary)' }}>
      {label}
      <input
        type="number"
        min={min}
        max={max}
        value={value}
        onChange={(event) => {
          const parsed = Number(event.target.value);
          if (Number.isFinite(parsed)) {
            onChange(Math.min(max, Math.max(min, Math.trunc(parsed))));
          }
        }}
        className="w-20 rounded-md border px-2 py-1 text-[13px] outline-none focus:border-[var(--accent)]"
        style={{ borderColor: 'var(--separator)', background: 'var(--pane-bg)', color: 'var(--text-primary)' }}
      />
    </label>
  );
}
