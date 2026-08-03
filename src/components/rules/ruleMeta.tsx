// Rule-kind display metadata: titles (§14.3) + the committed icon language
// (§14.13). Serialized kinds never change; these are display-only.

import {
  ArrowLeftToLine,
  ArrowRightToLine,
  CaseSensitive,
  Eraser,
  FileCog,
  Hash,
  Regex,
  Replace,
  ShieldCheck,
  SquareStack,
  type LucideIcon,
} from 'lucide-react';
import type { RuleKind } from '../../ipc/gen/RuleKind';

export const RULE_KINDS: RuleKind[] = [
  'removeText',
  'replaceText',
  'regexReplace',
  'addPrefix',
  'addSuffix',
  'changeCase',
  'changeExtension',
  'sanitize',
  'numberSequentially',
  'template',
];

export function ruleTitle(kind: RuleKind): string {
  switch (kind) {
    case 'removeText':
      return 'Remove Text';
    case 'replaceText':
      return 'Find & Replace';
    case 'regexReplace':
      return 'Regex Replace';
    case 'addPrefix':
      return 'Add Prefix';
    case 'addSuffix':
      return 'Add Suffix';
    case 'changeCase':
      return 'Change Case';
    case 'changeExtension':
      return 'Change Extension';
    case 'sanitize':
      return 'Fix Unsafe Characters';
    case 'numberSequentially':
      return 'Number Sequentially';
    case 'template':
      return 'New Name from Template';
  }
}

export function ruleIcon(kind: RuleKind): LucideIcon {
  switch (kind) {
    case 'removeText':
      return Eraser;
    case 'replaceText':
      return Replace;
    case 'regexReplace':
      return Regex;
    case 'addPrefix':
      return ArrowLeftToLine;
    case 'addSuffix':
      return ArrowRightToLine;
    case 'changeCase':
      return CaseSensitive;
    case 'changeExtension':
      return FileCog;
    case 'sanitize':
      return ShieldCheck;
    case 'numberSequentially':
      return Hash;
    case 'template':
      return SquareStack;
  }
}

export function newRule(kind: RuleKind) {
  return {
    id: crypto.randomUUID().toUpperCase(),
    kind,
    isEnabled: true,
    includesExtension: false,
    caseSensitive: true,
    text: '',
    replacement: '',
    caseStyle: 'lowercase' as const,
    numberPosition: 'after' as const,
    numberStart: 1,
    numberPadding: 3,
    stripsDiacritics: false,
    restartPerFolder: false,
    removesEmoji: false,
  };
}

/** The §6 token list for insert menus — key, insertion text, description. */
export const TOKENS: { insert: string; description: string }[] = [
  { insert: '{name}', description: 'Current name without extension' },
  { insert: '{ext}', description: 'Extension without the dot' },
  { insert: '{folder}', description: 'Name of the containing folder' },
  { insert: '{n}', description: 'Sequence number (deselected files don’t count)' },
  { insert: '{n:3}', description: 'Zero-padded sequence number (001, 002…)' },
  { insert: '{created}', description: 'Creation date, e.g. 2026-07-27' },
  { insert: '{created:yyyy-MM-dd HH.mm}', description: 'Creation date with a custom format' },
  { insert: '{modified}', description: 'Last-modified date' },
  { insert: '{date}', description: 'Today' },
  { insert: '{size}', description: 'File size, e.g. 1.2 MB' },
  { insert: '{kind}', description: 'File kind, e.g. JPEG image' },
  { insert: '{md:…}', description: 'A metadata attribute (see File Info for names)' },
];
