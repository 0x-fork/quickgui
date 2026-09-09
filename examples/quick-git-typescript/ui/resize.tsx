import { Splitter, type JSX } from "@quickgui/solid";
import type { Element } from "solid-js";
import { useApp } from "./context.tsx";

/** Rust owns dragging and cursor capture; the reported size only persists the result. */
export function ResizablePanel(props: {
  label: string;
  width: number;
  onResize: (width: number) => void;
  minimum: number;
  maximum: number;
  style?: JSX.StyleProp;
  children: Element;
}) {
  const app = useApp();
  return (
    <Splitter.Root
      value={[props.width, props.maximum - props.width]}
      panes={[{ min: props.minimum }, {}]}
      onSizesChange={(sizes) => {
        if (sizes.length === 2) props.onResize(sizes[0]!);
      }}
      style={{ display: "flex", flexShrink: 0, minWidth: 0, minHeight: 0 }}
    >
      <Splitter.Pane itemIndex={0} style={props.style}>
        {props.children}
      </Splitter.Pane>
      <Splitter.Handle
        itemIndex={0}
        aria-label={props.label}
        hitSlopLeft={4}
        hitSlopRight={4}
        style={{
          width: 1,
          flexShrink: 0,
          cursor: "col-resize",
          appRegion: "no-drag",
          backgroundColor: app.theme().border,
        }}
      />
    </Splitter.Root>
  );
}
