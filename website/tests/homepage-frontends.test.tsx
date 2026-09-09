import { expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { I18nextProvider } from "react-i18next";
import { MemoryRouter } from "react-router";
import { Hero } from "../src/components/sections/hero";
import { SwiftUi } from "../src/components/sections/swift-ui";
import { Quickstart } from "../src/components/sections/quickstart";
import { FinalCta } from "../src/components/sections/final-cta";
import { DocsShell } from "../src/components/docs/docs-shell";
import { DOCS_FRONTENDS, docsPath, frontendLabel } from "../src/lib/docs";
import { createI18n, SUPPORTED_LOCALES } from "../src/i18n";
import { getHighlightedSnippets } from "../src/server/highlight.server";

const plain = (html: string) => html.replace(/<[^>]*>/g, "");

test("every homepage frontend selects its own commands, SwiftUI example, and docs links", async () => {
  const highlighted = await getHighlightedSnippets();
  for (const locale of SUPPORTED_LOCALES) {
    const i18n = createI18n(locale);
    const prefix = locale === "en" ? "" : `/${locale}`;
    for (const frontend of DOCS_FRONTENDS) {
      const render = (children: React.ReactNode) =>
        renderToString(<I18nextProvider i18n={i18n}>{children}</I18nextProvider>);
      const hero = render(<Hero frontend={frontend} onFrontendChange={() => {}} />);
      const command = `bunx @quickgui/cli init my-app${frontend === "go" ? "" : ` --frontend ${frontend}`}`;
      expect(plain(hero)).toContain(command);
      const cta = render(<FinalCta />);
      for (const target of DOCS_FRONTENDS) {
        for (const html of [hero, cta]) {
          expect(html).toContain(`href="${prefix}${docsPath(target)}"`);
          expect(plain(html)).toContain(
            i18n.t("common.docsFor", { language: frontendLabel(target) }),
          );
        }
      }
      const swift = plain(
        render(
          <SwiftUi highlighted={highlighted} frontend={frontend} onFrontendChange={() => {}} />,
        ),
      );
      const extension = { go: "go", moonbit: "mbt", typescript: "tsx" }[frontend];
      expect(swift).toContain(`swiftui.${extension}`);
      expect(swift).toContain(
        {
          go: "ui.SwiftUI.Host",
          moonbit: "@ui.swift_ui_host",
          typescript: "@quickgui/solid/swift-ui",
        }[frontend],
      );
      const quickstart = plain(
        render(
          <Quickstart highlighted={highlighted} frontend={frontend} onFrontendChange={() => {}} />,
        ),
      );
      expect(quickstart.replace(/\\\s*\n\s*/g, " ").replace(/\s+/g, " ")).toContain(command);
    }
  }
});

test("the docs picker includes and selects every frontend in all locales", () => {
  for (const locale of SUPPORTED_LOCALES) {
    for (const frontend of DOCS_FRONTENDS) {
      const html = renderToString(
        <MemoryRouter>
          <DocsShell
            frontend={frontend}
            locale={locale}
            page={{
              title: "Getting started",
              description: "",
              outline: [],
              path: docsPath(frontend),
              area: "guide",
            }}
          >
            <p>Guide content</p>
          </DocsShell>
        </MemoryRouter>,
      );
      for (const option of DOCS_FRONTENDS) {
        expect(html).toContain(`<option value="${option}"`);
      }
      expect(html).toContain(`<option value="${frontend}" selected=""`);
    }
  }
});
