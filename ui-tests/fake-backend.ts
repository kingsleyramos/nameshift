// A stand-in for the Rust backend, injected before the app's own scripts run.
//
// The UI talks to Tauri through `window.__TAURI_INTERNALS__`. Replacing that
// one object lets the REAL frontend — commands, events, store, components —
// run unmodified in an ordinary browser, so these tests work identically on
// macOS, Windows, and Linux with no desktop-automation driver involved.
//
// It models just enough state for the interface to behave: a file list, a
// rule stack, the derived preview, and naming-conflict detection.

/** Runs inside the page. Self-contained: no imports survive injection. */
export function installFakeBackend(): void {
  interface Rule {
    kind: string;
    text: string;
    isEnabled: boolean;
  }
  interface Item {
    id: string;
    path: string;
    isSelected: boolean;
    isDirectory: boolean;
  }

  let version = 1;
  let seq = 0;
  const items: Item[] = [];
  let rules: Rule[] = [];
  let listMode: 'Files' | 'Folders' = 'Files';
  const callbacks = new Map<number, (payload: unknown) => void>();
  const listeners = new Map<string, number[]>();

  const baseName = (path: string) => path.split(/[\\/]/).pop() ?? path;

  /** Apply the rule stack to one name. Mirrors the engine's common kinds. */
  function rename(name: string): string {
    const dot = name.lastIndexOf('.');
    let stem = dot > 0 ? name.slice(0, dot) : name;
    const ext = dot > 0 ? name.slice(dot) : '';
    for (const rule of rules) {
      if (!rule.isEnabled) continue;
      if (rule.kind === 'addPrefix') stem = rule.text + stem;
      else if (rule.kind === 'addSuffix') stem = stem + rule.text;
      else if (rule.kind === 'template') stem = rule.text;
      else if (rule.kind === 'removeText') stem = stem.split(rule.text).join('');
    }
    return stem + ext;
  }

  function activeItems(): Item[] {
    return items.filter((item) => item.isDirectory === (listMode === 'Folders'));
  }

  function buildPreview() {
    const active = activeItems();
    const targets = new Map<string, number>();
    for (const item of active) {
      const next = rename(baseName(item.path));
      targets.set(next, (targets.get(next) ?? 0) + 1);
    }
    const entries = active.map((item) => {
      const currentName = baseName(item.path);
      const newName = rename(currentName);
      const duplicate = (targets.get(newName) ?? 0) > 1;
      return {
        id: item.id,
        currentName,
        newName,
        isSelected: item.isSelected,
        hasOverride: false,
        isChanged: newName !== currentName,
        problem: duplicate ? 'DuplicateTarget' : null,
      };
    });
    const changeCount = entries.filter((e) => e.isChanged && e.isSelected).length;
    const conflictCount = entries.filter((e) => e.problem != null && e.isSelected).length;
    return {
      version,
      entries,
      ruleImpact: [],
      counts: {
        selectedCount: entries.filter((e) => e.isSelected).length,
        changeCount,
        conflictCount,
        canApply: changeCount > 0 && conflictCount === 0,
      },
      applyDisabledReason: null,
    };
  }

  function snapshot() {
    return {
      files: items.map((item) => ({
        id: item.id,
        path: item.path,
        folderId: null,
        isSelected: item.isSelected,
        isDirectory: item.isDirectory,
        overrideName: null,
      })),
      rules: rules.map((rule, index) => ({
        id: `rule-${String(index)}`,
        kind: rule.kind,
        isEnabled: rule.isEnabled,
        includesExtension: false,
        caseSensitive: true,
        text: rule.text,
        replacement: '',
        caseStyle: 'lowercase',
        numberPosition: 'after',
        numberStart: 1,
        numberPadding: 3,
        stripsDiacritics: false,
        restartPerFolder: false,
        removesEmoji: false,
      })),
      watchedFolders: [],
      presets: [],
      trimsWhitespace: false,
      autoResolvesConflicts: false,
      keepRulesAfterApply: true,
      includeSubfolders: false,
      sortKey: 'Order Added',
      sortAscending: true,
      listMode,
      selectedSnapshotId: null,
      rulesClearedByApply: false,
      version,
      rulesRevision: 0,
      isProcessing: false,
      overrideCount: 0,
      historyCount: 0,
      undoAction: null,
      redoAction: null,
    };
  }

  /** Bump the version and wake the UI exactly like AppState::mutate does. */
  function mutated(): void {
    version += 1;
    for (const id of listeners.get('state-changed') ?? []) {
      callbacks.get(id)?.({ version });
    }
  }

  function invoke(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
    switch (cmd) {
      case 'plugin:event|listen': {
        const event = args.event as string;
        const handler = args.handler as number;
        listeners.set(event, [...(listeners.get(event) ?? []), handler]);
        return Promise.resolve(handler);
      }
      case 'plugin:event|unlisten':
        return Promise.resolve(null);
      case 'get_platform':
        return Promise.resolve({
          os: 'linux',
          channel: 'direct',
          hasSpotlight: false,
          version: '2.0.0',
        });
      case 'get_state':
        return Promise.resolve(snapshot());
      case 'get_preview':
        return Promise.resolve(buildPreview());
      case 'get_history':
        return Promise.resolve([]);
      case 'get_revert_preview':
        return Promise.resolve(null);
      case 'import_paths': {
        for (const path of (args.paths as string[] | undefined) ?? []) {
          seq += 1;
          items.push({ id: `item-${String(seq)}`, path, isSelected: true, isDirectory: false });
        }
        mutated();
        return Promise.resolve(null);
      }
      case 'set_rules': {
        rules = ((args.rules as Rule[] | undefined) ?? []).map((rule) => ({
          kind: rule.kind,
          text: rule.text,
          isEnabled: rule.isEnabled,
        }));
        mutated();
        return Promise.resolve(null);
      }
      case 'set_selected': {
        const item = items.find((i) => i.id === args.id);
        if (item) item.isSelected = args.isSelected as boolean;
        mutated();
        return Promise.resolve(null);
      }
      case 'set_all_selected': {
        for (const item of items) item.isSelected = args.isSelected as boolean;
        mutated();
        return Promise.resolve(null);
      }
      case 'deselect_conflicted': {
        const conflicted = new Set(
          buildPreview()
            .entries.filter((e) => e.problem != null)
            .map((e) => e.id),
        );
        for (const item of items) {
          if (conflicted.has(item.id)) item.isSelected = false;
        }
        mutated();
        return Promise.resolve(null);
      }
      case 'remove_items': {
        const ids = new Set((args.ids as string[] | undefined) ?? []);
        for (let i = items.length - 1; i >= 0; i -= 1) {
          if (ids.has(items[i]?.id ?? '')) items.splice(i, 1);
        }
        mutated();
        return Promise.resolve(null);
      }
      case 'set_list_mode': {
        listMode = args.mode as 'Files' | 'Folders';
        mutated();
        return Promise.resolve(null);
      }
      case 'clear_all': {
        items.length = 0;
        rules = [];
        mutated();
        return Promise.resolve(null);
      }
      default:
        // Every other command is a no-op for interface tests.
        return Promise.resolve(null);
    }
  }

  let nextCallbackId = 0;
  const internals = {
    invoke,
    transformCallback(callback: (payload: unknown) => void) {
      nextCallbackId += 1;
      callbacks.set(nextCallbackId, callback);
      return nextCallbackId;
    },
    convertFileSrc: (src: string) => src,
    metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
    plugins: {},
  };

  Object.defineProperty(window, '__TAURI_INTERNALS__', { value: internals, writable: true });
  Object.defineProperty(window, '__TAURI__', {
    value: { core: { invoke } },
    writable: true,
  });
}
