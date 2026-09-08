import { useTranslation } from "react-i18next";
import { site } from "../lib/site";
import type { Locale } from "../i18n";

const LINKS = [
  { labelKey: "nav.docs", href: site.links.docs, external: false },
  { label: "GitHub", href: site.links.github, external: true },
  { label: "Go SDK", href: site.links.go, external: true },
  { label: "MoonBit SDK", href: site.links.moonbit, external: true },
] as const;

export function SiteFooter() {
  const { t, i18n } = useTranslation();
  const current = (i18n.language as Locale) ?? "en";

  return (
    <footer>
      <div className="rail-joints rail-joints-top mx-auto flex w-full max-w-6xl flex-col justify-between gap-4 border-x border-t border-border px-6 py-8 sm:flex-row sm:items-center sm:px-12">
        <span className="font-mono text-xs text-muted-foreground/80">
          © 2026 EGOIST · {t("footer.license")}
        </span>
        <nav className="flex flex-wrap items-center gap-x-6 gap-y-3 text-sm text-muted-foreground">
          {LINKS.map((link) => (
            <a
              key={link.href}
              href={link.external || current === "en" ? link.href : `/${current}${link.href}`}
              target={link.external ? "_blank" : undefined}
              rel={link.external ? "noreferrer" : undefined}
              className="transition-colors hover:text-foreground"
            >
              {"labelKey" in link ? t(link.labelKey) : link.label}
            </a>
          ))}
        </nav>
      </div>
    </footer>
  );
}
