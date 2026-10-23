import { SignIn, SignUp, useAuth } from "@clerk/react";
import { ArrowLeft } from "lucide-react";
import { Link, Navigate, useLocation } from "react-router-dom";
import { Brand } from "../components/Brand";
import { authConfig } from "../auth/config";
import { authDestination } from "../auth/routing";
import { AuthLoading } from "../auth/RequireAuth";

export default function Login({ register = false }: { register?: boolean }) {
  const { isLoaded, isSignedIn } = useAuth();
  const { search } = useLocation();
  const destination = authDestination(
    search,
    register ? authConfig.afterSignUpUrl : authConfig.afterSignInUrl,
  );
  if (isLoaded && isSignedIn) return <Navigate to={destination} replace />;
  return (
    <main className="login-page">
      <div className="login-side">
        <Brand light />
        <div>
          <span className="eyebrow">YOUR NEXT CHAPTER</span>
          <h1>
            {register ? "Make room" : "Good to"}
            <br />
            {register ? (
              <>
                for <em>your</em>
              </>
            ) : (
              <>
                have <em>you</em>
              </>
            )}
            <br />
            {register ? "ideas." : "here."}
          </h1>
          <p>
            A little clarity. A lot of possibility.
            <br />
            Let’s make room for original thinking.
          </p>
        </div>
        <span className="login-side-bottom">
          For educators. For students. For integrity. <span>✳</span>
        </span>
      </div>
      <div className="login-main">
        <Link className="back-link" to="/">
          <ArrowLeft size={15} /> Back to home
        </Link>
        <div className="login-card auth-login-card">
          <div className="eyebrow">A SPACE FOR ORIGINAL MINDS</div>
          <h2>
            {register
              ? "Your next chapter starts here."
              : "Welcome to your workspace."}
          </h2>
          <p>
            {register
              ? "Create an account to start exploring."
              : "Sign in to pick up where you left off."}
          </p>
          <div className="clerk-auth-container">
            {!isLoaded ? (
              <AuthLoading />
            ) : register ? (
              <SignUp
                routing="path"
                path={authConfig.signUpUrl}
                signInUrl={`${authConfig.signInUrl}?redirect_url=${encodeURIComponent(destination)}`}
                forceRedirectUrl={destination}
              />
            ) : (
              <SignIn
                routing="path"
                path={authConfig.signInUrl}
                signUpUrl={`${authConfig.signUpUrl}?redirect_url=${encodeURIComponent(destination)}`}
                forceRedirectUrl={destination}
              />
            )}
          </div>
        </div>
        <span className="login-disclaimer">
          Independent prototype · Not an official Turnitin service
        </span>
      </div>
    </main>
  );
}
