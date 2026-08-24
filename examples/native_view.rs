#[cfg(target_os = "macos")]
mod app {
    use objc2::rc::Retained;
    use objc2_app_kit::NSTextField;
    use objc2_foundation::{MainThreadMarker, NSRect, NSString};
    use quickgui::{
        AccessibilityRole, AnchorPlacement, App, Color, View, ViewContext, button, div,
        native_view, overlay, text,
    };

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
            overlay_open: false,
        })
        .title("QuickGUI — Native NSView composition")
        .size(760.0, 500.0)
        .run()
    }

    struct NativeViewDemo {
        field: Retained<NSTextField>,
        overlay_open: bool,
    }

    impl View for NativeViewDemo {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
            let trigger = cx.focus_handle("native-overlay-trigger");
            let toggle = cx.listener(trigger.id(), |this, cx| {
                this.overlay_open = !this.overlay_open;
                cx.invalidate();
            });
            let dismiss = cx.dismiss_listener("native-overlay", |this, cx| {
                this.overlay_open = false;
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
                )
                .child(
                    native_view(self.field.as_ref())
                        .id("native-text-field")
                        .w_full()
                        .h(44.0)
                        .rounded_lg(),
                )
                .child(
                    div()
                        .flex_1()
                        .p_4()
                        .rounded_xl()
                        .border(1.0, Color::rgb8(55, 60, 71))
                        .bg(Color::rgb8(25, 27, 33))
                        .child(
                            text("The native field is retained by identity, clipped by QuickGUI layout, resized in logical points, and removed automatically when its element disappears. The second full-window WGPU swapchain is created only when the first GPU overlay opens.")
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
