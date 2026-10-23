# AcadGuard AI: An Intelligent Academic Integrity Platform for Plagiarism Detection, Semantic Similarity Analysis, and Citation Verification

A pnpm and Turborepo monorepo containing a TanStack Start application with TanStack Router, React, Clerk, and Convex, plus a Rust plagiarism detection engine.

Authentication is provided by Clerk. Convex stores each signed-in user's submission metadata, feedback, and workspace preferences. **Similarity scores, AI indicators, source matches, classes, and report content are illustrative.** The Rust engine is available through its library and CLI. The browser does not call it yet, and AI detection and citation verification are not implemented. It is an independent concept, unaffiliated with Turnitin.

![Landing page preview](docs/landing-preview.jpg)

## Run locally

Requires Node.js 22.13+, pnpm 11.10.0, and a Rust toolchain with Cargo for engine commands. The engine was validated with Rust 1.97.1. The application uses TypeScript 5.9.

```sh
corepack enable
corepack install
pnpm install --frozen-lockfile
cp apps/web/.env.example apps/web/.env.local
```

Set your Clerk keys in `apps/web/.env.local`. Keep an existing working environment file. If migrating an older checkout, move its root `.env.local` to `apps/web/.env.local`.

| Variable                       | Purpose                                               |
| ------------------------------ | ----------------------------------------------------- |
| `VITE_CLERK_PUBLISHABLE_KEY`   | Browser-safe Clerk publishable key                    |
| `CLERK_SECRET_KEY`             | Server-only key for Clerk's TanStack Start middleware |
| `VITE_CLERK_SIGN_IN_URL`       | `/auth/login`                                         |
| `VITE_CLERK_SIGN_UP_URL`       | `/auth/register`                                      |
| `VITE_CLERK_AFTER_SIGN_IN_URL` | `/dashboard`                                          |
| `VITE_CLERK_AFTER_SIGN_UP_URL` | `/dashboard`                                          |
| `VITE_CONVEX_URL`              | Existing Convex deployment's `.convex.cloud` URL      |
| `CONVEX_DEPLOYMENT`            | Deployment selected by the Convex CLI                 |

Never prefix a secret key with `VITE_`. Environment files are ignored by Git; `.env.example` contains placeholders only.

Connect an **existing development project** before starting the workspace:

```sh
pnpm --filter @acadguard/web exec convex login
pnpm --filter @acadguard/web exec convex dev --configure existing --once
```

The CLI records the deployment and frontend URL in `apps/web/.env.local`. Check the selected project before deploying: Convex deploys the schema and function definitions from this repository. If sharing a project with another application, merge its existing schema/functions into this checkout first. This app uses `acadguardDocuments`, `acadguardFeedback`, and `acadguardPreferences` table names.

On that Convex deployment, set `CLERK_JWT_ISSUER_DOMAIN` to your Clerk Frontend API URL (for example, `https://your-instance.clerk.accounts.dev`). Enable the Convex integration in Clerk, or create a Clerk JWT template named `convex` from its Convex preset with audience `convex`. The issuer must match the Clerk application used by this UI.

```sh
pnpm --filter @acadguard/web exec convex env set CLERK_JWT_ISSUER_DOMAIN https://your-instance.clerk.accounts.dev
pnpm convex:dev
```

In another terminal:

```sh
pnpm dev
```

