import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { DocsFrontend } from "../lib/docs";

export function FrontendPicker({
  value,
  onChange,
}: {
  value: DocsFrontend;
  onChange: (value: DocsFrontend) => void;
}) {
  const { t } = useTranslation();
  return (
    <div
      role="group"
      aria-label={t("common.frontend")}
      className="inline-flex border border-border bg-card-2 p-1"
    >
      {(["go", "moonbit"] as const).map((frontend) => (
        <button
          key={frontend}
          type="button"
          aria-pressed={value === frontend}
          onClick={() => onChange(frontend)}
          className={cn(
            "px-3 py-1.5 text-xs font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2",
            value === frontend
              ? "bg-foreground text-background"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          {frontend === "go" ? "Go" : "MoonBit"}
        </button>
      ))}
    </div>
  );
}
