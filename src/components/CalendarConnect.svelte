<script lang="ts">
  // Bringing in another calendar: subscribe to its iCal address (Google Calendar's secret address;
  // kept up to date every 30 minutes, one way), or import an .ics / .zip file once.
  // Used on the Calendar page and in the first-run guide (`compact`: subscription only).
  import { CalendarPlus, Link2, LoaderCircle, RefreshCw, Unlink } from '@lucide/svelte';
  import { api, type CalSub, type IcsPreview } from '../lib/api';
  import { fmtTs } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { hex } from '../lib/colors';
  import { mutate, toast } from '../lib/state.svelte';

  let { compact = false }: { compact?: boolean } = $props();

  let subs = $state<CalSub[]>([]);
  let name = $state('');
  let url = $state('');
  let adding = $state(false);
  let syncing = $state<string | null>(null);
  let removing = $state<string | null>(null);

  const load = () => api.calSubs().then((s) => (subs = s)).catch(() => {});
  load();

  async function add() {
    if (!url.trim() || adding) return;
    adding = true;
    try {
      const r = await api.calSubAdd(name, url);
      toast(t('sub.added', { n: r.added }), 'success');
      name = url = '';
      await load();
    } catch (e) {
      const msg = String(e);
      toast(
        msg.includes('already') ? t('sub.already') : msg.includes('bad url') ? t('sub.badUrl') : t('sub.failed', { error: msg }),
        'error',
      );
    } finally {
      adding = false;
    }
  }

  async function syncNow(id: string) {
    syncing = id;
    await api.calSubSync(id).catch((e) => toast(String(e), 'error'));
    await load();
    syncing = null;
  }

  async function remove(id: string, keepEvents: boolean) {
    await mutate(api.calSubRemove(id, keepEvents));
    removing = null;
    await load();
  }

  // ---- one-off file import
  let file = $state<{ path: string; preview: IcsPreview } | null>(null);
  let past = $state(false);
  let importing = $state(false);

  async function pickFile() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const path = await open({ multiple: false, directory: false, filters: [{ name: t('ics.fileType'), extensions: ['ics', 'zip'] }] });
      if (typeof path !== 'string') return;
      past = false;
      file = { path, preview: await api.icsPreview(path) };
    } catch (e) {
      toast(t('ics.readFailed', { error: String(e) }), 'error');
    }
  }

  async function importFile() {
    if (!file) return;
    importing = true;
    const r = await mutate(api.icsImport(file.path, past));
    importing = false;
    if (!r) return;
    file = null;
    toast(t('ics.done', { added: r.added, updated: r.updated }), 'success');
  }
</script>

<div class="connect">
  <section>
    <h3><Link2 size={16} /> {t('sub.title')}</h3>
    <p class="muted small">{t('sub.body')}</p>
    <ol class="how small">
      <li>{t('sub.how1')}</li>
      <li>{t('sub.how2')}</li>
      <li>{t('sub.how3')}</li>
    </ol>
    <div class="form">
      <input class="input name" bind:value={name} placeholder={t('sub.namePh')} aria-label={t('sub.namePh')} />
      <input class="input url" bind:value={url} placeholder="https://calendar.google.com/calendar/ical/…/basic.ics" aria-label={t('sub.urlLabel')}
        onkeydown={(e) => e.key === 'Enter' && add()} />
      <button class="btn primary" onclick={add} disabled={adding || !url.trim()}>
        {#if adding}<span class="spin"><LoaderCircle size={15} /></span>{:else}<Link2 size={15} />{/if} {t('sub.connect')}
      </button>
    </div>
    <p class="faint small">{t('sub.privacy')}</p>

    {#if subs.length}
      <ul class="subs">
        {#each subs as s (s.id)}
          <li>
            <span class="dot" style:background={hex(s.color)}></span>
            <span class="sub-text">
              <span class="strong small">{s.name}</span>
              <span class="faint small">
                {#if s.last_error}<span class="err">{t('sub.error', { error: s.last_error })}</span>
                {:else if s.last_sync}{t('sub.synced', { time: fmtTs(s.last_sync, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', hour12: false }), n: s.events })}{/if}
              </span>
            </span>
            {#if removing === s.id}
              <span class="ask small">
                <button class="btn small danger" onclick={() => remove(s.id, false)}>{t('sub.removeAll')}</button>
                <button class="btn small" onclick={() => remove(s.id, true)}>{t('sub.removeKeep')}</button>
                <button class="btn small ghost" onclick={() => (removing = null)}>{t('common.cancel')}</button>
              </span>
            {:else}
              <button class="icon-btn" onclick={() => syncNow(s.id)} disabled={syncing === s.id} title={t('sub.syncNow')} aria-label={t('sub.syncNow')}>
                <span class:spin={syncing === s.id}><RefreshCw size={15} /></span>
              </button>
              <button class="icon-btn" onclick={() => (removing = s.id)} title={t('sub.remove')} aria-label={t('sub.remove')}><Unlink size={15} /></button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  {#if !compact}
    <section>
      <h3><CalendarPlus size={16} /> {t('ics.import')}</h3>
      <p class="muted small">{t('ics.importHint')}</p>
      {#if !file}
        <button class="btn" onclick={pickFile}><CalendarPlus size={15} /> {t('ics.pick')}</button>
      {:else}
        {@const p = file.preview}
        <div class="preview">
          {#if p.calendars.length}<p class="strong small">{p.calendars.join(' · ')}</p>{/if}
          <ul class="small">
            <li>{t('ics.total', { n: p.total }) + (p.repeating ? ` · ${t('ics.repeating', { n: p.repeating })}` : '')}</li>
            {#if p.already}<li>{t('ics.already', { n: p.already })}</li>{/if}
            {#if p.past}<li>{t('ics.past', { n: p.past })}</li>{/if}
            {#if p.rules_dropped}<li class="err">{t('ics.rulesDropped', { n: p.rules_dropped })}</li>{/if}
          </ul>
          {#if p.past}
            <label class="past"><input type="checkbox" class="switch" bind:checked={past} /> <span class="small">{t('ics.includePast')}</span></label>
          {/if}
          <div class="row">
            <button class="btn ghost" onclick={() => (file = null)} disabled={importing}>{t('common.cancel')}</button>
            <button class="btn primary" onclick={importFile} disabled={importing || p.total === 0 || (!past && p.total === p.past)}>
              <CalendarPlus size={15} /> {t('ics.confirm', { n: past ? p.total : p.total - p.past })}
            </button>
          </div>
        </div>
      {/if}
      <p class="faint small">{t('ics.note')}</p>
    </section>
  {/if}
</div>

<style>
  .connect {
    display: flex;
    flex-direction: column;
    gap: 22px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  section + section {
    padding-top: 18px;
    border-top: 1px solid var(--border);
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 15px;
  }
  p {
    margin: 0;
  }
  .how {
    margin: 0;
    padding-left: 20px;
    color: var(--muted);
    line-height: 1.7;
  }
  .form {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .form .name {
    flex: 1 1 140px;
  }
  .form .url {
    flex: 3 1 260px;
    font-family: var(--mono);
    font-size: 12.5px;
  }
  .subs {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 4px 0 0;
    padding: 0;
    list-style: none;
  }
  .subs li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex: none;
  }
  .sub-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .strong {
    font-weight: 650;
  }
  .err {
    color: var(--danger);
  }
  .ask {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .preview {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .preview ul {
    margin: 0;
    padding-left: 18px;
  }
  .past {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .spin {
    display: inline-flex;
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
