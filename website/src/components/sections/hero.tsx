import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { CopyButton } from "../copy-button";
import { site } from "../../lib/site";
import { FrontendPicker } from "../frontend-picker";
import type { DocsFrontend } from "../../lib/docs";

export function Hero({
  frontend,
  onFrontendChange,
}: {
  frontend: DocsFrontend;
  onFrontendChange: (frontend: DocsFrontend) => void;
}) {
  const { t, i18n } = useTranslation();
  const prefix = i18n.language === "en" ? "" : `/${i18n.language}`;
  const docsHref = `${prefix}/docs/${frontend}`;
  const initCommand = `bunx @quickgui/cli init my-app${frontend === "moonbit" ? " --frontend moonbit" : ""}`;

  return (
    <section className="border-b border-border">
      <div className="px-6 py-24 sm:px-12 sm:py-32">
        <a
          href={docsHref}
          className="animate-fade-up inline-flex items-center gap-2 border border-border bg-card-2 px-3 py-1.5 font-mono text-xs text-muted-foreground transition-colors hover:text-foreground"
        >
          <span className="size-1.5 animate-pulse rounded-full bg-mint" />
          {t("hero.badge")}
          <span className="i-lucide-arrow-up-right size-3" aria-hidden />
        </a>

        <h1 className="animate-fade-up mt-8 max-w-4xl text-5xl leading-[1.08] font-semibold tracking-tight text-balance [animation-delay:60ms] lg:text-6xl">
          <span className="block">{t("hero.titleLine1")}</span>
          <span className="block">{t("hero.titleLine2")}</span>
        </h1>

        <p className="animate-fade-up mt-6 max-w-2xl text-base leading-relaxed text-muted-foreground [animation-delay:120ms] sm:text-lg">
          {t("hero.sub")}
        </p>

        <div className="animate-fade-up mt-10 flex flex-wrap items-center gap-3 [animation-delay:180ms]">
          <Button asChild variant="outline" className="h-10 gap-2 px-5 text-sm">
            <a href={`${prefix}${site.links.docs}`}>
              <span className="i-simple-icons-go size-6 text-[#00add8]" aria-hidden />
              {t("common.docsFor", { language: "Go" })}
              <span className="i-lucide-arrow-right size-4" aria-hidden />
            </a>
          </Button>
          <Button asChild variant="outline" className="h-10 gap-2 px-5 text-sm">
            <a href={`${prefix}/docs/moonbit`}>
              <span
                className="size-5 shrink-0 bg-[#7c3aed] [mask:url(/icons/moonbit.svg)_center/contain_no-repeat]"
                aria-hidden
              />
              {t("common.docsFor", { language: "MoonBit" })}
              <span className="i-lucide-arrow-right size-4" aria-hidden />
            </a>
          </Button>
        </div>
        <div className="animate-fade-up mt-6 flex flex-wrap items-center gap-3 [animation-delay:180ms]">
          <FrontendPicker value={frontend} onChange={onFrontendChange} />
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
