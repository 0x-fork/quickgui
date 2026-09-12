import { useId } from "react";
import { Tabs } from "radix-ui";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { CopyButton } from "../copy-button";
import { AnimatedCodeBlock } from "../animated-code-block";
import { SectionHeading } from "../section-heading";
import type { FrontendTokens } from "../../lib/snippets";
import { docsPath, type DocsFrontend } from "../../lib/docs";

const PANES = [
  { key: "go", file: "counter.go", name: "Go", icon: "i-simple-icons-go" },
  { key: "typescript", file: "counter.tsx", name: "TypeScript", icon: "i-simple-icons-typescript" },
  { key: "rust", file: "main.rs", name: "Rust", icon: "i-simple-icons-rust" },
] as const;

export function CodeShowcase({
  tokens,
  frontend,
  onFrontendChange,
}: {
  tokens: FrontendTokens;
  frontend: DocsFrontend;
  onFrontendChange: (frontend: DocsFrontend) => void;
}) {
  const { t, i18n } = useTranslation();
  const panelId = useId();
  const step = PANES.findIndex((pane) => pane.key === frontend);
  const pane = PANES[step];
  const prefix = i18n.language === "en" ? "" : `/${i18n.language}`;

  return (
    <section id="code" className="border-b border-border">
      <SectionHeading title={t("code.title")} lead={t("code.lead")} />

      <Tabs.Root
        value={frontend}
        onValueChange={(value) => onFrontendChange(value as DocsFrontend)}
        className="bg-card-2 p-4 sm:p-8 lg:p-12"
      >
        <div className="min-w-0 border border-border bg-background">
          <Tabs.List
            aria-label={t("common.frontend")}
            className="flex border-b border-border bg-card-2"
          >
            {PANES.map((option) => (
              <Tabs.Trigger
                key={option.key}
                value={option.key}
                aria-controls={panelId}
                className="relative flex min-h-14 flex-1 items-center justify-center gap-2 border-r border-border px-3 text-sm font-medium text-muted-foreground transition-colors last:border-r-0 after:absolute after:inset-x-0 after:bottom-[-1px] after:h-0.5 hover:bg-background hover:text-foreground focus-visible:z-10 focus-visible:outline-2 focus-visible:-outline-offset-2 data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:after:bg-peach sm:flex-none sm:px-7 sm:last:border-r"
              >
                <span aria-hidden className={cn("size-4 shrink-0", option.icon)} />
                {option.name}
              </Tabs.Trigger>
            ))}
          </Tabs.List>

          {/* Keep one mounted renderer so matching tokens can move between languages. */}
          <Tabs.Content value={frontend} id={panelId} className="outline-offset-[-2px]">
            <div className="flex items-center justify-between border-b border-border px-4 py-2 sm:px-6">
              <span className="flex items-center gap-2 font-mono text-xs text-muted-foreground">
                <span className="i-lucide-file-code-2 size-3.5" aria-hidden />
                {pane.file}
              </span>
              <CopyButton key={frontend} text={tokens[frontend].code} label={pane.file} />
            </div>

            <AnimatedCodeBlock tokens={tokens} frontend={frontend} />

            <div className="flex min-h-32 flex-col justify-center gap-3 border-t border-border bg-card-2 px-4 py-4 sm:min-h-24 sm:flex-row sm:items-center sm:justify-between sm:px-6 lg:min-h-20">
              <p className="max-w-2xl text-sm leading-relaxed text-muted-foreground">
                {t(`code.frontends.${frontend}`)}
              </p>
              <a
                href={`${prefix}${docsPath(frontend)}`}
                className="inline-flex shrink-0 items-center gap-1.5 text-sm font-medium text-foreground underline-offset-4 hover:underline focus-visible:outline-2 focus-visible:outline-offset-4"
              >
                {t("common.docsFor", { language: pane.name })}
                <span className="i-lucide-arrow-up-right size-4" aria-hidden />
              </a>
            </div>
          </Tabs.Content>
        </div>
      </Tabs.Root>
    </section>
  );
}
