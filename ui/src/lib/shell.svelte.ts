// **Which frame the app wears** (`decisions.md` 2026-11-17, *one frame*): the phone app wears the one
// bar of tabs (`TabBar`); elsewhere the app keeps its older bar (a paired tablet, the computer's
// fallback in another browser; on a computer formicaria's own browser shows feature pages instead).
// Tests ask for the bar with `useTabBar(true)`. Where the bar sits is the person's choice.
import { isPhone } from './platform';

const state = $state({ forced: null as boolean | null, position: readPosition() });

export const tabBar = (): boolean => state.forced ?? isPhone();
export function useTabBar(on: boolean | null): void {
  state.forced = on;
}

function readPosition(): 'top' | 'bottom' {
  try {
    return localStorage.getItem('fm-bar') === 'top' ? 'top' : 'bottom';
  } catch {
    return 'bottom';
  }
}
export const barPosition = (): 'top' | 'bottom' => state.position;
export function setBarPosition(p: 'top' | 'bottom'): void {
  state.position = p;
  try {
    localStorage.setItem('fm-bar', p);
  } catch {
    /* this device keeps no choice */
  }
}
