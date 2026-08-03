// The apply feedback banner (§14.1): bottom-center capsule, auto-dismiss
// 6 s, aria-live announced.

import { useEffect } from 'react';
import { useUiStore } from '../../state/uiState';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';

export default function Banner() {
  const banner = useUiStore((s) => s.banner);
  const setUi = useUiStore((s) => s.set);

  useEffect(() => {
    if (!banner) return;
    const timer = setTimeout(() => {
      useUiStore.getState().set({ banner: null });
    }, 6000);
    return () => {
      clearTimeout(timer);
    };
  }, [banner]);

  if (!banner) return null;
  const suffix = banner.clearedRules
    ? ` ${strings.bannerRulesCleared}`
    : banner.keptRules
      ? ` ${strings.bannerRulesKept}`
      : '';
  return (
    <div
      role="status"
      aria-live="polite"
      className="fixed bottom-14 left-1/2 z-40 flex -translate-x-1/2 items-center gap-2 px-4 py-2 shadow-lg"
      style={{
        background: 'var(--pane-bg)',
        border: '1px solid var(--separator)',
        borderRadius: '999px',
      }}
    >
      <span className="text-[13px]" style={{ color: 'var(--text-primary)' }}>
        {strings.bannerRenamed(banner.count, banner.isFolders)}
        {suffix}
      </span>
      {banner.snapshotId != null && (
        <button
          onClick={() => {
            const id = banner.snapshotId;
            setUi({ banner: null, sideTab: 'history' });
            void commands.selectSnapshot(id);
          }}
          className="text-[13px] font-medium"
          style={{ color: 'var(--link-text)' }}
        >
          {strings.revertButton}
        </button>
      )}
      <button
        aria-label="Dismiss"
        onClick={() => {
          setUi({ banner: null });
        }}
        className="flex h-5 w-5 items-center justify-center rounded-full hover:bg-[var(--wash-hover)]"
        style={{ color: 'var(--text-secondary)' }}
      >
        ✕
      </button>
    </div>
  );
}
