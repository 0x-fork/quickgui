export const snippets = {
  rust: {
    lang: 'rust',
    code: `use quickgui::{
    Application, Color, EventContext, IntoElement, View, ViewContext, WindowOptions, div, text,
};

struct Counter {
    count: usize,
}

impl View for Counter {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let increment = cx.listener("increment", |this, cx: &mut EventContext| {
            this.count += 1;
            cx.invalidate();
        });

        div()
            .size_full()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .bg(Color::rgb8(18, 18, 20))
            .text_color(Color::rgb8(240, 241, 244))
            .child(text(format!("Count: {}", self.count)).text_2xl())
            .child(
                div()
                    .on_click(increment)
                    .px_4()
                    .py_2()
                    .rounded_lg()
                    .bg(Color::rgb8(45, 105, 180))
                    .hover(|style| style.bg(Color::rgb8(56, 122, 204)))
                    .child("Increment"),
            )
    }
}

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions::new("Counter").size(480.0, 320.0),
            Counter { count: 0 },
        );
    })
}`,
  },
  solid: {
    lang: 'tsx',
    code: `import { app, Window } from "@quickgui/native";
import { Button, Text, View, createRenderer } from "@quickgui/solid";
import { createSignal } from "solid-js";

function Counter() {
  const [count, setCount] = createSignal(0);

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        alignItems: "center",
        justifyContent: "center",
        gap: 12,
      }}
    >
      <Text>Count: {count()}</Text>
      <Button onClick={() => setCount((value) => value + 1)}>
        Increment
      </Button>
    </View>
  );
}

await app.whenReady();
new Window({
  title: "Counter",
  width: 480,
  height: 320,
  renderer: createRenderer(() => <Counter />),
});`,
  },
  virtualList: {
    lang: 'rust',
    code: `// Mounts only the visible range of 100,000 rows.
let rows = ListState::new(100_000, 96.0).with_overscan(3);

rows.set_viewport_size(cx.size().width, cx.size().height);
let visible = rows.visible_rows();
rows.render_rows(visible.range, |index| {
    div().w_full().px_4().py_3().child(text(format!("Row {index}")))
})`,
  },
  cargoAdd: {
    lang: 'bash',
    code: `cargo add quickgui`,
  },
  cargoRun: {
    lang: 'bash',
    code: `cargo run --release`,
  },
  cliInit: {
    lang: 'bash',
    code: `bunx @quickgui/cli init my-app
cd my-app
bun run dev`,
  },
  cliBuild: {
    lang: 'bash',
    code: `quickgui build --target darwin-arm64 \\
  --sign "Developer ID Application: Example (TEAMID)" \\
  --notarize quickgui-notary`,
  },
} as const

export type SnippetKey = keyof typeof snippets
