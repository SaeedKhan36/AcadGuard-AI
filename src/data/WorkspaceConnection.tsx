import type { ReactNode } from "react";
import { useConvexAuth, useQuery } from "convex/react";
import { api } from "../../convex/_generated/api";
import { useBackendConfigured } from "./DataProvider";
import { AuthLoading } from "../auth/RequireAuth";
import { Link } from "../components/Navigation";
import type {
  DocumentItem,
  WorkspacePreferences,
} from "../../shared/workspace";

export function WorkspaceConnection({
  children,
}: {
  children: (data: {
    documents: DocumentItem[];
    preferences: WorkspacePreferences | null;
  }) => ReactNode;
}) {
  const configured = useBackendConfigured();
  if (!configured)
    return (
      <main className="auth-status">
        <h1>Your workspace isn’t connected yet</h1>
        <p>
          Finish connecting Convex to save your submissions, feedback, and
          preferences.
        </p>
        <Link className="button button-dark" to="/">
          Back to home
        </Link>
      </main>
    );
  return <AuthenticatedConnection>{children}</AuthenticatedConnection>;
}
function AuthenticatedConnection({
  children,
}: {
  children: (data: {
    documents: DocumentItem[];
    preferences: WorkspacePreferences | null;
  }) => ReactNode;
}) {
  const { isLoading, isAuthenticated } = useConvexAuth();
  if (isLoading) return <AuthLoading />;
  if (!isAuthenticated)
    return (
      <main className="auth-status">
        <h1>Connect your secure session</h1>
        <p>
          The Clerk and Convex connection is not ready. Complete the Clerk
          Convex integration, then reload this page.
        </p>
        <button
          className="button button-dark"
          onClick={() => window.location.reload()}
        >
          Try again
        </button>
        <Link to="/" className="back-link">
          Back to home
        </Link>
      </main>
    );
  return <LoadedConnection>{children}</LoadedConnection>;
}
function LoadedConnection({
  children,
}: {
  children: (data: {
    documents: DocumentItem[];
    preferences: WorkspacePreferences | null;
  }) => ReactNode;
}) {
  const documents = useQuery(api.documents.list, {});
  const preferences = useQuery(api.preferences.get, {});
  if (documents === undefined || preferences === undefined)
    return <AuthLoading />;
  return children({ documents, preferences });
}
