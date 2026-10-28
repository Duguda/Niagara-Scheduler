<script lang="ts">
  // Notes: the .md and .txt files in the user's note folders (an Obsidian vault is one of them).
  // Read them with links between notes working, edit them, create new ones, and switch a note
  // between plain text (.txt) and Markdown (.md). Memos are a separate thing kept in Niagara.
  import { open as pickFolder } from '@tauri-apps/plugin-dialog';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { ArrowLeft, Check, FileText, FolderOpen, NotebookText, Pencil, Plus, RefreshCw, Search, Settings2 } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import Select from '../components/Select.svelte';
  import { api, type NoteMeta } from '../lib/api';
  import { fmtTs } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import {
    folderName,
    isPlainText,
    MEMO_DIR_KEY,
    noteFolder,
    noteLinkTarget,
    READ_DIRS_KEY,
    readDirsFrom,
    renderNote,
    renderPlain,
    resolveWikiLink,
    WIKI_PREFIX,
  } from '../lib/obsidian';
  import { toast } from '../lib/state.svelte';

  /** Long lists are cut here; search narrows them. */
  const LIST_LIMIT = 300;

  let dirs = $state<string[]>([]);
  let memoDir = $state('');
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

  /** The note's text as loaded, and the editor's text while editing. */
  let text = $state('');
  let draft = $state('');
  let editing = $state(false);
  let saving = $state(false);
  /** The file's modified time when it was loaded, so edits made elsewhere are not overwritten. */
  let loadedAt = $state<number | null>(null);
  const dirty = $derived(editing && draft !== text);

  let newOpen = $state(false);
  let newRoot = $state('');
  let newSub = $state('');
  let newTitle = $state('');

  const topFolder = (n: NoteMeta) => noteFolder(n).split('/')[0];
  const noteKey = (n: NoteMeta) => `${n.root}|${n.path}`;
  const folders = $derived([...new Set(notes.map(topFolder))].sort((a, b) => a.localeCompare(b)));
  const folderOptions = $derived([
    { value: '', label: t('ob.allFolders') },
    ...folders.map((f) => ({ value: f || '/', label: f || t('ob.root') })),
  ]);
  const dirOptions = $derived(dirs.map((d) => ({ value: d, label: folderName(d) })));
  const filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return notes.filter(
      (n) => (!folder || (folder === '/' ? noteFolder(n) === '' : topFolder(n) === folder)) && (!q || n.path.toLowerCase().includes(q)),
    );
  });
  const shown = $derived(filtered.slice(0, LIST_LIMIT));

  async function init() {
    dirs = readDirsFrom(await api.getSetting(READ_DIRS_KEY).catch(() => null));
    memoDir = (await api.getSetting(MEMO_DIR_KEY).catch(() => null)) ?? '';
    ready = true;
    if (dirs.length) await refresh();
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

  async function saveDirs(next: string[]) {
    dirs = next;
    await api.setSetting(READ_DIRS_KEY, JSON.stringify(next));
    if (current && !next.includes(current.root)) closeNote();
    if (next.length) await refresh();
    else notes = [];
  }

  async function addDir() {
    const picked = await pickFolder({ directory: true, multiple: false, title: t('ob.pickVault') }).catch(() => null);
    if (typeof picked !== 'string' || dirs.includes(picked)) return;
    await saveDirs([...dirs, picked]);
  }

  async function chooseMemoDir() {
    const picked = await pickFolder({ directory: true, multiple: false, title: t('ob.memoDir') }).catch(() => null);
    if (typeof picked !== 'string') return;
    memoDir = picked;
    await api.setSetting(MEMO_DIR_KEY, picked);
  }

  function closeNote() {
    current = null;
    history = [];
    editing = false;
  }

  function show(n: NoteMeta, body: string) {
    text = body;
    if (isPlainText(n.path)) {
      html = renderPlain(body);
      tags = [];
    } else {
      const rendered = renderNote(body, t('ob.embed'));
      html = rendered.html;
      tags = rendered.tags;
    }
  }

  /** `list`: picked in the list (history restarts) · `link`: followed a link · `back`: Back button. */
  async function openNote(n: NoteMeta, how: 'list' | 'link' | 'back' = 'list') {
    if (!(await saveIfDirty())) return;
    try {
      const body = await api.obsidianRead(n.path, n.root);
      if (how === 'link' && current) history = [...history, current];
      if (how === 'list') history = [];
      current = n;
      loadedAt = n.modified;
      editing = false;
      show(n, body);
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  function back() {
    const prev = history.at(-1);
    if (!prev) return;
    history = history.slice(0, -1);
    openNote(prev, 'back');
  }

  function startEdit() {
    draft = text;
    editing = true;
  }

  /** Saves the editor's text. Returns false when it could not be saved (so nothing is lost). */
  async function save(): Promise<boolean> {
    if (!current || saving) return false;
    saving = true;
    try {
      const modified = await api.noteWrite(current.root, current.path, draft, loadedAt);
      loadedAt = modified;
      const updated = { ...current, modified };
      notes = notes.map((n) => (noteKey(n) === noteKey(updated) ? updated : n));
      current = updated;
      show(updated, draft);
      return true;
    } catch (e) {
      toast(String(e) === 'changed' ? t('ob.changedOutside') : String(e), 'error');
      return false;
    } finally {
      saving = false;
    }
  }

  async function saveIfDirty(): Promise<boolean> {
    return !dirty || (await save());
  }

  async function finishEdit() {
    if (dirty && !(await save())) return;
    editing = false;
  }

  async function reload() {
    if (!current) return;
    editing = false;
    await refresh();
    const fresh = notes.find((n) => current && noteKey(n) === noteKey(current));
    if (fresh) await openNote(fresh, 'back');
  }

  function editorKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      save().then((ok) => ok && toast(t('ob.saved'), 'success'));
    }
  }

  /** .txt ↔ .md: only the file's extension changes. */
  async function convert() {
    if (!current || !(await saveIfDirty())) return;
    try {
      const path = await api.noteConvert(current.root, current.path);
      const name = path.split('/').pop() ?? path;
      toast(t('ob.converted', { name }), 'success');
      const old = noteKey(current);
      const moved: NoteMeta = { ...current, path };
      notes = notes.map((n) => (noteKey(n) === old ? moved : n));
      current = moved;
      show(moved, editing ? draft : text);
    } catch (e) {
      const ext = isPlainText(current.path) ? '.md' : '.txt';
      const name = (current.path.split('/').pop() ?? '').replace(/\.[^.]+$/, ext);
      toast(String(e) === 'exists' ? t('ob.convertExists', { name }) : String(e), 'error');
    }
  }

  async function showInFolder() {
    if (!current) return;
    try {
      const { revealItemInDir } = await import('@tauri-apps/plugin-opener');
      await revealItemInDir(await api.noteFilePath(current.path, current.root));
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  function openNew() {
    newRoot = current?.root ?? dirs[0] ?? '';
    newSub = current?.folder ?? '';
    newTitle = '';
    newOpen = true;
  }

  async function createNote() {
    if (!newRoot || !(await saveIfDirty())) return;
    try {
      const path = await api.noteCreate(newRoot, newSub, newTitle.trim() || t('common.untitled'));
      newOpen = false;
      await refresh();
      const made = notes.find((n) => n.root === newRoot && n.path === path);
      if (made) {
        await openNote(made);
        startEdit();
      }
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  function readerClick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest('a');
    const href = a?.getAttribute('href');
    if (!href) return;
    e.preventDefault();
    if (/^https?:|^mailto:/i.test(href)) {
      openUrl(href);
      return;
    }
    let name: string | null = null;
    if (href.startsWith(WIKI_PREFIX)) name = decodeURIComponent(href.slice(WIKI_PREFIX.length));
    else if (/\.md(#.*)?$/i.test(href) && !/^[a-z]+:/i.test(href)) name = noteLinkTarget(href, current?.folder ?? '');
    if (name === null) return;
    const target = resolveWikiLink(name, notes);
    if (target) openNote(target, 'link');
    else toast(t('ob.linkMissing', { name }), 'info');
  }
</script>

{#if !ready}
  <div></div>
{:else if !dirs.length}
  <div class="connect">
    <div class="empty">
      <span class="empty-icon"><FileText size={30} /></span>
      <h2>{t('ob.connectTitle')}</h2>
      <p>{t('ob.connectBody')}</p>
      <button class="btn primary" onclick={addDir}><FolderOpen size={17} /> {t('ob.pickVault')}</button>
    </div>
  </div>
{:else}
  <div class="notes">
    <aside class="list-pane">
      <div class="list-head">
        <h1>{t('ob.title')}</h1>
        <div class="head-actions">
          <button class="icon-btn" onclick={openNew} title={t('ob.new')} aria-label={t('ob.new')}><Plus size={17} /></button>
          <button class="icon-btn" onclick={refresh} title={t('ob.refresh')} aria-label={t('ob.refresh')} disabled={loading}>
            <span class:spin={loading}><RefreshCw size={16} /></span>
          </button>
          <button class="icon-btn" onclick={() => (settingsOpen = true)} title={t('ob.settings')} aria-label={t('ob.settings')}>
            <Settings2 size={16} />
          </button>
        </div>
      </div>
      <p class="vault faint small truncate" title={dirs.join('\n')}>{dirs.map(folderName).join(' · ')} · {t('ob.count', { n: notes.length })}</p>

      <div class="filters">
        <Select bind:value={folder} options={folderOptions} label={t('ob.allFolders')} />
        <div class="search">
          <Search size={16} />
          <input class="input" placeholder={t('ob.search')} bind:value={search} />
        </div>
      </div>

      <div class="list">
        {#each shown as n (noteKey(n))}
          <button class="note-item" class:active={current !== null && noteKey(current) === noteKey(n)} onclick={() => openNote(n)}>
            <span class="n-title truncate">{n.title}{#if isPlainText(n.path)}<span class="ext">.txt</span>{/if}</span>
            <span class="n-meta truncate">{noteFolder(n) || t('ob.root')} · {fmtTs(n.modified, { month: 'short', day: 'numeric' })}</span>
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
          <span class="empty-icon"><NotebookText size={30} /></span>
          <h2>{t('ob.pickNote')}</h2>
          <p>{t('ob.pickNoteBody')}</p>
        </div>
      {:else}
        <div class="toolbar">
          {#if history.length}
            <button class="icon-btn" onclick={back} title={t('ob.back')} aria-label={t('ob.back')}><ArrowLeft size={18} /></button>
          {/if}
          <h1 class="title-view truncate">{current.title}</h1>
          {#if isPlainText(current.path)}<span class="badge faint small">{t('ob.plain')}</span>{/if}
          <button
            class="btn ghost small"
            onclick={convert}
            title={isPlainText(current.path) ? t('ob.toMarkdownHint') : t('ob.toTextHint')}
          >
            <FileText size={15} /> {isPlainText(current.path) ? t('ob.toMarkdown') : t('ob.toText')}
          </button>
          <button class="icon-btn" onclick={showInFolder} title={t('ob.showInFolder')} aria-label={t('ob.showInFolder')}><FolderOpen size={17} /></button>
          {#if editing}
            <button class="btn primary" onclick={finishEdit} disabled={saving}><Check size={15} /> {t('ob.view')}</button>
          {:else}
            <button class="btn" onclick={startEdit}><Pencil size={15} /> {t('ob.edit')}</button>
          {/if}
        </div>
        <p class="status faint small truncate">
          {noteFolder(current) || t('ob.root')} · {t('ob.modified', { date: fmtTs(current.modified, { dateStyle: 'medium', timeStyle: 'short' }) })}
          {#if editing}<span class="hint">· {t('ob.editHint')}</span>{/if}
          {#each tags as tag (tag)}<span class="tag">#{tag}</span>{/each}
          <button class="link-btn small" onclick={reload}>{t('ob.reload')}</button>
        </p>
        <div class="box">
          {#if editing}
            <textarea class="editor" bind:value={draft} onkeydown={editorKey} spellcheck="false" aria-label={current.title}></textarea>
          {:else}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="scroll prose reading" onclick={readerClick}>{@html html}</div>
          {/if}
        </div>
      {/if}
    </section>
  </div>
{/if}

{#if settingsOpen}
  <Modal title={t('ob.settings')} onclose={() => (settingsOpen = false)} width={480}>
    <div class="field">
      <span class="label">{t('ob.readDirs')}</span>
      <span class="faint small">{t('ob.readDirsHint')}</span>
      {#each dirs as dir, i (dir)}
        <div class="vault-row">
          <span class="path small truncate" title={dir}>{dir}</span>
          <button class="btn small ghost" onclick={() => saveDirs(dirs.filter((_, j) => j !== i))}>{t('ob.removeReadDir')}</button>
        </div>
      {/each}
      <button class="btn small add-dir" onclick={addDir}><FolderOpen size={14} /> {t('ob.addReadDir')}</button>
    </div>
    <div class="field">
      <span class="label">{t('ob.memoDir')}</span>
      <span class="faint small">{t('ob.memoDirHint')}</span>
      <div class="vault-row">
        <span class="path small truncate" class:unset={!memoDir} title={memoDir}>{memoDir || t('ob.memoDirNone')}</span>
        <button class="btn small" onclick={chooseMemoDir}><Pencil size={14} /> {t('ob.pickMemoDir')}</button>
      </div>
    </div>
    {#snippet footer()}
      <span class="spacer"></span>
      <button class="btn primary" onclick={() => (settingsOpen = false)}>{t('common.close')}</button>
    {/snippet}
  </Modal>
{/if}

{#if newOpen}
  <Modal title={t('ob.new')} onclose={() => (newOpen = false)} width={420}>
    <div class="field">
      <span class="label">{t('ob.newFolder')}</span>
      <Select bind:value={newRoot} options={dirOptions} label={t('ob.newFolder')} />
    </div>
    <div class="field">
      <label for="note-sub">{t('ob.newSub')}</label>
      <input id="note-sub" class="input" bind:value={newSub} placeholder="10 Study" />
    </div>
    <div class="field">
      <label for="note-title">{t('ob.newTitle')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="note-title" class="input" bind:value={newTitle} autofocus onkeydown={(e) => e.key === 'Enter' && createNote()} />
    </div>
    {#snippet footer()}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (newOpen = false)}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={createNote}><Plus size={15} /> {t('ob.create')}</button>
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
  .add-dir {
    align-self: flex-start;
  }
  .ext {
    margin-left: 4px;
    font-weight: 500;
    font-size: 11px;
    color: var(--faint);
  }
  .hint {
    margin-left: 6px;
  }
  .link-btn {
    margin-left: 10px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--primary);
    cursor: pointer;
  }
  .editor {
    flex: 1;
    min-width: 0;
    padding: 22px 28px;
    border: 0;
    outline: none;
    resize: none;
    background: transparent;
    color: inherit;
    font-family: var(--mono);
    font-size: 14px;
    line-height: 1.7;
    tab-size: 2;
  }
  .reading :global(.plain-note) {
    margin: 0;
    font-family: inherit;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .path.unset {
    font-family: inherit;
    color: var(--faint);
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
