import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { GitHubButton } from "./github-button";
import { LanguageMenu } from "./language-menu";
import { Logo } from "./logo";
import { site } from "../lib/site";
import type { RepoStats } from "../lib/stats";
import { localePath, type Locale } from "../i18n";
import type { DocsFrontend } from "../lib/docs";

const NAV_ITEMS = [
  { key: "nav.features", href: "#features" },
  { key: "nav.benchmarks", href: "#benchmarks" },
  { key: "nav.code", href: "#code" },
  { key: "nav.quickstart", href: "#quickstart" },
] as const;

export function SiteHeader({ stats, frontend }: { stats: RepoStats; frontend: DocsFrontend }) {
  const { t, i18n } = useTranslation();
  const current = (i18n.language as Locale) ?? "en";
  const docsHref = `${current === "en" ? "" : `/${current}`}/docs/${frontend}`;

  return (
    <header className="sticky top-0 z-50 bg-background/90 backdrop-blur-sm">
      <div className="rail-joints rail-joints-bottom mx-auto flex h-16 w-full max-w-6xl items-center justify-between gap-4 border-x border-b border-border px-6 sm:px-12">
        <a href={localePath(current)} className="flex items-center gap-2.5">
          <Logo />
          <span className="text-sm font-semibold tracking-tight">{site.name}</span>
        </a>

        <nav className="hidden items-center gap-7 text-sm text-muted-foreground lg:flex">
          {NAV_ITEMS.map((item) => (
            <a key={item.href} href={item.href} className="transition-colors hover:text-foreground">
              {t(item.key)}
            </a>
          ))}
          <a
            href={docsHref}
            className="transition-colors hover:text-foreground"
          >
            {t("nav.docs")}
          </a>
        </nav>

        <div className="flex items-center gap-2">
          <LanguageMenu locale={current} label={t("common.language")} hrefForLocale={localePath} />
          <GitHubButton stats={stats} />
          <Button asChild size="sm" className="hidden h-8 px-3.5 sm:inline-flex">
            <a href="#quickstart">{t("common.getStarted")}</a>
          </Button>
        </div>
      </div>
    </header>
  );
}
