#[cfg(target_os = "macos")]
mod app {
    use std::cell::Cell;

    use objc2::{
        ClassType, DeclaredClass, declare_class, msg_send_id, mutability::MainThreadOnly,
        rc::Retained, runtime::NSObjectProtocol, sel,
    };
    use objc2_app_kit::{NSAutoresizingMaskOptions, NSBezelStyle, NSButton, NSControlSize, NSView};
    use objc2_foundation::{
        MainThreadMarker, NSObject, NSOperatingSystemVersion, NSPoint, NSProcessInfo, NSRect,
        NSSize, NSString,
    };
    use quickgui::{
        Application, Color, IntoElement, View, ViewContext, WindowOptions, div, native_view,
    };

    // objc2-app-kit 0.2 predates the macOS 26 SDK spelling, but NSBezelStyle is a transparent
    // integer wrapper. Keep the native value local to this OS-gated example until the bindings are
    // upgraded and expose NSBezelStyle::Glass directly.
    const NS_BEZEL_STYLE_GLASS: NSBezelStyle = NSBezelStyle(16);
    // Liquid Glass expands beyond the button frame while pressed. QuickGUI clips a native root to
    // its resolved layout box, so mount the button inside a larger transparent AppKit view.
    const GLASS_EFFECT_INSET: f64 = 24.0;

    struct LiquidGlassButtonTargetIvars {
        clicks: Cell<usize>,
    }

    declare_class!(
        struct LiquidGlassButtonTarget;

        unsafe impl ClassType for LiquidGlassButtonTarget {
            type Super = NSObject;
            type Mutability = MainThreadOnly;
            const NAME: &'static str = "QuickGuiLiquidGlassButtonTarget";
        }

        impl DeclaredClass for LiquidGlassButtonTarget {
            type Ivars = LiquidGlassButtonTargetIvars;
        }

        unsafe impl NSObjectProtocol for LiquidGlassButtonTarget {}

        unsafe impl LiquidGlassButtonTarget {
            #[method(quickGuiLiquidGlassButtonPressed:)]
            fn pressed(&self, sender: &NSButton) {
                let clicks = self.ivars().clicks.get().saturating_add(1);
                self.ivars().clicks.set(clicks);

                let title = match clicks {
                    1 => "Clicked once".to_owned(),
                    count => format!("Clicked {count} times"),
                };
                unsafe { sender.setTitle(&NSString::from_str(&title)) };
            }
        }
    );

    impl LiquidGlassButtonTarget {
        fn new(mtm: MainThreadMarker) -> Retained<Self> {
            let allocated = mtm.alloc().set_ivars(LiquidGlassButtonTargetIvars {
                clicks: Cell::new(0),
            });
            unsafe { msg_send_id![super(allocated), init] }
        }
    }

    pub fn run() -> Result<(), quickgui::AppError> {
        let mtm =
            MainThreadMarker::new().expect("the QuickGUI runtime starts on AppKit's main thread");
        let glass_available = supports_liquid_glass();
        let target = LiquidGlassButtonTarget::new(mtm);
        let button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str("Try Liquid Glass"),
                Some(target.as_ref()),
                Some(sel!(quickGuiLiquidGlassButtonPressed:)),
                mtm,
            )
        };

        unsafe {
            button.setControlSize(NSControlSize::Large);
            button.setBezelStyle(if glass_available {
                NS_BEZEL_STYLE_GLASS
            } else {
                NSBezelStyle::Push
            });
        }
        let (glass_host, glass_host_height) = glass_button_host(button.as_ref(), mtm);

        Application::new().run(move |cx| {
            cx.open_window(
                WindowOptions::new("QuickGUI — Native Liquid Glass")
                    .size(480.0, 320.0)
                    .minimum_size(320.0, 220.0)
                    .background(Color::WHITE),
                LiquidGlassDemo {
                    glass_host,
                    glass_host_height,
                    _button: button,
                    _target: target,
                },
            );
        })
    }

    fn supports_liquid_glass() -> bool {
        let minimum = NSOperatingSystemVersion {
            majorVersion: 26,
            minorVersion: 0,
            patchVersion: 0,
        };
        unsafe { NSProcessInfo::processInfo().isOperatingSystemAtLeastVersion(minimum) }
    }

    fn glass_button_host(button: &NSButton, mtm: MainThreadMarker) -> (Retained<NSView>, f32) {
        unsafe {
            // AppKit knows the correct control height and bezel proportions for the current OS.
            // Keep those dimensions instead of stretching the glass bezel into an arbitrary box.
            button.sizeToFit();
            let button_size = button.frame().size;
            let host_size = NSSize::new(
                button_size.width + GLASS_EFFECT_INSET * 2.0,
                button_size.height + GLASS_EFFECT_INSET * 2.0,
            );
            let host = NSView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::ZERO, host_size));

            button.setFrame(NSRect::new(
                NSPoint::new(GLASS_EFFECT_INSET, GLASS_EFFECT_INSET),
                button_size,
            ));
            button.setAutoresizingMask(
                NSAutoresizingMaskOptions::NSViewMinXMargin
                    | NSAutoresizingMaskOptions::NSViewMaxXMargin
                    | NSAutoresizingMaskOptions::NSViewMinYMargin
                    | NSAutoresizingMaskOptions::NSViewMaxYMargin,
            );
            host.addSubview(button);

            (host, host_size.height as f32)
        }
    }

    struct LiquidGlassDemo {
        glass_host: Retained<NSView>,
        glass_host_height: f32,
        // The transparent host retains this button as a subview; this retain documents that its
        // native control identity remains stable while the declarative tree is rebuilt.
        _button: Retained<NSButton>,
        // NSControl doesn't retain its target. Keep the target alive for exactly as long as the
        // retained native button can send actions to it.
        _target: Retained<LiquidGlassButtonTarget>,
    }

    impl View for LiquidGlassDemo {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div()
                .size_full()
                .items_center()
                .justify_center()
                .bg(Color::WHITE)
                .child(
                    native_view(self.glass_host.as_ref())
                        .id("native-liquid-glass-button")
                        .w_full()
                        .h(self.glass_host_height)
                        .flex_none(),
                )
        }
    }
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), quickgui::AppError> {
    app::run()
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("The liquid_glass_button example requires macOS.");
}
