---
description: Frontend guidelines for Stardate (Svelte SPA)
applyTo: 'web/**'
---

# Stardate frontend

Stardate is a personal time-tracking app with a Rust/Axum backend. This document covers the Svelte frontend in `web/`.

## Stack

- SvelteKit 2 + Svelte 5 (runes and snippets only, no legacy `$:` or `export let`, no `svelte/store` for new code).
- TypeScript in strict mode.
- Vite (current stable). Stay on SvelteKit 2: Kit 3 is stable since 2026-10 but not adopted yet (Skeleton v5 targets Kit 2). `sv@0.17.1` is the last `sv` release that scaffolds Kit 2.
- Tailwind CSS v4 (CSS-first config, `@tailwindcss/vite` plugin, no `tailwind.config.js`).
- Skeleton v5 (`@skeletonlabs/skeleton` + `@skeletonlabs/skeleton-svelte`). Use its Tailwind components and Svelte framework components before writing custom ones.
- Icons: `@lucide/svelte` (`lucide-svelte` is deprecated).
- Documentation: https://www.skeleton.dev/docs/svelte/get-started/introduction and the LLM page at https://www.skeleton.dev/docs/svelte/resources/llms. Skeleton v5 differs from v4, so ignore v4 examples. If you cannot read the docs, ask me instead of guessing the API.

## Delivery model

- Static SPA: `@sveltejs/adapter-static` with `fallback: 'index.html'`, `ssr = false`, `prerender = false`.
- The build output is served by the Axum server (`tower-http` `ServeDir`) from the same origin as the API under `/api`. There is no Node runtime in production.
- Same origin means no CORS. Call the API with relative URLs (`/api/...`).
- In development, proxy `/api` to the Axum server in `vite.config.ts`.

## Architecture rules

- The backend does as much of the calculations (durations, totals, breaks, overtime, schedule resolution) as possible. The frontend only displays what the API returns and formats it. Never compute business values client-side.
- Formatting and parsing of dates and times uses `Intl` and ISO 8601 strings. Keep timestamps and time zones as the API provides them.
- Structure:
  - `src/lib/api/` typed API client and DTO types
  - `src/lib/components/` reusable UI
  - `src/lib/features/<feature>/` feature-specific components and state
  - `src/routes/` pages only, kept thin
- All HTTP goes through one client in `src/lib/api/client.ts`. No `fetch` calls in components.
- Shared state uses Svelte 5 runes in `.svelte.ts` modules.
- Never store auth tokens in `localStorage`. Auth uses whatever the backend provides (prefer httpOnly cookie). Never use `{@html}` with API data.

## Error handling and toasts

- Every non-2xx response and every network failure shows an error toast. The API client throws a typed `ApiError` and a single global handler turns it into a toast, so feature code does not repeat this.
- Align `ApiError` parsing with the backend's actual error response type: read it in the Rust code before implementing. Show the backend's user-facing message, falling back to a generic message. Never show stack traces or raw response bodies.
- Exceptions:
  - 401 redirects to sign-in instead of toasting repeatedly. Deferred until authentication exists (README, "Possible improvements").
  - Aborted requests show no toast.
  - Validation errors (400/422) also mark the relevant form fields where the response identifies them.
- Deduplicate identical toasts fired in quick succession.
- Use Skeleton's Toast component with one toaster instance exposed from `src/lib/toast.ts` and rendered once in the root layout. Mutations show a short success toast.

## App shell and theming

- Mobile first, modern, uncluttered. Test at 360px width before desktop.
- Navigation: bottom navigation on mobile, side rail or top links from the `md` breakpoint.
- Theme selector: pick from a small curated list of Skeleton themes plus light/dark/system mode. Apply via `data-theme` on `<html>` and persist the choice in `localStorage`. Set it in an inline script in `app.html` before first paint to avoid a flash. Use system default if not available.
- Use Skeleton design tokens and presets for color and spacing. No hardcoded colors.
- The UI text is English.

## Workflows

### Workday (default screen)

- The main screen answers "what is my state right now?" and offers the next valid action as a large, thumb-friendly primary button: sign in, start/end lunch, start/end break, sign out. Which actions are valid comes from the backend response.
- Show current status, job, and today's totals as returned by the API. A live ticking counter is acceptable only as a cosmetic display based on a server-provided start timestamp.
- Support multiple jobs (job selector) and back-dated entries (date and time pickers).
- Refresh state from the API after every action.

### History and edits

- List entries grouped by day. Use a table from `md` upwards and a card list below it.
- Edit entries in a Skeleton Dialog, presented as a full-screen sheet on mobile. Support create, edit, and delete (with confirmation).
- Schedule edits must make the effective date explicit, because changes must never alter historical entries. Follow the backend contract.
- Client-side validation is for usability only (required fields, formats). The backend is authoritative.

## UX and accessibility

- Loading, empty, and error states for every data view. Disable or show progress on buttons while a request is pending.
- Use native `<select>`, `<input type="time">`, and `<input type="date">` on mobile unless a Skeleton component is clearly better. Radios use the Skeleton form classes or Segmented Control.
- Keyboard accessible, visible focus, labelled controls, sufficient contrast in every theme.

## Code quality

- Follow the repository baseline guidelines. Descriptive names, small single-purpose components, no magic values, no dead code, comments explain why.
- Keep dependencies minimal and pinned. Ask before adding any beyond the stack above.

## Verification

- Run `npm run check` (svelte-check), `npm run lint` (Prettier check and ESLint), `npm test` and `npm run build` before considering work complete and fix errors you introduced.
- Component tests (`*.svelte.test.ts`) run in Vitest browser mode with Playwright; other tests (`*.test.ts`) run in Node. Browser mode needs Chromium once: `npx playwright install chromium`.
- Add Vitest tests for the API client (error mapping to toasts; 401 handling once authentication exists) and for non-trivial components. Tests must be deterministic and mock the network.

## Safety

- Do not delete files or branches, force-push, or push code without asking.
- Do not modify the Dockerfile or backend serving code without asking. Report what the frontend needs from them instead.

# Working rules

- Read and understand `../CLAUDE.md`
- Since Read the API from the type definitions inside the npm packages (@skeletonlabs/skeleton-svelte ships them). They’re authoritative for v5, but have no usage examples.
- The Skeleton v5 LLM docs are in `web/lmm/llms-svelte-skeleton.txt` (about 790 KB). Search it for the section you need (e.g. `# Toast`) instead of reading it whole.
- Usage in this app: `api` from `$lib/api/client` for every request; errors are toasted by the handler set in the root layout, so callers only handle what changes their own UI.

# Tooling

Tooling is npm with ESLint and Prettier.
