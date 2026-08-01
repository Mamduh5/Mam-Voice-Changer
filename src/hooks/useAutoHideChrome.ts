import { useCallback, useEffect, useReducer, useRef } from 'react';

const SCROLL_THRESHOLD = 12;
const TOP_REVEAL_DISTANCE = 24;

export type ChromeMode = 'automatic' | 'manuallyHidden' | 'manuallyVisible';

export type ChromeState = {
  mode: ChromeMode;
  visible: boolean;
};

export type ChromeAction =
  | { type: 'BACKGROUND_TOGGLE' }
  | { type: 'ESCAPE' }
  | { type: 'NAVIGATION_FOCUS' }
  | { type: 'AUTOMATIC_SHOW' }
  | { type: 'AUTOMATIC_HIDE' };

export const initialChromeState: ChromeState = { mode: 'automatic', visible: true };

export function isBackgroundChromeToggle(
  target: EventTarget | null,
  currentTarget: EventTarget | null,
  pointerTravel: number,
  hasSelection: boolean,
) {
  return target === currentTarget && pointerTravel < 4 && !hasSelection;
}

export function chromeReducer(state: ChromeState, action: ChromeAction): ChromeState {
  switch (action.type) {
    case 'BACKGROUND_TOGGLE':
      return state.mode === 'manuallyHidden'
        ? { mode: 'manuallyVisible', visible: true }
        : { mode: 'manuallyHidden', visible: false };
    case 'ESCAPE':
    case 'NAVIGATION_FOCUS':
      return { mode: 'manuallyVisible', visible: true };
    case 'AUTOMATIC_SHOW':
      return state.mode === 'automatic' ? { ...state, visible: true } : state;
    case 'AUTOMATIC_HIDE':
      return state.mode === 'automatic' ? { ...state, visible: false } : state;
  }
}

function currentScrollY() {
  return window.scrollY || document.documentElement.scrollTop || document.body.scrollTop || 0;
}

export function getChromeScrollDirective(
  previousScrollY: number,
  nextScrollY: number,
): 'show' | 'hide' | 'keep' {
  if (nextScrollY <= TOP_REVEAL_DISTANCE || previousScrollY - nextScrollY >= SCROLL_THRESHOLD) {
    return 'show';
  }
  if (nextScrollY - previousScrollY >= SCROLL_THRESHOLD) return 'hide';
  return 'keep';
}

export function useAutoHideChrome() {
  const [state, dispatch] = useReducer(chromeReducer, initialChromeState);
  const stateRef = useRef(state);
  const lastScrollY = useRef(0);
  const hideTimer = useRef<number | null>(null);

  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  const clearHideTimer = useCallback(() => {
    if (hideTimer.current !== null) window.clearTimeout(hideTimer.current);
    hideTimer.current = null;
  }, []);

  const automaticShow = useCallback(() => {
    if (stateRef.current.mode !== 'automatic') return;
    clearHideTimer();
    dispatch({ type: 'AUTOMATIC_SHOW' });
  }, [clearHideTimer]);

  const scheduleAutomaticHide = useCallback(() => {
    if (stateRef.current.mode !== 'automatic') return;
    clearHideTimer();
    hideTimer.current = window.setTimeout(() => {
      if (stateRef.current.mode !== 'automatic') return;
      const chromeHasFocus = Array.from(
        document.querySelectorAll<HTMLElement>('[data-application-chrome]'),
      ).some((chrome) => chrome.contains(document.activeElement));
      if (!chromeHasFocus) dispatch({ type: 'AUTOMATIC_HIDE' });
    }, 800);
  }, [clearHideTimer]);

  const navigationFocus = useCallback(() => {
    clearHideTimer();
    dispatch({ type: 'NAVIGATION_FOCUS' });
  }, [clearHideTimer]);

  const toggleBackground = useCallback(() => {
    clearHideTimer();
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    dispatch({ type: 'BACKGROUND_TOGGLE' });
  }, [clearHideTimer]);

  useEffect(() => {
    lastScrollY.current = currentScrollY();
    const onScroll = () => {
      if (stateRef.current.mode !== 'automatic') return;
      const nextScrollY = currentScrollY();
      const directive = getChromeScrollDirective(lastScrollY.current, nextScrollY);
      lastScrollY.current = nextScrollY;
      if (directive === 'show') automaticShow();
      if (directive === 'hide') scheduleAutomaticHide();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') navigationFocus();
    };

    window.addEventListener('scroll', onScroll, { passive: true });
    document.addEventListener('scroll', onScroll, { capture: true, passive: true });
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('scroll', onScroll);
      document.removeEventListener('scroll', onScroll, { capture: true });
      window.removeEventListener('keydown', onKeyDown);
      clearHideTimer();
    };
  }, [automaticShow, clearHideTimer, navigationFocus, scheduleAutomaticHide]);

  return {
    state,
    automaticShow,
    scheduleAutomaticHide,
    navigationFocus,
    toggleBackground,
  };
}
