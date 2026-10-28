// Obsidian notes, read-only: turn a note's Markdown (with Obsidian's own syntax) into safe HTML.
//
// Handled: front matter (hidden, tags kept), [[wikilinks]] (clickable, open the note here),
// ![[embeds]] (shown as a chip), ==highlights==, and > [!type] callouts.
import { marked } from 'marked';
import DOMPurify from 'dompurify';

export const VAULT_KEY = 'obsidian.vault';
export const EXPORT_DIR_KEY = 'obsidian.exportDir';
export const DEFAULT_EXPORT_DIR = 'Niagara';

/** Links to other notes carry this prefix in their href; the page handles the click. */
export const WIKI_PREFIX = '#wiki:';

export interface ParsedNote {
  /** Tags from the front matter (`tags:` list or inline). */
  tags: string[];
  /** The note without its front matter. */
  body: string;
}

export function splitFrontMatter(text: string): ParsedNote {
  const m = /^---\r?\n([\s\S]*?)\r?\n---[ \t]*(?:\r?\n|$)/.exec(text);
  if (!m) return { tags: [], body: text };
  const tags: string[] = [];
  const lines = m[1].split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const inline = /^tags:\s*\[?(.*?)\]?\s*$/.exec(lines[i]);
    if (!inline) continue;
    if (inline[1].trim()) {
      tags.push(...inline[1].split(',').map((s) => s.trim().replace(/^["']|["']$/g, '')).filter(Boolean));
    } else {
      // Block list: "tags:" followed by "  - tag" lines.
      for (let j = i + 1; j < lines.length && /^\s*-\s+/.test(lines[j]); j++) tags.push(lines[j].replace(/^\s*-\s+/, '').trim());
    }
  }
  return { tags: tags.map((t) => t.replace(/^#/, '')), body: text.slice(m[0].length) };
}

const escapeHtml = (s: string) => s.replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!);

/** Runs `fn` on the text outside fenced and inline code, so code is shown as written. */
function outsideCode(md: string, fn: (s: string) => string): string {
  return md
    .split(/(```[\s\S]*?```|~~~[\s\S]*?~~~|`[^`\n]*`)/)
    .map((part, i) => (i % 2 ? part : fn(part)))
    .join('');
}

function obsidianInline(md: string, embedLabel: string): string {
  return outsideCode(md, (s) =>
    s
      // ![[file]] embeds: images and other notes are not loaded, just named.
      .replace(/!\[\[([^\]\n]+)\]\]/g, (_, target: string) => `<span class="ob-embed">${escapeHtml(embedLabel)} · ${escapeHtml(target.split('|')[0])}</span>`)
      // [[target#heading|label]]
      .replace(/\[\[([^\]\n]+)\]\]/g, (_, inner: string) => {
        const [target, label] = inner.split('|');
        const note = target.split('#')[0].trim();
        const text = (label ?? target).trim();
        return note
          ? `<a class="ob-link" href="${WIKI_PREFIX}${encodeURIComponent(note)}">${escapeHtml(text)}</a>`
          : `<span class="ob-link">${escapeHtml(text)}</span>`;
      })
      .replace(/==([^=\n]+)==/g, '<mark>$1</mark>'),
  );
}

const CALLOUT = /^\[!([\w-]+)\]([+-]?)\s*(.*)$/;

/** `> [!tip] Title` blockquotes become callout boxes. Runs on the sanitized HTML. */
function callouts(html: string): string {
  const tpl = document.createElement('template');
  tpl.innerHTML = html;
  for (const quote of tpl.content.querySelectorAll('blockquote')) {
    const first = quote.querySelector(':scope > p');
    if (!first) continue;
    const firstLine = first.innerHTML.split(/<br\s*\/?>|\n/)[0];
    const m = CALLOUT.exec(firstLine.trim());
    if (!m) continue;
    const type = m[1].toLowerCase();
    quote.classList.add('ob-callout');
    quote.dataset.callout = type;
    const title = document.createElement('div');
    title.className = 'ob-callout-title';
    title.innerHTML = m[3] || type.charAt(0).toUpperCase() + type.slice(1);
    // Drop the "[!type] title" line from the first paragraph.
    first.innerHTML = first.innerHTML.slice(firstLine.length).replace(/^\s*(<br\s*\/?>)?\s*/, '');
    if (!first.innerHTML.trim()) first.remove();
    quote.prepend(title);
  }
  return tpl.innerHTML;
}

export function renderNote(text: string, embedLabel: string): { html: string; tags: string[] } {
  const { tags, body } = splitFrontMatter(text);
  const raw = marked.parse(obsidianInline(body, embedLabel), { async: false, gfm: true, breaks: true });
  const clean = DOMPurify.sanitize(raw, { ADD_ATTR: ['target'] });
  return { html: callouts(clean), tags };
}

/** Finds the note a wikilink points to: by path, else by title (Obsidian's shortest-path links). */
export function resolveWikiLink<T extends { path: string; title: string }>(target: string, notes: T[]): T | undefined {
  const want = target.replace(/\.md$/i, '').toLowerCase();
  return (
    notes.find((n) => n.path.replace(/\.md$/i, '').toLowerCase() === want) ??
    notes.find((n) => n.title.toLowerCase() === want.split('/').pop())
  );
}
