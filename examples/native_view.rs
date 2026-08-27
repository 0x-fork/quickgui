#[cfg(target_os = "macos")]
mod app {
    use std::sync::Arc;

    use objc2::rc::Retained;
    use objc2_app_kit::NSTextField;
    use objc2_foundation::{MainThreadMarker, NSRect, NSString};
    use quickgui::{
        AccessibilityRole, AnchorPlacement, App, Color, KeyBinding, Menu, MenuItem, OsAction, View,
        ViewContext, button, div, native_view, overlay, text, text_input,
    };

    quickgui::actions!(edit, [Cut, Copy, Paste, SelectAll, Undo, Redo]);

    fn edit_menu() -> Menu {
        Menu::new("Edit").items([
            MenuItem::os_action("Undo", Undo, OsAction::Undo),
            MenuItem::os_action("Redo", Redo, OsAction::Redo),
            MenuItem::separator(),
            MenuItem::os_action("Cut", Cut, OsAction::Cut),
            MenuItem::os_action("Copy", Copy, OsAction::Copy),
            MenuItem::os_action("Paste", Paste, OsAction::Paste),
            MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
        ])
    }

    pub fn run() -> Result<(), quickgui::AppError> {
        let mtm =
            MainThreadMarker::new().expect("the QuickGUI runtime starts on AppKit's main thread");
        let field = unsafe { NSTextField::initWithFrame(mtm.alloc(), NSRect::ZERO) };
        unsafe {
            field.setPlaceholderString(Some(&NSString::from_str(
                "This is a real AppKit NSTextField — type here",
            )));
            field.setStringValue(&NSString::from_str("Native child view"));
        }
        App::new(NativeViewDemo {
            field,
            gpu_value: Arc::from("GPU child input"),
            native_visible: true,
            overlay_open: false,
        })
        .title("QuickGUI — Native NSView composition")
        .size(760.0, 500.0)
        .bind_keys([
            KeyBinding::new("platform-z", Undo, None),
            KeyBinding::new("platform-shift-z", Redo, None),
            KeyBinding::new("platform-x", Cut, None),
            KeyBinding::new("platform-c", Copy, None),
            KeyBinding::new("platform-v", Paste, None),
            KeyBinding::new("platform-a", SelectAll, None),
        ])
        .menu(edit_menu())
        .run()
    }

    struct NativeViewDemo {
        field: Retained<NSTextField>,
        gpu_value: Arc<str>,
        native_visible: bool,
        overlay_open: bool,
    }

    impl View for NativeViewDemo {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
            let trigger = cx.focus_handle("native-overlay-trigger");
            let toggle = cx.listener(trigger.id(), |this, cx| {
                this.overlay_open = !this.overlay_open;
                cx.invalidate();
            });
            let native_trigger = cx.focus_handle("native-visibility-trigger");
            let toggle_native = cx.listener(native_trigger.id(), |this, cx| {
                this.native_visible = !this.native_visible;
                cx.invalidate();
            });
            let dismiss = cx.dismiss_listener("native-overlay", |this, cx| {
                this.overlay_open = false;
                cx.invalidate();
            });
            let edit_gpu = cx.input_listener("gpu-text-field", |this, value, cx| {
                this.gpu_value = Arc::from(value);
                cx.invalidate();
            });

