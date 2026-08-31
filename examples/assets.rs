use std::sync::Arc;

use quickgui::{
    Application, Assets, BundledAssets, Color, Svg, View, ViewContext, button, div, svg, text,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bundle = BundledAssets::new()
        .with("icons/bolt.svg", BOLT_ICON)?
        .with("copy/message.txt", MESSAGE)?;
    let assets = Assets::new(bundle);
    let icon = assets.svg("icons/bolt.svg")?;
    let message = load_message(&assets)?;

    Application::new().assets(assets).run(move |cx| {
        cx.open_window(
            quickgui::WindowOptions::new("QuickGUI — bundled assets").size(620.0, 420.0),
            AssetsDemo { icon, message },
        );
    })?;
    Ok(())
}

struct AssetsDemo {
    icon: Svg,
    message: Arc<str>,
}

impl View for AssetsDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let reload = cx.listener("reload-message", |this, cx| {
            this.message = load_message(cx.asset_source()).expect("the bundled message is valid");
            cx.invalidate();
        });

        div()
            .size_full()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .p_6()
            .bg(Color::rgb8(15, 23, 42))
            .text_color(Color::rgb8(241, 245, 249))
            .child(
                div()
                    .size(96.0, 96.0)
                    .items_center()
                    .justify_center()
                    .rounded_xl()
                    .bg(Color::rgb8(30, 64, 175))
                    .text_color(Color::rgb8(191, 219, 254))
                    .child(svg(&self.icon).size(56.0, 56.0)),
            )
            .child(text("Application assets").text_2xl().font_bold())
            .child(
                text(self.message.clone())
                    .text_color(Color::rgb8(148, 163, 184))
                    .text_center(),
            )
            .child(
                button()
                    .id("reload-message")
                    .on_click(reload)
                    .px_4()
                    .py_2()
                    .rounded_lg()
                    .bg(Color::rgb8(37, 99, 235))
                    .hover(|style| style.bg(Color::rgb8(59, 130, 246)))
                    .child("Load from ViewContext"),
            )
    }
}

fn load_message(assets: &Assets) -> Result<Arc<str>, Box<dyn std::error::Error>> {
    let bytes = assets.load_required("copy/message.txt")?;
    Ok(Arc::from(std::str::from_utf8(bytes.as_ref())?))
}

const MESSAGE: &[u8] = b"Static bytes are shared; clean windows do no asset work.";

const BOLT_ICON: &[u8] = br##"
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
  <path fill="#000" d="M13.2 1.5 4.8 13h6.1l-.8 9.5L19.2 10h-6.1z"/>
</svg>
"##;
