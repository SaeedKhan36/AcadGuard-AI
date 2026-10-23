# Verification

## Initial UI checks

Checked October 23, 2026.

- `npm run build` passes strict TypeScript checking and creates the production bundle.
- Local development server responds with HTTP 200.
- Browser-reviewed the landing page and educator dashboard.
- Checked the 390px mobile layout in a same-origin iframe; landing page and workspace document widths match the viewport after correcting heading and table accessibility-label overflow.
- Verified mobile navigation and access to classes.
- Verified demo entry from the welcome page.
- Verified search by student, needs-review filtering, report opening, Similarity/AI writing/Feedback views, and feedback notifications.
- Verified the safe local TXT fixture can be selected and adds a clearly labeled demo submission with fixed sample scores.
- Verified resource deep links, article dialogs, Escape dismissal, audience switching, and settings submission.

The authenticated Turnitin workspace was not accessed: the official sign-in page requires existing credentials. The internal design is an original concept. No real analysis, backend, or authentication was tested because they are outside this UI-only scope.

## Clerk authentication integration

Checked October 23, 2026.

- `npm test` passes four redirect regression tests: legacy workspace links, marketing anchors, local workspace return paths, and external redirect rejection.
- `npm run build` passes strict TypeScript checking and builds the Clerk-enabled bundle.
- Confirmed the real Clerk sign-in form renders at `/auth/login`, with email and the configured Google provider.
- Confirmed the real registration form renders at `/auth/register`.
- Confirmed signed-out access to `/dashboard/reports` redirects to `/auth/login` with the intended report path preserved, without rendering the workspace.
- Confirmed old `/#/app/resources?article=1` links migrate to the protected path and then sign-in.
- Checked that `.env.local` is ignored by Git. The server secret was not placed in this frontend checkout.
- A full successful login/registration, account management, and sign-out require a user to complete the identity-provider flow. They were not performed with a real account during implementation.

Authentication uses Clerk; document handling and report generation remain local UI demonstrations. The Rust backend was not accessed or modified.

## pnpm workspace and engine import

Checked October 23, 2026 with Node.js 24.22.1, pnpm 11.10.0, and Rust 1.97.1.
The sections above record checks and commands from before the monorepo migration.

- `pnpm install --frozen-lockfile` passes across the root, web app, and engine wrapper.
- `pnpm check` passes strict TypeScript checking and Cargo checks for both crates, including tests and examples.
- `pnpm test` passes 66 tests: four browser routing tests, 59 Rust unit tests, and three CLI integration tests.
- `pnpm build` builds the web bundle and Rust release binary. `pnpm build:web` builds the browser app independently.
- Root development and preview commands serve the relocated app, authentication and dashboard deep links, and favicon. Development also serves the relocated TypeScript entry point.
- `pnpm engine --help` forwards arguments to the Rust CLI.
- `pnpm demo:engine` ingests the bundled corpus and produces a 45% similarity report across two sources. Quote/bibliography filtering reduces similarity, and homoglyph flags appear in the report.
- All 32 imported engine source, lock, test, example, and configuration files match the original source checkout. The imported README adds workspace usage instructions.
- All 19 moved browser source, asset, and configuration files retain their original contents. Direct JavaScript dependencies retain their npm-locked versions.
- Turbo build hashes change when `VITE_*` environment settings or ignored web environment files change. The web build cache was reused by the full workspace build.
- Build artifacts, engine demo databases/reports, Turbo caches, and local environment files are ignored by Git.

The checked-in Vercel configuration now uses the web-only pnpm build and `apps/web/dist`. No deployment was performed. The engine is available through its library and CLI; browser submissions still use demo data.

## Main branch merge with TanStack Start and Convex

Checked October 23, 2026 after merging `7744950` from `main`.

- Preserved all 32 application source files from `main` under `apps/web/src`, including file routes, Clerk server middleware, and Convex data integration.
- Moved Convex configuration, functions, shared types, and Vitest configuration alongside the application so relative imports and CLI paths remain valid.
- Preserved every direct application dependency at its version in main's npm lockfile. Exact-version release-age exceptions permit installation of the recently published versions already used by main.
- `pnpm install --frozen-lockfile`, `pnpm check`, and `pnpm build` pass.
- `pnpm test` passes 75 tests: 13 application/Convex tests and 62 Rust tests.
- Root `pnpm start` launches the relocated Nitro server and serves its public assets.
- A local build with `VERCEL=1 NITRO_PRESET=vercel` emits the server function, public assets, and Build Output API v3 configuration in the repository root's `.vercel/output`.

The current web build is a TanStack Start server application in `apps/web/.output`, replacing the static build described in the earlier sections. No live Convex or Vercel commands were run.
