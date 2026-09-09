import { useTranslation } from "react-i18next";
import { CodeBlock } from "../code-block";
import { SectionHeading } from "../section-heading";
import type { HighlightedSnippets } from "../../lib/snippets";
import type { DocsFrontend } from "../../lib/docs";

export function Quickstart({
  highlighted,
  frontend,
}: {
  highlighted: HighlightedSnippets;
  frontend: DocsFrontend;
}) {
  const { t } = useTranslation();
  const steps = [
    { key: "create", snippet: "cliInit" },
    { key: "edit", snippet: frontend === "typescript" ? "typescriptCliCheck" : "cliFormat" },
    { key: "ship", snippet: "cliBuild" },
  ] as const;

  return (
    <section id="quickstart" className="border-b border-border">
      <SectionHeading title={t("quickstart.title")} />
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
    </section>
  );
}
