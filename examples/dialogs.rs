use std::sync::Arc;

use quickgui::{
    App, Color, Dialog, Element, EventContext, IntoElement, TitleBarStyle, View, ViewContext,
    WindowAppearance, button, div, text, text_input,
};

fn main() -> Result<(), quickgui::AppError> {
    App::new(DialogGallery::default())
        .title("QuickGUI — Unstyled dialogs")
        .size(880.0, 620.0)
        .title_bar_style(TitleBarStyle::HiddenInset)
        .run()
}

struct DialogGallery {
    dialog_open: bool,
    alert_open: bool,
    project_name: Arc<str>,
    status: Arc<str>,
}

impl Default for DialogGallery {
    fn default() -> Self {
        Self {
            dialog_open: false,
            alert_open: false,
            project_name: Arc::from("QuickGUI"),
            status: Arc::from("No dialog action yet"),
        }
    }
}

impl DialogGallery {
    fn close_dialog(&mut self, status: &'static str, cx: &mut EventContext) {
        self.dialog_open = false;
        self.alert_open = false;
        self.status = Arc::from(status);
        cx.invalidate();
    }
}

impl View for DialogGallery {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let light = cx.appearance() == WindowAppearance::Light;
        let colors = GalleryColors::new(light);
        let dialog = Dialog::new("project-dialog", self.dialog_open)
            .initial_focus("project-name")
            .restore_focus_to("open-project-dialog");
        let alert = Dialog::alert("delete-alert", self.alert_open)
            .initial_focus("cancel-delete")
            .restore_focus_to("delete-workspace");

        let open_dialog = cx.listener("open-project-dialog", move |view, cx| {
            view.dialog_open = true;
            dialog.focus_initial(cx);
            cx.invalidate();
        });
        let dismiss_dialog = cx.dismiss_listener(dialog.popover_id(), |view, cx| {
            view.dialog_open = false;
            view.alert_open = false;
            view.status = Arc::from("Dialog dismissed");
            cx.invalidate();
        });
        let cancel_dialog = cx.listener(dialog.close_id(), move |view, cx| {
            view.close_dialog("Changes cancelled", cx);
            dialog.focus_restore(cx);
        });
        let save_dialog = cx.listener("save-project", move |view, cx| {
            view.close_dialog("Project saved", cx);
            dialog.focus_restore(cx);
        });
        let edit_name = cx.input_listener("project-name", |view, value, cx| {
            view.project_name = Arc::from(value);
            cx.invalidate();
        });
        let open_alert = cx.listener("delete-workspace", move |view, cx| {
            view.alert_open = true;
            alert.focus_initial(cx);
            cx.invalidate();
        });
        let dismiss_alert = cx.dismiss_listener(alert.popover_id(), |view, cx| {
            view.alert_open = false;
            view.status = Arc::from("Delete cancelled with Escape");
            cx.invalidate();
        });
        let cancel_alert = cx.listener("cancel-delete", move |view, cx| {
            view.alert_open = false;
            view.status = Arc::from("Delete cancelled");
            alert.focus_restore(cx);
            cx.invalidate();
        });
        let confirm_alert = cx.listener("confirm-delete", move |view, cx| {
            view.close_dialog("Workspace deleted", cx);
            dialog.focus_restore(cx);
        });

        let mut root = div()
            .relative()
            .size_full()
            .bg(colors.background)
            .text_color(colors.text)
            .flex_col()
            .child(
                div()
                    .h(52.0)
                    .px(20.0)
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .app_region_drag()
                    .border(1.0, colors.border)
                    .child(text("QuickGUI dialogs").font_semibold())
                    .child(text(self.status.clone()).text_xs().text_color(colors.muted)),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .p(32.0)
                    .app_region_no_drag()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_5()
                    .child(
                        div()
                            .w(520.0)
                            .rounded_xl()
                            .border(1.0, colors.border)
                            .bg(colors.surface)
                            .p_6()
                            .flex_col()
                            .gap_4()
                            .child(text("Unstyled modal composition").text_2xl().font_bold())
                            .child(
                                text("The application owns every visible pixel. QuickGUI supplies the portal, focus trap, nested dismissal, focus restoration, accessibility relationships, and native-view occlusion.")
                                    .wrap()
                                    .text_sm()
                                    .text_color(colors.muted),
                            )
                            .child(
                                dialog
                                    .trigger_part(
                                        "open-project-dialog",
                                        gallery_button("Edit project", true, colors),
                                    )
                                    .on_click(open_dialog),
                            ),
                    ),
            );

