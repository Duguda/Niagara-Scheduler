import { mount } from 'svelte';
import 'pretendard/dist/web/variable/pretendardvariable-dynamic-subset.css';
import './app.css';
import App from './App.svelte';
import PanelApp from './PanelApp.svelte';
import { ui, type Page } from './lib/state.svelte';

async function start() {
  // Browser preview with sample data while developing the UI outside Tauri.
  // `VITE_PREVIEW=1 vite build` makes the same sample-data build as a static page (alpha testing).
  if ((import.meta.env.DEV || import.meta.env.VITE_PREVIEW) && !('__TAURI_INTERNALS__' in window)) {
    (await import('./lib/mock')).installMock();
    const params = new URLSearchParams(location.search);
    const page = params.get('page');
    if (page) ui.page = page as Page;
    const theme = params.get('theme');
    const accent = params.get('accent');
    try {
      if (theme) localStorage.setItem('nora.previewTheme', theme);
      if (accent) localStorage.setItem('nora.previewAccent', accent);
    } catch {
      // Storage can be unavailable in a hosted preview; the theme then follows the system.
    }
  }
  // The overlay panel window loads this same page; it shows the compact panel UI instead.
  const isPanel = '__TAURI_INTERNALS__' in window
    ? (await import('@tauri-apps/api/window')).getCurrentWindow().label === 'panel'
    : new URLSearchParams(location.search).has('panel');
  mount(isPanel ? PanelApp : App, { target: document.getElementById('app')! });
}

start();
