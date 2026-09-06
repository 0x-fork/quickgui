/**
 * Civil date and time controls: date field, time field, and calendar.
 *
 * Values are ISO civil strings with no time zone: `YYYY-MM-DD` dates and `HH:MM` or `HH:MM:SS`
 * times. The Rust core owns segment arithmetic, digit entry, leap years, month arithmetic, the
 * single Tab stop, and validity.
 */

import { NativePart, type NativeNode, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_COMPONENT_CHANGE } from "@quickgui/native/native-tree";

import {
  CODE_CIVIL_MAXIMUM,
  CODE_CIVIL_MINIMUM,
  CODE_CIVIL_VALUE,
  CODE_FIRST_WEEKDAY,
  CODE_ITEM_INDEX,
  CODE_SEGMENT,
  CODE_SEGMENT_ORDER,
  CODE_VARIANT,
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
import { createRenderEffect, signal, useContext, type Accessor, type Context } from "./reactive.ts";
import { setListener, setNumber, setString } from "./runtime.ts";

interface CivilValueProps extends PartProps {
  value?: Accessor<string | undefined>;
  defaultValue?: string;
  min?: string;
  max?: string;
  onValueChange?: (value: string | undefined, event: QuickGuiEvent) => void;
}

export interface DateFieldRootProps extends CivilValueProps {
  /** Segment order. Defaults to ISO `ymd`. */
  format?: "ymd" | "dmy" | "mdy";
}

export interface DateFieldSegmentProps extends PartProps {
  segment: "year" | "month" | "day";
}

export interface TimeFieldRootProps extends CivilValueProps {
  /** Add an AM/PM segment and display hours on a twelve-hour clock. */
  hour12?: boolean;
  /** Mount a seconds segment. Without it the core owns no seconds segment at all. */
  showSeconds?: boolean;
}

export interface TimeFieldSegmentProps extends PartProps {
  segment: "hour" | "minute" | "second" | "period";
}

export interface CalendarRootProps extends CivilValueProps {
  /** First weekday column, where Monday is `0` and Sunday is `6`. */
  firstWeekday?: number;
  /** The day the grid's single Tab stop moved to. */
  onFocusChange?: (day: string, event: QuickGuiEvent) => void;
  /** The displayed month, as `YYYY-MM`. */
  onMonthChange?: (month: string, event: QuickGuiEvent) => void;
}

export interface CalendarWeekProps extends PartProps {
  /** Week row index inside the displayed month. */
  index?: number;
}

export interface CalendarDayProps extends PartProps {
  /** ISO `YYYY-MM-DD` civil date this cell paints. */
  day: string;
}

// Each family carries its own instance scope so a segment or day cell reaches exactly the root it
// sits inside, even when one control is nested in another.
const DateFieldContext = createPartContext<string>();
const TimeFieldContext = createPartContext<string>();
const CalendarContext = createPartContext<string>();

/** Declare the controlled civil value and its bounds, and adopt every value the core reports. */
function createCivilRoot(
  part: string,
  scope: string,
  context: Context<string | undefined>,
  props: CivilValueProps,
  declare: (node: NativeNode) => void,
  report: (node: NativeNode, event: QuickGuiEvent) => void,
): NativeNode {
  const uncontrolled = signal<string | undefined>(props.defaultValue);
  const value = (): string | undefined => {
    const controlled = props.value;
    return controlled !== undefined ? controlled() : uncontrolled.read();
  };
  const node = createViewPart(props);
  setPart(node, part, scope, undefined);
  createRenderEffect(() => {
    setString(node, CODE_CIVIL_VALUE, value());
  });
  setString(node, CODE_CIVIL_MINIMUM, props.min);
  setString(node, CODE_CIVIL_MAXIMUM, props.max);
  declare(node);
  setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
    const details = componentChangeFromEvent(event);
    if (details === undefined) return;
    if (details.value !== undefined) {
      const next = details.value === null ? undefined : details.value;
      if (props.value === undefined) uncontrolled.write(next);
      const onValueChange = props.onValueChange;
      if (onValueChange !== undefined) onValueChange(next, event);
    }
    report(node, event);
  });
  return context.provide(scope, () => finishPart(node, props));
}

