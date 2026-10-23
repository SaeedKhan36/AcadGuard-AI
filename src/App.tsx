import { useEffect, useState } from "react";
import Landing from "./pages/Landing";
import Login from "./pages/Login";
import Workspace from "./pages/Workspace";
export default function App() {
  const [route, setRoute] = useState(window.location.hash || "#/");
  useEffect(() => {
    const change = () => {
      setRoute(window.location.hash || "#/");
      if (window.location.hash.startsWith("#/")) window.scrollTo(0, 0);
    };
    window.addEventListener("hashchange", change);
    return () => window.removeEventListener("hashchange", change);
  }, []);
  useEffect(() => {
    document.title = route.startsWith("#/app")
      ? "Workspace — Turnitin UI Concept"
      : route === "#/login"
        ? "Welcome — Turnitin UI Concept"
        : "Turnitin — Original thinking. Lasting impact.";
  }, [route]);
  return route.startsWith("#/app") ? (
    <Workspace route={route} />
  ) : route === "#/login" ? (
    <Login />
  ) : (
    <Landing />
  );
}
