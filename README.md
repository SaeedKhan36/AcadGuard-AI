# Turnitin UI — Original thinking. Lasting impact.

A responsive, independently designed frontend inspired by Turnitin’s public product offering, with Clerk authentication. Warm ivory, forest ink, soft sage, and lime create a distinct visual system for the marketing site and educator workspace.

**This is an independent prototype, not an official Turnitin service.** Authentication is real and provided by Clerk. Documents, originality scores, AI indicators, and reports remain illustrative. No plagiarism checking, AI detection, document transmission, or Rust backend integration is implemented. Turnitin and its product names belong to their respective owners.

![Landing page preview](docs/landing-preview.jpg)

## Run locally

Requires Node.js 22.12+.

```sh
npm install
cp .env.example .env.local
```

Replace the placeholder publishable key in `.env.local` with your own Clerk publishable key. The supplied application is already configured in the current local checkout; do not overwrite its working `.env.local`.

```sh
npm run dev
```

Open the local URL printed by Vite, normally http://127.0.0.1:5173/.

```sh
npm test
npm run build
npm run preview
```

The build performs strict TypeScript checking and produces static assets in `dist/`. The tests check compatibility redirects, workspace return paths, and rejection of external redirect destinations.

## Clerk configuration

This is a React + Vite app, so browser configuration uses `VITE_` variables rather than Next.js `NEXT_PUBLIC_` variables:

| Supplied setting                      | Vite setting                   | Default          |
| ------------------------------------- | ------------------------------ | ---------------- |
| `NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY`   | `VITE_CLERK_PUBLISHABLE_KEY`   | Required         |
| `NEXT_PUBLIC_CLERK_SIGN_IN_URL`       | `VITE_CLERK_SIGN_IN_URL`       | `/auth/login`    |
| `NEXT_PUBLIC_CLERK_SIGN_UP_URL`       | `VITE_CLERK_SIGN_UP_URL`       | `/auth/register` |
| `NEXT_PUBLIC_CLERK_AFTER_SIGN_IN_URL` | `VITE_CLERK_AFTER_SIGN_IN_URL` | `/dashboard`     |
| `NEXT_PUBLIC_CLERK_AFTER_SIGN_UP_URL` | `VITE_CLERK_AFTER_SIGN_UP_URL` | `/dashboard`     |

- Only the publishable key is used by this frontend. `CLERK_SECRET_KEY` is not needed, stored, bundled, or committed. Keep it in your Rust server environment when server integration is added.
- `.env*` files are ignored by Git, except for the placeholder-only `.env.example`.
- `ClerkProvider` supplies session state, themed sign-in/sign-up, user account management, and sign-out. The sign-in methods shown are controlled by the existing Clerk application's settings.
- `/dashboard` and every child route wait for Clerk to load, then require a signed-in session. Signed-out visitors return to sign-in with their intended workspace URL preserved.
- The landing page is public. Visiting sign-in or registration while signed in returns to the workspace.
- Sign-out returns to `/`. Authenticated users see their Clerk name and account menu in the workspace.
- Use a development publishable key for local testing and a production key for a configured production instance. Never prefix a Clerk secret key with `VITE_`.

Reference: [Clerk React quickstart](https://clerk.com/docs/react/getting-started/quickstart) and [custom sign-in/sign-up pages](https://clerk.com/docs/react/guides/development/custom-sign-in-or-up-page).

## Screens

| Route                    | Experience                                                                               |
| ------------------------ | ---------------------------------------------------------------------------------------- |
| `/`                      | Public landing page with audience tabs, solutions, product ecosystem, resources, and FAQ |
| `/auth/login/*`          | Clerk sign-in and nested authentication steps                                            |
| `/auth/register/*`       | Clerk registration and verification steps                                                |
| `/dashboard`             | Protected educator overview and recent submissions                                       |
| `/dashboard/submissions` | Searchable sample submissions and local demo file selection                              |
| `/dashboard/reports`     | Originality report list and detailed report reader                                       |
| `/dashboard/ai`          | Sample AI writing indicators with contextual guidance                                    |
| `/dashboard/classes`     | Classes, assignments, and class-specific submission search                               |
| `/dashboard/feedback`    | Sample document, editable feedback, and review checklist                                 |
| `/dashboard/resources`   | Educational guides and article dialogs                                                   |
| `/dashboard/settings`    | Demo workspace preferences; use the Clerk account menu for actual account settings       |

Old links such as `/#/app/reports` redirect to the matching protected `/dashboard/reports` route. They do not bypass authentication.

## Demo data and persistence

- All students, documents, classes, scores, source matches, highlights, and activity are illustrative.
- File selection accepts PDF, DOCX, and TXT up to 20 MB. Only filename and size are used. Files are not read, sent, stored, or analyzed.
- Adding a submission generates fixed demo scores.
- Submission, feedback, and workspace preference state is in React memory and resets on refresh or leaving the workspace. Clerk handles the actual user session separately.
- Passwords and session credentials are handled by Clerk. The application does not implement custom password storage.
- Dates and assignment schedules are fixed sample content for October 2026.
- Google Fonts loads DM Sans and Manrope; system fallbacks work offline. Icons use Lucide. Visual compositions are CSS and inline SVG.

## Structure

```text
src/
  App.tsx                   Browser routes and page selection
  main.tsx                  React, router, and auth entry point
  auth/
    AuthProvider.tsx        Clerk configuration and theme
    RequireAuth.tsx         Session loading and workspace protection
    config.ts               Public environment configuration
    routing.ts              Safe return URLs and legacy redirects
    routing.test.ts         Redirect regression tests
  components/
    Brand.tsx               Concept wordmark and shared link
    ReportPreview.tsx       Landing page report composition
  pages/
    Landing.tsx             Public marketing site
    Login.tsx               Clerk sign-in and registration
    Workspace.tsx           Protected screens and demo interactions
  styles.css                Shared visual system and responsive styles
public/favicon.svg          Custom concept favicon
docs/sample-submission.txt  Safe fixture for checking the local file flow
```

## Hosting and future Rust backend integration

Browser routes need an SPA fallback: serve `index.html` for `/auth/*` and `/dashboard/*`, while serving asset files normally. Vite's development and preview servers support this automatically. Set the public environment variables in the hosting service before building.

The route guard protects the UI only. Once the Rust backend is connected, validate Clerk session JWTs on **every protected API endpoint** before accessing data; frontend route guards do not authorize server requests. Replace the in-memory documents and feedback with authenticated API calls at that stage. No backend folder was accessed or changed.

## Content reference

The public [Turnitin homepage](https://www.turnitin.com/) informed the product themes: learning integrity, similarity, AI writing insights, feedback, research integrity, grading, and secure assessment. The internal workspace is an original conceptual design, not a reproduction of a private Turnitin account. Marketing copy was rewritten for this prototype.
