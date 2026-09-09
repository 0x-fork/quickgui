import { roundedPresets, styleHelpers, type HelperDeclaration } from "./style-helpers.generated.ts";

export { helperProperties, type StyleHelpers } from "./style-helpers.generated.ts";

/** Expand a Rust convenience into ordinary native properties before retained diffing. */
export function expandStyleHelper(name: string, value: unknown): HelperDeclaration | undefined {
  // Grid's existing shorthand writes the same fields as col-span/row-span and needs the same
  // ownership, so withdrawing a helper can restore the declared start/end lines.
  if (name === "gridColumn" || name === "gridRow") return gridPlacement(name, value);
  if (!Object.hasOwn(styleHelpers, name)) return undefined;
  if (value === undefined || value === null || value === false) return {};
  const helper = styleHelpers[name]!;
  // Preserve the CSS flex shorthand and flex-wrap values alongside the boolean Rust helpers.
  if (name === "flex" && value !== true) return flexDeclaration(value);
  if (name === "flex-wrap" && typeof value === "string") return { flexWrap: value };
  if (helper.parameters === 0) {
    if (value !== true) throw new TypeError(`QuickGUI ${name} must be a boolean`);
  } else if (helper.parameters === 2) {
    if (!Array.isArray(value) || value.length !== 2) {
      throw new TypeError(`QuickGUI ${name} accepts [width, height]`);
    }
  } else if (name.startsWith("rounded")) {
    if (typeof value === "string" && Object.hasOwn(roundedPresets, value)) {
      value = roundedPresets[value as keyof typeof roundedPresets];
    } else if (typeof value !== "number" || !Number.isFinite(value)) {
      throw new TypeError(
        `QuickGUI ${name} accepts a finite radius or ${Object.keys(roundedPresets).join(", ")}`,
      );
    }
  }
  return helper.resolve(value);
}

function flexDeclaration(value: unknown): HelperDeclaration {
  if (typeof value === "number") return { flexGrow: value, flexShrink: 1, flexBasis: 0 };
  const parts = String(value).trim().split(/\s+/);
  if (parts.length === 1 && parts[0] === "none") {
    return { flexGrow: 0, flexShrink: 0, flexBasis: "auto" };
  }
  const result: HelperDeclaration = { flexGrow: Number(parts[0]) };
  if (parts.length >= 2) result.flexShrink = Number(parts[1]);
  if (parts.length >= 3) result.flexBasis = parts[2];
  return result;
}

function gridPlacement(name: string, value: unknown): HelperDeclaration {
  if (value === null || value === undefined || value === false) return {};
  const startField = `${name}Start`,
    endField = `${name}End`,
    spanField = `${name}Span`;
  const result: HelperDeclaration = { [startField]: null, [endField]: null, [spanField]: null };
  if (typeof value === "number") return { ...result, [startField]: value };
  const [rawStart = "", rawEnd = ""] = String(value).split("/");
  const start = rawStart.trim(),
    end = rawEnd.trim();
  const startSpan = start.match(/^span\s+(\d+)$/),
    endSpan = end.match(/^span\s+(\d+)$/);
  const startLine = start === "" || start === "auto" ? null : Number(start);
  if (startSpan) return { ...result, [spanField]: Number(startSpan[1]) };
  if (startLine !== null && Number.isFinite(startLine)) result[startField] = startLine;
  if (endSpan) {
    if (startLine !== null && Number.isFinite(startLine))
      result[endField] = startLine + Number(endSpan[1]);
    else result[spanField] = Number(endSpan[1]);
  } else {
    const endLine = end === "" || end === "auto" ? null : Number(end);
    if (endLine !== null && Number.isFinite(endLine)) result[endField] = endLine;
  }
  return result;
}
