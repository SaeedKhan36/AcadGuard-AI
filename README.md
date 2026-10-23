# AcadGuard AI: An Intelligent Academic Integrity Platform for Plagiarism Detection, Semantic Similarity Analysis, and Citation Verification

A responsive academic integrity prototype built with **TanStack Start, TanStack Router, React, Clerk, and Convex**. The public landing page and educator workspace retain the original ivory, forest, sage, and lime design.

Authentication is provided by Clerk. Convex stores each signed-in user's submission metadata, feedback, and workspace preferences. **Similarity scores, AI indicators, source matches, classes, and report content are illustrative.** This project does not perform plagiarism analysis, AI detection, citation verification, or integrate the separate Rust backend. It is an independent concept, unaffiliated with Turnitin.

![Landing page preview](docs/landing-preview.jpg)

## Run locally

Requires Node.js 22.12+.

```sh
npm install
cp .env.example .env.local
```

Set your Clerk keys in `.env.local`. If this checkout already has a working `.env.local`, edit it instead of replacing it.

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
npx convex login
npx convex dev --configure existing --once
```

The CLI records the deployment and frontend URL in `.env.local`. Check the selected project before deploying: Convex deploys the schema and function definitions from this repository. If sharing a project with another application, merge its existing schema/functions into this checkout first. This app uses `acadguardDocuments`, `acadguardFeedback`, and `acadguardPreferences` table names.

On that Convex deployment, set `CLERK_JWT_ISSUER_DOMAIN` to your Clerk Frontend API URL (for example, `https://your-instance.clerk.accounts.dev`). Enable the Convex integration in Clerk, or create a Clerk JWT template named `convex` from its Convex preset with audience `convex`. The issuer must match the Clerk application used by this UI.

```sh
npx convex env set CLERK_JWT_ISSUER_DOMAIN https://your-instance.clerk.accounts.dev
npm run convex:dev
```

In another terminal:

```sh
npm run dev
```

Open [the local app](http://127.0.0.1:5173/). The landing and authentication pages work before Convex is connected; the protected workspace displays a connection notice until its configuration is ready. Samples are loaded only when the user chooses **Load sample documents**.

## Build and validation

```sh
npm test
npm run build
npm start
```

The build checks both application and Convex TypeScript and produces a server application in `.output/`. `npm start` serves that build (default port 3000); this is no longer a static SPA. The tests cover safe return URLs, query compatibility, authentication requirements, cross-user document/feedback isolation, preferences, and input validation using `convex-test`.

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
src/
  start.ts                   Start request middleware (CSRF and Clerk)
  router.tsx                 TanStack Router configuration
  routes/                    File routes, HTML shell, server authentication guards
  auth/                      Clerk theme, session server function, safe redirects
  data/                      Authenticated Convex provider and workspace queries
  components/                Shared links, branding, report composition
  pages/                     Landing, login, and workspace UI
  utils/                     Query parameter compatibility helpers
  styles.css                 Responsive visual system
convex/
  schema.ts                  Namespaced tables and indexes
  auth.config.ts             Clerk issuer/audience configuration
  documents.ts               Owned submission metadata and explicit sample loading
  feedback.ts                Owned feedback queries and mutations
  preferences.ts             Owned preference queries and mutations
  lib/auth.ts                Shared authentication and ownership checks
  _generated/                Typed function and data model helpers
  workspace.test.ts          Backend isolation and validation tests
shared/workspace.ts          Shared UI data types and sample fixtures
```

Run `npm run convex:codegen` after linking the existing project or changing backend functions. Do not edit the generated TanStack route tree.

## Vercel

`vercel.json` selects the `tanstack-start` framework. Nitro builds the server and assets for Vercel; remove any project-level legacy `dist` output override and SPA rewrite. Deep links are handled by Start's server router.

Add the Clerk public variables, server-only `CLERK_SECRET_KEY`, and `VITE_CONVEX_URL` to each relevant Vercel environment. Use matching Clerk/Convex environments. Browser variables are embedded during the build, so changing them requires redeployment. Production should use a configured Clerk production application.

Deploy Convex functions to the intended production deployment separately with `npm run convex:deploy`, then deploy the UI. If the existing Convex deployment has other functions, merge and preserve them before deploying. `CLERK_JWT_ISSUER_DOMAIN` belongs on the Convex deployment rather than the browser.

## References

[TanStack Start](https://tanstack.com/start/latest/docs/framework/react/overview), [Clerk for TanStack Start](https://clerk.com/docs/tanstack-react-start/getting-started/quickstart), and [Convex with Clerk](https://docs.convex.dev/auth/clerk).

The public [Turnitin homepage](https://www.turnitin.com/) informed the product themes. The workspace is an original conceptual design; no private Turnitin account was reproduced.
