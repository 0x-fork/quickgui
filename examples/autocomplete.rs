use std::sync::Arc;

use quickgui::{
    Application, AutocompleteOptionState, AutocompletePopoverLayout, AutocompleteSelectionBehavior,
    AutocompleteState, Color, HighlightStyle, IntoElement, PickerItem, StyledText, TitleBarStyle,
    View, ViewContext, combobox_key_bindings, div, text, text_input,
};

const SYMBOLS: usize = 20_000;

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .bind_keys(combobox_key_bindings())
        .run(|cx| {
            cx.open_window(
                quickgui::WindowOptions::new("QuickGUI — Free-form autocomplete")
                    .size(900.0, 620.0)
                    .title_bar_style(TitleBarStyle::HiddenInset)
                    .traffic_light_position(16.0, 13.0),
                AutocompleteGallery::new(),
            );
        })
}

struct AutocompleteGallery {
    symbols: AutocompleteState<usize>,
    query: Arc<str>,
    status: Arc<str>,
}

impl AutocompleteGallery {
    fn new() -> Self {
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
        });
        let mut symbols = AutocompleteState::new(items)
            .expect("autocomplete source is bounded")
            .with_layout(AutocompletePopoverLayout::new(460.0, 44.0).max_visible_rows(7));
        symbols.set_selection_behavior(AutocompleteSelectionBehavior::CompleteInput);
        Self {
            symbols,
            query: Arc::from(""),
            status: Arc::from(
                "Type anything. Suggestions are optional; Escape preserves text and Return commits only an active row.",
            ),
        }
    }

    fn symbols(view: &mut Self) -> &mut AutocompleteState<usize> {
        &mut view.symbols
    }
}

impl View for AutocompleteGallery {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let dark = cx.appearance().is_dark();
        let page = if dark {
            Color::rgb8(13, 15, 19)
        } else {
            Color::rgb8(240, 242, 246)
        };
        let surface = if dark {
            Color::rgb8(22, 25, 31)
        } else {
            Color::WHITE
        };
        let border = if dark {
            Color::rgb8(57, 63, 74)
        } else {
            Color::rgb8(208, 213, 222)
        };
        let foreground = if dark {
            Color::rgb8(237, 240, 246)
        } else {
            Color::rgb8(29, 33, 40)
        };
        let muted = if dark {
            Color::rgb8(151, 159, 175)
        } else {
            Color::rgb8(91, 98, 112)
        };
        let active = if dark {
            Color::rgb8(43, 68, 104)
        } else {
            Color::rgb8(220, 235, 252)
        };
        let matched = if dark {
            Color::rgb8(126, 231, 212)
        } else {
            Color::rgb8(0, 102, 204)
        };

        let input = text_input(self.symbols.value().clone())
            .w(460.0)
            .h(42.0)
            .px_3()
            .rounded_lg()
            .border(1.0, border)
            .bg(surface)
            .text_color(foreground)
            .placeholder("Search symbols or enter a custom path…")
            .focus(move |style| style.border(2.0, matched));
        let autocomplete = self.symbols.element(
            cx,
            "symbol-autocomplete",
            "Workspace symbol or path",
            Self::symbols,
            input,
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
                                text("No suggestions — the typed value is still valid")
                                    .text_sm()
                                    .text_color(muted),
                            ),
                    );
                }
                root
            },
            move |item, state: AutocompleteOptionState| {
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
                    .opacity(if state.disabled { 0.45 } else { 1.0 })
                    .bg(if state.active {
                        active
                    } else {
                        Color::TRANSPARENT
                    })
                    .child(content)
            },
            |view, value, cx| {
                view.query = value.clone();
                view.status = Arc::from(format!("Free-form value: {value}"));
                cx.invalidate();
            },
            |view, value, cx| {
                view.status = Arc::from(format!("Committed suggestion: Symbol {value:05}"));
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
                    .px(20.0)
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .app_region_drag()
                    .child(text("Free-form autocomplete").font_semibold())
                    .child(
                        text("unstyled parts · never-key overflow panel · owner IME")
                            .text_xs()
                            .text_color(muted),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .p_6()
                    .app_region_no_drag()
                    .flex_col()
                    .gap_5()
                    .child(
                        div()
                            .rounded_xl()
                            .border(1.0, border)
                            .bg(surface)
                            .p_5()
                            .flex_col()
                            .gap_3()
                            .child(text("20,000 local suggestions").font_semibold())
                            .child(
                                text("The application styles the input, popover, and rows. QuickGUI owns free-form value behavior, bounded fuzzy results, native placement beyond the window edge, keyboard and pointer interaction, and the owner-tree accessibility proxy.")
                                    .wrap()
                                    .text_sm()
                                    .text_color(muted),
                            )
                            .child(autocomplete)
                            .child(
                                text(format!(
                                    "query: {}",
                                    if self.query.is_empty() {
                                        "<empty>"
                                    } else {
                                        self.query.as_ref()
                                    }
                                ))
                                .text_xs()
                                .text_color(muted),
                            ),
                    )
                    .child(
                        text("Only the viewport plus bounded overscan is mounted. The child observes one shared snapshot and both windows sleep when settled.")
                            .wrap()
                            .text_sm()
                            .text_color(muted),
                    ),
            )
            .child(
                div()
                    .h(30.0)
                    .flex_none()
                    .px_4()
                    .flex_row()
                    .items_center()
                    .justify_between()
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
