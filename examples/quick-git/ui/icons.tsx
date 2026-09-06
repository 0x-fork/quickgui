import { Svg } from "@quickgui/ui";

export type IconName =
  | "alert"
  | "archive"
  | "arrow-down"
  | "arrow-up"
  | "branch"
  | "check"
  | "chevron-down"
  | "chevron-right"
  | "circle-dot"
  | "clock"
  | "cloud-down"
  | "commit"
  | "copy"
  | "external"
  | "file"
  | "folder"
  | "folder-open"
  | "history"
  | "minus"
  | "more"
  | "plus"
  | "refresh"
  | "search"
  | "sparkles"
  | "stop"
  | "tag"
  | "terminal"
  | "trash"
  | "undo"
  | "worktree"
  | "x";

export function Icon(props: { name: () => (IconName); size?: () => (number); color?: () => (string); strokeWidth?: () => (number) }) {
  const readsize = () => { const source = props.size; return source === undefined ? undefined : source(); };
  const readcolor = () => { const source = props.color; return source === undefined ? undefined : source(); };
  const readstrokeWidth = () => { const source = props.strokeWidth; return source === undefined ? undefined : source(); };
  return (
    <Svg
      source={icons[props.name()]}
      style={{
        width: readsize() ?? 16,
        height: readsize() ?? 16,
        flexShrink: 0,
        ...(readcolor() ? { color: readcolor()! } : {}),
      }}
    />
  );
}

const frame = (body: string, strokeWidth = 1.9) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="${strokeWidth}" stroke-linecap="round" stroke-linejoin="round">${body}</svg>`;

const icons: Readonly<Record<IconName, string>> = {
  alert: frame('<path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z"/><path d="M12 9v4M12 17h.01"/>'),
  archive: frame('<rect x="2.5" y="4" width="19" height="5" rx="1"/><path d="M4 9v9a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9M10 13h4"/>'),
  "arrow-down": frame('<path d="M12 5v14M5 12l7 7 7-7"/>'),
  "arrow-up": frame('<path d="M12 19V5M5 12l7-7 7 7"/>'),
  branch: frame('<circle cx="6" cy="18" r="2.5"/><circle cx="6" cy="6" r="2.5"/><circle cx="18" cy="8" r="2.5"/><path d="M6 8.5v7M18 10.5c0 3-2.5 4.5-6 5-2.5.3-5 1-6 2.5"/>'),
  check: frame('<path d="m5 12.5 4.5 4.5L19 7.5"/>', 2.4),
  "chevron-down": frame('<path d="m6 9 6 6 6-6"/>', 2.2),
  "chevron-right": frame('<path d="m9 6 6 6-6 6"/>', 2.2),
  "circle-dot": frame('<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="2.5" fill="#000"/>'),
  clock: frame('<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>'),
  "cloud-down": frame('<path d="M7 18a4.5 4.5 0 0 1-.7-8.95A6 6 0 0 1 17.8 8.5 4 4 0 0 1 18 16.5"/><path d="M12 12v9M8.5 17.5 12 21l3.5-3.5"/>'),
  commit: frame('<circle cx="12" cy="12" r="3.5"/><path d="M2.5 12h6M15.5 12h6"/>'),
  copy: frame('<rect x="9" y="9" width="12" height="12" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>'),
  external: frame('<path d="M15 3h6v6M10 14 21 3M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>'),
  file: frame('<path d="M14 2.5H7a2 2 0 0 0-2 2v15a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V7.5z"/><path d="M14 2.5v5h5"/>'),
  folder: frame('<path d="M3 7a2 2 0 0 1 2-2h4l2 2.5h8a2 2 0 0 1 2 2V18a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>'),
  "folder-open": frame('<path d="M3 7a2 2 0 0 1 2-2h4l2 2.5h8a2 2 0 0 1 2 2V11"/><path d="M3 11h18l-2 8.2a1.5 1.5 0 0 1-1.5 1.1H5.5a1.5 1.5 0 0 1-1.5-1.2z"/>'),
  history: frame('<path d="M3.5 12a8.5 8.5 0 1 0 2.5-6"/><path d="M3.5 3.5V8h4.5M12 7.5V12l3 2"/>'),
  minus: frame('<path d="M5 12h14"/>', 2.2),
  more: frame('<circle cx="5" cy="12" r="1.6" fill="#000"/><circle cx="12" cy="12" r="1.6" fill="#000"/><circle cx="19" cy="12" r="1.6" fill="#000"/>'),
  plus: frame('<path d="M12 5v14M5 12h14"/>', 2.2),
  refresh: frame('<path d="M20.5 12a8.5 8.5 0 1 1-2.5-6"/><path d="M20.5 3.5V8H16"/>'),
  search: frame('<circle cx="11" cy="11" r="6.5"/><path d="m20 20-4.3-4.3"/>'),
  sparkles: frame('<path d="M12 3.5 14 9l5.5 2-5.5 2-2 5.5-2-5.5-5.5-2L10 9z"/><path d="M19 15.5v3M17.5 17h3M5 3.5v3M3.5 5h3"/>'),
  stop: frame('<rect x="6" y="6" width="12" height="12" rx="2" fill="#000"/>'),
  tag: frame('<path d="M3 12.5V4.5A1.5 1.5 0 0 1 4.5 3h8l8.5 8.5-9.5 9.5z"/><circle cx="8" cy="8" r="1.3" fill="#000"/>'),
  terminal: frame('<rect x="3" y="4" width="18" height="16" rx="2.5"/><path d="m7 9 3 3-3 3M13 15h4"/>'),
  trash: frame('<path d="M4 7h16M9.5 7V4.5h5V7M6 7l1 13h10l1-13M10 11v6M14 11v6"/>'),
  undo: frame('<path d="M4 9.5h10a5 5 0 0 1 0 10h-3"/><path d="m8 5.5-4 4 4 4"/>'),
  worktree: frame('<circle cx="12" cy="5" r="2.5"/><circle cx="5" cy="19" r="2.5"/><circle cx="19" cy="19" r="2.5"/><path d="M12 7.5V12M12 12 6.5 17M12 12l5.5 5"/>'),
  x: frame('<path d="m7 7 10 10M17 7 7 17"/>', 2.2),
};
