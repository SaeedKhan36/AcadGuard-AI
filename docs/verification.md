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
