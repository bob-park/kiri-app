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

> All Rust data goes through React Query hooks in `src/domain/<name>/queries/<name>.tsx`, which wrap `apis/<name>.ts`. One `queryClient` per window (`RQProvider.tsx`). A change made in one window reaches the others through a Tauri event that calls `setQueryData`.

## Reads

```tsx
export function useThemePref() {
  const { data, isLoading } = useQuery<ThemePref>({ queryKey: THEME_QUERY_KEY, queryFn: getTheme });
  return { theme: data ?? 'system', isLoading };
}
```

- Return a small object with defaults applied (`data ?? 'system'`), not the raw query result.
- Query keys are `[domain, resource, ...specifier]`.

## Writes

```tsx
export function useThemeUpdate({ onSuccess, onError }: QueryMutationHandle<ThemePref> = {}) {
  const queryClient = useQueryClient();
  const { mutate, isPending } = useMutation<ThemePref, AppError, ThemePref>({
    mutationKey: ['settings', 'theme', 'update'],
    mutationFn: setTheme,
    onSuccess: (data) => {
      queryClient.setQueryData(THEME_QUERY_KEY, data);
      onSuccess?.(data);
    },
    onError: (err) => onError?.(err),
  });
  return { updateTheme: mutate, isLoading: isPending };
}
```

- When the command returns the new value, write it into the cache with `setQueryData`. Use `invalidateQueries` when the result affects lists you can't patch.
- Accept `QueryMutationHandle<T>` so callers can react to success or errors.

## Defaults (`RQProvider.tsx`)

- `staleTime` 60 s, `gcTime` 120 s, `refetchOnWindowFocus: false`.
- `refetchOnReconnect` is omitted: there is no network between the webview and Rust.
- Devtools render only in dev (`import.meta.env.DEV`).

## Multiple windows

Each Tauri window is a separate JS context, so it has **its own `queryClient` and Zustand store**. Nothing is shared automatically.

- When a command changes data another window shows, have Rust `emit` an event (`<domain>://<event>`) carrying the new value.
- Every window listens to it once, in a provider or hook, and calls `queryClient.setQueryData(key, payload)`. Examples: `ThemeProvider` (`settings://theme-changed`), `I18nProvider` (`settings://locale-changed`), `useUpdateStatus` (`update://status`).

## First paint

`main.tsx` awaits values that would otherwise flash, such as theme and locale, seeds them with `queryClient.setQueryData` **before** `render`, then mounts the providers.
