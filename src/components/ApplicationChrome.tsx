import type { ReactNode } from 'react';

type Props = {
  hidden: boolean;
  onAutomaticReveal: () => void;
  onNavigationFocus: () => void;
  onScheduleAutomaticHide: () => void;
  children: ReactNode;
};

export function ApplicationChrome({
  hidden,
  onAutomaticReveal,
  onNavigationFocus,
  onScheduleAutomaticHide,
  children,
}: Props) {
  return (
    <>
      <div
        aria-hidden="true"
        className="chrome-activation-zone"
        onPointerEnter={onAutomaticReveal}
      />
      <div
        className={`application-chrome${hidden ? ' application-chrome--hidden' : ''}`}
        data-application-chrome
        onFocusCapture={onNavigationFocus}
        onPointerEnter={onAutomaticReveal}
        onPointerLeave={onScheduleAutomaticHide}
      >
        {children}
      </div>
    </>
  );
}
