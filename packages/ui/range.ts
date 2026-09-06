/**
 * Declared range controls: slider and splitter.
 *
 * The Rust core owns clamping, step snapping, thumb ordering, pane size conservation, the
 * keyboard bindings, and captured pointer drags. The parts declare the values and receive what
 * the core decided through one asynchronous `componentchange` payload.
 */

import { NativePart, type NativeNode, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_COMPONENT_CHANGE } from "@quickgui/native/native-tree";

import {
  CODE_FORMAT,
  CODE_ITEMS,
  CODE_ITEM_INDEX,
  CODE_LARGE_STEP,
  CODE_MAX,
  CODE_MIN,
  CODE_MIN_STEPS_BETWEEN_VALUES,
  CODE_ORIENTATION,
  CODE_STEP,
  CODE_THUMB_ALIGNMENT,
  CODE_VALUES,
} from "./generated.ts";
import { componentChangeFromEvent } from "./index.ts";
import {
  createComponentScope,
  createPartContext,
  createViewPart,
  finishPart,
  setPart,
  type PartProps,
} from "./parts.ts";
import { createRenderEffect, signal, useContext, type Accessor } from "./reactive.ts";
import { setJson, setListener, setNumber, setString } from "./runtime.ts";

// --- slider --------------------------------------------------------------------------------------

/** Everything the core decided about one slider since the last frame. */
export interface SliderState {
  values: number[];
  /** Whether a captured drag is in flight, matching Base UI's `data-dragging`. */
  dragging: boolean;
  /** The value formatted through the declared `format`, or `null` without one. */
  displayValue: string | null;
}

function settledSlider(): SliderState {
  return { values: [], dragging: false, displayValue: null };
}

export interface SliderRootProps extends PartProps {
  /** Controlled thumb values, one entry per thumb. */
  value?: Accessor<number[]>;
  defaultValue?: number[];
  onValueChange?: (values: number[], event: QuickGuiEvent) => void;
  /** The core's own pointer boundary, matching Base UI's `onValueCommitted`. */
  onValueCommitted?: (values: number[], event: QuickGuiEvent) => void;
  min?: number;
  max?: number;
  step?: number;
  largeStep?: number;
  /** Whole steps the core holds open between adjacent thumbs. */
  minStepsBetweenValues?: number;
  /** `"center"` (default) or `"edge"`, matching Base UI's `thumbAlignment`. */
  thumbAlignment?: "center" | "edge";
  /** Bounded value formatter the core applies: `"percent"` or `"fraction"`. */
  format?: "percent" | "fraction";
  orientation?: "horizontal" | "vertical";
}

export interface SliderThumbProps extends PartProps {
  /** Which thumb this part paints, matching the index in `value`. */
  index?: number;
}

class SliderRootState {
  /** The instance scope every slider part declares so the binding resolves one core slider. */
  readonly scope = createComponentScope("qg-slider");
  readonly _state = signal<SliderState>(settledSlider());

  state(): SliderState {
    return this._state.read();
  }
}

const SliderContext = createPartContext<SliderRootState>();

/** Read the live slider state inside a `Slider.Root` subtree. */
export function useSliderState(): Accessor<SliderState> {
  const context = useContext(SliderContext);
  return context === undefined ? settledSlider : () => context.state();
}

/** The scope of the nearest `Slider.Root`, so every part reaches the same core slider. */
function sliderScope(): string | undefined {
  const context = useContext(SliderContext);
  return context === undefined ? undefined : context.scope;
}

function createSliderPart(part: string, props: PartProps): NativeNode {
  const node = createViewPart(props);
  setPart(node, part, sliderScope(), undefined);
  return finishPart(node, props);
}

