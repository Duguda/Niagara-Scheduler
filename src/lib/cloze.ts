// Cloze study: turn the **bold** parts of a rendered note into blanks to recall, reveal and mark ○/✕.
// Records live in the database (src-tauri/src/cloze.rs); this file only shapes the page.
import type { ClozeStat, DateStr } from './api';

/** Session state of one blank: still hidden, revealed and waiting for a mark, or marked. */
export type BlankState = 'hidden' | 'open' | 'right' | 'wrong';

/** Which blanks to hide: all of them, only those due for review today, or only weak ones. */
export type StudyMode = 'all' | 'due' | 'weak';

export const isWeak = (s: ClozeStat | undefined) => !!s && (!s.last_right || s.wrong > s.right);
export const isDue = (s: ClozeStat | undefined, today: DateStr) => !!s && s.due <= today;

export function hiddenIn(mode: StudyMode, s: ClozeStat | undefined, today: DateStr): boolean {
  if (mode === 'due') return isDue(s, today);
  if (mode === 'weak') return isWeak(s);
  return true;
}

/**
 * Replaces each <strong> in the (already sanitized) HTML with a blank button.
 * A blank's key is `<source>|<answer>|<n>`, n counting earlier blanks with the same answer.
 */
export function clozeHtml(html: string, source: string): { html: string; keys: string[] } {
  const tpl = document.createElement('template');
  tpl.innerHTML = html;
  const seen = new Map<string, number>();
  const keys: string[] = [];
  for (const strong of tpl.content.querySelectorAll('strong')) {
    const answer = (strong.textContent ?? '').trim();
    if (!answer) continue;
    const n = seen.get(answer) ?? 0;
    seen.set(answer, n + 1);
    const key = `${source}|${answer}|${n}`;
    keys.push(key);
    const blank = document.createElement('button');
    blank.type = 'button';
    blank.className = 'blank';
    blank.dataset.key = key;
    blank.textContent = answer;
    strong.replaceWith(blank);
  }
  return { html: tpl.innerHTML, keys };
}
