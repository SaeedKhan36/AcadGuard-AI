import { useEffect } from "react";
import { Navigate, Route, Routes, useLocation } from "react-router-dom";
import RequireAuth from "./auth/RequireAuth";
import { legacyDestination } from "./auth/routing";
import Landing from "./pages/Landing";
import Login from "./pages/Login";
import Workspace from "./pages/Workspace";

function ProtectedWorkspace() {
  const { pathname, search } = useLocation();
  return (
    <RequireAuth>
      <Workspace route={pathname.replace(/^\/dashboard/, "#/app") + search} />
    </RequireAuth>
  );
}

export default function App() {
  const location = useLocation();
  const legacy =
    location.pathname === "/" ? legacyDestination(location.hash) : null;
  useEffect(() => {
    document.title = location.pathname.startsWith("/dashboard")
      ? "Workspace — Turnitin UI Concept"
      : location.pathname.startsWith("/auth")
        ? "Welcome — Turnitin UI Concept"
        : "Turnitin — Original thinking. Lasting impact.";
    if (location.hash && !location.hash.startsWith("#/")) {
      document
        .getElementById(decodeURIComponent(location.hash.slice(1)))
        ?.scrollIntoView({ behavior: "smooth" });
    } else {
      window.scrollTo(0, 0);
    }
  }, [location.pathname, location.hash]);
  if (legacy) return <Navigate to={legacy} replace />;
  return (
    <Routes>
      <Route path="/" element={<Landing />} />
      <Route path="/auth/login/*" element={<Login />} />
      <Route path="/auth/register/*" element={<Login register />} />
      <Route path="/dashboard/*" element={<ProtectedWorkspace />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
