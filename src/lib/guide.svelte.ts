// The first-run guide: shown once on the first start, and any time from Settings.
import { api } from './api';

const DONE_KEY = 'ui.guideDone';

export const guide = $state({ open: false });

/** Opens the guide on start unless it has been finished or skipped before. */
export async function initGuide() {
  const done = await api.getSetting(DONE_KEY).catch(() => '1');
  if (!done) guide.open = true;
}

export function openGuide() {
  guide.open = true;
}

/** Closes the guide and remembers it, whether it was finished or skipped. */
export function closeGuide() {
  guide.open = false;
  api.setSetting(DONE_KEY, '1').catch(() => {});
}
