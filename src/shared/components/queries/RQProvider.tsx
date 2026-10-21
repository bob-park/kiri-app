import { MutationCache, QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';

import type { AppError } from '@/shared/api/common.dto';
import { useStore } from '@/shared/store/rootStore';

// One client per window: each Tauri window is its own JS context.
export const queryClient = new QueryClient({
  // Every failed mutation is shown by ErrorAlert. `call()` always rejects with an AppError.
  // A mutation with `meta.silent` shows its own error instead.
  mutationCache: new MutationCache({
    onError: (error, _variables, _context, mutation) => {
      if (mutation.meta?.silent === true) {
        return;
      }
      useStore.getState().showError(error as unknown as AppError);
    },
  }),
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      staleTime: 60 * 1_000,
      gcTime: 120 * 1_000,
    },
  },
});

export default function RQProvider({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <QueryClientProvider client={queryClient}>
      {children}
      {import.meta.env.DEV && <ReactQueryDevtools initialIsOpen={false} />}
    </QueryClientProvider>
  );
}
