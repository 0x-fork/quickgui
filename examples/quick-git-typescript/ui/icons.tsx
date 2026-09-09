import { Svg } from "@quickgui/solid";

// Shared shapes from the current Go example. Lucide refresh-cw: see ../ICONS-LICENSE.
const icons = {
  branch:
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M6 6v12m12-12v2a6 6 0 0 1-6 6H6"/><circle cx="6" cy="3" r="2"/><circle cx="6" cy="20" r="2"/><circle cx="18" cy="3" r="2"/></svg>',
  "cloud-down":
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12m-4-4 4 4 4-4M4 16v4h16v-4"/></svg>',
  "arrow-down":
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 4v16m-6-6 6 6 6-6"/></svg>',
  "arrow-up":
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20V4m-6 6 6-6 6 6"/></svg>',
  check:
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12 4 4L19 6"/></svg>',
  x: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="m6 6 12 12M6 18 18 6"/></svg>',
  "chevron-down":
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>',
  plus: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M5 12h14"/></svg>',
  minus:
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/></svg>',
  refresh:
    '<svg\n  xmlns="http://www.w3.org/2000/svg"\n  width="24"\n  height="24"\n  viewBox="0 0 24 24"\n  fill="none"\n  stroke="#000"\n  stroke-width="2"\n  stroke-linecap="round"\n  stroke-linejoin="round"\n>\n  <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" />\n  <path d="M21 3v5h-5" />\n  <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" />\n  <path d="M8 16H3v5" />\n</svg>\n',
  alert:
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3 2 21h20zM12 9v5M12 17h.01"/></svg>',
} as const;
export type IconName = keyof typeof icons;

export function Icon(props: { name: IconName; size?: number; color?: string }) {
  return (
    <Svg
      source={icons[props.name]}
      style={{
        width: props.size ?? 16,
        height: props.size ?? 16,
        flexShrink: 0,
        ...(props.color ? { color: props.color } : {}),
      }}
    />
  );
}
