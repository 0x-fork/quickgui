import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { ALL_COMPONENT_DOCS } from "../website/src/lib/component-docs";
import {
  localizedComponentDescription,
  localizedComponentOutline,
} from "../website/src/lib/docs-locales";
import { moonComponents } from "./moonbit-components";
import { childLists } from "./moonbit-child-lists.ts";
import { signalPairs } from "./moonbit-signal-pairs.ts";
import { directBindings } from "./moonbit-direct-bindings.ts";
import { fluentProps } from "./moonbit-fluent-props.ts";
import { createMoonbitParser } from "../packages/cli/src/moonbit/parser.ts";

const root = resolve(import.meta.dir, "..");
const viewParser = await createMoonbitParser();
const examples: Record<string, string> = {
  view: '@ui.div([\n@ui.text("Hello"),\n]).size_full().flex_col().gap(12).padding(24)',
  text: 'let count = @reactive.signal(0)\n@ui.text("Count: \\{count.get()}")',
  button: '@ui.button("Save").on_click(() => println("Saved")).px(16).py(8)',
  input:
    'let value = @reactive.signal("")\n@ui.input().on_input(next => value.set(next)).bind_value(() => value.get()).placeholder("Name")',
  "text-area":
    'let value = @reactive.signal("")\n@ui.text_area().on_input(next => value.set(next)).bind_value(() => value.get()).height(120)',
  markdown: '@ui.markdown("# Hello\\n\\nThis is **native** Markdown.").streaming(false)',
  image: '@ui.image("assets/avatar.png").width(64).height(64)',
  svg: '@ui.svg("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"M4 12h16M12 4v16\" stroke=\"black\" stroke-width=\"2\"/></svg>").width(20).height(20).text_color(@ui.rgb8(48, 120, 220))',
  shader:
    '@ui.shader("fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> { return vec4<f32>(input.uv, 0.5, 1.0); }").width(240).height(160)',
  "virtual-list":
    '@ui.virtual_list([\n@ui.text("One"), @ui.text("Two"), @ui.text("Three"),\n]).height(320).estimated_item_height(32).overscan(4)',
  terminal: '@terminal.view("/bin/zsh", args=["-l"]).size_full()',
  checkbox:
    "let checked = @reactive.signal(false)\nlet box = @ui.checkbox([]).bind_checked(() => checked.get()).on_checked_change(next => checked.set(next))\nbox.child(box.slot(@ui.Indicator)).width(20).height(20)",
  "checkbox-group":
    'let group = @ui.checkbox_group([]).items(["email", "push"]).values(["email"])\ngroup.flex_col().gap(8).children([group.slot(@ui.Item, value="email").label("Email"), group.slot(@ui.Item, value="push").label("Push"), group.slot(@ui.Parent).label("All notifications")])',
  radio:
    "let radio = @ui.radio([]).checked(true)\nradio.child(radio.slot(@ui.Indicator)).width(20).height(20)",
  "radio-group":
    'let group = @ui.radio_group([]).active_value("small")\ngroup.flex_row().gap(12).children([group.slot(@ui.Item, value="small").label("Small"), group.slot(@ui.Item, value="large").label("Large")])',
  switch:
    "let toggle = @ui.switch([])\ntoggle.width(40).height(24).border_radius(12).child(toggle.slot(@ui.Thumb).width(20).height(20).border_radius(10))",
  toggle: '@ui.toggle([]).pressed(false).label("Bold").px(12).py(8)',
  "toggle-group":
    'let group = @ui.toggle_group([]).multiple(true).items([{"value": "bold", "label": "Bold"}, {"value": "italic", "label": "Italic"}])\ngroup.flex_row().gap(8).children([group.slot(@ui.Item, value="bold").label("Bold"), group.slot(@ui.Item, value="italic").label("Italic")])',
  slider:
    "let slider = @ui.slider([]).values([40]).minimum(0).maximum(100).step(1)\nslider.width(260).child(slider.slot(@ui.Control).height(24).child(slider.slot(@ui.Track).height(4).child(slider.slot(@ui.Indicator))).child(slider.slot(@ui.Thumb, index=0).width(16).height(16).border_radius(8)))",
  "number-field":
    'let field = @ui.number_field([]).values([5]).minimum(0).maximum(10).step(1)\nfield.flex_row().children([field.slot(@ui.Decrement).label("Less"), field.slot(@ui.Input).width(80), field.slot(@ui.Increment).label("More")])',
  select:
    '@ui.select([]).items([{"value": "go", "label": "Go"}, {"value": "moonbit", "label": "MoonBit"}]).active_value("moonbit").width(180).height(32)',
  combobox:
    '@ui.combobox([]).items([{"value": "go", "label": "Go"}, {"value": "moonbit", "label": "MoonBit"}]).filter_mode("fuzzy").placeholder("Choose a language").width(220)',
  autocomplete:
    '@ui.autocomplete([]).items([{"value": "Paris", "label": "Paris"}, {"value": "Prague", "label": "Prague"}]).filter_mode("startsWith").placeholder("City").width(220)',
  field:
    'let field = @ui.field([]).required(true)\nfield.flex_col().gap(6).children([field.slot(@ui.Label).label("Email"), field.slot(@ui.Control).placeholder("you@example.com"), field.slot(@ui.Description).label("Used for notifications"), field.slot(@ui.Error)])',
  fieldset:
    'let fields = @ui.fieldset([])\nfields.flex_col().gap(8).children([fields.slot(@ui.Legend).label("Profile"), fields.slot(@ui.Control).placeholder("Display name")])',
  "date-field":
    'let field = @ui.date_field([]).civil_value("2026-09-08")\nfield.flex_row().gap(4).children([field.slot(@ui.Segment, value="year"), field.slot(@ui.Segment, value="month"), field.slot(@ui.Segment, value="day")])',
  "time-field":
    'let field = @ui.time_field([]).civil_value("09:30")\nfield.flex_row().gap(4).children([field.slot(@ui.Segment, value="hour"), @ui.text(":"), field.slot(@ui.Segment, value="minute")])',
  calendar:
    'let calendar = @ui.calendar([]).civil_value("2026-09-08").first_weekday(1)\ncalendar.child(calendar.slot(@ui.Week, index=0).flex_row().children([calendar.slot(@ui.Day, value="2026-09-07").label("7"), calendar.slot(@ui.Day, value="2026-09-08").label("8")]))',
  "otp-field":
    "let code = @ui.otp_field([]).length(6).auto_submit(true)\ncode.child(code.slot(@ui.Input).width(220))",
  tabs: 'let tabs = @ui.tabs([]).active_value("overview")\ntabs.flex_col().gap(12).children([tabs.slot(@ui.List).flex_row().gap(8).children([tabs.slot(@ui.Tab, value="overview", index=0).label("Overview"), tabs.slot(@ui.Tab, value="settings", index=1).label("Settings")]), tabs.slot(@ui.Panel, value="overview").child(@ui.text("Overview content")), tabs.slot(@ui.Panel, value="settings").child(@ui.text("Settings content"))])',
  accordion:
    'let accordion = @ui.accordion([]).multiple(true)\nlet item = accordion.slot(@ui.Item, value="details", index=0)\naccordion.child(item.flex_col().children([item.slot(@ui.Header).child(item.slot(@ui.Trigger).label("Details")), item.slot(@ui.Panel).child(@ui.text("Expanded content"))]))',
  collapsible:
    'let section = @ui.collapsible([])\nsection.flex_col().children([section.slot(@ui.Trigger).label("Details"), section.slot(@ui.Panel).child(@ui.text("Expanded content"))])',
  splitter:
    'let split = @ui.splitter([]).values([240, 480]).items([{"min": 160}, {"min": 240}])\nsplit.size_full().flex_row().children([split.slot(@ui.Pane, index=0).child(@ui.text("Sidebar")), split.slot(@ui.Handle, index=0).width(4).cursor("col-resize"), split.slot(@ui.Pane, index=1).child(@ui.text("Content"))])',
  "scroll-area":
    'let scroll = @ui.scroll_area([])\nscroll.width(280).height(160).child(scroll.slot(@ui.Viewport).size_full().overflow_y("scroll").child(scroll.slot(@ui.Content).height(600).child(@ui.text("Scrollable content"))))',
  table:
    'let table = @ui.data_table([]).columns([{"id": "name", "track": "1fr"}]).row_count(1).row_height(32).header_height(0).selection_mode("single")\ntable.height(160).overflow_y("scroll").child(table.slot(@ui.Row, index=0).child(table.slot(@ui.Cell, value="name").child(@ui.text("Ada"))))',
  tree: 'let tree = @ui.tree([]).nodes([{"id": "src", "label": "src", "children": [{"id": "main", "label": "main.mbt"}]}]).expanded(["src"]).row_height(28)\ntree.height(160).children([tree.slot(@ui.Row, value="src").label("src"), tree.slot(@ui.Row, value="main").label("main.mbt")])',
  separator: '@ui.separator([]).orientation("horizontal").height(1).bg(@ui.rgb8(210, 210, 215))',
  avatar:
    'let avatar = @ui.avatar([])\navatar.width(40).height(40).children([avatar.slot(@ui.Image).value("assets/avatar.png").size_full(), avatar.slot(@ui.Fallback).label("AJ")])',
  progress:
    'let progress = @ui.progress([]).values([0.65]).maximum(1)\nprogress.width(240).flex_col().gap(6).children([progress.slot(@ui.Label).label("Upload"), progress.slot(@ui.Track).height(6).child(progress.slot(@ui.Indicator).bg(@ui.rgb8(40, 120, 230))), progress.slot(@ui.Value)])',
  meter:
    "let meter = @ui.meter([]).values([72]).minimum(0).maximum(100).low(20).high(80).optimum(60)\nmeter.width(240).child(meter.slot(@ui.Track).height(8).child(meter.slot(@ui.Indicator).bg(@ui.rgb8(30, 150, 90))))",
  toolbar:
    'let toolbar = @ui.toolbar([]).orientation("horizontal").items([{"value": "save", "label": "Save"}])\ntoolbar.flex_row().child(toolbar.slot(@ui.Button, value="save", index=0).label("Save").on_click(() => println("Saved")))',
  "system-popover":
    '@ui.system_popover([]).label("Open details").system_content(() => @ui.div([\n@ui.text("Native child window"),\n]).padding(16), options={"width": 260, "height": 120, "popoverPlacement": "bottom-start", "popoverDismissOnEscape": true, "popoverDismissOnPointerOutside": true})',
  tooltip:
    'let tip = @ui.tooltip([]).delay(500).side("top")\ntip.children([tip.slot(@ui.Trigger).label("Hover for help"), tip.slot(@ui.Positioner).child(tip.slot(@ui.Popup).padding(8).child(@ui.text("Helpful information")))])',
  "preview-card":
    'let preview = @ui.preview_card([]).delay(300).side("bottom")\npreview.children([preview.slot(@ui.Trigger).label("Preview"), preview.slot(@ui.Positioner).child(preview.slot(@ui.Popup).padding(16).child(@ui.text("More information")))])',
  toast:
    'let viewport = @ui.toast_viewport([]).toasts([{"id": "saved", "title": "Saved", "timeout": 5000}]).timeout(5000)\nlet toast = viewport.slot(@ui.Item, value="saved")\nviewport.child(toast.padding(16).children([toast.slot(@ui.Title).label("Saved"), toast.slot(@ui.Close).label("Dismiss")]))',
  menu: 'let menu = @ui.menu([])\nmenu.children([menu.slot(@ui.Trigger).label("Actions"), menu.slot(@ui.Positioner).child(menu.slot(@ui.Popup).padding(6).child(menu.slot(@ui.Item, value="copy").label("Copy").on_click(() => println("Copy"))))])',
  "popover-menu":
    'let menu = @ui.popover_menu([]).menu({"items": [{"type": "action", "id": "refresh", "label": "Refresh"}]})\nmenu.child(menu.slot(@ui.Trigger).label("Actions")).when_open(() => menu.slot(@ui.Popup).width(224).on("select", @protocol.SELECT_LISTENER, _ => { println("Refresh"); ignore(menu.open(false)) }))',
  "context-menu":
    'let menu = @ui.context_menu([]).menu({"items": [{"type": "action", "id": "copy", "label": "Copy"}]})\nmenu.child(menu.slot(@ui.Trigger).padding(24).label("Right-click here").on("select", @protocol.SELECT_LISTENER, _ => println("Copy")))',
  menubar:
    'let bar = @ui.menubar([]).menu_count(2)\nbar.flex_row().children([bar.slot(@ui.Item, index=0).label("File"), bar.slot(@ui.Item, index=1).label("Edit")])',
  "navigation-menu":
    'let nav = @ui.navigation_menu([])\nlet item = nav.slot(@ui.Item, value="docs", index=0)\nnav.child(nav.slot(@ui.List).child(item.children([item.slot(@ui.Trigger).label("Docs"), item.slot(@ui.Content).child(@ui.text("Documentation links"))])))',
  router:
    'let router = @ui.router([@ui.layout((router, outlet) => @ui.div([\nrouter.link("/", "Home"), router.link("/settings", "Settings"), outlet(),\n]).flex_col(), [@ui.route("/", _ => @ui.text("Home")), @ui.route("/settings", _ => @ui.text("Settings"))])])\nrouter.outlet()',
};
for (const kind of ["dialog", "alert-dialog"])
  examples[kind] =
    `let dialog = @ui.${kind.replaceAll("-", "_")}([])\ndialog.children([dialog.slot(@ui.Trigger).label("Open dialog"), dialog.slot(@ui.Portal).children([dialog.slot(@ui.Backdrop), dialog.slot(@ui.Popup).padding(24).flex_col().gap(12).children([dialog.slot(@ui.Title).label("Confirm action"), dialog.slot(@ui.Description).label("Review before continuing."), dialog.slot(@ui.Close).label("Close")])])])`;
