import { useCallback, useEffect, useRef, useState } from 'react';

const SCROLL_THRESHOLD = 12;
const TOP_REVEAL_DISTANCE = 24;

export type ChromeScrollDirective = 'show' | 'hide' | 'keep';

export function getChromeScrollDirective(
  previousScrollY: number,
  nextScrollY: number,
): ChromeScrollDirective {
  if (nextScrollY <= TOP_REVEAL_DISTANCE || previousScrollY - nextScrollY >= SCROLL_THRESHOLD) {
    return 'show';
  }
  if (nextScrollY - previousScrollY >= SCROLL_THRESHOLD) return 'hide';
  return 'keep';
}

export function useAutoHideChrome() {
  const [hidden, setHidden] = useState(false);
  const lastScrollY = useRef(0);
  const hideTimer = useRef<number | null>(null);

  const clearHideTimer = useCallback(() => {
    if (hideTimer.current !== null) window.clearTimeout(hideTimer.current);
    hideTimer.current = null;
  }, []);

  const reveal = useCallback(() => {
    clearHideTimer();
    setHidden(false);
  }, [clearHideTimer]);

  const scheduleHide = useCallback(() => {
    clearHideTimer();
    hideTimer.current = window.setTimeout(() => {
      const activeElement = document.activeElement;
      const chromeHasFocus = [
        ...document.querySelectorAll<HTMLElement>('[data-application-chrome]'),
      ].some((chrome) => chrome.contains(activeElement));
      if (!chromeHasFocus) setHidden(true);
    }, 800);
  }, [clearHideTimer]);

  useEffect(() => {
    lastScrollY.current = window.scrollY;
    const onScroll = () => {
      const nextScrollY = window.scrollY;
      const directive = getChromeScrollDirective(lastScrollY.current, nextScrollY);
      lastScrollY.current = nextScrollY;
      if (directive === 'show') reveal();
      if (directive === 'hide') scheduleHide();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Alt' || event.key === 'Escape' || event.key === 'Tab') reveal();
    };

    window.addEventListener('scroll', onScroll, { passive: true });
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('scroll', onScroll);
      window.removeEventListener('keydown', onKeyDown);
      clearHideTimer();
    };
  }, [clearHideTimer, reveal, scheduleHide]);

  return { hidden, reveal, scheduleHide };
}