            let root = div()
                .relative()
                .size_full()
                .flex_col()
                .gap_4()
                .p_4()
                .bg(Color::rgb8(18, 19, 23))
                .text_color(Color::rgb8(234, 236, 241))
                .child(
                    div()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .gap_4()
                        .child(
                            div()
                                .flex_col()
                                .gap_1()
                                .child(
                                    text("Native NSView composition")
                                        .accessibility_role(AccessibilityRole::Heading)
                                        .text_2xl()
                                        .font_bold(),
                                )
                                .child(
                                    text("AppKit renders and handles input in the middle plane; QuickGUI keeps GPU overlays above it.")
                                        .text_sm()
                                        .text_color(Color::rgb8(159, 166, 180)),
                                ),
                        )
                        .child(
                            div()
                                .flex_row()
                                .gap_2()
                                .child(
                                    button()
                                        .on_click(toggle_native)
                                        .track_focus(native_trigger)
                                        .min_h(40.0)
                                        .flex_row()
                                        .items_center()
                                        .px_4()
                                        .py_2()
                                        .rounded_lg()
                                        .border(1.0, Color::rgb8(76, 82, 96))
                                        .bg(Color::rgb8(38, 42, 51))
                                        .hover(|style| style.bg(Color::rgb8(51, 57, 69)))
                                        .focus(|style| {
                                            style.border(2.0, Color::rgb8(94, 234, 212))
                                        })
                                        .child(if self.native_visible {
                                            "Hide native view"
                                        } else {
                                            "Show native view"
                                        }),
                                )
                                .child(
                                    button()
                                        .on_click(toggle)
                                        .track_focus(trigger)
                                        .min_h(40.0)
                                        .flex_row()
                                        .items_center()
                                        .px_4()
                                        .py_2()
                                        .rounded_lg()
                                        .border(1.0, Color::rgb8(76, 82, 96))
                                        .bg(Color::rgb8(38, 42, 51))
                                        .hover(|style| style.bg(Color::rgb8(51, 57, 69)))
                                        .focus(|style| {
                                            style.border(2.0, Color::rgb8(94, 234, 212))
                                        })
                                        .child(if self.overlay_open {
                                            "Close GPU overlay"
                                        } else {
                                            "Open GPU overlay"
                                        }),
                                ),
                        ),
                );

            let root = if self.native_visible {
                root.child(
                    text("Native AppKit input")
                        .text_xs()
                        .text_color(Color::rgb8(159, 166, 180)),
                )
                .child(
                    native_view(self.field.as_ref())
                        .id("native-text-field")
                        .w_full()
                        .h(44.0)
                        .rounded_lg()
                        .opacity(0.96)
                        .hover(|style| style.opacity(1.0))
                        .transition(std::time::Duration::from_millis(120)),
                )
            } else {
                root.child(
                    div()
                        .w_full()
                        .h(44.0)
                        .flex_row()
                        .items_center()
                        .px_4()
                        .rounded_lg()
                        .border(1.0, Color::rgb8(76, 82, 96))
                        .child(
                            text("Native NSView detached; showing it remounts the same retained identity.")
                                .text_sm()
                                .text_color(Color::rgb8(159, 166, 180)),
                        ),
                )
            };

            let root = root
                .child(
                    text("GPU-rendered controlled input")
                        .text_xs()
                        .text_color(Color::rgb8(159, 166, 180)),
                )
                .child(
                    text_input(self.gpu_value.clone())
                        .on_input(edit_gpu)
                        .placeholder("The same Edit menu works here")
                        .accessibility_label("GPU controlled input")
                        .w_full()
                        .h(44.0),
                );

            let root = root.child(
                div()
                    .flex_1()
                    .p_4()
                    .rounded_xl()
                    .border(1.0, Color::rgb8(55, 60, 71))
                    .bg(Color::rgb8(25, 27, 33))
                    .child(
                        text("The native field is retained by identity, clipped by QuickGUI layout, resized in logical points, and removed automatically when its element disappears. Native Edit commands follow AppKit's responder chain; the same menu falls back to bounded QuickGUI text history for the GPU field.")
                            .text_lg(),
                    ),
            );

            if self.overlay_open {
                root.child(
                    overlay()
                        .anchor_to(trigger.id(), AnchorPlacement::BottomEnd)
                        .on_dismiss(dismiss)
                        .restore_focus_to(trigger)
                        .accessibility_role(AccessibilityRole::Dialog)
                        .accessibility_label("GPU overlay above native view")
                        .w(300.0)
                        .flex_col()
                        .gap_2()
                        .p_4()
                        .rounded_xl()
                        .border(1.0, Color::rgb8(94, 234, 212))
                        .bg(Color::rgb8(29, 32, 39))
                        .child(text("GPU overlay above NSView").text_lg().font_semibold())
                        .child(
                            text("This surface is rendered after the native AppKit plane and captures dismissal input while visible.")
                                .text_sm()
                                .text_color(Color::rgb8(180, 187, 199)),
                        ),
                )
            } else {
                root
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), quickgui::AppError> {
    app::run()
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("The native_view example currently targets macOS.");
}
