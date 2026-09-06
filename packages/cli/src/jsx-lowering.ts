/**
 * Lower JSX into direct calls of the `@quickgui/ui` runtime.
 *
 * Host elements become one immediately-invoked block that creates the node, applies each prop
 * through its typed setter (inside a render effect when the value reads signals), and inserts the
 * children. Components become one call with a props object; a value passed to an accessor-typed
 * prop is wrapped in a thunk. Everything is type-directed through the TypeScript 7 checker, so
 * the output stays fully typed for the native compiler.
 *
 * The lowering is a source-to-source rewrite: every outermost JSX expression is replaced by its
 * lowered text and everything else is copied verbatim, comments included.
 */

import {
  SignatureKind,
  SymbolFlags,
  TypeFlags,
  type Checker,
  type Signature,
  type Type,
  type TypeReference,
} from "typescript/unstable/async";
import {
  SyntaxKind,
  type ArrowFunction,
  type BinaryExpression,
  type ConditionalExpression,
  type CallExpression,
  type ExportDeclaration,
  type Expression,
  type FunctionExpression,
  type Identifier,
  type ImportDeclaration,
  type ImportTypeNode,
  type JsxAttribute,
  type JsxAttributes,
  type JsxChild,
  type JsxElement,
  type JsxExpression,
  type JsxFragment,
  type JsxNamespacedName,
  type JsxSelfClosingElement,
  type JsxTagNameExpression,
  type JsxText,
  type Node,
  type NodeArray,
  type LiteralTypeNode,
  type ObjectLiteralExpression,
  type ParenthesizedExpression,
  type PropertyAccessExpression,
  type PropertyAssignment,
  type ShorthandPropertyAssignment,
  type SourceFile,
  type SpreadAssignment,
  type StringLiteral,
} from "typescript/unstable/ast";

import {
  KIND_BACKGROUND,
  KIND_COLOR,
  KIND_GRID_PLACEMENT,
  KIND_GROUP_STATE,
  KIND_JSON,
  KIND_STATE,
  elementTags,
  eventTable,
  propertyTable,
  setterForKind,
  stateNames,
  type PropertyEntry,
} from "@quickgui/ui/style-table";
import { parseColor } from "@quickgui/native/native-tree";

export class LoweringError extends Error {
  readonly file: string;
  readonly line: number;
  readonly column: number;

  constructor(file: string, text: string, node: Node, message: string) {
    const position = lineAndColumn(text, node.getStart());
    super(`${file}:${position.line}:${position.column}: ${message}`);
    this.file = file;
    this.line = position.line;
    this.column = position.column;
  }
}

/** One-based line and column of `offset` in `text`. */
export function lineAndColumn(text: string, offset: number): { line: number; column: number } {
  let line = 1;
  let lineStart = 0;
  for (let index = 0; index < offset && index < text.length; index += 1) {
    if (text.charCodeAt(index) === 10) {
      line += 1;
      lineStart = index + 1;
    }
  }
  return { line, column: offset - lineStart + 1 };
}

export interface LoweringOptions {
  checker: Checker;
  /** Absolute path of the `@quickgui/ui` package's `index.ts`, which declares the host elements. */
  uiIndexPath: string;
  /** Module specifier the lowered file imports the runtime helpers from. */
  runtimeSpecifier: string;
  /** Module specifier of `@quickgui/ui`'s `reactive.ts`, relative to the lowered file. */
  reactiveSpecifier: string;
  /** Module specifier of `@quickgui/ui`'s `generated.ts`, relative to the lowered file. */
  generatedSpecifier: string;
  /** Rewrite one import/export specifier, or resolve `undefined` to keep it. */
  rewriteSpecifier?: (specifier: string, literal: StringLiteral) => Promise<string | undefined>;
}

const HELPER_PREFIX = "__qg_";

/** Helpers that live in `reactive.ts` rather than `runtime.ts`. */
const REACTIVE_HELPERS = new Set(["createRenderEffect", "untrack"]);
/** Helpers that live in the generated style module rather than `runtime.ts`. */
const GENERATED_HELPERS = new Set(["bindStyle", "bindStyleList", "applyStyle", "applyStyleList"]);

const propertyByName = new Map<string, PropertyEntry>();
for (const entry of propertyTable) {
  if (!propertyByName.has(entry.name)) propertyByName.set(entry.name, entry);
}
const styleEntryByName = new Map<string, PropertyEntry>();
for (const entry of propertyTable) {
  if (entry.style && !styleEntryByName.has(entry.name)) styleEntryByName.set(entry.name, entry);
}
const eventByName = new Map<string, number>();
for (const event of eventTable) eventByName.set(event.name, event.type);
const elementTagByName = new Map<string, number>();
for (const element of elementTags) elementTagByName.set(element.name, element.tag);

const FUNCTION_LIKE = new Set<SyntaxKind>([
  SyntaxKind.FunctionExpression,
  SyntaxKind.ArrowFunction,
  SyntaxKind.FunctionDeclaration,
  SyntaxKind.MethodDeclaration,
  SyntaxKind.GetAccessor,
  SyntaxKind.SetAccessor,
  SyntaxKind.Constructor,
  SyntaxKind.ClassExpression,
  SyntaxKind.ClassDeclaration,
]);

function is<T extends Node>(node: Node | undefined, kind: SyntaxKind): node is T {
  return node !== undefined && node.kind === kind;
}

function isJsxLike(node: Node | undefined): node is JsxElement | JsxSelfClosingElement | JsxFragment {
  return (
    node !== undefined &&
    (node.kind === SyntaxKind.JsxElement || node.kind === SyntaxKind.JsxSelfClosingElement || node.kind === SyntaxKind.JsxFragment)
  );
}

