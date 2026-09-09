import { useId, useState } from "react";
import type { ComponentApi } from "../../lib/component-api";
import type { Locale } from "../../i18n";

export const apiLabels = {
  en: {
    title: "API reference",
    filter: "Filter API…",
    name: "Name",
    type: "Type",
    initial: "Default",
    source: "View source",
    empty: "No matching API entries.",
    binding: "Reactive binding",
    note: "Expand an entry for its description and complete signature. A dash means no declaration default is documented.",
    constructor: "Signature",
  },
  zh: {
    title: "API 参考",
    filter: "筛选 API…",
    name: "名称",
    type: "类型",
    initial: "默认值",
    source: "查看源码",
    empty: "没有匹配的 API。",
    binding: "响应式绑定",
    note: "展开条目查看说明和完整签名。短横线表示声明中未记录默认值。",
    constructor: "签名",
  },
  ja: {
    title: "API リファレンス",
    filter: "API を絞り込む…",
    name: "名前",
    type: "型",
    initial: "既定値",
    source: "ソースを見る",
    empty: "一致する API がありません。",
    binding: "リアクティブバインディング",
    note: "項目を展開すると説明と完全なシグネチャが表示されます。ダッシュは宣言に既定値が記載されていないことを示します。",
    constructor: "シグネチャ",
  },
};
const sourceUrl = (path: string) => `https://github.com/egoist/quickgui/blob/main/${path}`;
export function ComponentApiReference({ api, locale }: { api: ComponentApi; locale: Locale }) {
  const [filter, setFilter] = useState("");
  const id = useId();
  const labels = apiLabels[locale];
  const query = filter.trim().toLowerCase();
  const sections = api.sections
    .map((section) => ({
      ...section,
      entries: section.entries.filter(
        (entry) =>
          !query ||
          `${section.name} ${entry.name} ${entry.type} ${entry.description}`
            .toLowerCase()
            .includes(query),
      ),
    }))
    .filter((section) => !query || section.entries.length);
  return (
    <section className="component-api" aria-labelledby="api-reference">
      <h2 id="api-reference">
        <a className="docs-heading-anchor" href="#api-reference" aria-label={labels.title}>
          #
        </a>
        {labels.title}
      </h2>
      <p className="component-api-note">{labels.note}</p>
      <label className="component-api-filter" htmlFor={id}>
        <span className="i-lucide-search" aria-hidden />
        <input
          id={id}
          type="search"
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
          placeholder={labels.filter}
          aria-label={labels.filter}
        />
      </label>
      {sections.length === 0 && <p role="status">{labels.empty}</p>}
      {sections.map((section) => (
        <section key={section.name} className="component-api-part" aria-label={section.name}>
          <div className="component-api-part-title">
            <h3 id={`api-${section.name.toLowerCase().replaceAll(".", "-")}`}>
              <code>{section.name}</code>
            </h3>
            <a href={sourceUrl(section.source)} target="_blank" rel="noreferrer">
              {labels.source}
              <span className="i-lucide-arrow-up-right" aria-hidden />
            </a>
          </div>
          <pre className="component-signature" aria-label={labels.constructor}>
            <code>{section.signature}</code>
          </pre>
          {section.description && <p>{section.description}</p>}
          {section.entries.length > 0 && (
            <div className="component-api-rows">
              <div className="component-api-columns" aria-hidden>
                <span>{labels.name}</span>
                <span>{labels.type}</span>
                <span>{labels.initial}</span>
                <span />
              </div>
              {section.entries.map((entry) => (
                <details key={entry.name} className="component-api-row">
                  <summary>
                    <code className="component-api-name">{entry.name}</code>
                    <code className="component-api-type">{entry.type}</code>
                    <code className="component-api-default">{entry.default ?? "—"}</code>
                    <span className="i-lucide-chevron-down" aria-hidden />
                  </summary>
                  <div className="component-api-detail">
                    <p>{entry.description}</p>
                    <pre>
                      <code>{entry.type}</code>
                    </pre>
                    {entry.binding && (
                      <>
                        <span>{labels.binding}</span>
                        <pre>
                          <code>{entry.binding}</code>
                        </pre>
                      </>
                    )}
                    <a href={sourceUrl(entry.source)} target="_blank" rel="noreferrer">
                      {labels.source}
                      <span className="i-lucide-arrow-up-right" aria-hidden />
                    </a>
                  </div>
                </details>
              ))}
            </div>
          )}
        </section>
      ))}
    </section>
  );
}
