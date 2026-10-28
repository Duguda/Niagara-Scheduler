<script lang="ts">
  // Sidebar pages as icon tiles: press one to show or hide it. With `reorder`, drag a tile onto
  // another to move it there (or Alt+←/→ on a focused tile). Used in Settings and the first-run guide.
  import { tick } from 'svelte';
  import { Check } from '@lucide/svelte';
  import { t } from '../lib/i18n.svelte';
  import { movePage, orderedPages, pagePrefs, setPageVisible, visiblePages } from '../lib/pages.svelte';

  let { reorder = false }: { reorder?: boolean } = $props();
  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);
  let grid = $state<HTMLDivElement>();

  function drop(to: number) {
    if (dragFrom !== null) movePage(dragFrom, to);
    dragFrom = dragOver = null;
  }

  async function keyMove(e: KeyboardEvent, i: number) {
    if (!reorder || !e.altKey || (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight')) return;
    e.preventDefault();
    const to = i + (e.key === 'ArrowLeft' ? -1 : 1);
    movePage(i, to);
    await tick();
    grid?.querySelectorAll<HTMLButtonElement>('.tile')[Math.max(0, Math.min(to, pagePrefs.order.length - 1))]?.focus();
  }
</script>

<div class="tiles" role="group" aria-label={t('set.pages')} bind:this={grid}>
  {#each orderedPages() as page, i (page.id)}
    {@const shown = !pagePrefs.hidden.includes(page.id)}
    {@const last = shown && visiblePages().length <= 1}
    <button
      class="tile"
      class:on={shown}
      class:dragging={dragFrom === i}
      class:drop-before={dragOver === i && dragFrom !== null && dragFrom > i}
      class:drop-after={dragOver === i && dragFrom !== null && dragFrom < i}
      draggable={reorder}
      ondragstart={(e) => { dragFrom = i; e.dataTransfer?.setData('text/plain', page.id); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
      ondragover={(e) => { if (dragFrom === null) return; e.preventDefault(); dragOver = i; }}
      ondragleave={() => { if (dragOver === i) dragOver = null; }}
      ondrop={(e) => { e.preventDefault(); drop(i); }}
      ondragend={() => (dragFrom = dragOver = null)}
      onkeydown={(e) => keyMove(e, i)}
      aria-pressed={shown}
      disabled={last}
      title={last ? t('set.pageLast') : shown ? t('set.pageHideHint') : t('set.pageShowHint')}
      onclick={() => setPageVisible(page.id, !shown)}
    >
      <span class="mark">{#if shown}<Check size={11} strokeWidth={3} />{/if}</span>
      <page.icon size={20} />
      <span class="name">{t(page.label)}</span>
    </button>
  {/each}
</div>

<style>
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(92px, 1fr));
    gap: 8px;
  }
  .tile {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 12px 6px 10px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--faint);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s, color 0.15s;
  }
  .tile:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--primary) 45%, var(--border));
    color: var(--text);
  }
  .tile.on {
    border-color: var(--primary);
    background: var(--primary-soft);
    color: var(--primary);
  }
  .tile[draggable='true'] {
    cursor: grab;
  }
  .tile.dragging {
    opacity: 0.4;
  }
  .tile.drop-before {
    box-shadow: -4px 0 0 0 var(--primary);
  }
  .tile.drop-after {
    box-shadow: 4px 0 0 0 var(--primary);
  }
  .tile:disabled {
    cursor: not-allowed;
  }
  .tile:focus-visible {
    outline: 3px solid var(--primary);
    outline-offset: 2px;
  }
  .name {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mark {
    position: absolute;
    top: 6px;
    right: 6px;
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid var(--border);
    color: var(--primary-text);
  }
  .tile.on .mark {
    border-color: var(--primary);
    background: var(--primary);
  }
</style>
