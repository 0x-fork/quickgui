import { createMemo, createSignal } from "solid-js";
import { workload, type createIssues } from "../workload.ts";

export function createTracker(data: ReturnType<typeof createIssues>) {
  if (data.length !== workload.records) throw new Error("Expected 1,000 benchmark issues");
  const issues = data.map((item) => {
    const [status, setStatus] = createSignal(item.status);
    const [notes, setNotes] = createSignal(item.notes);
    return { ...item, status, setStatus, notes, setNotes };
  });
  const [query, setQuery] = createSignal("");
  const [filter, setFilter] = createSignal("All issues");
  const [page, setPage] = createSignal(0);
  const [selected, setSelected] = createSignal(0);
  const current = createMemo(() => issues[selected()]!);
  const completed = createMemo(() => issues.filter((issue) => issue.status() === "Done").length);
  const matching = createMemo(() => {
    const needle = query().trim().toLowerCase(),
      mode = filter();
    return issues.flatMap((issue, index) =>
      (mode === "All issues" ||
        (mode === "Open" ? issue.status() !== "Done" : issue.status() === "Done")) &&
      `${issue.id} ${issue.title} ${issue.project} ${issue.owner}`.toLowerCase().includes(needle)
        ? [index]
        : [],
    );
  });
  const pages = () => Math.max(1, Math.ceil(matching().length / workload.pageSize));
  const currentPage = () => Math.min(page(), pages() - 1);
  const visible = createMemo(() =>
    matching().slice(currentPage() * workload.pageSize, (currentPage() + 1) * workload.pageSize),
  );
  return {
    issues,
    query,
    filter,
    page: currentPage,
    selected,
    current,
    completed,
    matching,
    pages,
    visible,
    setSelected,
    search(value: string) {
      setQuery(value);
      setPage(0);
    },
    setFilter(value: string) {
      setFilter(value);
      setPage(0);
    },
    previous() {
      setPage(Math.max(0, currentPage() - 1));
    },
    next() {
      setPage(Math.min(pages() - 1, currentPage() + 1));
    },
    complete() {
      const issue = current();
      issue.setStatus(issue.status() === "Done" ? "Open" : "Done");
    },
  };
}
