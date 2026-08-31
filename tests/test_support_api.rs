#![cfg(feature = "test-support")]

use quickgui::{
    Application, EventContext, IntoElement, Key, KeyBinding, KeyboardLayout, Keystroke, Modifiers,
    TestAppContext, TestAppError, View, ViewContext, VisualSnapshot, VisualTolerance, WindowHandle,
    WindowOptions, button, text,
};

quickgui::actions!(test_support_api, [LocalizedCommand]);

struct Counter(usize);

impl View for Counter {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let increment = cx.listener("increment", |view, cx: &mut EventContext| {
            view.0 += 1;
            cx.invalidate();
        });
        button().on_click(increment).child(text(self.0.to_string()))
    }
}

#[test]
fn downstream_crates_can_use_the_feature_gated_test_api() {
    let (mut cx, counter) = Application::new()
        .bind_keys([KeyBinding::new("platform-[", LocalizedCommand, None).use_key_equivalents()])
        .on_keyboard_layout_change(|layout, cx| {
            assert_eq!(layout, cx.keyboard_layout());
        })
        .into_test_context(WindowOptions::default(), Counter(0))
        .unwrap();

    cx.click(counter.window_handle(), "increment").unwrap();

    assert_eq!(cx.read(counter, |counter| counter.0).unwrap(), 1);
    assert_eq!(
        cx.visual(counter.window_handle()).unwrap().window_handle(),
        counter.window_handle()
    );
    let _capture: fn(&mut TestAppContext, WindowHandle) -> Result<VisualSnapshot, TestAppError> =
        TestAppContext::capture_screenshot;
    cx.simulate_keyboard_layout_change(
        KeyboardLayout::new("com.example.test", "Test layout").unwrap(),
    )
    .unwrap();
    assert_eq!(cx.keyboard_layout().id(), "com.example.test");
    cx.simulate_keystroke(
        counter.window_handle(),
        Keystroke::new(Key::Character("a".to_owned()), Modifiers::ALT)
            .with_key_char(Key::Character("å".to_owned())),
    )
    .unwrap();
    assert_eq!(VisualTolerance::default(), VisualTolerance::EXACT);
}
