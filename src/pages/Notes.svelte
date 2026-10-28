<script lang="ts">
  // Obsidian notes: read the user's vault here (read-only). Links between notes work, and the
  // note can be opened in Obsidian. Memos are sent the other way from the Memos page.
  import { open as pickFolder } from '@tauri-apps/plugin-dialog';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { ArrowLeft, ExternalLink, FolderOpen, Gem, RefreshCw, Search, Settings2, Unplug } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import Select from '../components/Select.svelte';
  import { api, type NoteMeta } from '../lib/api';
  import { fmtTs } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { DEFAULT_EXPORT_DIR, EXPORT_DIR_KEY, renderNote, resolveWikiLink, VAULT_KEY, WIKI_PREFIX } from '../lib/obsidian';
  import { toast } from '../lib/state.svelte';

  /** Long lists are cut here; search narrows them. */
  const LIST_LIMIT = 300;

  let vault = $state<string | null>(null);
  let exportDir = $state(DEFAULT_EXPORT_DIR);
  let ready = $state(false);
  let notes = $state<NoteMeta[]>([]);
  let loading = $state(false);
  let folder = $state<string>('');
  let search = $state('');
  let current = $state<NoteMeta | null>(null);
  let html = $state('');
  let tags = $state<string[]>([]);
  /** Notes visited through links, for Back. */
  let history = $state<NoteMeta[]>([]);
  let settingsOpen = $state(false);
  let draftDir = $state('');

  const topFolder = (n: NoteMeta) => n.folder.split('/')[0];
  const folders = $derived([...new Set(notes.map(topFolder))].sort((a, b) => a.localeCompare(b)));
  const folderOptions = $derived([
    { value: '', label: t('ob.allFolders') },
    ...folders.map((f) => ({ value: f || '/', label: f || t('ob.root') })),
  ]);
  const filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return notes.filter(
      (n) => (!folder || (folder === '/' ? n.folder === '' : topFolder(n) === folder)) && (!q || n.path.toLowerCase().includes(q)),
    );
  });
  const shown = $derived(filtered.slice(0, LIST_LIMIT));

  async function init() {
    vault = (await api.getSetting(VAULT_KEY).catch(() => null)) || null;
    exportDir = (await api.getSetting(EXPORT_DIR_KEY).catch(() => null)) || DEFAULT_EXPORT_DIR;
    ready = true;
    if (vault) await refresh();
  }
  init();

  async function refresh() {
    loading = true;
    try {
      notes = await api.obsidianNotes();
    } catch (e) {
      notes = [];
      toast(t('ob.loadFailed', { error: String(e) }), 'error');
    } finally {
      loading = false;
    }
  }

  async function chooseVault() {
    const picked = await pickFolder({ directory: true, multiple: false, title: t('ob.pickVault') }).catch(() => null);
    if (typeof picked !== 'string') return;
    await api.setSetting(VAULT_KEY, picked);
    vault = picked;
    current = null;
    history = [];
    folder = '';
    await refresh();
  }

  async function disconnect() {
    await api.setSetting(VAULT_KEY, '');
    vault = null;
    notes = [];
    current = null;
    history = [];
    settingsOpen = false;
  }

  async function saveSettings() {
    const dir = draftDir.trim().replace(/^\/+|\/+$/g, '') || DEFAULT_EXPORT_DIR;
    if (dir.split('/').some((part) => part === '..' || part === '.')) return toast(t('ob.badFolder'), 'error');
    await api.setSetting(EXPORT_DIR_KEY, dir);
    exportDir = dir;
    settingsOpen = false;
  }

  async function openNote(n: NoteMeta, fromLink = false) {
    try {
      const text = await api.obsidianRead(n.path);
      const rendered = renderNote(text, t('ob.embed'));
      if (fromLink && current) history = [...history, current];
      if (!fromLink) history = [];
      current = n;
      html = rendered.html;
      tags = rendered.tags;
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  function back() {
    const prev = history.at(-1);
    if (!prev) return;
    history = history.slice(0, -1);
    openNote(prev).then(() => (history = history));
  }

  async function openInObsidian() {
    if (!current) return;
    try {
      await openUrl(await api.obsidianOpenUrl(current.path));
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  function readerClick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest('a');
    const href = a?.getAttribute('href');
    if (!href) return;
    e.preventDefault();
    if (href.startsWith(WIKI_PREFIX)) {
      const name = decodeURIComponent(href.slice(WIKI_PREFIX.length));
      const target = resolveWikiLink(name, notes);
      if (target) openNote(target, true);
      else toast(t('ob.linkMissing', { name }), 'info');
    } else if (/^https?:|^mailto:/i.test(href)) {
      openUrl(href);
    }
  }
</script>

{#if !ready}
  <div></div>
{:else if !vault}
  <div class="connect">
    <div class="empty">
      <span class="empty-icon"><Gem size={30} /></span>
      <h2>{t('ob.connectTitle')}</h2>
      <p>{t('ob.connectBody')}</p>
      <button class="btn primary" onclick={chooseVault}><FolderOpen size={17} /> {t('ob.pickVault')}</button>
    </div>
  </div>
{:else}
  <div class="notes">
    <aside class="list-pane">
      <div class="list-head">
        <h1>{t('ob.title')}</h1>
        <div class="head-actions">
          <button class="icon-btn" onclick={refresh} title={t('ob.refresh')} aria-label={t('ob.refresh')} disabled={loading}>
            <span class:spin={loading}><RefreshCw size={16} /></span>
          </button>
          <button class="icon-btn" onclick={() => { draftDir = exportDir; settingsOpen = true; }} title={t('ob.settings')} aria-label={t('ob.settings')}>
            <Settings2 size={16} />
          </button>
        </div>
      </div>
      <p class="vault faint small truncate" title={vault}>{vault} · {t('ob.count', { n: notes.length })}</p>

      <div class="filters">
        <Select bind:value={folder} options={folderOptions} label={t('ob.allFolders')} />
        <div class="search">
          <Search size={16} />
          <input class="input" placeholder={t('ob.search')} bind:value={search} />
        </div>
      </div>

      <div class="list">
        {#each shown as n (n.path)}
          <button class="note-item" class:active={current?.path === n.path} onclick={() => openNote(n)}>
            <span class="n-title truncate">{n.title}</span>
            <span class="n-meta truncate">{n.folder || t('ob.root')} · {fmtTs(n.modified, { month: 'short', day: 'numeric' })}</span>
          </button>
        {:else}
          <p class="muted small empty-list">{loading ? '' : notes.length ? t('common.noMatch') : t('ob.empty')}</p>
        {/each}
        {#if filtered.length > shown.length}
          <p class="faint small empty-list">{t('ob.more', { n: shown.length, total: filtered.length })}</p>
        {/if}
      </div>
    </aside>

    <section class="reader">
      {#if !current}
        <div class="empty">
          <span class="empty-icon"><Gem size={30} /></span>
          <h2>{t('ob.pickNote')}</h2>
          <p>{t('ob.pickNoteBody')}</p>
        </div>
      {:else}
        <div class="toolbar">
          {#if history.length}
            <button class="icon-btn" onclick={back} title={t('ob.back')} aria-label={t('ob.back')}><ArrowLeft size={18} /></button>
          {/if}
          <h1 class="title-view truncate">{current.title}</h1>
          <span class="badge faint small">{t('ob.readOnly')}</span>
          <button class="btn" onclick={openInObsidian}><ExternalLink size={15} /> {t('ob.openInObsidian')}</button>
        </div>
        <p class="status faint small truncate">
          {current.folder || t('ob.root')} · {t('ob.modified', { date: fmtTs(current.modified, { dateStyle: 'medium', timeStyle: 'short' }) })}
          {#each tags as tag (tag)}<span class="tag">#{tag}</span>{/each}
        </p>
        <div class="box">
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="scroll prose reading" onclick={readerClick}>{@html html}</div>
        </div>
      {/if}
    </section>
  </div>
{/if}

{#if settingsOpen && vault}
  <Modal title={t('ob.settings')} onclose={() => (settingsOpen = false)} width={460}>
    <div class="field">
      <span class="label">{t('ob.vaultPath')}</span>
      <div class="vault-row">
        <span class="path small truncate" title={vault}>{vault}</span>
        <button class="btn small" onclick={() => { settingsOpen = false; chooseVault(); }}>{t('ob.change')}</button>
      </div>
    </div>
    <div class="field">
      <label for="ob-export-dir">{t('ob.exportDir')}</label>
      <input id="ob-export-dir" class="input" bind:value={draftDir} placeholder={DEFAULT_EXPORT_DIR} />
      <span class="faint small">{t('ob.exportDirHint')}</span>
    </div>
    {#snippet footer()}
      <button class="btn ghost danger-text" onclick={disconnect}><Unplug size={15} /> {t('ob.disconnect')}</button>
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (settingsOpen = false)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={saveSettings}>{t('common.save')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .connect {
    display: grid;
    place-items: center;
    height: 100%;
    padding: 24px;
  }
  .notes {
    display: grid;
    grid-template-columns: 290px minmax(0, 1fr);
    height: 100%;
  }
  .list-pane {
    display: flex;
    flex-direction: column;
    padding: 28px 14px 14px;
    border-right: 1px solid var(--border);
    min-height: 0;
  }
  .list-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px 4px;
  }
  .head-actions {
    display: flex;
    gap: 2px;
  }
  .vault {
    padding: 0 8px 12px;
    margin: 0;
  }
  .filters {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 10px;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    color: var(--faint);
  }
  .search :global(svg) {
    position: absolute;
    left: 12px;
  }
  .search .input {
    padding-left: 36px;
    border-radius: var(--radius);
  }
  .list {
    flex: 1;
    min-width: 0;
    overflow-x: hidden;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 3px 4px 8px;
  }
  .note-item {
    display: flex;
    flex: none;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    min-width: 0;
    padding: 9px 12px;
    border: 2px solid transparent;
    border-radius: var(--radius);
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    overflow: hidden;
  }
  .note-item > * {
    display: block;
    min-width: 0;
    max-width: 100%;
  }
  .note-item:hover {
    background: var(--surface-2);
  }
  .note-item.active {
    background: var(--surface);
    border-color: var(--primary);
    box-shadow: var(--shadow-sm);
  }
  .n-title {
    font-weight: 650;
  }
  .n-meta {
    font-size: 11.5px;
    color: var(--faint);
  }
  .empty-list {
    padding: 12px;
  }
  .reader {
    display: flex;
    flex-direction: column;
    padding: 24px 32px;
    min-width: 0;
    min-height: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .title-view {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 26px;
    font-weight: 750;
    letter-spacing: -0.02em;
  }
  .badge {
    flex: none;
    padding: 3px 9px;
    border: 1px solid var(--border);
    border-radius: 999px;
  }
  .status {
    margin: 4px 0 14px;
  }
  .tag {
    margin-left: 8px;
    color: var(--primary);
  }
  .box {
    flex: 1;
    display: flex;
    min-height: 0;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
  }
  .scroll {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
  }
  .reading {
    padding: 26px 32px;
    font-size: 15px;
  }
  .reading :global(.ob-link) {
    color: var(--primary);
    text-decoration: none;
    border-bottom: 1px dashed color-mix(in srgb, var(--primary) 50%, transparent);
  }
  .reading :global(.ob-embed) {
    display: inline-block;
    padding: 1px 8px;
    border-radius: 6px;
    background: var(--surface-2);
    color: var(--muted);
    font-size: 12.5px;
  }
  .reading :global(mark) {
    background: color-mix(in srgb, #f5d547 55%, transparent);
    color: inherit;
    border-radius: 3px;
    padding: 0 2px;
  }
  .reading :global(blockquote.ob-callout) {
    margin: 14px 0;
    padding: 10px 14px;
    border: none;
    border-left: 4px solid var(--callout, var(--primary));
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--callout, var(--primary)) 9%, var(--surface));
    color: inherit;
    font-style: normal;
  }
  .reading :global(.ob-callout-title) {
    font-weight: 700;
    margin-bottom: 4px;
    color: var(--callout, var(--primary));
  }
  .reading :global(.ob-callout[data-callout='warning']),
  .reading :global(.ob-callout[data-callout='caution']),
  .reading :global(.ob-callout[data-callout='attention']) {
    --callout: #e08a1e;
  }
  .reading :global(.ob-callout[data-callout='danger']),
  .reading :global(.ob-callout[data-callout='error']),
  .reading :global(.ob-callout[data-callout='bug']),
  .reading :global(.ob-callout[data-callout='failure']) {
    --callout: #e05252;
  }
  .reading :global(.ob-callout[data-callout='tip']),
  .reading :global(.ob-callout[data-callout='success']),
  .reading :global(.ob-callout[data-callout='check']),
  .reading :global(.ob-callout[data-callout='done']) {
    --callout: #2f9e6e;
  }
  .reading :global(.ob-callout[data-callout='question']),
  .reading :global(.ob-callout[data-callout='faq']),
  .reading :global(.ob-callout[data-callout='help']) {
    --callout: #c08a1e;
  }
  .reading :global(.ob-callout[data-callout='quote']),
  .reading :global(.ob-callout[data-callout='cite']) {
    --callout: var(--muted);
  }
  .vault-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .path {
    flex: 1;
    min-width: 0;
    padding: 8px 10px;
    border-radius: var(--radius);
    background: var(--surface-2);
    font-family: var(--mono);
  }
  .danger-text {
    color: var(--danger, #e05252);
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
  @container main (max-width: 900px) {
    .notes {
      grid-template-columns: 230px minmax(0, 1fr);
    }
    .reader {
      padding: 22px 18px;
    }
  }
  @container main (max-width: 620px) {
    .notes {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: 280px minmax(0, 1fr);
    }
    .list-pane {
      padding: 22px 12px 8px;
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
    .toolbar {
      flex-wrap: wrap;
    }
  }
</style>
