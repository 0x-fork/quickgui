#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use std::time::{Duration, Instant};

    use objc2::rc::Retained;
    use objc2_app_kit::{NSApplication, NSView, NSWindow, NSWindowButton};
    use objc2_foundation::MainThreadMarker;
    use quickgui::{
        AppRunner, Application, Color, MacOsVibrancy, Point, Rect, TitleBarStyle, View,
        ViewContext, WindowBounds, WindowOptions, div,
    };

    struct EmptyView;

    impl View for EmptyView {
        fn render(&mut self, _: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
            div().size_full()
        }
    }

    fn pump_until(runner: &mut AppRunner, ready: impl Fn(&AppRunner) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            runner.pump(Some(Duration::ZERO)).unwrap();
            if ready(runner) {
                return;
            }
            assert!(Instant::now() < deadline, "native window command timed out");
        }
    }

    fn find_window(app: &NSApplication, title: &str) -> Option<Retained<NSWindow>> {
        app.windows()
            .into_iter()
            .find(|window| window.title().to_string() == title)
    }

    fn assert_inset(window: &NSWindow, inset: Point, step: f64) {
        for (index, kind) in [
            NSWindowButton::NSWindowCloseButton,
            NSWindowButton::NSWindowMiniaturizeButton,
            NSWindowButton::NSWindowZoomButton,
        ]
        .into_iter()
        .enumerate()
        {
            let button = window.standardWindowButton(kind).unwrap();
            let frame = button.convertRect_toView(button.bounds(), None);
            let x = frame.origin.x;
            let y = window.frame().size.height - frame.origin.y - frame.size.height;
            let expected_x = f64::from(inset.x) + step * index as f64;
            assert!(
                (x - expected_x).abs() < 0.01 && (y - f64::from(inset.y)).abs() < 0.01,
                "{}: button {index} has inset ({x}, {y}), expected ({expected_x}, {})",
                window.title(),
                inset.y,
            );
        }
    }

    let mtm = MainThreadMarker::new().expect("native window checks require the main thread");
    let app = NSApplication::sharedApplication(mtm);
    let mut runner = Application::new().into_runner().unwrap();
    pump_until(&mut runner, AppRunner::is_ready);

    for (index, vibrancy) in [None, Some(MacOsVibrancy::Sidebar)].into_iter().enumerate() {
        let title = format!("QuickGUI traffic lights {index}");
        let inset = Point::new(14.0 + index as f32 * 8.0, 19.0 + index as f32 * 8.0);
        let mut options = WindowOptions::new(title.clone())
            .size(640.0, 480.0)
            .show(false)
            .focus(false)
            .title_bar_style(TitleBarStyle::HiddenInset)
            .traffic_light_position(inset.x, inset.y);
        if let Some(vibrancy) = vibrancy {
            options = options
                .macos_vibrancy(vibrancy)
                .background(Color::TRANSPARENT);
        }
        let handle = runner.open_window(options, EmptyView).unwrap();
        pump_until(&mut runner, |_| find_window(&app, &title).is_some());
        let window = find_window(&app, &title).unwrap();
        let close = window
            .standardWindowButton(NSWindowButton::NSWindowCloseButton)
            .unwrap();
        let minimize = window
            .standardWindowButton(NSWindowButton::NSWindowMiniaturizeButton)
            .unwrap();
        let step = NSView::frame(&minimize).origin.x - NSView::frame(&close).origin.x;
        assert_inset(&window, inset, step);

        // Quick Git updates the title as its repository and branch load. A hidden window does
        // not get AppKit's later display/update repair: the command itself must keep its inset.
        for suffix in ["repository", "repository — main", "repository — feature"] {
            let title = format!("QuickGUI traffic lights {index}: {suffix}");
            runner.set_window_title(handle, title.clone()).unwrap();
            pump_until(&mut runner, |_| window.title().to_string() == title);
            assert!(!window.isVisible());
            assert_inset(&window, inset, step);
        }

        runner
            .set_window_bounds(
                handle,
                WindowBounds::Windowed(Rect::new(100.0, 100.0, 720.0, 520.0)),
            )
            .unwrap();
        pump_until(&mut runner, |_| {
            (window.frame().size.height - 520.0).abs() < 0.01
        });
        assert_inset(&window, inset, step);
        assert!(runner.close_window(handle));
        pump_until(&mut runner, |runner| {
            !runner.window_registry().contains(handle)
        });
    }
    println!("native traffic lights: creation, title changes, resize, and vibrancy passed");
}
