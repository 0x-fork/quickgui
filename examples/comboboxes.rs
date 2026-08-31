use std::sync::Arc;

use quickgui::{
    Application, Color, ComboboxOptionState, ComboboxPopoverLayout, ComboboxState, HighlightStyle,
    IntoElement, PickerItem, SelectPopoverLayout, SelectState, StyledText, TitleBarStyle, View,
    ViewContext, combobox_key_bindings, div, select_key_bindings, text, text_input,
};

const SYMBOLS: usize = 20_000;

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .bind_keys(select_key_bindings())
        .bind_keys(combobox_key_bindings())
        .run(|cx| {
            cx.open_window(
                quickgui::WindowOptions::new("QuickGUI — Select and constrained combobox")
                    .size(920.0, 660.0)
                    .title_bar_style(TitleBarStyle::HiddenInset)
                    .traffic_light_position(16.0, 13.0),
                ComboboxGallery::new(),
            );
        })
}

struct ComboboxGallery {
    theme: SelectState<&'static str>,
    symbol: ComboboxState<usize>,
    query: Arc<str>,
    status: Arc<str>,
}

impl ComboboxGallery {
    fn new() -> Self {
        let mut theme = SelectState::new([
            PickerItem::new("Follow system", "system").id("system"),
            PickerItem::new("Light", "light").id("light"),
            PickerItem::new("Dark", "dark").id("dark"),
            PickerItem::new("High contrast", "contrast")
                .id("contrast")
                .disabled(true),
        ])
        .expect("theme options are valid")
        .with_layout(SelectPopoverLayout::new(320.0, 40.0).max_visible_rows(8));
        theme.select_id("system");

        let items = (0..SYMBOLS).map(|index| {
            PickerItem::new(format!("Symbol {index:05}"), index)
                .id(index as u64 + 1)
                .detail(if index % 3 == 0 {
                    "Function · src/runtime.rs"
                } else if index % 3 == 1 {
                    "Struct · src/element.rs"
                } else {
                    "Method · src/ui_tree.rs"
                })
                .keywords(format!("definition workspace item-{index}"))
                .shortcut(if index < 10 {
                    format!("⌘{index}")
                } else {
                    String::new()
                })
        });
        let symbol = ComboboxState::new(items)
            .expect("symbol options are valid")
            .with_layout(ComboboxPopoverLayout::new(420.0, 42.0).max_visible_rows(7))
            .with_selected_source(42);

        Self {
            theme,
            symbol,
            query: Arc::from(""),
            status: Arc::from(
                "Tab once per control. Arrow navigation previews; Return commits; Escape cancels.",
            ),
        }
    }

    fn theme(view: &mut Self) -> &mut SelectState<&'static str> {
        &mut view.theme
    }

    fn symbol(view: &mut Self) -> &mut ComboboxState<usize> {
        &mut view.symbol
    }
}

impl View for ComboboxGallery {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let dark = cx.appearance().is_dark();

        let page = if dark {
            Color::rgb8(13, 15, 19)
        } else {
            Color::rgb8(240, 242, 246)
        };
        let surface = if dark {
            Color::rgb8(21, 24, 29)
        } else {
            Color::WHITE
        };
        let border = if dark {
            Color::rgb8(53, 58, 68)
        } else {
            Color::rgb8(211, 215, 223)
        };
        let foreground = if dark {
            Color::rgb8(235, 238, 244)
        } else {
            Color::rgb8(29, 33, 40)
        };
        let muted = if dark {
            Color::rgb8(150, 158, 174)
        } else {
            Color::rgb8(91, 98, 112)
        };
        let active = if dark {
            Color::rgb8(47, 68, 102)
        } else {
            Color::rgb8(220, 235, 252)
        };
        let matched = if dark {
            Color::rgb8(126, 231, 212)
        } else {
            Color::rgb8(0, 102, 204)
        };