examples.popover =
  'let popover = @ui.popover([]).side("bottom").side_offset(6)\npopover.children([popover.slot(@ui.Trigger).label("Details"), popover.slot(@ui.Positioner).child(popover.slot(@ui.Popup).padding(16).child(@ui.text("Popover content")))])';

const swift: Record<string, string> = {
  host: '@ui.swift_ui_host([\n@ui.swift_ui_button().value("Save").on_click(() => println("Saved")),\n])',
  button:
    '@ui.swift_ui_host([\n@ui.swift_ui_button().value("Save").swift_ui_system_image("square.and.arrow.down").on_click(() => println("Saved")),\n])',
  slider:
    "let value = @reactive.signal(40.0)\n@ui.swift_ui_host([\n@ui.swift_ui_slider().bind_number(() => Float::from_double(value.get())).minimum(0).maximum(100).on_number_input(next => value.set(next)),\n])",
  toggle:
    'let checked = @reactive.signal(false)\n@ui.swift_ui_host([\n@ui.swift_ui_toggle().bind_checked(() => checked.get()).on_bool_input(next => checked.set(next)).label("Notifications"),\n])',
  "progress-view":
    '@ui.swift_ui_host([\n@ui.swift_ui_progress_view().value(0.65).maximum(1).label("Uploading"),\n])',
  stepper:
    'let value = @reactive.signal(5.0)\n@ui.swift_ui_host([\n@ui.swift_ui_stepper().bind_number(() => Float::from_double(value.get())).minimum(0).maximum(10).step(1).on_number_input(next => value.set(next)).label("Copies"),\n])',
  "text-field":
    'let value = @reactive.signal("")\n@ui.swift_ui_host([\n@ui.swift_ui_text_field().bind_value(() => value.get()).on_input(next => value.set(next)).placeholder("Name"),\n])',
  "secure-field":
    'let value = @reactive.signal("")\n@ui.swift_ui_host([\n@ui.swift_ui_secure_field().bind_value(() => value.get()).on_input(next => value.set(next)).placeholder("Password"),\n])',
  picker:
    'let value = @reactive.signal("go")\n@ui.swift_ui_host([\n@ui.swift_ui_picker().bind_value(() => value.get()).on_input(next => value.set(next)).items([{"value": "go", "label": "Go"}, {"value": "moonbit", "label": "MoonBit"}]).label("Language"),\n])',
  "segmented-control":
    'let value = @reactive.signal("go")\n@ui.swift_ui_host([\n@ui.swift_ui_segmented_control().bind_value(() => value.get()).on_input(next => value.set(next)).items([{"value": "go", "label": "Go"}, {"value": "moonbit", "label": "MoonBit"}]),\n])',
  "date-picker":
    '@ui.swift_ui_host([\n@ui.swift_ui_date_picker().civil_value("2026-09-08").swift_ui_date_picker_components("date").label("Date"),\n])',
  "color-picker":
    'let value = @reactive.signal("#2563eb")\n@ui.swift_ui_host([\n@ui.swift_ui_color_picker().bind_value(() => value.get()).on_input(next => value.set(next)).swift_ui_color_supports_opacity(true).label("Color"),\n])',
  gauge:
    '@ui.swift_ui_host([\n@ui.swift_ui_gauge().value(65).minimum(0).maximum(100).swift_ui_gauge_style("accessoryLinear").label("Storage"),\n])',
  "quickgui-host-view":
    '@ui.swift_ui_host([\n@ui.swift_ui_quickgui_host_view(() => @ui.div([\n@ui.text("QuickGUI inside SwiftUI"),\n]).padding(12), width=320, height=120),\n])',
  popover:
    'let opened = @reactive.signal(false)\n@ui.swift_ui_host([\n@ui.swift_ui_popover([\n@ui.swift_ui_popover_trigger([\n@ui.swift_ui_button().value("Details").on_click(() => opened.update(value => !value)),\n]), @ui.swift_ui_popover_content([\n@ui.swift_ui_button().value("Close").on_click(() => opened.set(false)),\n]),\n]).bind_swift_ui_is_presented(() => opened.get()).on("presentation", 0, event => opened.set(event.value == Some("true"))),\n])',
};
const descriptions = {
  en: {
    import: "Add these package imports to `moon.pkg`:",
    scopes:
      "Create parts from the root or an item with `.slot(...)`. They inherit the component scope and item identity. Each call returns an `Element`, so the same fluent styling and child methods work throughout.",
    binding:
      "Read signal getters directly in fluent property setters. The QuickGUI CLI creates controlled bindings for those expressions. Use change callbacks to write state; literal properties initialize component-owned state. Explicit `bind_` methods remain available for lower-level composition.",
    native:
      "SwiftUI controls require macOS and a `swift_ui_host([])` ancestor. Use `label(...)` for a native control label, `value(...)` for its value, and `swift_ui_modifiers(...)` for native SwiftUI modifiers.",
    note: "The native core owns interaction, layout, accessibility and animation. These parts are unstyled: set sizes, colors and spacing for your application.",
    method: "Method",
    purpose: "Purpose",
    prop: "Native component configuration; `bind_` accepts a reactive accessor.",
    common: "Layout, typography, colors, interaction states and child composition.",
    event: "Receives semantic component state reported by the native core.",
  },
  zh: {
    import: "在 `moon.pkg` 中添加以下包：",
    scopes:
      "通过根元素或条目的 `.slot(...)` 创建部件。部件继承组件作用域和条目标识。每次调用都返回 `Element`，可继续使用相同的链式样式和子元素方法。",
    binding:
      "在链式属性方法中直接读取信号访问器，QuickGUI CLI 会为表达式创建受控绑定。通过变更回调写入状态；字面量属性初始化组件内部状态。底层组合仍可使用显式 `bind_` 方法。",
    native:
      "SwiftUI 控件仅支持 macOS，并且必须位于 `swift_ui_host([])` 中。使用 `label(...)` 设置原生标签，`value(...)` 设置值，`swift_ui_modifiers(...)` 设置 SwiftUI 修饰符。",
    note: "原生核心负责交互、布局、无障碍和动画。这些部件不预设视觉样式，请按应用需要设置尺寸、颜色和间距。",
    method: "方法",
    purpose: "用途",
    prop: "原生组件配置；`bind_` 接收响应式访问器。",
    common: "布局、字体、颜色、交互状态和子元素组合。",
    event: "接收原生核心报告的组件语义状态。",
  },
  ja: {
    import: "`moon.pkg` に次のパッケージを追加します。",
    scopes:
      "ルートまたは項目の `.slot(...)` からパーツを作成します。パーツはコンポーネントのスコープと項目の識別子を継承します。すべて `Element` を返すため、共通の fluent API でスタイルと子要素を設定できます。",
    binding:
      "fluent プロパティメソッドでシグナルの getter を直接読みます。QuickGUI CLI が式を制御用バインディングに変換します。変更コールバックで状態を書き込み、リテラルは内部状態の初期値に使います。低レベルの組み立てには明示的な `bind_` メソッドも使えます。",
    native:
      "SwiftUI コントロールは macOS 専用で、`swift_ui_host([])` の内側に配置します。`label(...)` はラベル、`value(...)` は値、`swift_ui_modifiers(...)` は SwiftUI 修飾子を設定します。",
    note: "ネイティブコアが操作、レイアウト、アクセシビリティ、アニメーションを管理します。パーツに既定の見た目はないため、サイズ、色、間隔を設定してください。",
    method: "メソッド",
    purpose: "用途",
    prop: "ネイティブコンポーネントの設定。`bind_` はリアクティブなアクセサーを受け取ります。",
    common: "レイアウト、文字、色、操作状態、子要素の構成。",
    event: "ネイティブコアが通知するコンポーネントの状態を受け取ります。",
  },
};
for (const component of ALL_COMPONENT_DOCS) {
  let body = component.kind === "swift-ui" ? swift[component.slug] : examples[component.slug];
  if (!body) throw new Error(`Missing MoonBit example: ${component.kind}/${component.slug}`);
  // SVG quotes must be escaped inside the MoonBit string literal.
  if (component.slug === "svg")
    body = body
      .replaceAll('="', '=\\"')
      .replaceAll('" ', '\\" ')
      .replaceAll('"><', '\\"><')
      .replaceAll('"/>', '\\"/>');
  const source = `fn component_example() -> @ui.Element {\n${body
    .split("\n")
    .map((line) => `  ${line}`)
    .join("\n")}\n}\n`;
  const formatted = Bun.spawnSync(
    [Bun.which("moonfmt") ?? resolve(root, "target/moonbit-toolchain/bin/moonfmt"), "-"],
    {
      stdin: Buffer.from(
        fluentProps(
          viewParser,
          directBindings(
            viewParser,
            signalPairs(viewParser, childLists(viewParser, source, component.slug), component.slug),
            component.slug,
          ),
          component.slug,
        ),
      ),
    },
  );
  if (formatted.exitCode) throw new Error(`${component.slug}: ${formatted.stderr}`);
  const code = formatted.stdout
    .toString()
    .replace(/^\/\/\/\|\n/, "")
    .trim();
  const spec =
    component.kind === "ui"
      ? moonComponents.find(
          (item) => item.name === (component.slug === "toast" ? "toast-viewport" : component.slug),
        )
      : undefined;
  for (const locale of ["en", "zh", "ja"] as const) {
    const text = descriptions[locale];
    const outline = localizedComponentOutline(component, locale);
    // Existing localized outline IDs are localized too, so preserve their order.
    const headings = outline.map((item) => item.title);
    const imports =
      component.slug === "terminal"
        ? ["egoist/quickgui/ui", "egoist/quickgui/terminal"]
        : [
            "egoist/quickgui/ui",
            "egoist/quickgui/reactive",
            ...(body.includes("@protocol") ? ["egoist/quickgui/protocol"] : []),
          ];
    let page = `${localizedComponentDescription(component, locale)}\n\n## ${headings[0]}\n\n${text.import}\n\n\`\`\`moonbit\nimport {\n${imports.map((name) => `  "${name}",`).join("\n")}\n}\n\`\`\`\n\n## ${headings[1]}\n\n\`\`\`moonbit\n${code}\n\`\`\`\n\n${component.kind === "swift-ui" ? text.native : text.note}\n\n`;
    if (component.slug === "terminal")
      page += {
        en: "Call `@terminal.register()` before `@native.run`. Add the separate terminal extension to `[native].extensions` in `quickgui.toml`; see [Authoring Extensions](/docs/moonbit/extensions).\n\n",
        zh: "在 `@native.run` 前调用 `@terminal.register()`，并在 `quickgui.toml` 的 `[native].extensions` 中添加独立的 terminal 扩展。参见[编写扩展](/docs/moonbit/extensions)。\n\n",
        ja: "`@native.run` の前に `@terminal.register()` を呼び、`quickgui.toml` の `[native].extensions` に terminal 拡張を追加します。[拡張の作成](/docs/moonbit/extensions)を参照してください。\n\n",
      }[locale];
    if (component.parts.length) {
      const slots = spec ? Object.keys(spec.parts) : [];
      const anatomy = slots.length
        ? text.scopes
        : component.kind === "swift-ui"
          ? text.native
          : text.note;
      page += `## ${headings[2]}\n\n${anatomy}\n\n`;
      page +=
        slots
          .map(
            (slot) =>
              `- \`root.slot(@ui.${slot
                .split("-")
                .map((word) => word[0]!.toUpperCase() + word.slice(1))
                .join("")})\``,
          )
          .join("\n") + "\n\n";
      if (!slots.length)
        page += `\`${component.slug === "router" ? "router(...).outlet()" : component.slug === "system-popover" ? "system_popover([]).system_content(...)" : "swift_ui_popover_trigger([]) / swift_ui_popover_content([])"}\`\n\n`;
    }
    page += `## ${headings.at(-1)}\n\n${spec ? text.binding : component.kind === "swift-ui" ? text.native : text.note}\n\n| ${text.method} | ${text.purpose} |\n| --- | --- |\n`;
    const methods = spec?.properties.map((name) => name.toLowerCase()) ?? [
      ...new Set([...body.matchAll(/\.([a-z_]+)\(/g)].map((match) => match[1])),
    ];
    const goSource = readFileSync(
      resolve(
        root,
        "website/src/content/docs/go/components",
        locale === "en" ? "" : locale,
        component.kind,
        `${component.slug}.mdx`,
      ),
      "utf8",
    );
    const purposes = new Map(
      [...goSource.matchAll(/^\| `([^`]+)` \| (.+) \|$/gm)].map((match) => [match[1], match[2]]),
    );
    const aliases: Record<string, string> = {
      values: "Value",
      active_value: "Value",
      minimum: "Min",
      maximum: "Max",
      civil_value: "Value",
      civil_minimum: "Min",
      civil_maximum: "Max",
      part_value: "Value",
      swipe_direction: "SwipeDirection",
      menu: "Items",
      nodes: "Nodes",
      checked: "Checked",
      swift_ui_is_presented: "IsPresented",
      swift_ui_modifiers: "Modifiers",
      swift_ui_picker_style: "Style",
      swift_ui_date_picker_components: "DisplayedComponents",
      swift_ui_date_picker_style: "Style",
      swift_ui_color_supports_opacity: "SupportsOpacity",
      swift_ui_gauge_style: "Style",
      on_bool_input: "OnIsOnChange",
      on_number_input: "OnValueChange",
      bind_number: "Value",
      on_click: component.kind === "swift-ui" ? "OnPress" : "OnClick",
    };
    page += methods
      .map((method) => {
        const base = method!.replace(/^bind_/, "");
        const prop =
          aliases[base] ??
          base
            .split("_")
            .map((word) => word[0]!.toUpperCase() + word.slice(1))
            .join("");
        const checkedPurpose = {
          en: "Boolean checked state. Use `indeterminate(true)` for a mixed checkbox.",
          zh: "布尔选中状态。复选框的中间态使用 `indeterminate(true)`。",
          ja: "真偽値の選択状態。チェックボックスの中間状態には `indeterminate(true)` を使います。",
        }[locale];
        const purpose =
          base === "checked" && component.kind === "ui"
            ? checkedPurpose
            : (purposes.get(prop) ?? `${text.prop} (${prop})`);
        return `| \`${method}(...)\` | ${purpose} |`;
      })
      .join("\n");
    if (spec && !["separator", "fieldset"].includes(spec.name))
      page += `\n| \`on_change(handler)\` | ${text.event} |`;
    page += `\n| \`style(...), [children]\` | ${text.common} |\n`;
    const dir = resolve(
      root,
      "website/src/content/docs/moonbit/components",
      locale === "en" ? "" : locale,
      component.kind === "ui" ? "ui" : "swift-ui",
    );
    mkdirSync(dir, { recursive: true });
    writeFileSync(resolve(dir, `${component.slug}.mdx`), page);
  }
}
viewParser.delete();
console.log(`Generated ${ALL_COMPONENT_DOCS.length} MoonBit component pages in three locales`);
