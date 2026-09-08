import { useEffect, useState } from "react";
import { I18nextProvider, useTranslation } from "react-i18next";
import type { Route } from "./+types/home";
import { SiteHeader } from "../components/site-header";
import { SiteFooter } from "../components/site-footer";
import { Hero } from "../components/sections/hero";
import { Features } from "../components/sections/features";
import { Benchmarks } from "../components/sections/benchmarks";
import { CodeShowcase } from "../components/sections/code-showcase";
import { SwiftUi } from "../components/sections/swift-ui";
import { Quickstart } from "../components/sections/quickstart";
import { Platforms } from "../components/sections/platforms";
import { FinalCta } from "../components/sections/final-cta";
import { getHighlightedSnippets } from "../server/highlight.server";
import { fetchRepoStats } from "../server/stats.server";
import { siteMeta } from "../lib/meta";
import type { DocsFrontend } from "../lib/docs";
import {
  OG_LOCALES,
  SUPPORTED_LOCALES,
  createI18n,
  localePath,
  messages,
  resolveLocale,
} from "../i18n";

export async function loader({ params, request }: Route.LoaderArgs) {
  const locale = resolveLocale(params.locale);
  if (locale === null) {
    throw new Response("Page not found", { status: 404 });
  }

  const [highlighted, stats] = await Promise.all([getHighlightedSnippets(), fetchRepoStats()]);

  return {
    highlighted,
    locale,
    origin: new URL(request.url).origin,
    stats,
  };
}

export const meta: Route.MetaFunction = ({ loaderData, params }) => {
  const locale = loaderData?.locale ?? resolveLocale(params.locale) ?? "en";
  const origin = loaderData?.origin ?? "";
  const localizedMeta = messages[locale].meta;

  return [
    ...siteMeta(origin, localizedMeta.title),
    { name: "description", content: localizedMeta.description },
    { property: "og:title", content: localizedMeta.title },
    { property: "og:description", content: localizedMeta.description },
    { property: "og:locale", content: OG_LOCALES[locale] },
    ...SUPPORTED_LOCALES.filter((other) => other !== locale).map((other) => ({
      property: "og:locale:alternate",
      content: OG_LOCALES[other],
    })),
    {
      tagName: "link",
      rel: "canonical",
      href: `${origin}${localePath(locale)}`,
    },
    ...SUPPORTED_LOCALES.map((other) => ({
      tagName: "link" as const,
      rel: "alternate",
      hrefLang: other,
      href: `${origin}${localePath(other)}`,
    })),
    {
      tagName: "link",
      rel: "alternate",
      hrefLang: "x-default",
      href: `${origin}/`,
    },
  ];
};

function SkipLink() {
  const { t } = useTranslation();
  return (
    <a
      href="#features"
      className="sr-only focus:not-sr-only focus:fixed focus:top-2 focus:left-2 focus:z-[60] focus:bg-primary focus:px-3 focus:py-2 focus:text-sm focus:text-primary-foreground"
    >
      {t("common.skipToContent")}
    </a>
  );
}

export default function Home({ loaderData }: Route.ComponentProps) {
  const { highlighted, locale, stats } = loaderData;
  const [i18n] = useState(() => createI18n(locale));
  const [frontend, setFrontend] = useState<DocsFrontend>("go");

  useEffect(() => {
    if (i18n.language !== locale) void i18n.changeLanguage(locale);
  }, [i18n, locale]);

  return (
    <I18nextProvider i18n={i18n}>
      <SkipLink />
      <SiteHeader stats={stats} />
      <main>
        {/* One bordered column runs the whole page; sections stack flush,
            separated by hairlines. */}
        <div className="mx-auto w-full max-w-6xl border-x border-border">
          <Hero frontend={frontend} onFrontendChange={setFrontend} />
          <Benchmarks />
          <Features />
          <CodeShowcase
            highlighted={highlighted}
            frontend={frontend}
            onFrontendChange={setFrontend}
          />
          <SwiftUi highlighted={highlighted} frontend={frontend} onFrontendChange={setFrontend} />
          <Quickstart
            highlighted={highlighted}
            frontend={frontend}
            onFrontendChange={setFrontend}
          />
          <Platforms />
          <FinalCta />
        </div>
      </main>
      <SiteFooter />
    </I18nextProvider>
  );
}
