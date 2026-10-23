import { useEffect } from "react";
import {
  createFileRoute,
  useLocation,
  useNavigate,
} from "@tanstack/react-router";
import Landing from "../pages/Landing";
import { legacyDestination } from "../auth/routing";
import { destination } from "../components/Navigation";

export const Route = createFileRoute("/")({ component: Home });
function Home() {
  const hash = useLocation({ select: (location) => location.hash });
  const navigate = useNavigate();
  useEffect(() => {
    if (!hash) return;
    const legacy = legacyDestination(hash.startsWith("#") ? hash : "#" + hash);
    if (legacy) void navigate({ ...destination(legacy), replace: true });
  }, [hash, navigate]);
  return <Landing />;
}
