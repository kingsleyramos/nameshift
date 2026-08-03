// The File Info drawer (§14.10): general info + every metadata attribute,
// each with a ⧉ button copying the matching token.

import { useEffect, useState } from 'react';
import { Copy } from 'lucide-react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import type { InspectorPayload } from '../../ipc/gen/InspectorPayload';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { metadataSectionTitle } from '../../lib/platform';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';

export default function InspectorView() {
  const inspectedId = useUiStore((s) => s.inspectedId);
  const os = useAppStore((s) => s.platform?.os ?? 'macos');
  const version = useAppStore((s) => s.snapshot?.version ?? 0);
  const [payload, setPayload] = useState<InspectorPayload | null>(null);

  useEffect(() => {
    let cancelled = false;
    if (inspectedId == null) {
      setPayload(null);
      return;
    }
    void commands
      .getFileMetadata(inspectedId)
      .then((data) => {
        if (!cancelled) setPayload(data);
      })
      .catch(() => {
        if (!cancelled) setPayload(null);
      });
    return () => {
      cancelled = true;
    };
  }, [inspectedId, version]);

  return (
    <aside
      className="flex h-full w-72 shrink-0 flex-col overflow-auto"
      style={{ borderLeft: '1px solid var(--separator)', background: 'var(--window-bg)' }}
      aria-label="File Info"
      key={inspectedId ?? 'empty'}
    >
      {payload == null ? (
        <p className="px-3 py-6 text-center text-[13px]" style={{ color: 'var(--text-secondary)' }}>
          {strings.inspectorEmpty}
        </p>
      ) : (
        <>
          <header className="px-3 py-2" style={{ borderBottom: '1px solid var(--separator)' }}>
            <p className="truncate text-[13px] font-medium" style={{ color: 'var(--text-primary)' }}>
              {payload.name}
            </p>
            <p className="truncate text-[11.5px]" style={{ color: 'var(--text-secondary)' }} title={payload.path}>
              {payload.path}
            </p>
          </header>
          <section className="px-3 py-2">
            <h3 className="text-[11px] font-semibold uppercase tracking-wide" style={{ color: 'var(--text-secondary)' }}>
              {strings.inspectorGeneral}
            </h3>
            <GeneralRow label={strings.inspectorKind} value={payload.kind} token="{kind}" />
            <GeneralRow label={strings.inspectorSize} value={payload.size} token="{size}" />
            <GeneralRow label={strings.inspectorCreated} value={payload.created} token="{created}" />
            <GeneralRow label={strings.inspectorModified} value={payload.modified} token="{modified}" />
            <GeneralRow label={strings.inspectorFolder} value={payload.folder} token="{folder}" />
          </section>
          <section className="px-3 py-2">
            <h3 className="text-[11px] font-semibold uppercase tracking-wide" style={{ color: 'var(--text-secondary)' }}>
              {metadataSectionTitle(os)}
            </h3>
            {payload.attributes.length === 0 && (
              <p className="py-2 text-[12px]" style={{ color: 'var(--text-secondary)' }}>
                No metadata attributes for this file.
              </p>
            )}
            {payload.attributes.map((attribute) => (
              <div key={attribute.name} className="group flex items-center gap-1 py-0.5">
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-[11.5px]" style={{ color: 'var(--text-secondary)' }}>
                    {attribute.name}
                  </span>
                  <span className="block truncate text-[12.5px]" style={{ color: 'var(--text-primary)' }} title={attribute.value}>
                    {attribute.value}
                  </span>
                </span>
                <CopyTokenButton token={`{md:${attribute.name}}`} />
              </div>
            ))}
          </section>
        </>
      )}
    </aside>
  );
}

function GeneralRow({ label, value, token }: { label: string; value: string | null; token: string }) {
  if (value == null) return null;
  return (
    <div className="group flex items-center gap-1 py-0.5">
      <span className="w-16 shrink-0 text-[11.5px]" style={{ color: 'var(--text-secondary)' }}>
        {label}
      </span>
      <span className="min-w-0 flex-1 truncate text-[12.5px]" style={{ color: 'var(--text-primary)' }} title={value}>
        {value}
      </span>
      <CopyTokenButton token={token} />
    </div>
  );
}

function CopyTokenButton({ token }: { token: string }) {
  return (
    <button
      aria-label={`Copy ${token}`}
      title={`Copy ${token}`}
      onClick={() => {
        void writeText(token);
      }}
      className="flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded opacity-0 hover:bg-[var(--wash-hover)] group-hover:opacity-100"
      style={{ color: 'var(--text-secondary)' }}
    >
      <Copy size={12} aria-hidden />
    </button>
  );
}
