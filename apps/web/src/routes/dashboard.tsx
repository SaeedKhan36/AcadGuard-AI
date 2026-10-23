import {
  ClientOnly,
  createFileRoute,
  redirect,
  useLocation,
} from "@tanstack/react-router";
import Workspace from "../pages/Workspace";
import RequireAuth, { AuthLoading } from "../auth/RequireAuth";
import { getSession } from "../auth/session";

export const Route = createFileRoute("/dashboard")({
  beforeLoad: async ({ location }) => {
    const session = await getSession();
    if (!session.userId)
      throw redirect({
        to: "/auth/login",
        search: { redirect_url: location.href },
      });
    return { userId: session.userId };
  },
  head: () => ({ meta: [{ title: "Workspace — AcadGuard AI" }] }),
  component: Dashboard,
});
function Dashboard() {
  const { pathname, searchStr } = useLocation();
  return (
    <RequireAuth>
      <ClientOnly fallback={<AuthLoading />}>
        <Workspace
          route={pathname.replace(/^\/dashboard/, "#/app") + searchStr}
        />
      </ClientOnly>
    </RequireAuth>
  );
}
