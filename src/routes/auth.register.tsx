import { createFileRoute, redirect } from "@tanstack/react-router";
import Login from "../pages/Login";
import { getSession } from "../auth/session";
import { authDestination } from "../auth/routing";
import { destination } from "../components/Navigation";

export const Route = createFileRoute("/auth/register")({
  beforeLoad: async ({ location }) => {
    if ((await getSession()).userId)
      throw redirect(destination(authDestination(location.searchStr)));
  },
  head: () => ({ meta: [{ title: "Create an account — AcadGuard AI" }] }),
  component: () => <Login register />,
});
