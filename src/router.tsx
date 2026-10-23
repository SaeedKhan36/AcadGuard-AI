import { createRouter } from "@tanstack/react-router";
import { parseSearch, stringifySearch } from "./utils/search";
import { routeTree } from "./routeTree.gen";
import { AuthLoading } from "./auth/RequireAuth";

export function getRouter() {
  return createRouter({
    routeTree,
    parseSearch,
    stringifySearch,
    scrollRestoration: true,
    defaultPreload: "intent",
    defaultPendingComponent: AuthLoading,
    defaultErrorComponent: ({ reset }) => (
      <main className="auth-status">
        <h1>We couldn’t load this page</h1>
        <p>Please try again. Your account and saved work haven’t changed.</p>
        <button className="button button-dark" onClick={reset}>
          Try again
        </button>
        <a className="back-link" href="/">
          Back to home
        </a>
      </main>
    ),
    defaultNotFoundComponent: () => (
      <main className="auth-status">
        <h1>This page has moved</h1>
        <a className="button button-dark" href="/">
          Back to home
        </a>
      </main>
    ),
  });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof getRouter>;
  }
}
