import { useTranslation } from "react-i18next";
import { CopyButton } from "../copy-button";
import { FrontendDocsLinks } from "../frontend-docs-links";

export function Hero() {
  const { t } = useTranslation();
  const initCommand = "bunx @quickgui/cli init my-app";

  return (
    <section className="border-b border-border">
      <div className="px-6 py-24 sm:px-12 sm:py-32">
        <p className="inline-flex items-stretch border border-peach/30">
          <span className="flex items-center bg-peach px-2.5 py-1.5 font-mono text-[10px] leading-none font-semibold tracking-[0.16em] text-primary-foreground uppercase">
            {t("hero.badge")}
          </span>
          <span className="flex items-center bg-peach/[0.06] px-3 py-1.5 font-mono text-xs leading-none text-muted-foreground">
            {t("hero.badgeHint")}
          </span>
        </p>

        <h1 className="mt-8 max-w-4xl text-5xl leading-[1.08] font-semibold tracking-tight text-balance lg:text-6xl">
          <span className="block">{t("hero.titleLine1")}</span>
          <span className="block">{t("hero.titleLine2")}</span>
        </h1>

        <p className="mt-6 max-w-2xl text-base leading-relaxed text-muted-foreground sm:text-lg">
          {t("hero.sub")}
        </p>

        <div className="mt-10 flex flex-wrap items-center gap-3">
          <FrontendDocsLinks />
        </div>
        <div className="mt-6 flex flex-wrap items-center gap-3">
          <div className="flex min-h-10 max-w-full items-center gap-2 border border-border bg-card-2 pr-1 pl-3 font-mono text-xs sm:text-sm">
            <span className="text-peach select-none">$</span>
            <span className="min-w-0 overflow-x-auto whitespace-nowrap">{initCommand}</span>
            <CopyButton text={initCommand} className="size-8 shrink-0" />
          </div>
        </div>
      </div>
    </section>
  );
}
