// Typed listen wrappers — one per §12.2 event.

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Alert } from './gen/Alert';
import type { ApplyFinished } from './gen/ApplyFinished';
import type { OpenPaths } from './gen/OpenPaths';
import type { Processing } from './gen/Processing';
import type { RevertFinished } from './gen/RevertFinished';
import type { StateChanged } from './gen/StateChanged';

export const onStateChanged = (handler: (payload: StateChanged) => void): Promise<UnlistenFn> =>
  listen<StateChanged>('state-changed', (event) => {
    handler(event.payload);
  });

export const onProcessing = (handler: (payload: Processing | null) => void): Promise<UnlistenFn> =>
  listen<Processing | null>('processing', (event) => {
    handler(event.payload);
  });

export const onApplyFinished = (handler: (payload: ApplyFinished) => void): Promise<UnlistenFn> =>
  listen<ApplyFinished>('apply-finished', (event) => {
    handler(event.payload);
  });

export const onRevertFinished = (
  handler: (payload: RevertFinished) => void,
): Promise<UnlistenFn> =>
  listen<RevertFinished>('revert-finished', (event) => {
    handler(event.payload);
  });

export const onAlert = (handler: (payload: Alert) => void): Promise<UnlistenFn> =>
  listen<Alert>('alert', (event) => {
    handler(event.payload);
  });

export const onOpenPaths = (handler: (payload: OpenPaths) => void): Promise<UnlistenFn> =>
  listen<OpenPaths>('open-paths', (event) => {
    handler(event.payload);
  });
