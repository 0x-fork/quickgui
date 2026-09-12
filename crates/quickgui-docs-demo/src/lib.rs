//! Interactive documentation examples built with the production Rust components.
use quickgui::*;
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
#[path = "../../../examples/base_ui_components.rs"]
mod base_ui_components;
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
#[path = "../../../examples/data_collections.rs"]
mod data_collections;
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
#[path = "../../../examples/date_fields.rs"]
mod date_fields;
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
#[path = "../../../examples/dialogs.rs"]
mod dialogs;
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
#[path = "../../../examples/disclosures.rs"]
mod disclosures;
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
#[path = "../../../examples/menubar.rs"]
mod menubar;
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
#[path = "../../../examples/popovers.rs"]
mod popovers;
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
#[path = "../../../examples/range_controls.rs"]
mod range_controls;
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
#[path = "../../../examples/tabs.rs"]
mod tabs;
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
#[path = "../../../examples/toolbar_toast.rs"]
mod toolbar_toast;
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
#[path = "../../../examples/tooltips_context_menu.rs"]
mod tooltips_context_menu;

const INK: Color = Color::linear(0.01680738, 0.01938236, 0.02518686, 1.0);
const ACCENT: Color = Color::linear(0.07818742, 0.06124605, 0.78353779, 1.0);
const BORDER: Color = Color::linear(0.70110189, 0.71569350, 0.76052450, 1.0);
const MUTED: Color = Color::linear(0.14702727, 0.16202938, 0.20507874, 1.0);

pub const COMPONENTS: &[&str] = &[
    "view",
    "text",
    "button",
    "input",
    "text-area",
    "checkbox",
    "radio",
    "switch",
    "toggle",
    "slider",
    "progress",
    "meter",
    "collapsible",
    "accordion",
    "separator",
    "avatar",
    "checkbox-group",
    "preview-card",
    "scroll-area",
    "otp-field",
    "navigation-menu",
    "number-field",
    "splitter",
    "toolbar",
    "toggle-group",
    "toast",
    "date-field",
    "time-field",
    "calendar",
    "tabs",
    "dialog",
    "alert-dialog",
    "table",
    "tree",
    "tooltip",
    "menubar",
    "markdown",
    "image",
    "svg",
    "shader",
    "virtual-list",
    "field",
    "fieldset",
    "router",
    "radio-group",
    "popover",
    "menu",
];

struct Demo {
    component: String,
    checked: bool,
    count: usize,
    input: String,
    slider: SliderState,
    markdown: Markdown,
    image: Option<Image>,
    icon: Option<Svg>,
    shader: Option<CustomShader>,
    list: VirtualList,
    router: Router,
}

