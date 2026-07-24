import type { ReactNode } from 'react';

type Props = {
  hidden: boolean;
  onReveal: () => void;
  onScheduleHide: () => void;
  children: ReactNode;
};

export function ApplicationChrome({ hidden, onReveal, onScheduleHide, children }: Props) {
  return (
    <>
      <div aria-hidden="true" className="chrome-activation-zone" onPointerEnter={onReveal} />
      <div
        className={`application-chrome${hidden ? ' application-chrome--hidden' : ''}`}
        data-application-chrome
        onFocusCapture={onReveal}
        onPointerEnter={onReveal}
        onPointerLeave={onScheduleHide}
        onClick={onReveal}
      >
        {children}
      </div>
    </>
  );
}
