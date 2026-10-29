<script lang="ts">
  // A month heading ("2026년 10월") whose year can be clicked and typed, so far-off years are one
  // jump away instead of dozens of arrow presses. ↑/↓ in the box also move a year.
  import { tick } from 'svelte';
  import type { DateStr } from '../lib/api';
  import { addMonths, parseYmd } from '../lib/dates';
  import { intlLocale, t } from '../lib/i18n.svelte';

  let {
    date,
    onchange,
    opts = { year: 'numeric', month: 'long' },
  }: {
    date: DateStr;
    /** Gets the same month and day in the chosen year. */
    onchange: (date: DateStr) => void;
    opts?: Intl.DateTimeFormatOptions;
  } = $props();

  const parts = $derived(new Intl.DateTimeFormat(intlLocale(), opts).formatToParts(parseYmd(date)));
  const year = $derived(Number(date.slice(0, 4)));

  let editing = $state(false);
  let draft = $state('');
  let input = $state<HTMLInputElement>();

  async function edit() {
    draft = String(year);
    editing = true;
    await tick();
    input?.select();
  }
  function go(y: number) {
    if (Number.isInteger(y) && y >= 1900 && y <= 2200 && y !== year) onchange(addMonths(date, (y - year) * 12));
  }
  function commit() {
    if (!editing) return;
    editing = false;
    go(Number(draft.trim()));
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') commit();
    else if (e.key === 'Escape') {
      e.stopPropagation();
      editing = false;
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      const n = (Number(draft) || year) + (e.key === 'ArrowUp' ? 1 : -1);
      draft = String(n);
    }
  }
</script>

{#each parts as part, i (i)}
  {#if part.type === 'year'}
    {#if editing}
      <input
        bind:this={input}
        bind:value={draft}
        class="year-input"
        inputmode="numeric"
        maxlength="4"
        aria-label={t('cal.typeYear')}
        {onkeydown}
        onblur={commit}
      />
    {:else}
      <button class="year" onclick={edit} title={t('cal.typeYear')}>{part.value}</button>
    {/if}
  {:else}{part.value}{/if}
{/each}

<style>
  .year {
    padding: 0 2px;
    margin: 0 -2px;
    border: none;
    border-radius: 4px;
    background: none;
    color: inherit;
    font: inherit;
    cursor: text;
    text-decoration: underline dotted color-mix(in srgb, currentColor 35%, transparent);
    text-underline-offset: 4px;
  }
  .year:hover {
    background: var(--surface-2);
    color: var(--primary);
  }
  .year-input {
    width: 4.2ch;
    padding: 0 2px;
    border: 1.5px solid var(--primary);
    border-radius: 4px;
    background: var(--surface);
    color: inherit;
    font: inherit;
    text-align: center;
    outline: none;
  }
</style>
