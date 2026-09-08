import { useTranslation } from "react-i18next";
import { CodeBlock } from "../code-block";
import { SectionHeading } from "../section-heading";
import type { HighlightedSnippets } from "../../lib/snippets";
import type { DocsFrontend } from "../../lib/docs";
import { FrontendPicker } from "../frontend-picker";

export function Quickstart({
  highlighted,
  frontend,
  onFrontendChange,
}: {
  highlighted: HighlightedSnippets;
  frontend: DocsFrontend;
  onFrontendChange: (frontend: DocsFrontend) => void;
}) {
  const { t } = useTranslation();
  const steps = [
    { key: "create", snippet: frontend === "go" ? "cliInit" : "moonbitCliInit" },
    { key: "edit", snippet: frontend === "go" ? "cliFormat" : "moonbitCliFormat" },
    { key: "ship", snippet: "cliBuild" },
  ] as const;

  return (
    <section id="quickstart" className="border-b border-border">
      <SectionHeading title={t("quickstart.title")} />
      <div className="border-b border-border px-6 py-6 sm:px-12">
        <FrontendPicker value={frontend} onChange={onFrontendChange} />
      </div>
      <div className="grid gap-px bg-border lg:grid-cols-3">
        {steps.map((step, index) => (
          <div key={step.key} className="min-w-0 space-y-5 bg-background p-6 sm:p-8">
            <div className="flex items-baseline gap-3">
              <span className="font-mono text-xs text-peach">0{index + 1}</span>
              <h3 className="text-[15px] font-medium">{t(`quickstart.${step.key}`)}</h3>
            </div>
            <div className="overflow-hidden border border-border bg-card-2">
              <CodeBlock html={highlighted[step.snippet]} />
            </div>
          </div>
        ))}
      </div>
      <p className="border-t border-border px-6 py-5 text-sm leading-relaxed text-muted-foreground sm:px-8">
        {t("quickstart.note")}
      </p>
    </section>
  );
}