        let theme_label = self
            .theme
            .selected_item()
            .map(|item| item.label().clone())
            .unwrap_or_else(|| Arc::from("Choose a theme…"));
        let theme_open = self.theme.is_open();
        let theme = self.theme.element(
            cx,
            "theme-select",
            "Editor theme",
            Self::theme,
            div()
                .w(320.0)
                .h(38.0)
                .px_3()
                .rounded_lg()
                .border(1.0, border)
                .bg(surface)
                .flex_row()
                .items_center()
                .justify_between()
                .child(text(theme_label).text_sm().no_wrap().text_ellipsis())
                .child(
                    text(if theme_open { "⌃" } else { "⌄" })
                        .text_xs()
                        .text_color(muted),
                ),
            move |state| {
                let root = div()
                    .rounded_lg()
                    .border(1.0, border)
                    .bg(surface)
                    .text_color(foreground);
                if state.option_count == 0 {
                    root.child(
                        div()
                            .size_full()
                            .px_3()
                            .flex_row()
                            .items_center()
                            .child(text("No options").text_sm().text_color(muted)),
                    )
                } else {
                    root
                }
            },
            move |item, state| {
                div()
                    .px_3()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .opacity(if state.disabled { 0.45 } else { 1.0 })
                    .bg(if state.active {
                        if dark {
                            Color::rgb8(47, 68, 102)
                        } else {
                            Color::rgb8(220, 235, 252)
                        }
                    } else {
                        Color::TRANSPARENT
                    })
                    .child(
                        text(item.label().clone())
                            .text_sm()
                            .no_wrap()
                            .text_ellipsis(),
                    )
                    .child(
                        text(if state.selected { "✓" } else { "" })
                            .text_sm()
                            .text_color(if dark {
                                Color::rgb8(126, 231, 212)
                            } else {
                                Color::rgb8(0, 102, 204)
                            }),
                    )
            },
            |view, value, cx| {
                view.status = Arc::from(format!("Theme committed: {value}"));
                cx.invalidate();
            },
        );
        let symbol_input = text_input(self.symbol.input_value().clone())
            .w(420.0)
            .h(38.0)
            .px_3()
            .rounded_lg()
            .border(1.0, border)
            .bg(surface)
            .text_color(foreground)
            .placeholder("Search 20,000 symbols…")
            .focus(move |style| style.border(2.0, matched));
        let symbol = self.symbol.element(
            cx,
            "symbol-combobox",
            "Workspace symbol",
            Self::symbol,
            symbol_input,
            move |state| {
                let mut root = div()
                    .relative()
                    .rounded_lg()
                    .border(1.0, border)
                    .shadow_xl()
                    .bg(surface)
                    .text_color(foreground);
                if state.result_count == 0 {
                    root = root.child(
                        div()
                            .absolute()
                            .size_full()
                            .px_3()
                            .flex_row()
                            .items_center()
                            .child(
                                text("No declared option matches")
                                    .text_sm()
                                    .text_color(muted),
                            ),
                    );
                }
                root
            },
            move |item, state: ComboboxOptionState| {
                let label = if state.label_ranges.is_empty() {
                    text(item.label().clone())
                } else {
                    StyledText::new(item.label().clone())
                        .with_highlights(state.label_ranges.iter().cloned().map(|range| {
                            (
                                range,
                                HighlightStyle::default().color(matched).font_semibold(),
                            )
                        }))
                        .into_element()
                }
                .text_sm()
                .no_wrap()
                .text_ellipsis();
                let mut content = div().min_w(0.0).flex_1().flex_col().child(label);
                if let Some(detail) = item.detail_text() {
                    content = content.child(
                        text(detail.clone())
                            .text_xs()
                            .no_wrap()
                            .text_ellipsis()
                            .text_color(muted),
                    );
                }
                div()
                    .px_3()
                    .min_w(0.0)
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .opacity(if state.disabled { 0.45 } else { 1.0 })
                    .bg(if state.active {
                        active
                    } else {
                        Color::TRANSPARENT
                    })
                    .child(content)
                    .child(
                        text(if state.selected { "✓" } else { "" })
                            .text_sm()
                            .text_color(matched),
                    )
            },
            |view, query, cx| {
                view.query = query;
                cx.invalidate();
            },
            |view, value, cx| {
                view.status = Arc::from(format!("Opened Symbol {value:05}"));
                cx.invalidate();
            },
        );

        let metrics = cx.metrics();
        div()
            .size_full()
            .flex_col()
            .bg(page)
            .text_color(foreground)
            .child(
                div()
                    .h(52.0)
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px(20.0)
                    .app_region_drag()
                    .child(text("Select and constrained combobox").font_semibold())
                    .child(
                        text("Web interaction · native accessibility · retained WGPU")
                            .text_xs()
                            .text_color(muted),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .p_5()
                    .app_region_no_drag()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .rounded_xl()
                            .border(1.0, border)
                            .bg(surface)
                            .p_5()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .flex_col()
                                    .gap_1()
                                    .child(text("Standalone unstyled select").font_semibold())
                                    .child(
                                        text("The caller owns every pixel. QuickGUI supplies a separate native popover surface, preview/commit/cancel behavior, typeahead, virtualization, and exact close synchronization. Disabled choices remain in source order and cannot be committed.")
                                            .wrap()
                                            .text_sm()
                                            .text_color(muted),
                                    ),
                            )
                            .child(theme),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h(0.0)
                            .rounded_xl()
                            .border(1.0, border)
                            .bg(surface)
                            .p_5()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .flex_row()
                                    .items_end()
                                    .justify_between()
                                    .gap_4()
                                    .child(
                                        div()
                                            .min_w(0.0)
                                            .flex_1()
                                            .flex_col()
                                            .gap_1()
                                            .child(text("Unstyled constrained combobox").font_semibold())
                                            .child(
                                                text("The caller owns the input, popover, and rows. Typing filters a bounded 20,000-item source; only a declared enabled option can become the value. Escape, Tab, or an outside press restores the last committed label.")
                                                    .wrap()
                                                    .text_sm()
                                                    .text_color(muted),
                                            ),
                                    )
                                    .child(
                                        text(format!(
                                            "query: {}",
                                            if self.query.is_empty() {
                                                "<none>"
                                            } else {
                                                &self.query
                                            }
                                        ))
                                        .text_xs()
                                        .no_wrap()
                                        .text_color(muted),
                                    ),
                            )
                            .child(symbol)
                            .child(
                                text("Only the viewport plus one overscan row is mounted; closing the popover retains no overlay and schedules no frame.")
                                    .wrap()
                                    .text_xs()
                                    .text_color(muted),
                            ),
                    ),
            )
            .child(
                div()
                    .h(30.0)
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .text_color(muted)
                    .child(text(self.status.clone()).text_xs().no_wrap().text_ellipsis())
                    .child(
                        text(format!(
                            "{:.2} ms CPU · {} draw calls · {} shaped",
                            metrics.cpu_milliseconds(),
                            metrics.render.draw_calls,
                            metrics.render.reshaped_text_areas,
                        ))
                        .text_xs()
                        .no_wrap(),
                    ),
            )
    }
}
