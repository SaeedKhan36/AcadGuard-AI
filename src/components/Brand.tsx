import { Link } from "react-router-dom";
import { ArrowUpRight } from "lucide-react";
export function Brand({ light = false }: { light?: boolean }) {
  return (
    <Link
      className={`brand ${light ? "brand-light" : ""}`}
      to="/"
      aria-label="Turnitin home"
    >
      <span className="brand-symbol">
        <span />
        <span />
        <span />
      </span>
      <span>
        turnitin<span className="brand-dot">®</span>
      </span>
    </Link>
  );
}
export function ArrowLink({
  children,
  href = "/dashboard",
}: {
  children: React.ReactNode;
  href?: string;
}) {
  return (
    <Link className="arrow-link" to={href}>
      {children}
      <ArrowUpRight size={17} />
    </Link>
  );
}
