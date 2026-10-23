import { ClerkProvider } from "@clerk/tanstack-react-start";
import { useAppNavigate } from "../components/Navigation";
import { authConfig } from "./config";

export default function AuthProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const navigate = useAppNavigate();
  if (!authConfig.publishableKey) {
    return (
      <main className="auth-status">
        <h1>Connect your Clerk application</h1>
        <p>
          Add VITE_CLERK_PUBLISHABLE_KEY to .env.local, then restart the
          development server.
        </p>
        <p>See the project README for setup instructions.</p>
      </main>
    );
  }
  return (
    <ClerkProvider
      publishableKey={authConfig.publishableKey}
      signInUrl={authConfig.signInUrl}
      signUpUrl={authConfig.signUpUrl}
      signInFallbackRedirectUrl={authConfig.afterSignInUrl}
      signUpFallbackRedirectUrl={authConfig.afterSignUpUrl}
      afterSignOutUrl="/"
      routerPush={(to) => navigate(to)}
      routerReplace={(to) => navigate(to, { replace: true })}
      appearance={{
        variables: {
          colorPrimary: "#24392f",
          colorBackground: "#fafbf7",
          colorForeground: "#263a31",
          colorMutedForeground: "#768078",
          colorInput: "#ffffff",
          colorInputForeground: "#263a31",
          borderRadius: "0.5rem",
          fontFamily: '"DM Sans", sans-serif',
        },
        elements: {
          cardBox: "clerk-card-box",
          card: "clerk-card",
          formButtonPrimary: "clerk-primary",
          footerActionLink: "clerk-footer-link",
        },
      }}
    >
      {children}
    </ClerkProvider>
  );
}
