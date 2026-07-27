import { useEffect, useState } from 'react';
import { tauriAudioApi } from '../services/tauriAudioApi';
import type { ProductInformation } from '../types/product';

export function useProductInformation(enabled = true) {
  const [information, setInformation] = useState<ProductInformation | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!enabled) return undefined;
    let active = true;
    void tauriAudioApi
      .getProductInformation()
      .then((next) => {
        if (active) {
          setInformation(next);
          setError(null);
        }
      })
      .catch((cause) => {
        if (active) setError(`Unable to load product version information: ${String(cause)}`);
      });
    return () => {
      active = false;
    };
  }, [enabled]);

  return { information, error };
}
