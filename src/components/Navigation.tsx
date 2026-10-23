import type { AnchorHTMLAttributes } from "react";
import {
  Link as RouterLink,
  Navigate as RouterNavigate,
  useLocation,
  useNavigate,
} from "@tanstack/react-router";

export function destination(to: string, pathname = "/") {
  const url = new URL(
    to.startsWith("#") ? pathname + to : to,
    "http://app.local",
  );
  return {
    to: url.pathname,
    search: Object.fromEntries(url.searchParams),
    hash: url.hash.slice(1),
  };
}

/** Shared link adapter preserves existing query and section links during migration. */
export function Link({
  to,
  ...props
}: Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "href"> & { to: string }) {
  const pathname = useLocation({ select: (location) => location.pathname });
  if (/^(https?:|mailto:|tel:)/.test(to)) return <a href={to} {...props} />;
  return <RouterLink {...destination(to, pathname)} {...props} />;
}

export function Navigate({ to, replace }: { to: string; replace?: boolean }) {
  return <RouterNavigate {...destination(to)} replace={replace} />;
}

export function useAppNavigate() {
  const navigate = useNavigate();
  return (to: string, options?: { replace?: boolean }) => {
    void navigate({ ...destination(to), ...options });
  };
}