/** Compound parts for a slider. */
export class Slider {
  /**
   * Controlled slider root.
   *
   * `value` carries one entry per thumb, so a single-thumb slider and a range slider are the
   * same component. The core answers arrows, Page keys, Home, End, and captured pointer drags;
   * it reports the snapped, ordered, clamped result through `onValueChange`.
   */
  static Root(props: SliderRootProps): NativeNode {
    const state = new SliderRootState();
    const uncontrolled = signal<number[]>(props.defaultValue ?? [0]);
    const values = (): number[] => {
      const controlled = props.value;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const node = createViewPart(props);
    setPart(node, NativePart.Slider, state.scope, undefined);
    createRenderEffect(() => {
      setJson(node, CODE_VALUES, 65536, values());
    });
    if (props.min !== undefined) setNumber(node, CODE_MIN, props.min);
    if (props.max !== undefined) setNumber(node, CODE_MAX, props.max);
    if (props.step !== undefined) setNumber(node, CODE_STEP, props.step);
    if (props.largeStep !== undefined) setNumber(node, CODE_LARGE_STEP, props.largeStep);
    if (props.minStepsBetweenValues !== undefined) {
      setNumber(node, CODE_MIN_STEPS_BETWEEN_VALUES, props.minStepsBetweenValues);
    }
    if (props.thumbAlignment !== undefined) setString(node, CODE_THUMB_ALIGNMENT, props.thumbAlignment);
    if (props.format !== undefined) setString(node, CODE_FORMAT, props.format);
    if (props.orientation !== undefined) setString(node, CODE_ORIENTATION, props.orientation);
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const next = details.values;
      if (next === undefined) return;
      state._state.write({
        values: next,
        dragging: details.dragging === true,
        displayValue: details.displayValue ?? null,
      });
      if (props.value === undefined) uncontrolled.write(next);
      const onValueChange = props.onValueChange;
      if (onValueChange !== undefined) onValueChange(next, event);
      // `onValueCommitted` is the core's own pointer boundary, not a debounce in the application.
      if (details.committed === true) {
        const onValueCommitted = props.onValueCommitted;
        if (onValueCommitted !== undefined) onValueCommitted(next, event);
      }
    });
    return SliderContext.provide(state, () => finishPart(node, props));
  }

  /** Slider label. The core points the root's accessible name at it. */
  static Label(props: PartProps): NativeNode {
    return createSliderPart(NativePart.SliderLabel, props);
  }

  /** Slider value readout. Render `useSliderState()`'s `displayValue`; the core formatted it. */
  static Value(props: PartProps): NativeNode {
    return createSliderPart(NativePart.SliderValue, props);
  }

  /** Clickable control box the track and thumbs are laid out inside. */
  static Control(props: PartProps): NativeNode {
    return createSliderPart(NativePart.SliderControl, props);
  }

  /** Application-owned slider track painted inside the interactive control. */
  static Track(props: PartProps): NativeNode {
    return createSliderPart(NativePart.SliderTrack, props);
  }

  /** Application-owned slider fill, hidden from the accessible name by the core. */
  static Range(props: PartProps): NativeNode {
    return createSliderPart(NativePart.SliderRange, props);
  }

  /** Base UI's name for the range fill. It decorates exactly the same core part. */
  static Indicator(props: PartProps): NativeNode {
    return createSliderPart(NativePart.SliderIndicator, props);
  }

  /** Application-owned slider thumb. A range slider gives each thumb its own keyboard focus. */
  static Thumb(props: SliderThumbProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.SliderThumb, sliderScope(), undefined);
    if (props.index !== undefined) setNumber(node, CODE_ITEM_INDEX, props.index);
    return finishPart(node, props);
  }
}

// --- splitter ------------------------------------------------------------------------------------

/** One pane constraint in a declared splitter. */
export interface SplitterPaneDeclaration {
  min?: number;
  collapsible?: boolean;
}

export interface SplitterRootProps extends PartProps {
  /** Controlled pane sizes in logical pixels. */
  value?: Accessor<number[]>;
  defaultValue?: number[];
  panes?: SplitterPaneDeclaration[];
  step?: number;
  orientation?: "horizontal" | "vertical";
  onSizesChange?: (sizes: number[], event: QuickGuiEvent) => void;
}

export interface SplitterPaneProps extends PartProps {
  /** Which pane or handle this part paints. */
  index?: number;
}

const SplitterContext = createPartContext<string>();

function createSplitterPart(part: string, props: SplitterPaneProps): NativeNode {
  const node = createViewPart(props);
  setPart(node, part, useContext(SplitterContext), undefined);
  if (props.index !== undefined) setNumber(node, CODE_ITEM_INDEX, props.index);
  return finishPart(node, props);
}

/** Compound parts for an adjustable splitter. */
export class Splitter {
  /**
   * Controlled splitter root.
   *
   * `value` carries one size per pane. The core conserves the total across captured drags and
   * typed keyboard resizing and reports every pane size together through `onSizesChange`.
   */
  static Root(props: SplitterRootProps): NativeNode {
    const uncontrolled = signal<number[]>(props.defaultValue ?? []);
    const sizes = (): number[] => {
      const controlled = props.value;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const scope = createComponentScope("qg-splitter");
    const node = createViewPart(props);
    setPart(node, NativePart.Splitter, scope, undefined);
    createRenderEffect(() => {
      setJson(node, CODE_VALUES, 65536, sizes());
    });
    if (props.panes !== undefined) setJson(node, CODE_ITEMS, 65536, props.panes);
    if (props.step !== undefined) setNumber(node, CODE_STEP, props.step);
    if (props.orientation !== undefined) setString(node, CODE_ORIENTATION, props.orientation);
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const next = details.sizes;
      if (next === undefined) return;
      if (props.value === undefined) uncontrolled.write(next);
      const onSizesChange = props.onSizesChange;
      if (onSizesChange !== undefined) onSizesChange(next, event);
    });
    return SplitterContext.provide(scope, () => finishPart(node, props));
  }

  /** Application-owned splitter pane. */
  static Pane(props: SplitterPaneProps): NativeNode {
    return createSplitterPart(NativePart.SplitterPane, props);
  }

  /** Application-owned splitter handle carrying the core's numeric resize semantics. */
  static Handle(props: SplitterPaneProps): NativeNode {
    return createSplitterPart(NativePart.SplitterHandle, props);
  }
}
