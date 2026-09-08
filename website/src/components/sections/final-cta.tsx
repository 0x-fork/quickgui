import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Logo } from "../logo";
import { site } from "../../lib/site";

export function FinalCta() {
  const { t, i18n } = useTranslation();
  const prefix = i18n.language === "en" ? "" : `/${i18n.language}`;

  return (
    <section className="relative">
      <div
        aria-hidden
        className="bg-dots absolute inset-0 [mask-image:radial-gradient(ellipse_60%_80%_at_50%_50%,black,transparent)]"
      />
      <div className="relative flex flex-col items-center px-6 py-28 text-center sm:py-36">
        <Logo className="size-11" />
        <h2 className="mt-8 max-w-xl text-4xl font-semibold tracking-tight text-balance sm:text-5xl">
          {t("cta.title")}
        </h2>
        <p className="mt-5 max-w-lg text-base leading-relaxed text-muted-foreground sm:text-lg">
          {t("cta.body")}
        </p>
        <div className="mt-10 flex flex-wrap items-center justify-center gap-3">
          <Button asChild variant="outline" className="h-10 gap-2 px-5 text-sm">
            <a href={`${prefix}${site.links.docs}`}>
              <span className="i-simple-icons-go size-6 text-[#00add8]" aria-hidden />
              {t("common.docsFor", { language: "Go" })}
              <span className="i-lucide-arrow-up-right size-4" aria-hidden />
            </a>
          </Button>
          <Button asChild variant="outline" className="h-10 gap-2 px-5 text-sm">
            <a href={`${prefix}/docs/moonbit`}>
              <span
                className="size-5 shrink-0 bg-[#7c3aed] [mask:url(/icons/moonbit.svg)_center/contain_no-repeat]"
                aria-hidden
              />
              {t("common.docsFor", { language: "MoonBit" })}
              <span className="i-lucide-arrow-up-right size-4" aria-hidden />
            </a>
          </Button>
          <Button asChild variant="outline" className="h-10 gap-2 px-5 text-sm">
            <a href={site.links.github} target="_blank" rel="noreferrer">
              <span className="i-simple-icons-github size-4" aria-hidden />
              {t("cta.star")}
            </a>
          </Button>
        </div>
      </div>
    </section>
  );
}
