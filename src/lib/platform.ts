// Per-OS labels and shortcut rendering (§15: ⌘ means Cmd on macOS, Ctrl
// elsewhere).

export const isMac: boolean =
  typeof navigator !== 'undefined' && /Mac|iP(hone|ad|od)/.test(navigator.userAgent);

/** Render a shortcut like `mod+Enter` per OS: `⌘↩` / `Ctrl+Enter`. */
export function shortcutLabel(parts: { mod?: boolean; shift?: boolean; key: string }): string {
  const key = parts.key === 'Enter' ? (isMac ? '↩' : 'Enter') : parts.key;
  if (isMac) {
    return `${parts.mod ? '⌘' : ''}${parts.shift ? '⇧' : ''}${key}`;
  }
  const pieces: string[] = [];
  if (parts.mod) pieces.push('Ctrl');
  if (parts.shift) pieces.push('Shift');
  pieces.push(key);
  return pieces.join('+');
}

/** The file-manager verb per OS (§14.2 context menu). */
export function revealLabel(os: string): string {
  if (os === 'macos') return 'Reveal in Finder';
  if (os === 'windows') return 'Show in Explorer';
  return 'Show in File Manager';
}

/** The inspector's metadata section title per OS (§14.10). */
export function metadataSectionTitle(os: string): string {
  if (os === 'macos') return 'Spotlight Metadata';
  if (os === 'windows') return 'File Properties';
  return 'Metadata';
}

/** True when the event's primary modifier (⌘ / Ctrl) is held. */
export function hasPrimaryModifier(event: { metaKey: boolean; ctrlKey: boolean }): boolean {
  return isMac ? event.metaKey : event.ctrlKey;
}
