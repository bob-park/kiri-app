---
title: React Components & Section Comments
scope: src/**/*.tsx
applies_to: components, hooks (useXxx.tsx), providers
related:
  - ./naming.md
  - ../libs/zustand-slice.md
  - ../libs/react-query.md
---

# React Components & Section Comments

> Function components, `default export`. Bodies of components, hooks and providers are divided by section comments in a **fixed order**. Omit unused sections; never leave an empty header. (This is a Vite SPA — there are no server components or `'use client'`.)

## Section order

1. `// ref` — `useRef`
2. `// context` — `useContext`
3. `// state` — `useState`, `useReducer`
4. `// store` — Zustand selectors (`useStore((s) => …)`)
5. `// hooks` — other hooks (`useI18n`, `useQueryClient`, custom hooks)
6. `// queries` — React Query hooks, including mutation hooks
7. `// useEffect`
8. `// useLayoutEffect`
9. `// handle` — event handlers (`handleXxx`) and other plain functions
10. `// memorize` — `useMemo`
11. `// callback` — `useCallback`

Write any number of lines within a section. Small sub-components co-located in the same file follow the same rule.

## Example

```tsx
export default function UpdateSection() {
  // state
  const [version, setVersion] = useState('');

  // hooks
  const { t } = useI18n();

  // queries
  const { status } = useUpdateStatus();
  const { checkUpdate, isLoading } = useUpdateCheck();

  // useEffect
  useEffect(() => {
    void getVersion().then(setVersion);
  }, []);

  return (/* … */);
}
```

## Other rules

- Call `call()` / `invoke` only through `apis/` + `queries/`; components never import `@tauri-apps/api/core` directly.
- Fire-and-forget promises are marked `void` (ESLint `no-floating-promises`).
- Event listeners (`listen`, `matchMedia`, `keydown`) are added in a `useEffect` and removed in its cleanup.