function ignoreReport(_node: NativeNode, _event: QuickGuiEvent): void {}

/** Compound parts for a date field. */
export class DateField {
  /** Controlled date field. `value`, `min`, and `max` are ISO `YYYY-MM-DD` civil values. */
  static Root(props: DateFieldRootProps): NativeNode {
    return createCivilRoot(
      NativePart.DateField,
      createComponentScope("qg-date-field"),
      DateFieldContext,
      props,
      (node: NativeNode): void => {
        setString(node, CODE_SEGMENT_ORDER, props.format);
      },
      ignoreReport,
    );
  }

  /** One date-field segment carrying the core's spin-button semantics and typed actions. */
  static Segment(props: DateFieldSegmentProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.DateFieldSegment, useContext(DateFieldContext), undefined);
    setString(node, CODE_SEGMENT, props.segment);
    return finishPart(node, props);
  }
}

/** Compound parts for a time field. */
export class TimeField {
  /**
   * Controlled time field.
   *
   * `value`, `min`, and `max` are `HH:MM` or `HH:MM:SS` civil times. A twelve-hour field gains
   * an AM/PM segment, and a field without `showSeconds` mounts no seconds segment at all.
   */
  static Root(props: TimeFieldRootProps): NativeNode {
    return createCivilRoot(
      NativePart.TimeField,
      createComponentScope("qg-time-field"),
      TimeFieldContext,
      props,
      (node: NativeNode): void => {
        setString(node, CODE_SEGMENT_ORDER, props.hour12 === true ? "h12" : "h23");
        if (props.showSeconds === true) setString(node, CODE_VARIANT, "seconds");
      },
      ignoreReport,
    );
  }

  /** One time-field segment carrying the core's spin-button semantics and typed actions. */
  static Segment(props: TimeFieldSegmentProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.TimeFieldSegment, useContext(TimeFieldContext), undefined);
    setString(node, CODE_SEGMENT, props.segment);
    return finishPart(node, props);
  }
}

/** Compound parts for a month grid. */
export class Calendar {
  /**
   * Controlled month grid.
   *
   * The core owns day, week, month, and year movement, the single Tab stop, and selectability
   * inside the declared civil bounds. `onFocusChange` reports the day the grid moved to.
   */
  static Root(props: CalendarRootProps): NativeNode {
    return createCivilRoot(
      NativePart.Calendar,
      createComponentScope("qg-calendar"),
      CalendarContext,
      props,
      (node: NativeNode): void => {
        if (props.firstWeekday !== undefined) setNumber(node, CODE_FIRST_WEEKDAY, props.firstWeekday);
      },
      (_node: NativeNode, event: QuickGuiEvent): void => {
        const details = componentChangeFromEvent(event);
        if (details === undefined) return;
        const focused = details.focused;
        if (focused !== undefined && focused !== null) {
          const onFocusChange = props.onFocusChange;
          if (onFocusChange !== undefined) onFocusChange(focused, event);
        }
        const month = details.month;
        if (month !== undefined) {
          const onMonthChange = props.onMonthChange;
          if (onMonthChange !== undefined) onMonthChange(month, event);
        }
      },
    );
  }

  /** One calendar week row. */
  static Week(props: CalendarWeekProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.CalendarWeek, useContext(CalendarContext), undefined);
    if (props.index !== undefined) setNumber(node, CODE_ITEM_INDEX, props.index);
    return finishPart(node, props);
  }

  /** One calendar day cell, named by its ISO `YYYY-MM-DD` civil date. */
  static Day(props: CalendarDayProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.CalendarDay, useContext(CalendarContext), undefined);
    setString(node, CODE_CIVIL_VALUE, props.day);
    return finishPart(node, props);
  }
}
