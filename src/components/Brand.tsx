import { ArrowUpRight } from "lucide-react";
export function Brand({ light = false }: { light?: boolean }) {
  return (
    <a
      className={`brand ${light ? "brand-light" : ""}`}
      href="#/"
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
    </a>
  );
}
export function ArrowLink({
  children,
  href = "#/app",
}: {
  children: React.ReactNode;
  href?: string;
}) {
  return (
    <a className="arrow-link" href={href}>
      {children}
      <ArrowUpRight size={17} />
    </a>
  );
}