        if self.dialog_open {
            let popover = dialog
                .popover_part(
                    div()
                        .relative()
                        .w(460.0)
                        .rounded_xl()
                        .border(1.0, colors.border)
                        .bg(colors.surface)
                        .shadow_2xl()
                        .p_6()
                        .flex_col()
                        .gap_4()
                        .child(
                            dialog.title_part(text("Edit project").text_xl().font_bold()),
                        )
                        .child(dialog.description_part(
                            text("Rename the project or open the nested destructive confirmation.")
                                .wrap()
                                .text_sm()
                                .text_color(colors.muted),
                        ))
                        .child(
                            div()
                                .flex_col()
                                .gap_2()
                                .child(text("Project name").text_sm().font_medium())
                                .child(
                                    text_input(self.project_name.clone())
                                        .id("project-name")
                                        .on_input(edit_name)
                                        .h(38.0)
                                        .w_full()
                                        .px_3()
                                        .rounded_lg()
                                        .border(1.0, colors.border)
                                        .bg(colors.input)
                                        .focus(|focused| focused.border(2.0, colors.accent)),
                                ),
                        )
                        .child(
                            div()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .gap_3()
                                .child(
                                    gallery_button("Delete workspace…", false, colors)
                                        .id("delete-workspace")
                                        .text_color(colors.danger)
                                        .on_click(open_alert),
                                )
                                .child(
                                    div()
                                        .flex_row()
                                        .gap_2()
                                        .child(
                                            dialog
                                                .close_part(
                                                    "Cancel project changes",
                                                    gallery_button("Cancel", false, colors),
                                                )
                                                .on_click(cancel_dialog),
                                        )
                                        .child(
                                            gallery_button("Save", true, colors)
                                                .id("save-project")
                                                .on_click(save_dialog),
                                        ),
                                ),
                        ),
                )
                .on_dismiss(dismiss_dialog);
            root = root.child(
                dialog
                    .root_part(div().p_6().flex_row().items_center().justify_center())
                    .child(dialog.backdrop_part(div().bg(colors.backdrop)))
                    .child(popover),
            );
        }

        if self.alert_open {
            let popover = alert
                .popover_part(
                    div()
                        .relative()
                        .w(400.0)
                        .rounded_xl()
                        .border(1.0, colors.border)
                        .bg(colors.surface)
                        .shadow_2xl()
                        .p_6()
                        .flex_col()
                        .gap_4()
                        .child(alert.title_part(text("Delete workspace?").text_xl().font_bold()))
                        .child(alert.description_part(
                            text("This action cannot be undone. A backdrop press is intentionally blocked; use Cancel, Delete, or Escape.")
                                .wrap()
                                .text_sm()
                                .text_color(colors.muted),
                        ))
                        .child(
                            div()
                                .flex_row()
                                .justify_end()
                                .gap_2()
                                .child(
                                    gallery_button("Cancel", false, colors)
                                        .id("cancel-delete")
                                        .on_click(cancel_alert),
                                )
                                .child(
                                    gallery_button("Delete", false, colors)
                                        .id("confirm-delete")
                                        .bg(colors.danger)
                                        .text_color(Color::WHITE)
                                        .on_click(confirm_alert),
                                ),
                        ),
                )
                .on_dismiss(dismiss_alert);
            root = root.child(
                alert
                    .root_part(div().p_6().flex_row().items_center().justify_center())
                    .z_index(1)
                    .child(alert.backdrop_part(div().bg(colors.backdrop_strong)))
                    .child(popover),
            );
        }

        root
    }
}

fn gallery_button(label: &'static str, primary: bool, colors: GalleryColors) -> Element {
    button()
        .h(36.0)
        .px_4()
        .rounded_lg()
        .border(
            1.0,
            if primary {
                colors.accent
            } else {
                colors.border
            },
        )
        .bg(if primary { colors.accent } else { colors.input })
        .text_color(if primary { Color::WHITE } else { colors.text })
        .hover(|hover| hover.opacity(0.86))
        .child(text(label).text_sm().font_medium())
}

#[derive(Clone, Copy)]
struct GalleryColors {
    background: Color,
    surface: Color,
    input: Color,
    border: Color,
    text: Color,
    muted: Color,
    accent: Color,
    danger: Color,
    backdrop: Color,
    backdrop_strong: Color,
}

impl GalleryColors {
    fn new(light: bool) -> Self {
        if light {
            Self {
                background: Color::rgb8(242, 244, 248),
                surface: Color::WHITE,
                input: Color::rgb8(248, 249, 252),
                border: Color::rgb8(205, 210, 220),
                text: Color::rgb8(28, 32, 40),
                muted: Color::rgb8(94, 101, 116),
                accent: Color::rgb8(0, 112, 224),
                danger: Color::rgb8(204, 45, 63),
                backdrop: Color::rgba8(16, 20, 28, 92),
                backdrop_strong: Color::rgba8(16, 20, 28, 132),
            }
        } else {
            Self {
                background: Color::rgb8(16, 18, 23),
                surface: Color::rgb8(27, 30, 37),
                input: Color::rgb8(20, 23, 29),
                border: Color::rgb8(67, 74, 88),
                text: Color::rgb8(236, 239, 245),
                muted: Color::rgb8(151, 159, 176),
                accent: Color::rgb8(25, 126, 230),
                danger: Color::rgb8(220, 65, 83),
                backdrop: Color::rgba8(0, 0, 0, 130),
                backdrop_strong: Color::rgba8(0, 0, 0, 170),
            }
        }
    }
}