impl Demo {
    fn new(component: String) -> Self {
        let image = (component == "image").then(|| {
            let pixels: Vec<u8> = (0..128 * 128)
                .flat_map(|i| [70 + (i % 128) as u8, 65 + (i / 128) as u8, 200, 255])
                .collect();
            Image::from_rgba(128, 128, pixels).expect("bounded image")
        });
        let icon = (component == "svg").then(|| Svg::from_bytes(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="black" d="M12 2 15 8l7 1-5 5 1 8-6-4-6 4 1-8-5-5 7-1z"/></svg>"#).expect("static SVG"));
        let shader = (component == "shader").then(|| CustomShader::new("fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> { return vec4<f32>(input.uv.x, 0.3, input.uv.y, 1.0); }").expect("static shader"));
        Self {
            image,
            icon,
            shader,
            markdown: Markdown::with_text(
                "### A small note\n\nBuild **native interfaces** with composable components.\n\n- Retained layout\n- Selectable text\n- Familiar Markdown",
            ),
            list: VirtualList::new(1000, 34.0),
            router: Router::new(
                [
                    RouteDefinition::new("home", "/"),
                    RouteDefinition::new("settings", "/settings"),
                ],
                "/",
            )
            .expect("static routes"),
            component,
            checked: false,
            count: 0,
            input: String::new(),
            slider: SliderState::new(0.0, 100.0, 40.0).step(1.0),
        }
    }
    fn slider_state(view: &mut Self) -> &mut SliderState {
        &mut view.slider
    }
    fn action(label: impl Into<std::sync::Arc<str>>) -> Element {
        button()
            .px(16.0)
            .h(40.0)
            .rounded(8.0)
            .bg(ACCENT)
            .text_color(Color::WHITE)
            .flex_row()
            .items_center()
            .justify_center()
            .child(text(label))
            .hover(|s| s.bg(Color::rgb8(67, 56, 202)))
    }

    fn preview_markdown(&mut self, _cx: &mut ViewContext<'_, Self>) -> Element {
        self.markdown.element("note").w(290.0)
    }

    fn preview_image(&mut self, _cx: &mut ViewContext<'_, Self>) -> Element {
        img(self.image.as_ref().unwrap().clone())
            .size(160.0, 160.0)
            .rounded(12.0)
    }

    fn preview_svg(&mut self, _cx: &mut ViewContext<'_, Self>) -> Element {
        svg(self.icon.as_ref().unwrap().clone())
            .size(72.0, 72.0)
            .text_color(ACCENT)
    }

    fn preview_shader(&mut self, _cx: &mut ViewContext<'_, Self>) -> Element {
        custom_shader(self.shader.as_ref().unwrap().clone())
            .size(260.0, 150.0)
            .rounded(12.0)
    }

    fn preview_virtual_list(&mut self, _cx: &mut ViewContext<'_, Self>) -> Element {
        {
            self.list.set_viewport_height(210.0);
            let visible = self.list.visible_rows();
            div()
                .w(280.0)
                .h(210.0)
                .relative()
                .overflow_hidden()
                .virtual_scroll(&self.list)
                .children(visible.range.map(|i| {
                    div()
                        .id(ElementId::new(100 + i as u64))
                        .absolute()
                        .top(i as f32 * 34.0 - self.list.scroll_offset())
                        .h(34.0)
                        .w_full()
                        .px(12.0)
                        .flex_row()
                        .items_center()
                        .bg(if i % 2 == 0 {
                            Color::WHITE
                        } else {
                            Color::rgb8(244, 244, 248)
                        })
                        .child(text(format!("Item {:04}", i + 1)))
                }))
        }
    }

    fn preview_field(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        {
            let field = Field::new("email")
                .required(true)
                .invalid(!self.input.is_empty() && !self.input.contains('@'));
            let content = field
                .root()
                .w(270.0)
                .flex_col()
                .gap(10.0)
                .child(field.label_with(text("Email address").font_medium()))
                .child(
                    field.control_with(
                        text_input(self.input.clone())
                            .id("email")
                            .placeholder("you@example.com")
                            .p(10.0)
                            .rounded(7.0)
                            .border(1.0, BORDER)
                            .bg(Color::WHITE)
                            .on_input(cx.input_listener("email", |view, value, cx| {
                                view.input = value.to_owned();
                                cx.invalidate();
                            })),
                    ),
                )
                .child(
                    field.description_with(
                        text("We’ll only use this for account updates.")
                            .text_sm()
                            .wrap()
                            .text_color(MUTED),
                    ),
                );
            if self.component == "fieldset" {
                let group = Fieldset::new("account");
                group
                    .root_with(
                        div()
                            .p(20.0)
                            .rounded(8.0)
                            .border(1.0, BORDER)
                            .flex_col()
                            .gap(14.0),
                    )
                    .child(group.legend_with(text("Account details").font_semibold()))
                    .child(content)
            } else {
                content
            }
        }
    }

    fn preview_radio_group(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        radio_group()
            .flex_col()
            .gap(12.0)
            .accessibility_label("Theme")
            .children(
                ["Light", "Dark", "System"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, label)| {
                        radio(self.count == i)
                            .id(ElementId::new(200 + i as u64))
                            .on_click(cx.listener(
                                ElementId::new(200 + i as u64),
                                move |view, cx| {
                                    view.count = i;
                                    cx.invalidate();
                                },
                            ))
                            .flex_row()
                            .gap(10.0)
                            .items_center()
                            .child(
                                div()
                                    .size(20.0, 20.0)
                                    .rounded(10.0)
                                    .border(2.0, if self.count == i { ACCENT } else { BORDER })
                                    .bg(if self.count == i {
                                        ACCENT
                                    } else {
                                        Color::WHITE
                                    }),
                            )
                            .child(text(label))
                    }),
            )
    }

    fn preview_router(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        div()
            .flex_col()
            .gap(16.0)
            .items_center()
            .child(text(format!(
                "Current page: {}",
                self.router.location().pathname()
            )))
            .child(
                Self::action("Navigate").on_click(cx.listener("navigate", |view, cx| {
                    let next = if view.router.location().pathname() == "/" {
                        "/settings"
                    } else {
                        "/"
                    };
                    view.router.push(next).expect("static destination");
                    cx.invalidate();
                })),
            )
    }

    fn preview_view(&mut self, _cx: &mut ViewContext<'_, Self>) -> Element {
        div()
            .flex_row()
            .gap(12.0)
            .children(["Layout", "Compose", "Style"].into_iter().map(|label| {
                div()
                    .p(16.0)
                    .rounded(8.0)
                    .border(1.0, BORDER)
                    .bg(Color::WHITE)
                    .child(text(label))
            }))
    }

    fn preview_text(&mut self, _cx: &mut ViewContext<'_, Self>) -> Element {
        div()
            .flex_col()
            .gap(8.0)
            .child(text("Make yourself at home.").text_xl().font_semibold())
            .child(text("Native text shaping, selection, and layout.").text_color(MUTED))
    }

    fn preview_button(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        div()
            .flex_col()
            .items_center()
            .gap(16.0)
            .child(
                Self::action(if self.count == 0 {
                    "Save changes".to_owned()
                } else {
                    format!(
                        "Saved {} time{}",
                        self.count,
                        if self.count == 1 { "" } else { "s" }
                    )
                })
                .on_click(cx.listener("increment", |view, cx| {
                    view.count += 1;
                    cx.invalidate();
                })),
            )
            .child(
                text("Click or focus and press Enter.")
                    .text_sm()
                    .text_color(MUTED),
            )
    }

    fn preview_input(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        div()
            .flex_col()
            .gap(12.0)
            .w(260.0)
            .child(
                if self.component == "text-area" {
                    text_area(self.input.clone()).id("message")
                } else {
                    text_input(self.input.clone()).id("message")
                }
                .placeholder("Write something…")
                .w_full()
                .h(if self.component == "text-area" {
                    108.0
                } else {
                    42.0
                })
                .p(10.0)
                .rounded(8.0)
                .border(1.0, BORDER)
                .bg(Color::WHITE)
                .on_input(cx.input_listener("message", |view, value, cx| {
                    view.input = value.to_string();
                    cx.invalidate();
                })),
            )
            .child(
                text(format!("{} characters", self.input.chars().count()))
                    .text_sm()
                    .text_color(MUTED),
            )
    }

    fn preview_checkbox(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        let toggle_id = if self.component == "collapsible" {
            Collapsible::new("details", self.checked).trigger_id()
        } else {
            ElementId::from("choice")
        };
        let toggle = cx.listener(toggle_id, |view, cx| {
            view.checked = view.component == "radio" || !view.checked;
            cx.invalidate();
        });
        {
            let mark = if self.checked { "✓" } else { "" };
            let control = match self.component.as_str() {
                "checkbox" => checkbox(self.checked),
                "radio" => radio(self.checked),
                "switch" => switch(self.checked),
                _ => quickgui::toggle(self.checked),
            };
            div()
                .flex_row()
                .items_center()
                .gap(12.0)
                .child(
                    control
                        .id("choice")
                        .on_click(toggle)
                        .size(
                            if self.component == "switch" {
                                44.0
                            } else {
                                24.0
                            },
                            24.0,
                        )
                        .rounded(if self.component == "checkbox" {
                            5.0
                        } else {
                            12.0
                        })
                        .bg(if self.checked { ACCENT } else { Color::WHITE })
                        .border(1.0, if self.checked { ACCENT } else { BORDER })
                        .accessibility_label("Send me updates")
                        .text_color(Color::WHITE)
                        .flex_row()
                        .items_center()
                        .justify_center()
                        .child(if self.component == "switch" {
                            div()
                                .size(18.0, 18.0)
                                .rounded(9.0)
                                .bg(if self.checked { Color::WHITE } else { MUTED })
                                .relative()
                                .left(if self.checked { 8.0 } else { -8.0 })
                        } else {
                            text(mark)
                        }),
                )
                .child(text(if self.component == "toggle" {
                    "Pin this item"
                } else {
                    "Send me updates"
                }))
        }
    }

    fn preview_slider(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        {
            let size = Size::new(240.0, 24.0);
            let slider = Slider::new("volume", &self.slider);
            let drag = cx.pointer_listener(slider.track_id(), move |view, event, cx| {
                if view.slider.apply_pointer_change(event, size).changed {
                    cx.invalidate();
                }
            });
            let thumb = slider.thumb(0).unwrap();
            let body = slider
                .root()
                .flex_col()
                .gap(14.0)
                .child(text(format!("Volume  ·  {:.0}%", self.slider.value())).font_medium())
                .child(
                    slider
                        .track_with(
                            div()
                                .relative()
                                .w(size.width)
                                .h(size.height)
                                .on_pointer(drag),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(10.0)
                                .h(4.0)
                                .w(size.width)
                                .rounded(2.0)
                                .bg(BORDER),
                        )
                        .child(
                            slider.indicator_with(
                                div()
                                    .absolute()
                                    .top(10.0)
                                    .h(4.0)
                                    .w(self.slider.fraction(0) * size.width)
                                    .rounded(2.0)
                                    .bg(ACCENT),
                            ),
                        )
                        .child(
                            thumb.thumb_with(
                                div()
                                    .absolute()
                                    .top(3.0)
                                    .left(thumb.offset(size.width, 18.0))
                                    .size(18.0, 18.0)
                                    .rounded(9.0)
                                    .bg(Color::WHITE)
                                    .border(2.0, ACCENT),
                            ),
                        ),
                );
            slider.key_with(cx, body, Self::slider_state)
        }
    }

    fn preview_progress(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        {
            let value = 25.0 + (self.count % 4) as f64 * 25.0;
            let track = div()
                .w(240.0)
                .h(7.0)
                .rounded(4.0)
                .bg(BORDER)
                .child(div().w(value as f32 * 2.4).h(7.0).rounded(4.0).bg(ACCENT));
            let gauge = if self.component == "meter" {
                Meter::new(value, 0.0, 100.0).root_with(track)
            } else {
                Progress::new(value, 100.0).root_with(track)
            };
            div()
                .flex_col()
                .gap(16.0)
                .child(text(format!("Storage used  ·  {value:.0}%")))
                .child(gauge)
                .child(
                    Self::action("Increase").on_click(cx.listener("increment", |view, cx| {
                        view.count += 1;
                        cx.invalidate();
                    })),
                )
        }
    }

    fn preview_collapsible(&mut self, cx: &mut ViewContext<'_, Self>) -> Element {
        let toggle_id = if self.component == "collapsible" {
            Collapsible::new("details", self.checked).trigger_id()
        } else {
            ElementId::from("choice")
        };
        let toggle = cx.listener(toggle_id, |view, cx| {
            view.checked = view.component == "radio" || !view.checked;
            cx.invalidate();
        });
        {
            let disclosure = Collapsible::new("details", self.checked);
            let mut root = disclosure.root().w(270.0).flex_col().gap(12.0).child(
                disclosure.trigger_with(
                    Self::action(if self.checked {
                        "Hide details −"
                    } else {
                        "Show details +"
                    })
                    .on_click(toggle),
                ),
            );
            if let Some(panel) = disclosure.panel_with(
                div()
                    .p(16.0)
                    .rounded(8.0)
                    .bg(Color::WHITE)
                    .border(1.0, BORDER)
                    .child(text("Your settings are saved on this device.").wrap()),
            ) {
                root = root.child(panel);
            }
            root
        }
    }

    fn preview_separator(&mut self, _cx: &mut ViewContext<'_, Self>) -> Element {
        div()
            .w(250.0)
            .flex_col()
            .gap(18.0)
            .child(text("Workspace"))
            .child(
                Separator::new(SeparatorOrientation::Horizontal)
                    .root()
                    .w_full()
                    .h(1.0)
                    .bg(BORDER),
            )
            .child(text("Personal settings").text_color(MUTED))
    }
}

impl View for Demo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let content = match self.component.as_str() {
            "markdown" => self.preview_markdown(cx),
            "image" => self.preview_image(cx),
            "svg" => self.preview_svg(cx),
            "shader" => self.preview_shader(cx),
            "virtual-list" => self.preview_virtual_list(cx),
            "field" | "fieldset" => self.preview_field(cx),
            "radio-group" => self.preview_radio_group(cx),
            "router" => self.preview_router(cx),
            "view" => self.preview_view(cx),
            "text" => self.preview_text(cx),
            "button" => self.preview_button(cx),
            "input" | "text-area" => self.preview_input(cx),
            "checkbox" | "radio" | "switch" | "toggle" => self.preview_checkbox(cx),
            "slider" => self.preview_slider(cx),
            "progress" | "meter" => self.preview_progress(cx),
            "collapsible" => self.preview_collapsible(cx),
            "separator" => self.preview_separator(cx),
            _ => text("Unknown example"),
        };
        div()
            .size_full()
            .flex_col()
            .items_center()
            .justify_center()
            .p(24.0)
            .bg(Color::rgb8(250, 250, 252))
            .text_color(INK)
            .text_sm()
            .child(content)
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn start(
    canvas: web_sys::HtmlCanvasElement,
    component: String,
    width: f32,
    height: f32,
) -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    if !COMPONENTS.contains(&component.as_str()) {
        return Err(wasm_bindgen::JsValue::from_str("Unknown component demo"));
    }
    Application::new()
        .font(include_bytes!(
            "../../../tests/fixtures/fonts/Inter-Regular.ttf"
        ))
        .bind_keys(slider_key_bindings())
        .bind_keys(splitter_key_bindings())
        .bind_keys(date_field_key_bindings())
        .bind_keys(time_field_key_bindings())
        .bind_keys(calendar_key_bindings())
        .bind_keys(otp_field_key_bindings())
        .bind_keys(navigation_menu_key_bindings())
        .bind_keys(table_key_bindings())
        .bind_keys(tree_key_bindings())
        .bind_keys(toolbar_key_bindings())
        .bind_keys(toggle_group_key_bindings())
        .bind_keys(popover_menu_key_bindings())
        .bind_keys(menubar_key_bindings())
        .run_web(canvas, move |cx| {
            let options = WindowOptions::new("Component preview").size(width, height);
            match component.as_str() {
                "avatar" | "checkbox-group" | "preview-card" | "scroll-area" | "otp-field"
                | "navigation-menu" => {
                    cx.open_window(options, base_ui_components::docs_demo(component.clone()))
                }
                "number-field" | "splitter" => {
                    cx.open_window(options, range_controls::docs_demo(component.clone()))
                }
                "toolbar" | "toggle-group" | "toast" => {
                    cx.open_window(options, toolbar_toast::docs_demo(component.clone()))
                }
                "date-field" | "time-field" | "calendar" => {
                    cx.open_window(options, date_fields::docs_demo(component.clone()))
                }
                "tabs" => cx.open_window(options, tabs::docs_demo(component.clone())),
                "dialog" | "alert-dialog" => {
                    cx.open_window(options, dialogs::docs_demo(component.clone()))
                }
                "accordion" => cx.open_window(options, disclosures::docs_demo(component.clone())),
                "table" | "tree" => {
                    cx.open_window(options, data_collections::docs_demo(component.clone()))
                }
                "tooltip" => {
                    cx.open_window(options, tooltips_context_menu::docs_demo(component.clone()))
                }
                "popover" | "menu" => {
                    cx.open_window(options, popovers::docs_demo(component.clone()))
                }
                "menubar" => cx.open_window(options, menubar::docs_demo(component.clone())),
                _ => cx.open_window(options, Demo::new(component)),
            };
        })
        .await
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_examples_keep_listener_and_part_identities_consistent() {
        for component in ["checkbox", "radio", "switch", "toggle", "collapsible"] {
            let (mut cx, view) = TestAppContext::new(Demo::new(component.to_owned())).unwrap();
            let window = view.window_handle();
            let target = if component == "collapsible" {
                Collapsible::new("details", false).trigger_id()
            } else {
                ElementId::from("choice")
            };
            cx.click(window, target).unwrap();
            assert!(cx.read(view, |view| view.checked).unwrap(), "{component}");
            cx.click(window, target).unwrap();
            assert_eq!(
                cx.read(view, |view| view.checked).unwrap(),
                component == "radio",
                "{component}"
            );
        }
    }

    #[test]
    fn button_and_router_examples_update_retained_state() {
        let (mut cx, view) = TestAppContext::new(Demo::new("button".to_owned())).unwrap();
        cx.click(view.window_handle(), "increment").unwrap();
        assert_eq!(cx.read(view, |view| view.count).unwrap(), 1);
        let (mut cx, view) = TestAppContext::new(Demo::new("router".to_owned())).unwrap();
        cx.click(view.window_handle(), "navigate").unwrap();
        assert_eq!(
            cx.read(view, |view| view.router.location().pathname().to_owned())
                .unwrap(),
            "/settings"
        );
    }
}