function stripParentheses(expression: Node): Node {
  let current = expression;
  while (is<ParenthesizedExpression>(current, SyntaxKind.ParenthesizedExpression)) current = current.expression;
  return current;
}

/** Whether evaluating `expression` may read reactive state: it contains a call outside nested functions. */
export function isDynamicExpression(expression: Node): boolean {
  let dynamic = false;
  const visit = (node: Node): undefined => {
    if (dynamic || FUNCTION_LIKE.has(node.kind)) return undefined;
    if (
      node.kind === SyntaxKind.CallExpression ||
      node.kind === SyntaxKind.NewExpression ||
      node.kind === SyntaxKind.TaggedTemplateExpression ||
      node.kind === SyntaxKind.AwaitExpression
    ) {
      dynamic = true;
      return undefined;
    }
    node.forEachChild(visit);
    return undefined;
  };
  visit(expression);
  return dynamic;
}

/** Trim JSX text the way Babel and TypeScript do: lines are trimmed and joined with one space. */
export function cleanJsxText(text: string): string {
  const lines = text.split(/\r\n|\n|\r/);
  const kept: string[] = [];
  for (let index = 0; index < lines.length; index += 1) {
    let line = lines[index]!;
    if (index !== 0) line = line.replace(/^[ \t]+/, "");
    if (index !== lines.length - 1) line = line.replace(/[ \t]+$/, "");
    if (line.length > 0) kept.push(line);
  }
  return kept.join(" ");
}

/** Outermost JSX expressions inside `node`, in source order; JSX nested in them is not listed. */
function collectJsxRoots(node: Node, roots: Node[]): void {
  node.forEachChild((child) => {
    if (isJsxLike(child)) roots.push(child);
    else collectJsxRoots(child, roots);
    return undefined;
  });
}

export interface LoweredFile {
  /** The rewritten TypeScript, or `undefined` when nothing in the file needed rewriting. */
  text: string | undefined;
}

interface Edit {
  start: number;
  end: number;
  text: string;
}

/** Lower every JSX expression in `source` and rewrite its module specifiers. */
export async function lowerSourceFile(source: SourceFile, options: LoweringOptions): Promise<LoweredFile> {
  const text = source.text;
  const lowering = new Lowering(source.fileName, text, options);
  const edits: Edit[] = [];
  const specifiers: StringLiteral[] = [];
  const roots: Node[] = [];
  source.forEachChild((statement) => {
    if (
      (is<ImportDeclaration>(statement, SyntaxKind.ImportDeclaration) || is<ExportDeclaration>(statement, SyntaxKind.ExportDeclaration)) &&
      statement.moduleSpecifier !== undefined &&
      is<StringLiteral>(statement.moduleSpecifier, SyntaxKind.StringLiteral)
    ) {
      specifiers.push(statement.moduleSpecifier);
    }
    if (isJsxLike(statement)) roots.push(statement);
    else collectJsxRoots(statement, roots);
    return undefined;
  });
  const collectModuleReferences = (node: Node): undefined => {
    if (is<ImportTypeNode>(node, SyntaxKind.ImportType) && is<LiteralTypeNode>(node.argument, SyntaxKind.LiteralType) && is<StringLiteral>(node.argument.literal, SyntaxKind.StringLiteral)) {
      specifiers.push(node.argument.literal);
    } else if (is<CallExpression>(node, SyntaxKind.CallExpression) && node.expression.kind === SyntaxKind.ImportKeyword && is<StringLiteral>(node.arguments[0], SyntaxKind.StringLiteral)) {
      specifiers.push(node.arguments[0]);
    }
    node.forEachChild(collectModuleReferences);
    return undefined;
  };
  collectModuleReferences(source);
  const rewrite = options.rewriteSpecifier;
  if (rewrite !== undefined) {
    for (const literal of specifiers) {
      const next = await rewrite(literal.text, literal);
      if (next !== undefined) {
        const edit = { start: literal.getStart() + 1, end: literal.end - 1, text: next };
        lowering.specifierEdits.push(edit);
        if (!roots.some((root) => edit.start >= root.getStart() && edit.end <= root.end)) edits.push(edit);
      }
    }
  }
  for (const root of roots) {
    edits.push({ start: root.getStart(), end: root.end, text: await lowering.lowerJsx(root) });
  }
  if (edits.length === 0) return { text: undefined };
  edits.sort((a, b) => a.start - b.start);
  let output = "";
  let cursor = 0;
  for (const edit of edits) {
    output += text.slice(cursor, edit.start) + edit.text;
    cursor = edit.end;
  }
  output += text.slice(cursor);
  return { text: lowering.importLines() + output };
}

interface LoweredValue {
  /** The expression as written, or `undefined` for a bare boolean attribute. */
  original: Node | undefined;
  /** The lowered expression text. */
  text: string;
  dynamic: boolean;
}

type ChildShape = "text" | "node" | "maybeNode" | "nodeList" | "children" | "nothing" | "unsupported";

class Lowering {
  readonly specifierEdits: Edit[] = [];
  readonly file: string;
  readonly text: string;
  readonly options: LoweringOptions;
  readonly checker: Checker;
  readonly helpers = new Set<string>();
  readonly typeImports = new Set<string>();
  #nextTemp = 1;
  #types = new Map<Node, Type | undefined>();

  constructor(file: string, text: string, options: LoweringOptions) {
    this.file = file;
    this.text = text;
    this.options = options;
    this.checker = options.checker;
  }

  error(node: Node, message: string): LoweringError {
    return new LoweringError(this.file, this.text, node, message);
  }

  helper(name: string): string {
    this.helpers.add(name);
    return HELPER_PREFIX + name;
  }

  /** The `Style` type of the generated module, imported under the helper prefix. */
  styleType(): string {
    this.typeImports.add("Style");
    return HELPER_PREFIX + "Style";
  }

