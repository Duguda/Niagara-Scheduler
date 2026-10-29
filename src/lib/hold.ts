// Press-and-hold repeat for arrow and +/- buttons, so big jumps don't need a burst of clicks.
// One step on press; held for 0.5 s it starts repeating, and every further 0.5 s it gets faster.
// Enter/Space still step once per press (those clicks have `detail === 0`).
import type { Action } from 'svelte/action';

/** `level`: 0 for the press itself, then how many 0.5 s stretches the button has been held. */
export type HoldStep = (level: number) => void;
export type HoldParam = HoldStep | { step: HoldStep; done?: () => void };

const DELAY = 500;
const SPEEDS = [220, 140, 90, 60]; // ms between steps, one entry per 0.5 s held

export const holdRepeat: Action<HTMLElement, HoldParam> = (node, param) => {
  let p = typeof param === 'function' ? { step: param } : param;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let since = 0;

  const disabled = () => (node as unknown as HTMLButtonElement).disabled;
  const tick = () => {
    if (disabled()) return stop();
    const level = Math.floor((Date.now() - since) / DELAY);
    p.step(level);
    timer = setTimeout(tick, SPEEDS[Math.min(level - 1, SPEEDS.length - 1)]);
  };
  function stop() {
    if (timer === undefined) return;
    clearTimeout(timer);
    timer = undefined;
    p.done?.();
  }
  const down = (e: PointerEvent) => {
    if (e.button !== 0 || disabled()) return;
    e.preventDefault(); // keeps text from being selected while held
    since = Date.now();
    p.step(0);
    timer = setTimeout(tick, DELAY);
  };
  const click = (e: MouseEvent) => {
    if (e.detail !== 0) return; // pointer presses were handled on pointerdown
    p.step(0);
    p.done?.();
  };

  node.addEventListener('pointerdown', down);
  node.addEventListener('pointerup', stop);
  node.addEventListener('pointerleave', stop);
  node.addEventListener('pointercancel', stop);
  node.addEventListener('click', click);
  return {
    update(next) {
      p = typeof next === 'function' ? { step: next } : next;
    },
    destroy() {
      stop();
      node.removeEventListener('pointerdown', down);
      node.removeEventListener('pointerup', stop);
      node.removeEventListener('pointerleave', stop);
      node.removeEventListener('pointercancel', stop);
      node.removeEventListener('click', click);
    },
  };
};
