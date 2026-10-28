<script lang="ts">
  // Cloze study (retrieval practice): the **bold** parts of a note or memo are hidden. Say the answer
  // first, reveal it, then mark yourself ○ or ✕. Records and review days live in cloze.rs.
  import { onMount, tick } from 'svelte';
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  import { Brain, Eye, EyeOff, Gem, NotebookPen, RotateCcw, Search } from '@lucide/svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import { api, type ClozeSource, type ClozeStat } from '../lib/api';
  import { clozeHtml, hiddenIn, isDue, isWeak, type BlankState, type StudyMode } from '../lib/cloze';
  import { today as todayStr } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { renderNote } from '../lib/obsidian';
  import { toast } from '../lib/state.svelte';

  const today = todayStr();

  let sources = $state<ClozeSource[]>([]);
  let loading = $state(true);
  let search = $state('');
  let current = $state<ClozeSource | null>(null);
  let html = $state('');
  let keys = $state<string[]>([]);
  let stats = $state<Record<string, ClozeStat>>({});
  let mode = $state<StudyMode>('all');
  /** This sitting only; records are saved as each mark is made. */
  const states = new Map<string, BlankState>();
  let tallyVersion = $state(0);
  let doc = $state<HTMLDivElement>();

  const dueTotal = $derived(sources.reduce((n, s) => n + s.due, 0));
  const shownSources = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return sources
      .filter((s) => !q || s.title.toLowerCase().includes(q) || s.place.toLowerCase().includes(q))
      .sort((a, b) => b.due - a.due || b.weak - a.weak || a.title.localeCompare(b.title));
  });
  const inScope = $derived(keys.filter((k) => hiddenIn(mode, stats[k], today)));
  const tally = $derived.by(() => {
    tallyVersion;
    let right = 0;
    let wrong = 0;
    for (const k of inScope) {
      const s = states.get(k);
      if (s === 'right') right++;
      else if (s === 'wrong') wrong++;
    }
    return { right, wrong, left: inScope.length - right - wrong };
  });

  async function loadSources() {
    loading = true;
    try {
      sources = await api.clozeSources();
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      loading = false;
    }
  }
  onMount(loadSources);

  async function open(s: ClozeSource, startMode?: StudyMode) {
    try {
      let rendered: string;
      if (s.source.startsWith('memo:')) {
        const id = Number(s.source.slice(5));
        const memo = (await api.memos()).find((m) => m.id === id);
        if (!memo) throw new Error(t('study.gone'));
        rendered = DOMPurify.sanitize(marked.parse(memo.body, { async: false, gfm: true, breaks: true }));
      } else {
        rendered = renderNote(await api.obsidianRead(s.source.slice(3)), t('ob.embed')).html;
      }
      const list = await api.clozeStats(s.source);
      const cloze = clozeHtml(rendered, s.source);
      stats = Object.fromEntries(list.map((x) => [x.key, x]));
      states.clear();
      keys = cloze.keys;
      html = cloze.html;
      current = s;
      mode = startMode ?? (s.due > 0 ? 'due' : 'all');
      await paint();
      focusNext();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  const blankEl = (key: string) => doc?.querySelector<HTMLButtonElement>(`.blank[data-key="${CSS.escape(key)}"]`);

  /** Applies the session and mode to the blanks in the page (no re-render of the text). */
  async function paint() {
    await tick();
    if (!doc) return;
    doc.querySelectorAll('.judge').forEach((j) => j.remove());
    for (const el of doc.querySelectorAll<HTMLButtonElement>('.blank')) paintOne(el);
    tallyVersion++;
  }

  function paintOne(el: HTMLButtonElement) {
    const key = el.dataset.key!;
    const hidden = hiddenIn(mode, stats[key], today);
    const st = states.get(key) ?? 'hidden';
    el.className = 'blank';
    if (el.nextElementSibling?.classList.contains('judge')) el.nextElementSibling.remove();
    if (!hidden) {
      el.classList.add('given');
      el.tabIndex = -1;
      el.setAttribute('aria-label', el.textContent ?? '');
      return;
    }
    el.tabIndex = 0;
    // 'concealed', not 'hidden': the app has a global .hidden helper.
    el.classList.add(st === 'hidden' ? 'concealed' : st);
    if (isWeak(stats[key])) el.classList.add('weak-mark');
    el.setAttribute('aria-label', st === 'hidden' ? t('study.hiddenLabel') : (el.textContent ?? ''));
    if (st === 'open') {
      const judge = document.createElement('span');
      judge.className = 'judge';
      judge.innerHTML = `<button type="button" class="jo" data-mark="right" aria-label="${t('study.right')}">○</button><button type="button" class="jx" data-mark="wrong" aria-label="${t('study.wrong')}">✕</button>`;
      el.after(judge);
    }
  }

  function setState(key: string, st: BlankState) {
    states.set(key, st);
    const el = blankEl(key);
    if (el) paintOne(el);
    tallyVersion++;
  }

  function reveal(key: string) {
    setState(key, 'open');
    blankEl(key)?.nextElementSibling?.querySelector<HTMLButtonElement>('.jo')?.focus({ preventScroll: true });
  }

  async function mark(key: string, right: boolean) {
    if (!current) return;
    setState(key, right ? 'right' : 'wrong');
    focusNext(key);
    try {
      stats[key] = await api.clozeRecord(current.source, key, right);
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  /** Moves to the next blank still hidden, after `from` (wrapping to the top). */
  function focusNext(from?: string) {
    const scope = inScope;
    const start = from ? scope.indexOf(from) + 1 : 0;
    const order = [...scope.slice(start), ...scope.slice(0, start)];
    const next = order.find((k) => (states.get(k) ?? 'hidden') === 'hidden');
    const el = next ? blankEl(next) : null;
    if (el) {
      el.focus({ preventScroll: true });
      el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
  }

  function onDocClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    const judge = target.closest<HTMLButtonElement>('.judge button');
    if (judge) {
      const key = (judge.parentElement?.previousElementSibling as HTMLElement | null)?.dataset.key;
      if (key) mark(key, judge.dataset.mark === 'right');
      return;
    }
    const blank = target.closest<HTMLButtonElement>('.blank');
    if (blank && !blank.classList.contains('given')) {
      reveal(blank.dataset.key!);
      return;
    }
    if (target.closest('a')) e.preventDefault();
  }

  function onkeydown(e: KeyboardEvent) {
    if (!current || e.metaKey || e.ctrlKey || e.altKey) return;
    const active = document.activeElement as HTMLElement | null;
    const openKey =
      (active?.closest('.judge')?.previousElementSibling as HTMLElement | null)?.dataset.key ??
      (active?.classList.contains('blank') && states.get(active.dataset.key!) === 'open' ? active.dataset.key : undefined);
    if (openKey && (e.key === '1' || e.key.toLowerCase() === 'o')) {
      e.preventDefault();
      mark(openKey, true);
    } else if (openKey && (e.key === '2' || e.key.toLowerCase() === 'x')) {
      e.preventDefault();
      mark(openKey, false);
    }
  }

  async function setMode(m: StudyMode) {
    mode = m;
    await paint();
    focusNext();
  }

  async function hideAll() {
    states.clear();
    await paint();
    focusNext();
  }

  async function showAll() {
    for (const k of inScope) if ((states.get(k) ?? 'hidden') === 'hidden') states.set(k, 'open');
    await paint();
  }

  async function resetRecords() {
    if (!current) return;
    await api.clozeReset(current.source).catch((e) => toast(String(e), 'error'));
    stats = {};
    states.clear();
    mode = 'all';
    await paint();
    loadSources();
  }

  async function finishSource() {
    await loadSources();
    if (current) current = sources.find((s) => s.source === current!.source) ?? current;
  }

  const counts = $derived({
    all: keys.length,
    due: keys.filter((k) => isDue(stats[k], today)).length,
    weak: keys.filter((k) => isWeak(stats[k])).length,
  });
</script>

<svelte:window {onkeydown} />

<div class="study">
  <aside class="list-pane">
    <div class="list-head">
      <h1>{t('study.title')}</h1>
    </div>
    <p class="summary small">
      {#if dueTotal > 0}<span class="due-pill">{t('study.dueToday', { n: dueTotal })}</span>{:else}<span class="faint">{t('study.noneDue')}</span>{/if}
    </p>
    <div class="search">
      <Search size={16} />
      <input class="input" placeholder={t('study.search')} bind:value={search} />
    </div>
    <div class="list">
      {#each shownSources as s (s.source)}
        <button class="src-item" class:active={current?.source === s.source} onclick={() => open(s)}>
          <span class="s-title">
            {#if s.source.startsWith('ob:')}<Gem size={13} />{:else}<NotebookPen size={13} />{/if}
            <span class="truncate">{s.title || t('common.untitled')}</span>
          </span>
          <span class="s-meta">
            <span class="truncate">{s.place || (s.source.startsWith('ob:') ? t('ob.root') : t('memo.ungrouped'))}</span>
            <span class="tags">
              <span class="faint">{t('study.blanks', { n: s.blanks })}</span>
              {#if s.due}<span class="due">{t('study.due', { n: s.due })}</span>{/if}
              {#if s.weak}<span class="weak">{t('study.weak', { n: s.weak })}</span>{/if}
            </span>
          </span>
        </button>
      {:else}
        {#if !loading}<p class="muted small empty-list">{sources.length ? t('common.noMatch') : t('study.emptyList')}</p>{/if}
      {/each}
    </div>
  </aside>

  <section class="reader">
    {#if !current}
      <div class="empty">
        <span class="empty-icon"><Brain size={30} /></span>
        <h2>{t('study.emptyTitle')}</h2>
        <p>{t('study.emptyBody')}</p>
      </div>
    {:else}
      <div class="toolbar">
        <h1 class="title-view truncate">{current.title || t('common.untitled')}</h1>
        <div class="segmented">
          <button class:active={mode === 'all'} onclick={() => setMode('all')}>{t('study.modeAll')} <span class="n">{counts.all}</span></button>
          <button class:active={mode === 'due'} onclick={() => setMode('due')} disabled={!counts.due}>{t('study.modeDue')} <span class="n">{counts.due}</span></button>
          <button class:active={mode === 'weak'} onclick={() => setMode('weak')} disabled={!counts.weak}>{t('study.modeWeak')} <span class="n">{counts.weak}</span></button>
        </div>
      </div>
      <div class="bar">
        <span class="score">
          <span>○ <b class="o">{tally.right}</b></span>
          <span>✕ <b class="x">{tally.wrong}</b></span>
          <span>{t('study.left')} <b>{tally.left}</b></span>
        </span>
        <span class="faint small hint">{t('study.keys')}</span>
        <span class="tools">
          <button class="btn small" onclick={hideAll}><EyeOff size={14} /> {t('study.hideAll')}</button>
          <button class="btn small" onclick={showAll}><Eye size={14} /> {t('study.showAll')}</button>
          <ConfirmButton onconfirm={resetRecords} label={t('study.reset')} question={t('study.resetQ')} />
        </span>
      </div>
      <div class="box">
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="scroll prose reading" bind:this={doc} onclick={onDocClick}>{@html html}</div>
      </div>
      {#if tally.left === 0 && inScope.length}
        <div class="done">
          <span>{t('study.doneLine', { o: tally.right, x: tally.wrong })}</span>
          {#if tally.wrong}<button class="btn small" onclick={() => { finishSource(); setMode('weak'); }}><RotateCcw size={14} /> {t('study.againWeak')}</button>{/if}
          <button class="btn small primary" onclick={finishSource}>{t('study.finish')}</button>
        </div>
      {/if}
    {/if}
  </section>
</div>

<style>
  .study {
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
    padding: 0 8px 4px;
  }
  .summary {
    padding: 0 8px 12px;
    margin: 0;
  }
  .due-pill {
    display: inline-block;
    padding: 2px 10px;
    border-radius: 999px;
    background: var(--primary-soft);
    color: var(--primary);
    font-weight: 700;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    color: var(--faint);
    margin-bottom: 10px;
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
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 3px 4px 8px;
  }
  .src-item {
    display: flex;
    flex: none;
    flex-direction: column;
    gap: 3px;
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
  .src-item:hover {
    background: var(--surface-2);
  }
  .src-item.active {
    background: var(--surface);
    border-color: var(--primary);
    box-shadow: var(--shadow-sm);
  }
  .s-title {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-weight: 650;
  }
  .s-title :global(svg) {
    flex: none;
    color: var(--faint);
  }
  .s-meta {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    min-width: 0;
    font-size: 11.5px;
    color: var(--faint);
  }
  .tags {
    display: flex;
    gap: 6px;
    flex: none;
  }
  .due {
    color: var(--primary);
    font-weight: 700;
  }
  .weak {
    color: var(--danger, #e05252);
    font-weight: 700;
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
    gap: 12px;
  }
  .title-view {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 26px;
    font-weight: 750;
    letter-spacing: -0.02em;
  }
  .segmented .n {
    font-size: 11px;
    opacity: 0.7;
    margin-left: 2px;
  }
  .bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    margin: 10px 0 14px;
  }
  .score {
    display: flex;
    gap: 14px;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .score .o {
    color: var(--success, #2f9e6e);
  }
  .score .x {
    color: var(--danger, #e05252);
  }
  .hint {
    flex: 1;
    min-width: 0;
  }
  .tools {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .box {
    flex: 1;
    display: flex;
    min-height: 0;
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
    font-size: 15.5px;
    line-height: 1.9;
  }
  .reading :global(.blank) {
    display: inline;
    padding: 0 4px;
    margin: 0 1px;
    border: 0;
    border-bottom: 2px solid var(--primary);
    border-radius: 4px 4px 0 0;
    background: var(--surface-2);
    color: transparent;
    font: inherit;
    font-weight: 700;
    line-height: inherit;
    cursor: pointer;
    user-select: none;
    -webkit-box-decoration-break: clone;
    box-decoration-break: clone;
  }
  .reading :global(.blank.concealed:hover) {
    background: color-mix(in srgb, var(--primary) 12%, var(--surface-2));
  }
  .reading :global(.blank.weak-mark.concealed) {
    border-bottom-color: var(--danger, #e05252);
  }
  .reading :global(.blank.open) {
    color: var(--text);
    cursor: default;
  }
  .reading :global(.blank.right) {
    color: var(--text);
    background: color-mix(in srgb, var(--success, #2f9e6e) 16%, transparent);
    border-bottom-color: var(--success, #2f9e6e);
  }
  .reading :global(.blank.wrong) {
    color: var(--text);
    background: color-mix(in srgb, var(--danger, #e05252) 14%, transparent);
    border-bottom-color: var(--danger, #e05252);
  }
  .reading :global(.blank.given) {
    color: var(--text);
    background: transparent;
    border-bottom: 1px dotted var(--muted);
    cursor: default;
    font-weight: 600;
  }
  .reading :global(.blank:focus-visible),
  .reading :global(.judge button:focus-visible) {
    outline: 3px solid var(--primary);
    outline-offset: 2px;
  }
  .reading :global(.judge) {
    display: inline-flex;
    gap: 4px;
    margin: 0 4px;
    vertical-align: middle;
  }
  .reading :global(.judge button) {
    width: 34px;
    height: 28px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--surface);
    font: inherit;
    font-weight: 800;
    line-height: 1;
    cursor: pointer;
  }
  .reading :global(.judge .jo) {
    color: var(--success, #2f9e6e);
    border-color: var(--success, #2f9e6e);
  }
  .reading :global(.judge .jx) {
    color: var(--danger, #e05252);
    border-color: var(--danger, #e05252);
  }
  .done {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
    padding: 12px 16px;
    border-radius: var(--radius);
    background: var(--primary-soft);
    font-weight: 600;
  }
  .done span {
    flex: 1;
  }
  @container main (max-width: 900px) {
    .study {
      grid-template-columns: 230px minmax(0, 1fr);
    }
    .reader {
      padding: 22px 18px;
    }
    .hint {
      display: none;
    }
  }
  @container main (max-width: 620px) {
    .study {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: 260px minmax(0, 1fr);
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