Open [the local app](http://127.0.0.1:5173/). The landing and authentication pages work before Convex is connected; the protected workspace displays a connection notice until its configuration is ready. Samples are loaded only when the user chooses **Load sample documents**.

## Build and validation

```sh
pnpm check
pnpm test
pnpm build
pnpm start
```

The full build checks application and Convex TypeScript, produces the server application in `apps/web/.output/`, and builds the Rust release binary in `packages/plagengine/target/release/plagengine`. `pnpm start` serves the application build on port 3000 by default. `pnpm preview` runs the web preview command. Use `pnpm build:web` for an application-only build without Rust. Tests cover safe return URLs, query compatibility, authentication, cross-user data isolation, preferences, input validation, and the Rust engine.

Turborepo caches web builds, including their `VITE_*` settings and environment files in the inputs. Server-only Clerk configuration is passed to development and build tasks. Cargo manages native artifacts; Rust tasks bypass Turbo caching. Turbo cache files stay in this checkout’s `.turbo/cache`.

## Plagiarism engine

Both Rust crates, their dependency lockfile, tests, evaluation tools, and demo corpus were imported from `/home/shujanforge/Projects/plagenine` into `packages/plagengine`. The source directory is unchanged.

- `plagengine-core` extracts TXT, Markdown, PDF, and DOCX documents, normalizes text, fingerprints token sequences with winnowing, retrieves candidates from a redb index, aligns passages, attributes overlapping sources, and produces filtered similarity reports.
- It classifies quotations and citations and flags hidden text, homoglyph replacements, and invisible character manipulation.
- `plagengine` exposes ingestion, checking, saved report filtering, source inspection, and PAN/adversarial evaluation through the CLI.

```sh
pnpm engine --help
pnpm demo:engine
pnpm engine --db /tmp/acadguard-example.db stats
```

The demo creates its own local index and reports under `packages/plagengine/examples/demo/`. Generated databases, reports, and build artifacts are ignored by Git. All Cargo scripts use the imported `Cargo.lock` with `--locked`.

See the [engine README](packages/plagengine/README.md) for CLI examples, the report schema, algorithm details, evaluation results, and limitations. The engine compares against sources you ingest; it does not search the web or provide semantic or AI writing detection. Browser-to-engine API integration remains future work.

## Authentication and data

- Start's Clerk middleware verifies the server session before entering `/dashboard` or a child route. Signed-out users return to sign-in with their intended workspace URL preserved.
- Clerk handles sign-in, registration, verification, account management, and sign-out. Login methods are controlled by the Clerk application's settings.
- Every Convex query and mutation independently requires an authenticated identity and scopes data to its subject. Document feedback checks document ownership as well.
- PDF, DOCX, and TXT selection accepts files up to 20 MB. Only filename and size are transmitted as submission metadata. File contents are not read, uploaded, or analyzed. New reports receive fixed illustrative scores.
- Feedback and workspace preferences persist in Convex and update reactively across sessions. Profile preferences do not change the underlying Clerk account.
- Samples, classes, dates, report highlights, and activity are demonstrative content. Do not use illustrative scores to assess actual student work.

## Routes

| Route                    | Experience                                                        |
| ------------------------ | ----------------------------------------------------------------- |
| `/`                      | Public landing page, audience tabs, solutions, resources, and FAQ |
| `/auth/login/*`          | Clerk sign-in and nested steps                                    |
| `/auth/register/*`       | Clerk registration and verification                               |
| `/dashboard`             | Protected overview and submissions                                |
| `/dashboard/submissions` | Search, submission metadata, optional sample loading              |
| `/dashboard/reports`     | Illustrative originality report reader                            |
| `/dashboard/ai`          | Illustrative AI writing indicators                                |
| `/dashboard/classes`     | Sample classes and assignments                                    |
| `/dashboard/feedback`    | Persisted document feedback and sample review checklist           |
| `/dashboard/resources`   | Educational guides and article dialogs                            |
| `/dashboard/settings`    | Persisted workspace preferences and Clerk account menu            |

Legacy links such as `/#/app/reports` redirect to the matching protected route. Query parameters and section anchors remain supported.

## Structure

```text
apps/web/
  package.json               TanStack Start, Convex, and validation commands
  .env.example               Clerk and Convex configuration template
  src/
    start.ts                 CSRF and Clerk request middleware
    router.tsx               TanStack Router configuration
    routes/                  File routes, HTML shell, server auth guards
    auth/                    Clerk theme, server session, safe redirects
    data/                    Authenticated Convex provider and queries
    components/              Shared links, branding, report composition
    pages/                   Landing, login, and workspace UI
    utils/                   Query compatibility helpers
    styles.css               Responsive visual system
  convex/                    Schema, authenticated functions, generated types, tests
  convex.json                Local Convex function directory configuration
  shared/workspace.ts        Shared UI data types and sample fixtures
  vitest.config.ts           Application and Convex tests
packages/plagengine/
  Cargo.toml                 Rust workspace containing core and CLI crates
  Cargo.lock                 Locked Rust dependencies
  crates/                    Algorithms, extraction, index, reports, tests
  examples/demo/             Local corpus, sample essay, demo script
  README.md                  Engine usage, report format, evaluations
docs/                        UI previews, fixtures, verification history
pnpm-workspace.yaml          Workspace packages and allowed dependency builds
pnpm-lock.yaml               Locked JavaScript dependencies
turbo.json                   Build, check, test, and development tasks
vercel.json                  Server application build configuration
```

Run `pnpm convex:codegen` after linking the existing project or changing backend functions. Do not edit the generated TanStack route tree.

## Vercel

The root `vercel.json` selects the `tanstack-start` framework, installs with pnpm, and runs `pnpm build:web`. When building on Vercel, Nitro writes its server and assets to the repository root's `.vercel/output`. Remove any project-level legacy `dist` output override and SPA rewrite. Deep links are handled by Start's server router.

Add the Clerk public variables, server-only `CLERK_SECRET_KEY`, and `VITE_CONVEX_URL` to each relevant Vercel environment. Use matching Clerk/Convex environments. Browser variables are embedded during the build, so changing them requires redeployment. Production should use a configured Clerk production application.

Deploy Convex functions to the intended production deployment separately with `pnpm convex:deploy`, then deploy the UI. If the existing Convex deployment has other functions, merge and preserve them before deploying. `CLERK_JWT_ISSUER_DOMAIN` belongs on the Convex deployment rather than the browser.

## References

[TanStack Start](https://tanstack.com/start/latest/docs/framework/react/overview), [Clerk for TanStack Start](https://clerk.com/docs/tanstack-react-start/getting-started/quickstart), and [Convex with Clerk](https://docs.convex.dev/auth/clerk).

The public [Turnitin homepage](https://www.turnitin.com/) informed the product themes. The workspace is an original conceptual design; no private Turnitin account was reproduced.
