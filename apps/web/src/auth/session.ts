import { createServerFn } from "@tanstack/react-start";
import { auth } from "@clerk/tanstack-react-start/server";

/** Read only public session metadata; Clerk tokens and secrets stay on the server. */
export const getSession = createServerFn({ method: "GET" }).handler(
  async () => {
    const { userId } = await auth();
    return { userId };
  },
);
