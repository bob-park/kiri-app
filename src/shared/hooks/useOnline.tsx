import { useSyncExternalStore } from 'react';

function subscribe(onChange: () => void) {
  window.addEventListener('online', onChange);
  window.addEventListener('offline', onChange);

  return () => {
    window.removeEventListener('online', onChange);
    window.removeEventListener('offline', onChange);
  };
}

/** `navigator.onLine`, kept current by the `online`/`offline` events. */
export default function useOnline() {
  return useSyncExternalStore(subscribe, () => navigator.onLine);
}
