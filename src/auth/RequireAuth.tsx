import { useAuth } from "@clerk/tanstack-react-start";
import { useEffect, useState } from "react";
import { useLocation } from "@tanstack/react-router";
import { Navigate } from "../components/Navigation";
import { authConfig } from "./config";

export function AuthLoading() {
  const [delayed, setDelayed] = useState(false);
  useEffect(() => {
    const timer = window.setTimeout(() => setDelayed(true), 12000);
    return () => window.clearTimeout(timer);
  }, []);
  return (
    <div className="auth-status" role="status" aria-live="polite">
      <span className="auth-spinner" />
      <h1>
        {delayed
          ? "Sign-in is taking a little longer"
          : "Getting your workspace ready"}
      </h1>
      <p>
        {delayed
          ? "Check your connection and Clerk application settings, then try again."
          : "Checking your secure session…"}
      </p>
      {delayed && (
        <button
          className="button button-dark"
          onClick={() => window.location.reload()}
        >
          Try again
        </button>
      )}
    </div>
  );
}

export default function RequireAuth({
  children,
}: {
  children: React.ReactNode;
}) {
  const { isLoaded, isSignedIn } = useAuth();
  const location = useLocation();
  if (!isLoaded) return <AuthLoading />;
  if (!isSignedIn) {
    const destination =
      location.pathname +
      location.searchStr +
      (location.hash ? "#" + location.hash : "");
    return (
      <Navigate
        to={`${authConfig.signInUrl}?redirect_url=${encodeURIComponent(destination)}`}
        replace
      />
    );
  }
  return children;
}
