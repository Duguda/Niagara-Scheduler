// App updates: on start (main window only) Niagara asks the latest GitHub release whether a newer,
// signed version exists, and offers it. Settings → About can check by hand. Installing restarts the app.
import { t } from './i18n.svelte';
import { toast } from './state.svelte';

const inTauri = () => '__TAURI_INTERNALS__' in window;

export const updater = $state<{
  checking: boolean;
  available: string | null;
  /** Release notes of the found version (from latest.json). */
  notes: string;
  /** Show the "new version" dialog. */
  prompt: boolean;
  installing: boolean;
  error: string | null;
}>({
  checking: false,
  available: null,
  notes: '',
  prompt: false,
  installing: false,
  error: null,
});

let pending: import('@tauri-apps/plugin-updater').Update | null = null;

/** Asks for a newer version. `quiet`: on start, say nothing unless one is found. */
export async function checkForUpdate(quiet = false): Promise<void> {
  if (!inTauri() || updater.checking || updater.installing) return;
  updater.checking = true;
  updater.error = null;
  try {
    const { check } = await import('@tauri-apps/plugin-updater');
    pending = await check();
    updater.available = pending?.version ?? null;
    updater.notes = pending?.body ?? '';
    if (pending) {
      updater.prompt = true;
    } else if (!quiet) {
      toast(t('upd.latest'), 'success');
    }
  } catch (e) {
    updater.error = String(e);
    if (!quiet) toast(t('upd.failed', { error: String(e) }), 'error');
  } finally {
    updater.checking = false;
  }
}

/** Downloads and installs the found update, then restarts. Data stays where it is. */
export async function installUpdate(): Promise<void> {
  if (!pending || updater.installing) return;
  updater.installing = true;
  try {
    await pending.downloadAndInstall();
    const { relaunch } = await import('@tauri-apps/plugin-process');
    await relaunch();
  } catch (e) {
    toast(t('upd.failed', { error: String(e) }), 'error');
  } finally {
    updater.installing = false;
  }
}
