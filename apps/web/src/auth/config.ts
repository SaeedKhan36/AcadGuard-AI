export const authConfig = {
  publishableKey: import.meta.env.VITE_CLERK_PUBLISHABLE_KEY as
    string | undefined,
  signInUrl: import.meta.env.VITE_CLERK_SIGN_IN_URL || "/auth/login",
  signUpUrl: import.meta.env.VITE_CLERK_SIGN_UP_URL || "/auth/register",
  afterSignInUrl: import.meta.env.VITE_CLERK_AFTER_SIGN_IN_URL || "/dashboard",
  afterSignUpUrl: import.meta.env.VITE_CLERK_AFTER_SIGN_UP_URL || "/dashboard",
};