  importLines(): string {
    const helpers = [...this.helpers].sort();
    const importFrom = (names: string[], specifier: string): string =>
      names.length === 0
        ? ""
        : `import { ${names.map((helper) => `${helper} as ${HELPER_PREFIX}${helper}`).join(", ")} } from ${JSON.stringify(specifier)};\n`;
    const types = [...this.typeImports].sort();
    const typeLine =
      types.length === 0
        ? ""
        : `import type { ${types.map((name) => `${name} as ${HELPER_PREFIX}${name}`).join(", ")} } from ${JSON.stringify(this.options.generatedSpecifier)};\n`;
    return (
      importFrom(helpers.filter((helper) => !REACTIVE_HELPERS.has(helper) && !GENERATED_HELPERS.has(helper)), this.options.runtimeSpecifier) +
      importFrom(helpers.filter((helper) => REACTIVE_HELPERS.has(helper)), this.options.reactiveSpecifier) +
      importFrom(helpers.filter((helper) => GENERATED_HELPERS.has(helper)), this.options.generatedSpecifier) +
      typeLine
    );
  }

  // -------------------------------------------------------------------------
  // Types
  // -------------------------------------------------------------------------

  async typeAt(node: Node): Promise<Type | undefined> {
    if (this.#types.has(node)) return this.#types.get(node);
    const type = await this.checker.getTypeAtLocation(node);
    this.#types.set(node, type);
    return type;
  }

  callSignatures(type: Type): Promise<readonly Signature[]> {
    return this.checker.getSignaturesOfType(type, SignatureKind.Call);
  }

  async nonNullable(type: Type): Promise<Type> {
    return (await this.checker.getNonNullableType(type)) ?? type;
  }

  async members(type: Type): Promise<readonly Type[]> {
    if (!type.isUnionType()) return [type];
    return (await type.getTypes()) ?? [type];
  }

  async isNativeNodeType(type: Type): Promise<boolean> {
    if ((type.flags & TypeFlags.Object) === 0) return false;
    const symbol = await type.getSymbol();
    return symbol !== undefined && symbol.name === "NativeNode" && (symbol.flags & SymbolFlags.Class) !== 0;
  }

  async arrayElement(type: Type): Promise<Type | undefined> {
    if (!(await this.checker.isArrayType(type))) return undefined;
    const arguments_ = await this.checker.getTypeArguments(type as TypeReference);
    return arguments_[0];
  }

  async propertyType(propsType: Type, name: string, at: Node): Promise<Type | undefined> {
    const property = await this.checker.getPropertyOfType(propsType, name);
    if (property === undefined) return undefined;
    return this.checker.getTypeOfSymbolAtLocation(property, at);
  }

  /**
   * Whether a prop is declared as a zero-argument accessor (`() => T`), on its own or as one arm
   * of a union such as `T | Accessor<T>`. The union's members are inspected one by one: the
   * checker reports call signatures for a union only when every member is callable.
   */
  async acceptsAccessor(declared: Type): Promise<boolean> {
    for (const member of await this.members(await this.nonNullable(declared))) {
      const signatures = await this.callSignatures(member);
      if (signatures.length > 0 && signatures.every((signature) => signature.parameters.length === 0)) return true;
    }
    return false;
  }

  /** Whether a prop declared with an accessor arm also accepts the plain value. */
  async acceptsPlainValue(declared: Type): Promise<boolean> {
    const nonNull = await this.nonNullable(declared);
    if (!nonNull.isUnionType()) return false;
    for (const member of await this.members(nonNull)) {
      if ((await this.callSignatures(member)).length === 0) return true;
    }
    return false;
  }

  async isFunctionValue(expression: Node | undefined): Promise<boolean> {
    if (expression === undefined) return false;
    const inner = stripParentheses(expression);
    if (inner.kind === SyntaxKind.ArrowFunction || inner.kind === SyntaxKind.FunctionExpression) return true;
    const type = await this.typeAt(inner);
    return type !== undefined && (await this.callSignatures(type)).length > 0;
  }

  async typeToString(type: Type | undefined): Promise<string> {
    return type === undefined ? "unknown" : this.checker.typeToString(type);
  }

  // -------------------------------------------------------------------------
  // Text
  // -------------------------------------------------------------------------

  /** The source text of `node` with every JSX expression inside it lowered. */
  async loweredText(node: Node): Promise<string> {
    if (isJsxLike(node)) return this.lowerJsx(node);
    const roots: Node[] = [];
    collectJsxRoots(node, roots);
    const start = node.getStart();
    if (roots.length === 0) return this.sourceSlice(start, node.end);
    let output = "";
    let cursor = start;
    for (const root of roots) {
      output += this.sourceSlice(cursor, root.getStart()) + (await this.lowerJsx(root));
      cursor = root.end;
    }
    return output + this.sourceSlice(cursor, node.end);
  }

  sourceText(node: Node): string {
    return this.sourceSlice(node.getStart(), node.end);
  }

  sourceSlice(start: number, end: number): string {
    let output = "";
    let cursor = start;
    for (const edit of this.specifierEdits.filter((edit) => edit.start >= start && edit.end <= end).sort((a, b) => a.start - b.start)) {
      output += this.text.slice(cursor, edit.start) + edit.text;
      cursor = edit.end;
    }
    return output + this.text.slice(cursor, end);
  }

  // -------------------------------------------------------------------------
  // JSX
  // -------------------------------------------------------------------------

  async lowerJsx(node: Node): Promise<string> {
    if (is<JsxFragment>(node, SyntaxKind.JsxFragment)) return this.fragmentExpression(node.children);
    const opening = is<JsxElement>(node, SyntaxKind.JsxElement) ? node.openingElement : (node as JsxSelfClosingElement);
    const children: readonly JsxChild[] = is<JsxElement>(node, SyntaxKind.JsxElement) ? node.children : [];
    const tag = opening.tagName;
    if (is<Identifier>(tag, SyntaxKind.Identifier)) {
      const name = tag.text;
      if (name.length > 0 && name[0] === name[0]!.toLowerCase() && name[0] !== name[0]!.toUpperCase()) {
        throw this.error(tag, `QuickGUI has no intrinsic element <${name}>; import a component such as View or Text from @quickgui/ui`);
      }
      const elementTag = await this.hostElementTag(tag);
      if (elementTag !== undefined) return this.lowerHostElement(elementTag, opening.attributes, children);
    }
    return this.lowerComponent(tag, opening.attributes, children);
  }

  /** The host tag when the tag names a host element component from `@quickgui/ui`. */
  async hostElementTag(tag: Identifier): Promise<number | undefined> {
    const elementTag = elementTagByName.get(tag.text);
    if (elementTag === undefined) return undefined;
    let symbol = await this.checker.getSymbolAtLocation(tag);
    if (symbol === undefined) return undefined;
    if ((symbol.flags & SymbolFlags.Alias) !== 0) symbol = await this.checker.getAliasedSymbol(symbol);
    const declaration = symbol.declarations[0];
    if (declaration === undefined) return undefined;
    return String(declaration.path).toLowerCase() === this.options.uiIndexPath.toLowerCase() ? elementTag : undefined;
  }

  async fragmentExpression(children: readonly JsxChild[]): Promise<string> {
    const nodes: string[] = [];
    for (const child of children) {
      const expression = await this.childNodeExpression(child);
      if (expression !== undefined) nodes.push(expression);
    }
    if (nodes.length === 1) return nodes[0]!;
    return `${this.helper("fragment")}([${nodes.join(", ")}])`;
  }

  // -------------------------------------------------------------------------
  // Host elements
  // -------------------------------------------------------------------------

  async lowerHostElement(tag: number, attributes: JsxAttributes, children: readonly JsxChild[]): Promise<string> {
    const element = `_el$${this.#nextTemp++}`;
    const statements: string[] = [`const ${element} = ${this.helper("element")}(${tag});`];
    for (const attribute of attributes.properties) {
      if (!is<JsxAttribute>(attribute, SyntaxKind.JsxAttribute)) {
        throw this.error(attribute, "QuickGUI host elements do not accept spread attributes; pass each prop explicitly");
      }
      const name = attributeName(attribute);
      if (name === "key") continue;
      if (name === "children") {
        throw this.error(attribute, "pass children between the element's tags instead of a `children` attribute");
      }
      const value = await this.attributeValue(attribute);
      if (name === "style") {
        statements.push(...(await this.lowerStyleAttribute(element, attribute, value)));
        continue;
      }
      if (name === "ref") {
        statements.push(`${this.helper("setRef")}(${element}, ${value.text});`);
        continue;
      }
      const eventType = eventByName.get(name);
      if (eventType !== undefined) {
        statements.push(`${this.helper("setListener")}(${element}, ${eventType}, ${await this.handlerExpression(value)});`);
        continue;
      }
      // `aria-label` and friends are the camel-cased props written the HTML way.
      const entry = propertyByName.get(name) ?? propertyByName.get(camelCase(name));
      if (entry === undefined) throw this.error(attribute, `unknown QuickGUI element prop \`${name}\``);
      statements.push(this.setterStatement(element, this.propFlagEntry(name) ?? entry, value, attribute));
    }
    for (const child of children) {
      const expression = await this.childNodeExpression(child);
      if (expression === undefined) continue;
      statements.push(`${this.helper("insert")}(${element}, ${expression});`);
    }
    statements.push(`return ${element};`);
    return `(() => {\n${statements.map((statement) => `  ${statement}`).join("\n")}\n})()`;
  }

  /**
   * An event handler with exactly the listener's shape: a handler written without the event
   * parameter is wrapped, because the native compiler does not widen function types.
   */
  async handlerExpression(value: LoweredValue): Promise<string> {
    const original = value.original;
    if (original === undefined) return value.text;
    const inner = stripParentheses(original);
    const isFunctionExpression = inner.kind === SyntaxKind.ArrowFunction || inner.kind === SyntaxKind.FunctionExpression;
    if (isFunctionExpression && (inner as ArrowFunction | FunctionExpression).parameters.length === 1) return value.text;
    if (!isFunctionExpression) {
      const type = await this.typeAt(inner);
      const signatures = type === undefined ? [] : await this.callSignatures(type);
      if (signatures.length > 0 && signatures[0]!.parameters.length === 1) return value.text;
    }
    return `(_event$) => (${value.text})()`;
  }

  /** One setter call, wrapped in a render effect when the value is reactive. */
  setterStatement(element: string, entry: PropertyEntry, value: LoweredValue, at: Node): string {
    let expression = value.text;
    const propEntry = entry;
    const setter = this.helper(setterForKind(propEntry.kind));
    const args: string[] = [element];
    if (propEntry.kind === KIND_BACKGROUND) {
      args.push(String(propEntry.code), String(propEntry.secondary ?? 0));
    } else if (propEntry.kind === KIND_GRID_PLACEMENT) {
      args.push(propEntry.secondary === 1 ? "true" : "false");
    } else if (propEntry.kind === KIND_STATE || propEntry.kind === KIND_GROUP_STATE) {
      args.push(String(propEntry.code), JSON.stringify(propEntry.name));
    } else if (propEntry.kind === KIND_JSON) {
      args.push(String(propEntry.code), String(propEntry.secondary ?? 65536));
    } else {
      args.push(String(propEntry.code));
    }
    if (propEntry.kind === KIND_COLOR && value.original !== undefined && is<StringLiteral>(value.original, SyntaxKind.StringLiteral)) {
      try {
        expression = String(parseColor(value.original.text));
      } catch (error) {
        throw this.error(at, error instanceof Error ? error.message : String(error));
      }
    }
    args.push(expression);
    const call = `${setter}(${args.join(", ")});`;
    return value.dynamic ? this.effectStatement(call) : call;
  }

  /** `disabled`, `invalid`, and `selected` as element props are flags, not nested states. */
  propFlagEntry(name: string): PropertyEntry | undefined {
    for (const entry of propertyTable) {
      if (!entry.style && entry.name === name) return entry;
    }
    return undefined;
  }

  effectStatement(statement: string): string {
    return `${this.helper("createRenderEffect")}(() => {\n    ${statement}\n  });`;
  }

  async lowerStyleAttribute(element: string, attribute: JsxAttribute, value: LoweredValue): Promise<string[]> {
    const original = value.original;
    if (original !== undefined && is<ObjectLiteralExpression>(original, SyntaxKind.ObjectLiteralExpression)) {
      const properties = original.properties;
      const hasSpread = properties.some((property) => property.kind === SyntaxKind.SpreadAssignment);
      const plain = properties.every(
        (property) => property.kind === SyntaxKind.PropertyAssignment || property.kind === SyntaxKind.ShorthandPropertyAssignment,
      );
      if (plain) {
        const statements: string[] = [];
        for (const property of properties) {
          const assignment = property as PropertyAssignment | ShorthandPropertyAssignment;
          const name = propertyKeyName(assignment);
          if (name === undefined) throw this.error(property, "QuickGUI style properties must be plain names");
          const entry = styleEntryByName.get(name);
          if (entry === undefined) throw this.error(property, `unknown QuickGUI style property \`${name}\``);
          const initializer = is<PropertyAssignment>(assignment, SyntaxKind.PropertyAssignment) ? assignment.initializer : assignment.name;
          statements.push(
            this.setterStatement(
              element,
              entry,
              { original: initializer, text: await this.loweredText(initializer), dynamic: isDynamicExpression(initializer) },
              property,
            ),
          );
        }
        return statements;
      }
      if (hasSpread) {
        // `{ ...base, color }` is a style list: the spread sources and the declared properties
        // in order, merged left to right by the runtime.
        const segments: string[] = [];
        let pending: string[] = [];
        const flush = (): void => {
          if (pending.length > 0) segments.push(`{ ${pending.join(", ")} }`);
          pending = [];
        };
        for (const property of properties) {
          if (is<SpreadAssignment>(property, SyntaxKind.SpreadAssignment)) {
            flush();
            segments.push(await this.loweredText(property.expression));
          } else if (is<PropertyAssignment>(property, SyntaxKind.PropertyAssignment)) {
            const name = propertyKeyName(property);
            if (name === undefined) throw this.error(property, "QuickGUI style properties must be plain names");
            pending.push(`${propertyName(name)}: ${await this.loweredText(property.initializer)}`);
          } else if (is<ShorthandPropertyAssignment>(property, SyntaxKind.ShorthandPropertyAssignment)) {
            const shorthand = (property.name as Identifier).text;
            pending.push(`${shorthand}: ${shorthand}`);
          } else {
            throw this.error(property, "unsupported style property");
          }
        }
        flush();
        return [`${this.helper("bindStyleList")}(${element}, (): ${this.styleType()}[] => [${segments.join(", ")}]);`];
      }
    }
    const type = original === undefined ? undefined : await this.typeAt(original);
    const list = type !== undefined && ((await this.checker.isArrayType(type)) || (await this.checker.isTupleType(type)));
    if (list) return [`${this.helper("bindStyleList")}(${element}, (): ${this.styleType()}[] => ${value.text});`];
    return [`${this.helper("bindStyle")}(${element}, (): ${this.styleType()} => ${value.text});`];
  }

  async attributeValue(attribute: JsxAttribute): Promise<LoweredValue> {
    const initializer = attribute.initializer;
    if (initializer === undefined) return { original: undefined, text: "true", dynamic: false };
    if (is<StringLiteral>(initializer, SyntaxKind.StringLiteral)) {
      return { original: initializer, text: JSON.stringify(initializer.text), dynamic: false };
    }
    if (is<JsxExpression>(initializer, SyntaxKind.JsxExpression)) {
      const expression = initializer.expression;
      if (expression === undefined) throw this.error(attribute, "empty JSX expression");
      return { original: expression, text: await this.loweredText(expression), dynamic: isDynamicExpression(expression) };
    }
    // A JSX element as an attribute value.
    return { original: initializer, text: await this.lowerJsx(initializer), dynamic: false };
  }

  // -------------------------------------------------------------------------
  // Children
  // -------------------------------------------------------------------------

  /** One child as a node-valued expression, or `undefined` when it renders nothing. */
  async childNodeExpression(child: JsxChild): Promise<string | undefined> {
    if (is<JsxText>(child, SyntaxKind.JsxText)) {
      const text = cleanJsxText(child.text);
      if (text.length === 0) return undefined;
      return `${this.helper("text")}(${JSON.stringify(text)})`;
    }
    if (isJsxLike(child)) return this.lowerJsx(child);
    if (is<JsxExpression>(child, SyntaxKind.JsxExpression)) {
      const expression = child.expression;
      if (expression === undefined) return undefined;
      return this.expressionChild(expression);
    }
    throw this.error(child, "unsupported JSX child");
  }

  async expressionChild(expression: Expression): Promise<string | undefined> {
    const inner = stripParentheses(expression);
    // `cond && <X/>` renders X while cond holds.
    if (
      is<BinaryExpression>(inner, SyntaxKind.BinaryExpression) &&
      inner.operatorToken.kind === SyntaxKind.AmpersandAmpersandToken &&
      isJsxLike(stripParentheses(inner.right))
    ) {
      const condition = await this.loweredText(inner.left);
      const branch = await this.lowerJsx(stripParentheses(inner.right));
      return `${this.helper("dynamicMaybe")}(() => (${condition}) ? ${branch} : undefined)`;
    }
    // `cond ? <A/> : <B/>` and `cond ? <A/> : null`.
    if (
      is<ConditionalExpression>(inner, SyntaxKind.ConditionalExpression) &&
      (isJsxLike(stripParentheses(inner.whenTrue)) || isJsxLike(stripParentheses(inner.whenFalse)))
    ) {
      const condition = await this.loweredText(inner.condition);
      const whenTrue = await this.branchExpression(inner.whenTrue);
      const whenFalse = await this.branchExpression(inner.whenFalse);
      const maybe = whenTrue.maybe || whenFalse.maybe;
      return `${this.helper(maybe ? "dynamicMaybe" : "dynamic")}(() => (${condition}) ? ${whenTrue.text} : ${whenFalse.text})`;
    }
    const type = await this.typeAt(inner);
    const lowered = await this.loweredText(expression);
    const dynamic = isDynamicExpression(inner);
    const shape = await this.classifyChildType(type);
    switch (shape) {
      case "text": {
        const isString = type !== undefined && (type.flags & (TypeFlags.String | TypeFlags.StringLiteral)) !== 0 && !type.isUnionType();
        const asString = isString ? lowered : `String(${lowered})`;
        if (dynamic) return `${this.helper("dynamicText")}(() => ${asString})`;
        return `${this.helper("text")}(${asString})`;
      }
      case "node":
        return dynamic ? `${this.helper("dynamic")}(() => ${lowered})` : lowered;
      case "maybeNode":
        return `${this.helper("dynamicMaybe")}(() => ${lowered})`;
      case "nodeList":
        return dynamic ? `${this.helper("dynamicList")}(() => ${lowered})` : `${this.helper("fragment")}(${lowered})`;
      case "children":
        return `${this.helper("childrenFragment")}(${lowered})`;
      case "nothing":
        return undefined;
      default:
        throw this.error(
          expression,
          `a JSX child of type \`${await this.typeToString(type)}\` cannot be rendered; render a string, a number, a node, or a node array`,
        );
    }
  }

  async branchExpression(expression: Expression): Promise<{ text: string; maybe: boolean }> {
    const inner = stripParentheses(expression);
    if (isJsxLike(inner)) return { text: await this.lowerJsx(inner), maybe: false };
    if (
      inner.kind === SyntaxKind.NullKeyword ||
      (is<Identifier>(inner, SyntaxKind.Identifier) && inner.text === "undefined") ||
      inner.kind === SyntaxKind.FalseKeyword
    ) {
      return { text: "undefined", maybe: true };
    }
    const type = await this.typeAt(inner);
    const shape = await this.classifyChildType(type);
    const lowered = await this.loweredText(expression);
    if (shape === "node") return { text: lowered, maybe: false };
    if (shape === "maybeNode") return { text: lowered, maybe: true };
    if (shape === "text") return { text: `${this.helper("text")}(String(${lowered}))`, maybe: false };
    throw this.error(expression, `a conditional JSX branch of type \`${await this.typeToString(type)}\` cannot be rendered`);
  }

  /** Decide how a child expression renders from its type. */
  async classifyChildType(type: Type | undefined): Promise<ChildShape> {
    if (type === undefined) return "unsupported";
    const members = await this.members(type);
    let text = false;
    let node = false;
    let nullish = false;
    let list = false;
    let boolean = false;
    let other = false;
    for (const member of members) {
      if ((member.flags & (TypeFlags.String | TypeFlags.StringLiteral | TypeFlags.Number | TypeFlags.NumberLiteral)) !== 0) text = true;
      else if ((member.flags & (TypeFlags.Undefined | TypeFlags.Null | TypeFlags.Void)) !== 0) nullish = true;
      else if ((member.flags & (TypeFlags.Boolean | TypeFlags.BooleanLiteral)) !== 0) boolean = true;
      else if (await this.isNativeNodeType(member)) node = true;
      else {
        const element = await this.arrayElement(member);
        if (element !== undefined && (await this.isNativeNodeType(element))) list = true;
        else if (element !== undefined) {
          // An array of plain children.
          text = true;
          other = other || !(await this.isChildrenElement(element));
        } else other = true;
      }
    }
    if (other) return "unsupported";
    if (node && !text && !list) return nullish || boolean ? "maybeNode" : "node";
    if (text && !node && !list && !boolean) return "text";
    if (list && !text && !node && !nullish && !boolean) return "nodeList";
    if (!text && !node && !list) return "nothing";
    return "children";
  }

  /** Whether an array element type is a plain child: text, node, or nullish. */
  async isChildrenElement(type: Type): Promise<boolean> {
    for (const member of await this.members(type)) {
      const primitive =
        (member.flags &
          (TypeFlags.String | TypeFlags.StringLiteral | TypeFlags.Number | TypeFlags.NumberLiteral | TypeFlags.Boolean | TypeFlags.BooleanLiteral | TypeFlags.Undefined | TypeFlags.Null)) !==
        0;
      if (primitive || (await this.isNativeNodeType(member))) continue;
      const element = await this.arrayElement(member);
      if (element !== undefined && (await this.isChildrenElement(element))) continue;
      return false;
    }
    return true;
  }

  async singleNodeExpression(children: readonly JsxChild[]): Promise<string> {
    const nodes: string[] = [];
    for (const child of children) {
      const expression = await this.childNodeExpression(child);
      if (expression !== undefined) nodes.push(expression);
    }
    if (nodes.length === 1) return nodes[0]!;
    return `${this.helper("fragment")}([${nodes.join(", ")}])`;
  }

  // -------------------------------------------------------------------------
  // Components
  // -------------------------------------------------------------------------

  async lowerComponent(tag: JsxTagNameExpression, attributes: JsxAttributes, children: readonly JsxChild[]): Promise<string> {
    const propsType = await this.componentPropsType(tag);
    const entries: string[] = [];
    for (const attribute of attributes.properties) {
      if (!is<JsxAttribute>(attribute, SyntaxKind.JsxAttribute)) {
        throw this.error(attribute, "QuickGUI components do not accept spread attributes; pass each prop explicitly");
      }
      const name = attributeName(attribute);
      const value = await this.attributeValue(attribute);
      const declared = propsType === undefined ? undefined : await this.propertyType(propsType, name, tag);
      let expression = value.text;
      if (declared !== undefined && (await this.acceptsAccessor(declared)) && !(await this.isFunctionValue(value.original))) {
        // A prop declared as `T | Accessor<T>` takes a static value as it is; anything else becomes
        // an accessor so the component reads it lazily and reactively.
        if (value.dynamic || !(await this.acceptsPlainValue(declared))) {
          expression = await this.accessorThunk(tag, name, declared, expression);
        }
      } else if (declared !== undefined && value.original !== undefined) {
        expression = await this.callbackExpression(declared, value);
      }
      entries.push(`${propertyName(name)}: ${expression}`);
    }
    const meaningful = children.filter((child) => !(is<JsxText>(child, SyntaxKind.JsxText) && cleanJsxText(child.text).length === 0));
    if (meaningful.length > 0) {
      const declared = propsType === undefined ? undefined : await this.propertyType(propsType, "children", tag);
      if (declared === undefined) throw this.error(tag, `component <${this.sourceText(tag)}> does not declare a \`children\` prop`);
      entries.push(`children: ${await this.componentChildren(declared, meaningful, tag)}`);
    }
    // A component body runs untracked, as Solid's `createComponent` does: a signal it reads while
    // building its subtree belongs to the effects it creates, not to whichever effect mounted it.
    // Otherwise a readout inside the component would re-create the whole subtree on every change.
    const callee = this.sourceText(tag);
    const untrack = this.helper("untrack");
    if (entries.length === 0 && (await this.componentTakesNoProps(tag))) return `${untrack}(() => ${callee}())`;
    if (entries.length <= 1) return `${untrack}(() => ${callee}({ ${entries.join(", ")} }))`;
    return `${untrack}(() => ${callee}({\n${entries.map((entry) => `  ${entry},`).join("\n")}\n}))`;
  }

  /**
   * A callback written with fewer parameters than the prop declares is wrapped to the declared
   * arity, because the native compiler does not widen function types.
   */
  async callbackExpression(declared: Type, value: LoweredValue): Promise<string> {
    const original = value.original;
    if (original === undefined) return value.text;
    const inner = stripParentheses(original);
    let written: number;
    if (inner.kind === SyntaxKind.ArrowFunction || inner.kind === SyntaxKind.FunctionExpression) {
      const parameters = (inner as ArrowFunction | FunctionExpression).parameters;
      if (parameters.some((parameter) => parameter.dotDotDotToken !== undefined)) return value.text;
      written = parameters.length;
    } else if (inner.kind === SyntaxKind.Identifier) {
      const actual = await this.typeAt(inner);
      if (actual === undefined || actual.isUnionType()) return value.text;
      const actualSignatures = await this.callSignatures(actual);
      if (actualSignatures.length !== 1 || actualSignatures[0]!.hasRestParameter) return value.text;
      written = actualSignatures[0]!.parameters.length;
    } else return value.text;
    const signatures = await this.callSignatures(await this.nonNullable(declared));
    if (signatures.length !== 1) return value.text;
    const expected = signatures[0]!.parameters.length;
    if (written >= expected || signatures[0]!.hasRestParameter) return value.text;
    const names: string[] = [];
    for (let index = 0; index < expected; index += 1) names.push(`_p$${index}`);
    const call = `(${value.text})(${names.slice(0, written).join(", ")})`;
    const result = await this.checker.getReturnTypeOfSignature(signatures[0]!);
    const body = result !== undefined && (result.flags & TypeFlags.Void) !== 0 ? `{ ${call}; }` : call;
    return `(${names.join(", ")}) => ${body}`;
  }

  async componentTakesNoProps(tag: Node): Promise<boolean> {
    const type = await this.typeAt(tag);
    if (type === undefined) return false;
    const signatures = await this.callSignatures(type);
    return signatures.length > 0 && signatures[0]!.parameters.length === 0;
  }

  async componentPropsType(tag: Node): Promise<Type | undefined> {
    const type = await this.typeAt(tag);
    if (type === undefined) return undefined;
    const signatures = await this.callSignatures(type);
    if (signatures.length === 0) return undefined;
    const parameters = await signatures[0]!.getParameters();
    const parameter = parameters[0];
    if (parameter === undefined) return undefined;
    return this.checker.getTypeOfSymbolAtLocation(parameter, tag);
  }

  /**
   * Wrap a prop value in a zero-argument arrow function.
   *
   * The static compiler matches function types exactly, so an arrow returning an object literal
   * gets the accessor's declared return type spelled out through the component's own signature:
   * `(): ReturnType<Extract<NonNullable<Parameters<typeof Tag>[0]["name"]>, () => unknown>> => value`.
   * Primitive results and generic components keep the plain arrow, whose inferred type already
   * matches.
   */
  async accessorThunk(tag: JsxTagNameExpression, name: string, declared: Type, expression: string): Promise<string> {
    // The accessor arm may sit inside a union such as `T | Accessor<T>`.
    let accessor: Signature | undefined = undefined;
    for (const member of await this.members(await this.nonNullable(declared))) {
      accessor = (await this.callSignatures(member)).find((signature) => signature.parameters.length === 0);
      if (accessor !== undefined) break;
    }
    // An object literal body must be parenthesized, or the arrow would parse it as a block.
    const body = `(${expression})`;
    const primitive =
      TypeFlags.StringLike |
      TypeFlags.NumberLike |
      TypeFlags.BooleanLike |
      TypeFlags.BigIntLike |
      TypeFlags.EnumLike |
      TypeFlags.VoidLike |
      TypeFlags.Null |
      TypeFlags.Undefined;
    const entity = entityName(tag);
    if (accessor === undefined || entity === undefined) return `() => ${body}`;
    const returnType = await this.checker.getReturnTypeOfSignature(accessor);
    if (returnType === undefined || (returnType.flags & primitive) !== 0) return `() => ${body}`;
    const componentType = await this.typeAt(tag);
    const componentSignatures = componentType === undefined ? [] : await this.callSignatures(componentType);
    const generic = componentSignatures.some((signature) => signature.typeParameters !== undefined && signature.typeParameters.length > 0);
    if (generic) return `() => ${body}`;
    return `(): ReturnType<Extract<NonNullable<Parameters<typeof ${entity}>[0][${JSON.stringify(name)}]>, () => unknown>> => ${body}`;
  }

  async componentChildren(declared: Type, children: readonly JsxChild[], tag: JsxTagNameExpression): Promise<string> {
    const nonNull = await this.nonNullable(declared);
    const signatures = await this.callSignatures(nonNull);
    if (signatures.length > 0) {
      const signature = signatures[0]!;
      if (signature.parameters.length === 0) {
        // A lazily created subtree: `() => node`.
        return `() => ${await this.singleNodeExpression(children)}`;
      }
      // A render callback such as For's `(item, index) => node`: passed through.
      const only = children.length === 1 ? children[0] : undefined;
      if (only === undefined || !is<JsxExpression>(only, SyntaxKind.JsxExpression) || only.expression === undefined) {
        throw this.error(tag, `<${this.sourceText(tag)}> expects one function child`);
      }
      return this.loweredText(only.expression);
    }
    const element = await this.arrayElement(nonNull);
    if (element !== undefined) {
      const entries: string[] = [];
      if (await this.isNativeNodeType(element)) {
        for (const child of children) {
          const expression = await this.childNodeExpression(child);
          if (expression !== undefined) entries.push(expression);
        }
      } else {
        for (const child of children) {
          if (isJsxLike(child)) entries.push(await this.lowerJsx(child));
          else if (is<JsxExpression>(child, SyntaxKind.JsxExpression) && child.expression !== undefined) {
            entries.push(await this.loweredText(child.expression));
          } else if (!is<JsxText>(child, SyntaxKind.JsxText)) throw this.error(child, "unsupported JSX child");
        }
      }
      return `[${entries.join(", ")}]`;
    }
    if (await this.isNativeNodeType(nonNull)) return this.singleNodeExpression(children);
    // A `Children` value: strings, numbers, and nodes as written.
    const entries: string[] = [];
    for (const child of children) {
      if (is<JsxText>(child, SyntaxKind.JsxText)) entries.push(JSON.stringify(cleanJsxText(child.text)));
      else if (is<JsxExpression>(child, SyntaxKind.JsxExpression)) {
        if (child.expression !== undefined) entries.push(await this.loweredText(child.expression));
      } else entries.push(await this.lowerJsx(child));
    }
    return entries.length === 1 ? entries[0]! : `[${entries.join(", ")}]`;
  }
}

function attributeName(attribute: JsxAttribute): string {
  const name = attribute.name;
  if (is<JsxNamespacedName>(name, SyntaxKind.JsxNamespacedName)) return `${name.namespace.text}:${name.name.text}`;
  return (name as Identifier).text;
}

function propertyKeyName(property: PropertyAssignment | ShorthandPropertyAssignment): string | undefined {
  const name = property.name;
  if (is<Identifier>(name, SyntaxKind.Identifier)) return name.text;
  if (is<StringLiteral>(name, SyntaxKind.StringLiteral)) return name.text;
  return undefined;
}

function camelCase(name: string): string {
  return name.replace(/-([a-z])/g, (_match, letter: string) => letter.toUpperCase());
}

function propertyName(name: string): string {
  return /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(name) ? name : JSON.stringify(name);
}

/** The JSX tag as a type-query entity name (`Foo` or `Foo.Bar`), or nothing for `this.Foo` and namespaced tags. */
function entityName(tag: Node): string | undefined {
  if (is<Identifier>(tag, SyntaxKind.Identifier)) return tag.text;
  if (is<PropertyAccessExpression>(tag, SyntaxKind.PropertyAccessExpression) && is<Identifier>(tag.name, SyntaxKind.Identifier)) {
    const left = entityName(tag.expression);
    return left === undefined ? undefined : `${left}.${tag.name.text}`;
  }
  return undefined;
}

/** Whether the JSX children include one non-blank entry (used by callers building fragments). */
export function hasMeaningfulChildren(children: readonly JsxChild[]): boolean {
  return children.some((child) => !(is<JsxText>(child, SyntaxKind.JsxText) && cleanJsxText(child.text).length === 0));
}

/** The kinds of nodes that carry JSX, exported for the project compiler's quick scans. */
export const JSX_KINDS: readonly SyntaxKind[] = [SyntaxKind.JsxElement, SyntaxKind.JsxSelfClosingElement, SyntaxKind.JsxFragment];

export type { NodeArray };
