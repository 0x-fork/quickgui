/**
 * Progress bars and meters. The Rust core derives the status, the completion fraction, and the
 * formatted value from the declared range; the parts only paint.
 */

import { NativePart, type NativeNode, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_COMPONENT_CHANGE } from "@quickgui/native/native-tree";

import {
  CODE_FORMAT,
  CODE_HIGH,
  CODE_INDETERMINATE,
  CODE_LOW,
  CODE_MAX,
  CODE_MIN,
  CODE_OPTIMUM,
  CODE_VALUES,
  CODE_VALUE_TEXT,
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
import { setExplicitBool, setJson, setListener, setNumber, setString } from "./runtime.ts";

/** Everything the core decided about one progress bar or meter. */
export interface GaugeState {
  /** `progressing`, `complete`, or `indeterminate`. */
  status: string;
  /** The value formatted through the declared `format`, or `null` without one. */
  displayValue: string | null;
  /** How far the value has travelled, from 0 to 1, or `null` while indeterminate. */
  completion: number | null;
}

function settledGauge(): GaugeState {
  return { status: "indeterminate", displayValue: null, completion: null };
}

export interface GaugeFormatProps extends PartProps {
  /** Bounded value formatter the core applies: `"percent"` or `"fraction"`. */
  format?: "percent" | "fraction";
  /** Everything the core derived from the declared value. */
  onStatusChange?: (state: GaugeState, event: QuickGuiEvent) => void;
}

export interface ProgressProps extends GaugeFormatProps {
  /** Completed amount. Omit, or declare `indeterminate`, for unknown progress. */
  value?: Accessor<number | undefined>;
  /** Completion maximum. Defaults to `1`. */
  max?: number;
  indeterminate?: Accessor<boolean>;
  /** Human-readable value such as `"3 of 12 files"`, preferred by assistive technology. */
  valueText?: Accessor<string | undefined>;
}

export interface MeterProps extends GaugeFormatProps {
  value?: Accessor<number | undefined>;
  min?: number;
  max?: number;
  low?: number;
  high?: number;
  optimum?: number;
}

class GaugeContextValue {
  readonly _state = signal<GaugeState>(settledGauge());

  state(): GaugeState {
    return this._state.read();
  }
}

const GaugeContext = createPartContext<GaugeContextValue>();

/**
 * Read the live status inside a `Progress.Root` or `Meter.Root` subtree: the core's own derived
 * status and the value its declared `format` produced.
 */
export function useGaugeState(): Accessor<GaugeState> {
  const context = useContext(GaugeContext);
  return context === undefined ? settledGauge : () => context.state();
}

function createGaugeRoot(part: string, props: GaugeFormatProps, declare: (node: NativeNode) => void): NativeNode {
  const context = new GaugeContextValue();
  const node = createViewPart(props);
  setPart(node, part, createComponentScope("qg-gauge"), undefined);
  if (props.format !== undefined) setString(node, CODE_FORMAT, props.format);
  declare(node);
  setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
    const details = componentChangeFromEvent(event);
    if (details === undefined) return;
    const status = details.status;
    if (status === undefined) return;
    const next: GaugeState = { status, displayValue: details.displayValue ?? null, completion: details.completion ?? null };
    context._state.write(next);
    const onStatusChange = props.onStatusChange;
    if (onStatusChange !== undefined) onStatusChange(next, event);
  });
  return GaugeContext.provide(context, () => finishPart(node, props));
}

function createGaugePart(part: string, props: PartProps): NativeNode {
  const node = createViewPart(props);
  setPart(node, part, undefined, undefined);
  return finishPart(node, props);
}

function bindGaugeValue(node: NativeNode, value: Accessor<number | undefined> | undefined): void {
  if (value === undefined) return;
  createRenderEffect(() => {
    const current = value();
    setJson(node, CODE_VALUES, 65536, current === undefined ? undefined : [current]);
  });
}

/** Compound parts for a progress indicator. */
export class Progress {
  /** Determinate or indeterminate progress root carrying the core's exact value range. */
  static Root(props: ProgressProps): NativeNode {
    return createGaugeRoot(NativePart.Progress, props, (node) => {
      bindGaugeValue(node, props.value);
      setNumber(node, CODE_MAX, props.max ?? 1);
      const indeterminate = props.indeterminate;
      if (indeterminate !== undefined) {
        createRenderEffect(() => {
          setExplicitBool(node, CODE_INDETERMINATE, indeterminate());
        });
      }
      const valueText = props.valueText;
      if (valueText !== undefined) {
        createRenderEffect(() => {
          setString(node, CODE_VALUE_TEXT, valueText());
        });
      }
    });
  }

  /** Progress track. The core derives its identity from the compound scope. */
  static Track(props: PartProps): NativeNode {
    return createGaugePart(NativePart.ProgressTrack, props);
  }

  /** Application-owned progress fill, hidden from the accessible name by the core. */
  static Indicator(props: PartProps): NativeNode {
    return createGaugePart(NativePart.ProgressIndicator, props);
  }

  /** Progress label. The core points the root's accessible name at it. */
  static Label(props: PartProps): NativeNode {
    return createGaugePart(NativePart.ProgressLabel, props);
  }

  /** Progress value readout, described by the core through the root. */
  static Value(props: PartProps): NativeNode {
    return createGaugePart(NativePart.ProgressValue, props);
  }
}

/** Compound parts for a static measurement gauge with optional low, high, and optimum markers. */
export class Meter {
  static Root(props: MeterProps): NativeNode {
    return createGaugeRoot(NativePart.Meter, props, (node) => {
      bindGaugeValue(node, props.value);
      if (props.min !== undefined) setNumber(node, CODE_MIN, props.min);
      if (props.max !== undefined) setNumber(node, CODE_MAX, props.max);
      if (props.low !== undefined) setNumber(node, CODE_LOW, props.low);
      if (props.high !== undefined) setNumber(node, CODE_HIGH, props.high);
      if (props.optimum !== undefined) setNumber(node, CODE_OPTIMUM, props.optimum);
    });
  }

  static Track(props: PartProps): NativeNode {
    return createGaugePart(NativePart.MeterTrack, props);
  }

  /** Application-owned meter fill, hidden from the accessible name by the core. */
  static Indicator(props: PartProps): NativeNode {
    return createGaugePart(NativePart.MeterIndicator, props);
  }

  static Label(props: PartProps): NativeNode {
    return createGaugePart(NativePart.MeterLabel, props);
  }

  static Value(props: PartProps): NativeNode {
    return createGaugePart(NativePart.MeterValue, props);
  }
}
