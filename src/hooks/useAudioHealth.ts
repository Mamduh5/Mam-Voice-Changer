import { useEffect, useRef, useState } from 'react';
import type { EngineStatus } from '../types/engine';

const SIGNAL_THRESHOLD = 0.003;
const NO_SIGNAL_DELAY_MS = 4_000;

export function useAudioHealth(status: EngineStatus) {
  const [inputActivityDetected, setInputActivityDetected] = useState<boolean | null>(null);
  const noSignalTimerRef = useRef<number | null>(null);
  const active = ['running', 'degraded', 'recovering'].includes(status.state);
  const session = active ? `${status.routePurpose ?? 'unknown'}` : null;

  useEffect(() => {
    if (noSignalTimerRef.current !== null) {
      window.clearTimeout(noSignalTimerRef.current);
      noSignalTimerRef.current = null;
    }
    const resetTimer = window.setTimeout(() => setInputActivityDetected(null), 0);
    if (active) {
      noSignalTimerRef.current = window.setTimeout(() => {
        noSignalTimerRef.current = null;
        setInputActivityDetected(false);
      }, NO_SIGNAL_DELAY_MS);
    }
    return () => {
      window.clearTimeout(resetTimer);
      if (noSignalTimerRef.current !== null) {
        window.clearTimeout(noSignalTimerRef.current);
        noSignalTimerRef.current = null;
      }
    };
  }, [active, session]);

  useEffect(() => {
    if (!active || status.inputLevel <= SIGNAL_THRESHOLD) return;
    const signalTimer = window.setTimeout(() => setInputActivityDetected(true), 0);
    if (noSignalTimerRef.current !== null) window.clearTimeout(noSignalTimerRef.current);
    noSignalTimerRef.current = window.setTimeout(() => {
      noSignalTimerRef.current = null;
      setInputActivityDetected(false);
    }, NO_SIGNAL_DELAY_MS);
    return () => window.clearTimeout(signalTimer);
  }, [active, status.inputLevel]);

  return {
    inputActivityDetected: active ? inputActivityDetected : null,
    noSignal: active && inputActivityDetected === false,
  };
}
