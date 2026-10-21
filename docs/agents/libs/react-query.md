---
title: React Query
scope: src/domain/*/queries/**, src/shared/components/queries/**, src/app/main.tsx
applies_to: reading or writing data through Tauri commands, caching, cross-window sync
related:
  - ../conventions/tauri-commands.md
  - ../conventions/error-handling.md
  - ../conventions/naming.md
---

# React Query

> All Rust data (settings, the queue, tool status, update status, CLI status) goes through React Query hooks in `src/domain/<name>/queries/<name>.tsx`, which wrap `apis/<name>.ts`. One `queryClient` per window (`RQProvider.tsx`). Changes reach every window through a Tauri event whose listener calls `setQueryData`.

## Reads

```tsx
export const QUEUE_QUERY_KEY = ['queue', 'jobs'];

export function useJobs() {
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery<Job[]>({
    queryKey: QUEUE_QUERY_KEY,
    queryFn: listJobs,
    staleTime: Infinity, // the listener below keeps the cache current
  });

  useEffect(() => {
    const unlisten = listen<Job[]>('queue://changed', (e) => queryClient.setQueryData(QUEUE_QUERY_KEY, e.payload));
    return () => void unlisten.then((fn) => fn());
  }, [queryClient]);

  return { jobs: data ?? [], isLoading };
}
```

- Return a small object with defaults applied (`data ?? []`), not the raw query result.
- Query keys are `[domain, resource, ...specifier]`.
- Data that an event keeps current uses `staleTime: Infinity`; refetching would only race the events.

## Writes

```tsx
export function useSettingsPatch() {
  const queryClient = useQueryClient();
  const { mutate, isPending } = useMutation<Settings, AppError, SettingsPatch, { prev?: Settings }>({
    mutationKey: ['settings', 'patch'],
    mutationFn: patchSettings,
    onMutate: async (patch) => { /* optimistic: merge the patch into the cache, remember prev */ },
    onError: (_err, _patch, context) => queryClient.setQueryData(SETTINGS_QUERY_KEY, context?.prev),
    onSettled: () => pendingPatches.end(),
  });
  return { patchSettings: mutate, isLoading: isPending };
}
```

- When Rust emits an event with the new value (queue, tools, settings), the mutation needs no cache write: the listener does it.
- Settings are patched optimistically. `settingsFromEcho` lays still-pending patches over the `settings://changed` echo so a slow echo can't undo a newer click.
- Failed mutations go to `ErrorAlert` automatically; pass `meta: { silent: true }` when the caller shows the error itself (the options sheet) ([error-handling](../conventions/error-handling.md)).

## Defaults (`RQProvider.tsx`)

- `staleTime` 60 s, `gcTime` 120 s, `refetchOnWindowFocus: false`.
- `refetchOnReconnect` is omitted: there is no network between the webview and Rust.
- Devtools render only in dev (`import.meta.env.DEV`).

## Multiple windows

Each Tauri window is a separate JS context, so it has **its own `queryClient` and Zustand store**. Nothing is shared automatically.

- When a command (or the CLI, or a background loop) changes data a window shows, Rust `emit`s an event (`<domain>://<event>`) carrying the new value.
- Every window listens once, in a provider or hook, and calls `queryClient.setQueryData(key, payload)`: `ThemeProvider` (`settings://changed`), `useJobs` (`queue://changed`), `useToolsStatus` (`tools://changed`), `useUpdateStatus` (`update://status`).
- Mount listening hooks for the window's **whole life**. A hook mounted only inside a tab misses events sent while another tab is open — `SettingsApp` calls `useJobs`, `useToolsStatus` and `useUpdateStatus` at the top for that reason.

## First paint

`main.tsx` awaits `getSettings()` (theme and language would otherwise flash), seeds `SETTINGS_QUERY_KEY` with `queryClient.setQueryData` and calls `applyTheme` **before** `render`, then mounts the providers. A failed read shows the error and falls back to `DEFAULT_SETTINGS`.
