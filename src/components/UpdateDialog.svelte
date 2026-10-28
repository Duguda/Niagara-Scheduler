<script lang="ts">
  // "A new version is out": shown when the start-up check (or Settings → About) finds an update.
  // Later closes it for now; the sidebar's Settings item keeps a dot until it is installed.
  import { Download, LoaderCircle, Sparkles } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import { t } from '../lib/i18n.svelte';
  import { installUpdate, updater } from '../lib/updater.svelte';
</script>

<Modal title={t('upd.title')} onclose={() => !updater.installing && (updater.prompt = false)} width={460}>
  <div class="body">
    <div class="head">
      <span class="icon"><Sparkles size={20} /></span>
      <div>
        <p class="strong">{t('upd.available', { v: updater.available ?? '' })}</p>
        <p class="muted small">{t('upd.version', { v: __APP_VERSION__ })}</p>
      </div>
    </div>
    {#if updater.notes}
      <div class="notes small">{updater.notes}</div>
    {/if}
    <p class="faint small">{updater.installing ? t('upd.downloading') : t('upd.restartNote')}</p>
  </div>
  {#snippet footer()}
    <span class="spacer"></span>
    <button class="btn ghost" onclick={() => (updater.prompt = false)} disabled={updater.installing}>{t('upd.later')}</button>
    <button class="btn primary" onclick={installUpdate} disabled={updater.installing}>
      {#if updater.installing}<span class="spin"><LoaderCircle size={15} /></span>{:else}<Download size={15} />{/if}
      {t('upd.now')}
    </button>
  {/snippet}
</Modal>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .head p {
    margin: 0;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: 12px;
    background: var(--primary-soft);
    color: var(--primary);
  }
  .strong {
    font-weight: 700;
  }
  .notes {
    max-height: 220px;
    overflow-y: auto;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--surface-2);
    white-space: pre-wrap;
    line-height: 1.6;
  }
  p {
    margin: 0;
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
