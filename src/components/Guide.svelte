<script lang="ts">
  // First-run guide in five short steps. Everything here can be changed later in Settings,
  // and the guide itself can be reopened from Settings → About.
  import { Brain, Check, FolderOpen, FileText, FolderTree, Languages, LayoutGrid } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import PageToggles from './PageToggles.svelte';
  import { api } from '../lib/api';
  import { closeGuide } from '../lib/guide.svelte';
  import { i18n, setLocale, t, type Locale } from '../lib/i18n.svelte';
  import { folderName, READ_DIRS_KEY, readDirsFrom } from '../lib/obsidian';
  import { ui } from '../lib/state.svelte';

  const STEPS = [
    { icon: Languages, title: 'guide.s1Title' },
    { icon: LayoutGrid, title: 'guide.s2Title' },
    { icon: FileText, title: 'guide.s3Title' },
    { icon: Brain, title: 'guide.s4Title' },
    { icon: FolderTree, title: 'guide.s5Title' },
  ] as const;
  const LANGUAGES: { id: Locale; name: string }[] = [
    { id: 'ko', name: '한국어' },
    { id: 'en', name: 'English' },
    { id: 'ja', name: '日本語' },
  ];

  let step = $state(0);
  let dirs = $state<string[]>([]);
  const last = $derived(step === STEPS.length - 1);
  const Icon = $derived(STEPS[step].icon);

  api.getSetting(READ_DIRS_KEY).then((v) => (dirs = readDirsFrom(v))).catch(() => {});

  async function addFolder() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const picked = await open({ directory: true, multiple: false, title: t('ob.pickVault') });
      if (typeof picked !== 'string' || dirs.includes(picked)) return;
      dirs = [...dirs, picked];
      await api.setSetting(READ_DIRS_KEY, JSON.stringify(dirs));
    } catch {
      // No folder dialog (browser preview): folders can still be added on the Notes page.
    }
  }

  function finish(goTo?: 'study' | 'notes') {
    closeGuide();
    if (goTo) ui.page = goTo;
  }
</script>

<Modal title={t('guide.title')} onclose={() => finish()} width={600}>
  <div class="guide">
    <ol class="dots" aria-label={t('guide.progress', { n: step + 1, total: STEPS.length })}>
      {#each STEPS as s, i (s.title)}
        <li class:done={i < step} class:now={i === step}>
          <button onclick={() => (step = i)} aria-label={t(s.title)} aria-current={i === step ? 'step' : undefined}></button>
        </li>
      {/each}
    </ol>

    <div class="head">
      <span class="icon"><Icon size={22} /></span>
      <h2>{t(STEPS[step].title)}</h2>
    </div>

    {#if step === 0}
      <p>{t('guide.s1Body')}</p>
      <div class="langs">
        {#each LANGUAGES as lang (lang.id)}
          <button class="btn" class:primary={i18n.locale === lang.id} onclick={() => setLocale(lang.id)}>
            {#if i18n.locale === lang.id}<Check size={15} />{/if} {lang.name}
          </button>
        {/each}
      </div>
    {:else if step === 1}
      <p>{t('guide.s2Body')}</p>
      <PageToggles />
    {:else if step === 2}
      <p>{t('guide.s3Body')}</p>
      <div class="vault">
        <span class="path small truncate" class:unset={!dirs.length} title={dirs.join('\n')}>{dirs.map(folderName).join(' · ') || t('store.notSet')}</span>
        <button class="btn" onclick={addFolder}><FolderOpen size={15} /> {dirs.length ? t('ob.addReadDir') : t('ob.pickVault')}</button>
      </div>
      <p class="faint small">{t('guide.s3Later')}</p>
    {:else if step === 3}
      <p>{t('guide.s4Body')}</p>
      <pre class="sample"><code>{t('guide.s4Sample')}</code></pre>
      <ul class="keys small">
        <li><kbd>Space</kbd> {t('guide.keyReveal')}</li>
        <li><kbd>1</kbd> ○ {t('study.right')}</li>
        <li><kbd>2</kbd> ✕ {t('study.wrong')}</li>
      </ul>
      <p class="faint small">{t('guide.s4Schedule')}</p>
    {:else}
      <p>{t('guide.s5Body')}</p>
      <ul class="where small">
        <li><b>data\</b> {t('store.dataHint')}</li>
        <li><b>brief\</b> {t('brief.body')}</li>
      </ul>
      <p class="faint small">{t('guide.s5Again')}</p>
    {/if}
  </div>

  {#snippet footer()}
    <button class="btn ghost" onclick={() => finish()}>{t('guide.skip')}</button>
    <span class="spacer"></span>
    {#if step > 0}<button class="btn" onclick={() => step--}>{t('guide.back')}</button>{/if}
    {#if last}
      <button class="btn primary" onclick={() => finish()}><Check size={15} /> {t('guide.start')}</button>
    {:else}
      <button class="btn primary" onclick={() => step++}>{t('guide.next')}</button>
    {/if}
  {/snippet}
</Modal>

<style>
  .guide {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 300px;
  }
  .guide p {
    margin: 0;
    line-height: 1.65;
  }
  .dots {
    display: flex;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .dots li {
    flex: 1;
  }
  .dots button {
    display: block;
    width: 100%;
    height: 5px;
    border: 0;
    border-radius: 999px;
    background: var(--border);
    cursor: pointer;
  }
  .dots li.done button,
  .dots li.now button {
    background: var(--primary);
  }
  .dots li.done button {
    opacity: 0.45;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .head h2 {
    margin: 0;
    font-size: 19px;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 12px;
    background: var(--primary-soft);
    color: var(--primary);
    flex: none;
  }
  .langs {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .vault {
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
  .path.unset {
    font-family: inherit;
    color: var(--faint);
  }
  .sample {
    margin: 0;
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--surface-2);
    font-family: var(--mono);
    font-size: 13px;
    white-space: pre-wrap;
    overflow-x: auto;
  }
  .keys,
  .where {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  kbd {
    display: inline-block;
    min-width: 22px;
    margin-right: 6px;
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 5px;
    background: var(--surface);
    font-family: var(--mono);
    font-size: 12px;
    text-align: center;
  }
</style>
