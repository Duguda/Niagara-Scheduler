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

// ---- n-reading (n회독) and blocks -----------------------------------------------------------
//
// Besides the bold blanks, every word of the text can be a blank. Each reading hides one of
// GROUPS groups of words, so after GROUPS readings every word has been hidden once; the next
// cycle shuffles the groups again. These marks are not recorded: only the reading number is kept.

export const GROUPS = 4;

/** Words that carry no content to recall. */
const STOP = new Set(
  '즉 또한 반면 그러나 결국 따라서 그리고 여러분 여러분과 우리는 우리 우리가 우리에게 이는 이 그 저는 먼저 다음은 마지막으로 그렇다면 특히 지금도 동시에 쉽게 말해 같은 위해 통해 대한 모두 가장 있는 하는 되는 없는 있고 하고 등 것 수 더 때 중 한 또 바로 다시 이런 그런 어떤 것은 것이 the a an of to and or in on is are'.split(
    ' ',
  ),
);

export function isWordCandidate(core: string): boolean {
  if (!core || core.startsWith('#')) return false;
  if (/\d/.test(core)) return true;
  if (core.replace(/[^가-힣A-Za-z]/g, '').length < 2) return false;
  if (STOP.has(core.toLowerCase())) return false;
  return !/(니다|습니다)$/.test(core);
}

/** FNV-1a, so a word's group stays the same for a given cycle. */
function hash(s: string): number {
  let x = 2166136261;
  for (const c of s) {
    x ^= c.codePointAt(0)!;
    x = Math.imul(x, 16777619);
  }
  return x >>> 0;
}

/** Whether a word blank is hidden in reading `round` (1-based). */
export function hiddenInRound(id: string, round: number): boolean {
  const cycle = Math.floor((round - 1) / GROUPS);
  return hash(`${id}#${cycle}`) % GROUPS === (round - 1) % GROUPS;
}

const SKIP_WORDS = 'code, pre, a, h1, h2, h3, h4, h5, h6, button, .ob-callout-title, .ob-embed';

/**
 * Wraps each content word of the text (outside bold, code, links and headings) in a word blank
 * `<button class="blank word" data-key="w<n>">`. Ids follow the reading order.
 */
function wrapWords(root: DocumentFragment | Element): number {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  const texts: Text[] = [];
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    const parent = (n as Text).parentElement;
    if (!parent || parent.closest(SKIP_WORDS) || parent.closest('strong')) continue;
    if ((n as Text).data.trim()) texts.push(n as Text);
  }
  let id = 0;
  for (const text of texts) {
    const frag = document.createDocumentFragment();
    for (const piece of text.data.split(/(\s+)/)) {
      if (!piece) continue;
      const m = /^([("'“‘「『[]*)(.*?)([.,)"'”’!?」』:;\]]*)$/.exec(piece);
      const [pre, core, post] = m ? [m[1], m[2], m[3]] : ['', piece, ''];
      if (/^\s+$/.test(piece) || !isWordCandidate(core)) {
        frag.append(piece);
        continue;
      }
      if (pre) frag.append(pre);
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'blank word';
      b.dataset.key = `w${id++}`;
      b.textContent = core;
      frag.append(b);
      if (post) frag.append(post);
    }
    text.replaceWith(frag);
  }
  return id;
}

export interface StudyBlock {
  /** The `##` heading, or '' for the text before the first one. */
  title: string;
}

/**
 * Splits the note at its top-level `#`/`##` headings into `<section class="blk" data-bi>` blocks
 * and adds word blanks. Returns the HTML, the blocks, and every blank in reading order with its block.
 */
export function studyHtml(html: string): { html: string; blocks: StudyBlock[]; order: { key: string; block: number; word: boolean }[] } {
  const tpl = document.createElement('template');
  tpl.innerHTML = html;
  const blocks: StudyBlock[] = [];
  const sections: HTMLElement[] = [];
  let cur: HTMLElement | null = null;
  for (const node of [...tpl.content.childNodes]) {
    const isHead = node instanceof HTMLElement && /^H[12]$/.test(node.tagName);
    if (!cur || isHead) {
      cur = document.createElement('section');
      cur.className = 'blk';
      cur.dataset.bi = String(sections.length);
      sections.push(cur);
      blocks.push({ title: isHead ? (node.textContent ?? '').replace(/(^|\s)#[^\s#]+/g, '').trim() : '' });
    }
    cur.append(node);
  }
  // A leading block with nothing but whitespace is dropped.
  if (sections.length > 1 && !blocks[0].title && !sections[0].textContent?.trim()) {
    sections.shift();
    blocks.shift();
    sections.forEach((s, i) => (s.dataset.bi = String(i)));
  }
  tpl.content.replaceChildren(...sections);
  wrapWords(tpl.content);
  const order = [...tpl.content.querySelectorAll<HTMLElement>('.blank')].map((el) => ({
    key: el.dataset.key!,
    block: Number(el.closest<HTMLElement>('.blk')?.dataset.bi ?? 0),
    word: el.classList.contains('word'),
  }));
  return { html: tpl.innerHTML, blocks, order };
}
