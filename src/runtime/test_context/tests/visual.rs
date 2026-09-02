use super::*;

struct EmptyVisualTestView;

impl View for EmptyVisualTestView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div()
    }
}

#[test]
fn oversized_visual_capture_fails_before_allocating_a_gpu() {
    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(3_000.0, 32.0),
            EmptyVisualTestView,
        )
        .unwrap();
    assert!(matches!(
        cx.capture_screenshot(view.window_handle()),
        Err(TestAppError::Visual(
            crate::VisualTestError::InvalidDimensions
        ))
    ));
    assert!(cx.visual_renderer.is_none());
}

#[cfg(target_os = "macos")]
struct VisualTestView;

#[cfg(target_os = "macos")]
impl View for VisualTestView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(Color::rgb8(190, 30, 45))
            .child(
                div()
                    .id("visual-card")
                    .w(20.0)
                    .h(10.0)
                    .bg(Color::rgb8(20, 80, 210)),
            )
            .child(
                text("Visual")
                    .id("visual-label")
                    .text_sm()
                    .text_color(Color::WHITE),
            )
    }
}

#[cfg(target_os = "macos")]
#[test]
fn visual_context_uses_production_layout_and_offscreen_wgpu_capture() {
    let (mut cx, view) = Application::new()
        .into_test_context(WindowOptions::default().size(64.0, 48.0), VisualTestView)
        .unwrap();
    let window = view.window_handle();
    let mut visual = cx.visual(window).unwrap();
    visual
        .assert_element_bounds("visual-card", Rect::new(0.0, 0.0, 20.0, 10.0), 0.0)
        .unwrap();
    let label = visual.element_bounds("visual-label").unwrap();
    assert_eq!((label.x, label.y), (20.0, 0.0));
    assert!(label.width > 0.0 && label.height > 0.0);
    let first = visual.capture_screenshot().unwrap();
    let second = visual.capture_screenshot().unwrap();
    assert_eq!((first.width(), first.height()), (128, 96));
    assert_eq!(first.pixel(20, 10), Some([20, 80, 210, 255]));
    assert_eq!(first.pixel(127, 95), Some([190, 30, 45, 255]));
    assert_eq!(
        second
            .assert_matches(&first, crate::VisualTolerance::EXACT)
            .unwrap()
            .differing_pixels,
        0
    );
}

#[cfg(target_os = "macos")]
struct RightBorderVisualView;

#[cfg(target_os = "macos")]
impl View for RightBorderVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex_row()
            .bg(Color::rgb8(237, 237, 238))
            .child(
                div()
                    .id("border-sidebar")
                    .w(20.0)
                    .h_full()
                    .flex_none()
                    .border_right(1.0, Color::rgb8(204, 204, 204)),
            )
            .child(
                div()
                    .id("border-content")
                    .flex_1()
                    .h_full()
                    .bg(Color::WHITE),
            )
    }
}

#[cfg(target_os = "macos")]
#[test]
fn right_border_stays_attached_to_its_fill_at_retina_scale() {
    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(40.0, 20.0),
            RightBorderVisualView,
        )
        .unwrap();
    let mut visual = cx.visual(view.window_handle()).unwrap();
    visual
        .assert_element_bounds("border-sidebar", Rect::new(0.0, 0.0, 20.0, 20.0), 0.0)
        .unwrap();
    visual
        .assert_element_bounds("border-content", Rect::new(20.0, 0.0, 20.0, 20.0), 0.0)
        .unwrap();
    let snapshot = visual.capture_screenshot().unwrap();
    assert_eq!(snapshot.pixel(37, 20), Some([237, 237, 238, 255]));
    assert_eq!(snapshot.pixel(38, 20), Some([204, 204, 204, 255]));
    assert_eq!(snapshot.pixel(39, 20), Some([204, 204, 204, 255]));
    assert_eq!(snapshot.pixel(40, 20), Some([255, 255, 255, 255]));
}

#[cfg(target_os = "macos")]
struct WavyUnderlineVisualView;

#[cfg(target_os = "macos")]
impl View for WavyUnderlineVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div().size_full().p_4().bg(Color::rgb8(8, 10, 14)).child(
            text("GPU WAVY UNDERLINE")
                .id("wavy-label")
                .text_size(24.0)
                .line_height(36.0)
                .text_color(Color::TRANSPARENT)
                .text_decoration_color(Color::rgb8(248, 113, 113))
                .text_decoration_2()
                .text_decoration_wavy()
                .whitespace_nowrap(),
        )
    }
}

#[cfg(target_os = "macos")]
#[test]
fn visual_wavy_underline_is_analytic_stable_and_not_a_solid_bar() {
    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(280.0, 72.0),
            WavyUnderlineVisualView,
        )
        .unwrap();
    let mut visual = cx.visual(view.window_handle()).unwrap();
    let first = visual.capture_screenshot().unwrap();
    let second = visual.capture_screenshot().unwrap();
    assert_eq!(
        second
            .assert_matches(&first, crate::VisualTolerance::EXACT)
            .unwrap()
            .differing_pixels,
        0
    );

    let mut colored_pixels = 0_usize;
    let mut colored_rows = HashSet::new();
    let mut columns = HashMap::<u32, (u64, u32)>::new();
    for (index, pixel) in first.rgba().as_chunks::<4>().0.iter().enumerate() {
        if pixel[0] > 180
            && (70..=160).contains(&pixel[1])
            && (70..=160).contains(&pixel[2])
            && pixel[3] > 0
        {
            let x = index as u32 % first.width();
            let y = index as u32 / first.width();
            colored_pixels += 1;
            colored_rows.insert(y);
            let column = columns.entry(x).or_default();
            column.0 += u64::from(y);
            column.1 += 1;
        }
    }
    assert!(
        colored_pixels > 100,
        "the underline must paint a visible span"
    );
    assert!(
        colored_rows.len() >= 6,
        "a wavy underline must cover several physical rows"
    );
    let center_rows = columns
        .values()
        .filter(|(_, count)| *count > 0)
        .map(|(sum, count)| (sum / u64::from(*count)) as u32)
        .collect::<HashSet<_>>();
    assert!(
        center_rows.len() >= 4,
        "the underline center must vary across x instead of forming a solid bar"
    );
}

#[cfg(target_os = "macos")]
struct OpacityVisualView;

#[cfg(target_os = "macos")]
impl View for OpacityVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let rich = || {
            crate::styled_text("RICH").with_highlights([(
                0..4,
                crate::HighlightStyle::default().color(Color::rgb8(248, 40, 72)),
            )])
        };
        div()
            .size_full()
            .flex_col()
            .bg(Color::BLACK)
            .child(
                div()
                    .id("opacity-reference")
                    .h(36.0)
                    .flex_none()
                    .text_size(24.0)
                    .line_height(32.0)
                    .child(rich()),
            )
            .child(
                div().h(36.0).flex_none().opacity(0.5).child(
                    div()
                        .id("opacity-target")
                        .size_full()
                        .opacity(0.5)
                        .hover(|style| style.opacity(1.0))
                        .transition(Duration::from_millis(100))
                        .text_size(24.0)
                        .line_height(32.0)
                        .child(rich()),
                ),
            )
    }
}

#[cfg(target_os = "macos")]
#[test]
fn subtree_opacity_multiplies_rich_text_and_transitions_without_reshaping() {
    fn maximum_red(snapshot: &crate::VisualSnapshot, bounds: Rect, scale: f32) -> u8 {
        let left = (bounds.x * scale).floor().max(0.0) as u32;
        let top = (bounds.y * scale).floor().max(0.0) as u32;
        let right = (bounds.right() * scale).ceil().min(snapshot.width() as f32) as u32;
        let bottom = (bounds.bottom() * scale)
            .ceil()
            .min(snapshot.height() as f32) as u32;
        (top..bottom)
            .flat_map(|y| (left..right).filter_map(move |x| snapshot.pixel(x, y)))
            .map(|pixel| pixel[0])
            .max()
            .unwrap_or(0)
    }

    let (mut cx, view) = Application::new()
        .into_test_context(WindowOptions::default().size(96.0, 72.0), OpacityVisualView)
        .unwrap();
    let window = view.window_handle();
    let render_count = cx.render_count(window).unwrap();
    {
        let mut visual = cx.visual(window).unwrap();
        let reference_bounds = visual.element_bounds("opacity-reference").unwrap();
        let target_bounds = visual.element_bounds("opacity-target").unwrap();
        let scale = 2.0;

        let initial = visual.capture_screenshot().unwrap();
        let stable = visual.capture_screenshot().unwrap();
        stable
            .assert_matches(&initial, crate::VisualTolerance::EXACT)
            .unwrap();
        assert_eq!(
            visual
                .context
                .visual_renderer
                .as_ref()
                .unwrap()
                .last_reshaped_text_areas(),
            0
        );
        let reference_red = maximum_red(&initial, reference_bounds, scale);
        let initial_red = maximum_red(&initial, target_bounds, scale);
        assert!(reference_red > initial_red && initial_red > 40);

        assert!(visual.move_pointer(Point::new(12.0, 48.0)).unwrap());
        visual.capture_screenshot().unwrap();
        assert_eq!(
            visual
                .context
                .visual_renderer
                .as_ref()
                .unwrap()
                .last_reshaped_text_areas(),
            0
        );

        visual.advance_time(Duration::from_millis(50)).unwrap();
        let midpoint = visual.capture_screenshot().unwrap();
        let midpoint_red = maximum_red(&midpoint, target_bounds, scale);
        assert!(midpoint_red > initial_red);
        assert_eq!(
            visual
                .context
                .visual_renderer
                .as_ref()
                .unwrap()
                .last_reshaped_text_areas(),
            0
        );

        visual.advance_time(Duration::from_millis(50)).unwrap();
        let completed = visual.capture_screenshot().unwrap();
        let completed_red = maximum_red(&completed, target_bounds, scale);
        assert!(reference_red > completed_red && completed_red > midpoint_red);
        assert_eq!(
            visual
                .context
                .visual_renderer
                .as_ref()
                .unwrap()
                .last_reshaped_text_areas(),
            0
        );
    }
    assert_eq!(cx.render_count(window).unwrap(), render_count);
}

#[cfg(target_os = "macos")]
struct TextOverflowVisualView;

#[cfg(target_os = "macos")]
impl View for TextOverflowVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex_col()
            .gap_2()
            .p_2()
            .bg(Color::rgb8(18, 19, 23))
            .text_color(Color::WHITE)
            .child(
                text("Unicode 🙂 alpha beta gamma delta epsilon zeta eta theta iota kappa")
                    .id("clamped-label")
                    .w(132.0)
                    .flex_none()
                    .text_sm()
                    .line_clamp(2)
                    .text_ellipsis(),
            )
            .child(
                text("/Users/example/a-very-long-directory/important-file.rs")
                    .id("middle-label")
                    .w(132.0)
                    .h(20.0)
                    .flex_none()
                    .text_sm()
                    .whitespace_nowrap()
                    .text_ellipsis_middle()
                    .overflow_hidden(),
            )
    }
}

#[cfg(target_os = "macos")]
#[test]
fn visual_text_overflow_clamps_layout_and_reuses_an_exact_frame() {
    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(180.0, 100.0),
            TextOverflowVisualView,
        )
        .unwrap();
    let mut visual = cx.visual(view.window_handle()).unwrap();
    let clamped = visual.element_bounds("clamped-label").unwrap();
    assert_eq!((clamped.width, clamped.height), (132.0, 40.0));
    let middle = visual.element_bounds("middle-label").unwrap();
    assert_eq!((middle.width, middle.height), (132.0, 20.0));

    let first = visual.capture_screenshot().unwrap();
    let second = visual.capture_screenshot().unwrap();
    assert_eq!(
        second
            .assert_matches(&first, crate::VisualTolerance::EXACT)
            .unwrap()
            .differing_pixels,
        0
    );
}

#[cfg(target_os = "macos")]
struct TransitionVisualView;

#[cfg(target_os = "macos")]
impl View for TransitionVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div()
            .id("transition-surface")
            .size_full()
            .bg(Color::BLACK)
            .hover(|style| style.bg(Color::WHITE).rounded(12.0))
            .transition(Duration::from_millis(100))
    }
}

#[cfg(target_os = "macos")]
#[test]
fn paint_only_hover_transitions_reverse_without_rebuilding_the_view() {
    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(32.0, 32.0),
            TransitionVisualView,
        )
        .unwrap();
    let window = view.window_handle();
    let render_count = cx.render_count(window).unwrap();
    {
        let mut visual = cx.visual(window).unwrap();
        let initial = visual.capture_screenshot().unwrap();
        assert_eq!(initial.pixel(32, 32), Some([0, 0, 0, 255]));

        assert!(visual.move_pointer(Point::new(16.0, 16.0)).unwrap());
        let start = visual.capture_screenshot().unwrap();
        assert_eq!(start.pixel(32, 32), Some([0, 0, 0, 255]));

        visual.advance_time(Duration::from_millis(50)).unwrap();
        let midpoint = visual.capture_screenshot().unwrap();
        let midpoint_pixel = midpoint.pixel(32, 32).unwrap();
        assert!((186..=190).contains(&midpoint_pixel[0]));
        assert_eq!(midpoint_pixel[0], midpoint_pixel[1]);
        assert_eq!(midpoint_pixel[1], midpoint_pixel[2]);

        assert!(visual.move_pointer(Point::new(48.0, 48.0)).unwrap());
        let reversed = visual.capture_screenshot().unwrap();
        assert_eq!(reversed.pixel(32, 32), Some(midpoint_pixel));

        visual.advance_time(Duration::from_millis(100)).unwrap();
        let completed = visual.capture_screenshot().unwrap();
        assert_eq!(completed.pixel(32, 32), Some([0, 0, 0, 255]));
    }
    assert_eq!(cx.render_count(window).unwrap(), render_count);
}

#[cfg(target_os = "macos")]
struct DetachedTooltipAnimationVisualView;

#[cfg(target_os = "macos")]
impl View for DetachedTooltipAnimationVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let tooltip = Tooltip::new(div().w(24.0).h(12.0).with_animation(
            "tooltip-fade",
            Animation::new(Duration::from_millis(100)),
            |element, phase| element.bg(Color::interpolate(Color::BLACK, Color::WHITE, phase)),
        ))
        .placement(AnchorPlacement::Bottom)
        .delay(Duration::ZERO)
        .gap(0.0)
        .viewport_margin(0.0);
        div()
            .size_full()
            .items_center()
            .justify_center()
            .bg(Color::BLACK)
            .child(
                div()
                    .id("detached-tooltip-trigger")
                    .w(24.0)
                    .h(12.0)
                    .tooltip(tooltip),
            )
    }
}

#[cfg(target_os = "macos")]
#[test]
fn detached_tooltip_motion_rebuilds_only_its_tree_and_drops_its_frame_source() {
    fn brightest(snapshot: &crate::VisualSnapshot) -> u8 {
        snapshot
            .rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| pixel[0].max(pixel[1]).max(pixel[2]))
            .max()
            .unwrap_or(0)
    }

    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(64.0, 64.0),
            DetachedTooltipAnimationVisualView,
        )
        .unwrap();
    let window = view.window_handle();
    let render_count = cx.render_count(window).unwrap();
    {
        let mut visual = cx.visual(window).unwrap();
        let baseline = brightest(&visual.capture_screenshot().unwrap());
        assert!(baseline <= 4);
        assert!(visual.move_pointer(Point::new(32.0, 32.0)).unwrap());

        let start = visual.capture_screenshot().unwrap();
        assert_eq!(brightest(&start), baseline);
        assert!(
            visual
                .context
                .window(window)
                .unwrap()
                .ui
                .detached_animation_frame_requested()
        );

        visual.advance_time(Duration::from_millis(50)).unwrap();
        let midpoint = brightest(&visual.capture_screenshot().unwrap());
        assert!(
            (186..=190).contains(&midpoint),
            "unexpected tooltip midpoint brightness: {midpoint}"
        );

        visual.advance_time(Duration::from_millis(50)).unwrap();
        assert_eq!(brightest(&visual.capture_screenshot().unwrap()), 255);
        assert!(
            !visual
                .context
                .window(window)
                .unwrap()
                .ui
                .detached_animation_frame_requested()
        );

        assert!(visual.move_pointer(Point::new(80.0, 80.0)).unwrap());
        assert_eq!(brightest(&visual.capture_screenshot().unwrap()), baseline);
    }
    let ui = &cx.window(window).unwrap().ui;
    assert_eq!(ui.animation_counts().1, 0);
    assert_eq!(ui.next_animation_deadline(), None);
    assert!(!ui.detached_animation_frame_requested());
    assert_eq!(cx.render_count(window).unwrap(), render_count);
}

#[cfg(target_os = "macos")]
struct DetachedDragPreviewAnimationVisualView;

#[cfg(target_os = "macos")]
impl View for DetachedDragPreviewAnimationVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div()
            .size_full()
            .items_center()
            .justify_center()
            .bg(Color::BLACK)
            .child(div().id("detached-drag-source").w(12.0).h(12.0))
    }
}

#[cfg(target_os = "macos")]
#[test]
fn detached_drag_preview_motion_drops_all_scheduling_when_cleared() {
    fn brightest(snapshot: &crate::VisualSnapshot) -> u8 {
        snapshot
            .rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| pixel[0].max(pixel[1]).max(pixel[2]))
            .max()
            .unwrap_or(0)
    }

    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(64.0, 64.0),
            DetachedDragPreviewAnimationVisualView,
        )
        .unwrap();
    let window = view.window_handle();
    let render_count = cx.render_count(window).unwrap();
    {
        let mut visual = cx.visual(window).unwrap();
        let baseline = brightest(&visual.capture_screenshot().unwrap());
        assert!(baseline <= 4);
        let preview = div().w(16.0).h(12.0).with_animation(
            "drag-preview-fade",
            Animation::new(Duration::from_millis(100)),
            |element, phase| {
                element
                    .bg(Color::interpolate(Color::BLACK, Color::WHITE, phase))
                    .clickable()
            },
        );
        let now = visual.context.now();
        let mut renderer = visual.context.visual_renderer.take().unwrap();
        let installed = visual
            .context
            .window_mut(window)
            .unwrap()
            .ui
            .set_drag_preview(
                Some(preview),
                ElementId::named("detached-drag-source"),
                Point::new(32.0, 32.0),
                Point::new(16.0, 16.0),
                Some(Point::ZERO),
                &mut renderer,
                now,
            )
            .unwrap();
        visual.context.visual_renderer = Some(renderer);
        assert!(installed);

        assert_eq!(brightest(&visual.capture_screenshot().unwrap()), baseline);
        assert!(
            visual
                .context
                .window(window)
                .unwrap()
                .ui
                .detached_animation_frame_requested()
        );

        visual.advance_time(Duration::from_millis(50)).unwrap();
        let midpoint = brightest(&visual.capture_screenshot().unwrap());
        assert!((186..=190).contains(&midpoint));

        visual.advance_time(Duration::from_millis(50)).unwrap();
        assert_eq!(brightest(&visual.capture_screenshot().unwrap()), 255);
        assert!(
            visual
                .context
                .window_mut(window)
                .unwrap()
                .ui
                .clear_drag_preview()
        );
        assert_eq!(brightest(&visual.capture_screenshot().unwrap()), baseline);
    }
    let ui = &cx.window(window).unwrap().ui;
    assert_eq!(ui.animation_counts().1, 0);
    assert_eq!(ui.next_animation_deadline(), None);
    assert!(!ui.detached_animation_frame_requested());
    assert_eq!(cx.render_count(window).unwrap(), render_count);
}

#[cfg(target_os = "macos")]
struct VariableListVisualView {
    list: ListState,
}

#[cfg(target_os = "macos")]
impl View for VariableListVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        self.list.set_viewport_size(100.0, 60.0);
        let rows = self
            .list
            .render_rows(self.list.visible_rows().range, |index| {
                div()
                    .id(ElementId::new(0x7000 + index as u64))
                    .h([20.0, 40.0, 30.0, 18.0, 26.0][index])
                    .bg(Color::rgb8(20 + index as u8 * 20, 80, 160))
            });
        div().size_full().child(
            div()
                .id("measured-list")
                .relative()
                .size(100.0, 60.0)
                .variable_virtual_scroll(&self.list)
                .child(rows),
        )
    }
}

#[cfg(target_os = "macos")]
#[test]
fn visual_context_converges_variable_list_measurements_without_view_or_idle_frames() {
    let list = ListState::new(5, 24.0).with_overscan(1);
    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(100.0, 60.0),
            VariableListVisualView { list: list.clone() },
        )
        .unwrap();
    let window = view.window_handle();
    let initial_renders = cx.render_count(window).unwrap();
    let mut visual = cx.visual(window).unwrap();
    visual
        .assert_element_bounds(
            ElementId::new(0x7001),
            Rect::new(0.0, 20.0, 100.0, 40.0),
            0.0,
        )
        .unwrap();
    assert!(list.stats().measured_items >= 3);
    let converged_renders = cx.render_count(window).unwrap();
    assert_eq!(
        converged_renders, initial_renders,
        "mounted measurement convergence must not rerender the declarative view"
    );

    // Re-reading settled geometry performs layout on demand but does not rebuild the view.
    cx.element_bounds(window, ElementId::new(0x7001)).unwrap();
    assert_eq!(cx.render_count(window).unwrap(), converged_renders);
}

struct LoopView;

impl View for LoopView {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let focus = cx.focus_handle("loop");
        let listener = cx.action_listener("loop", |_view, _: &LoopForTest, cx| {
            cx.dispatch_action(LoopForTest);
        });
        div().track_focus(focus).auto_focus().on_action(listener)
    }
}

#[test]
fn recursive_effects_fail_at_a_bounded_turn_instead_of_hanging() {
    let (mut cx, view) = TestAppContext::new(LoopView).unwrap();
    assert!(matches!(
        cx.dispatch_action(view.window_handle(), LoopForTest),
        Err(TestAppError::EffectTurnLimit)
    ));
}

#[cfg(target_os = "macos")]
struct LinearGradientVisualView;

#[cfg(target_os = "macos")]
impl View for LinearGradientVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div().size_full().id("gradient-band").bg_gradient(
            crate::Gradient::linear(
                crate::GradientDirection::ToRight,
                [Color::BLACK, Color::WHITE],
            )
            .color_space(crate::GradientColorSpace::Srgb),
        )
    }
}

#[cfg(target_os = "macos")]
#[test]
fn visual_linear_gradient_interpolates_monotonically_across_the_element() {
    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(64.0, 8.0),
            LinearGradientVisualView,
        )
        .unwrap();
    let mut visual = cx.visual(view.window_handle()).unwrap();
    visual
        .assert_element_bounds("gradient-band", Rect::new(0.0, 0.0, 64.0, 8.0), 0.0)
        .unwrap();
    let snapshot = visual.capture_screenshot().unwrap();
    assert_eq!((snapshot.width(), snapshot.height()), (128, 16));

    let row = 8;
    let first = snapshot.pixel(0, row).unwrap();
    let last = snapshot.pixel(127, row).unwrap();
    assert!(
        first[0] < 12,
        "the first stop must stay near black: {first:?}"
    );
    assert!(
        last[0] > 243,
        "the last stop must stay near white: {last:?}"
    );
    // Encoded-sRGB interpolation reaches roughly half intensity at the midpoint.
    let middle = snapshot.pixel(64, row).unwrap();
    assert!(
        (i32::from(middle[0]) - 128).abs() <= 4,
        "the midpoint must be mid gray: {middle:?}"
    );
    let mut previous = 0_u8;
    for x in 0..snapshot.width() {
        let pixel = snapshot.pixel(x, row).unwrap();
        assert!(
            pixel[0] >= previous,
            "a left-to-right gradient must never darken at {x}"
        );
        assert_eq!(
            [pixel[1], pixel[2], pixel[3]],
            [pixel[0], pixel[0], 255],
            "a black-to-white ramp stays neutral and opaque at {x}"
        );
        previous = pixel[0];
    }
    // A second capture of the same settled scene is byte identical.
    let repeat = visual.capture_screenshot().unwrap();
    assert_eq!(
        repeat
            .assert_matches(&snapshot, crate::VisualTolerance::EXACT)
            .unwrap()
            .differing_pixels,
        0
    );
}

#[cfg(target_os = "macos")]
struct RadialConicGradientVisualView;

#[cfg(target_os = "macos")]
impl View for RadialConicGradientVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex_row()
            .child(
                div()
                    .id("radial")
                    .w(32.0)
                    .h(32.0)
                    .flex_none()
                    .bg_radial_gradient_at(
                        crate::RadialGradientShape::Circle,
                        crate::GradientCenter::CENTER,
                        [Color::WHITE, Color::BLACK],
                    ),
            )
            .child(
                div()
                    .id("conic")
                    .w(32.0)
                    .h(32.0)
                    .flex_none()
                    .bg_conic_gradient(0.0, [Color::BLACK, Color::WHITE]),
            )
    }
}

#[cfg(target_os = "macos")]
#[test]
fn visual_radial_and_conic_gradients_use_distinct_geometry() {
    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(64.0, 32.0),
            RadialConicGradientVisualView,
        )
        .unwrap();
    let mut visual = cx.visual(view.window_handle()).unwrap();
    visual
        .assert_element_bounds("radial", Rect::new(0.0, 0.0, 32.0, 32.0), 0.0)
        .unwrap();
    visual
        .assert_element_bounds("conic", Rect::new(32.0, 0.0, 32.0, 32.0), 0.0)
        .unwrap();
    let snapshot = visual.capture_screenshot().unwrap();

    // The radial gradient is brightest at its center and dark at the farthest corner.
    let center = snapshot.pixel(32, 32).unwrap();
    let corner = snapshot.pixel(1, 1).unwrap();
    assert!(center[0] > 240, "radial center must stay white: {center:?}");
    // Linear-light interpolation is the default, so the farthest corner is nearly black.
    assert!(corner[0] < 70, "radial corner must reach black: {corner:?}");
    let midway = snapshot.pixel(16, 16).unwrap();
    assert!(
        corner[0] < midway[0] && midway[0] < center[0],
        "a radial ramp must darken with distance: {center:?} {midway:?} {corner:?}"
    );
    // Distance, not axis, drives a circular radial gradient.
    let horizontal = snapshot.pixel(52, 32).unwrap();
    let vertical = snapshot.pixel(32, 12).unwrap();
    assert!(
        i32::from(horizontal[0]).abs_diff(i32::from(vertical[0])) <= 6,
        "a circle must be radially symmetric: {horizontal:?} {vertical:?}"
    );

    // The conic gradient sweeps clockwise from the top: up is the first stop, down the last.
    let up = snapshot.pixel(96, 12).unwrap();
    let down = snapshot.pixel(96, 52).unwrap();
    let right = snapshot.pixel(116, 32).unwrap();
    assert!(up[0] < 40, "the conic sweep must start dark: {up:?}");
    // Half a turn from the start is the midpoint of a two-stop sweep.
    assert!(
        down[0] > 150,
        "the conic sweep must advance half way opposite the start: {down:?}"
    );
    assert!(
        up[0] < right[0] && right[0] < down[0],
        "the conic sweep must advance clockwise: {up:?} {right:?} {down:?}"
    );
}

#[cfg(target_os = "macos")]
struct CornerRadiiVisualView;

#[cfg(target_os = "macos")]
impl View for CornerRadiiVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div().size_full().bg(Color::WHITE).child(
            div()
                .id("corner-card")
                .size_full()
                .bg(Color::BLACK)
                .rounded_tl(16.0)
                .rounded_br(16.0),
        )
    }
}

#[cfg(target_os = "macos")]
#[test]
fn visual_per_corner_radii_round_only_the_declared_corners() {
    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(32.0, 32.0),
            CornerRadiiVisualView,
        )
        .unwrap();
    let mut visual = cx.visual(view.window_handle()).unwrap();
    visual
        .assert_element_bounds("corner-card", Rect::new(0.0, 0.0, 32.0, 32.0), 0.0)
        .unwrap();
    let snapshot = visual.capture_screenshot().unwrap();
    assert_eq!((snapshot.width(), snapshot.height()), (64, 64));

    // Rounded corners reveal the white parent; square corners stay filled.
    assert_eq!(snapshot.pixel(1, 1), Some([255, 255, 255, 255]));
    assert_eq!(snapshot.pixel(62, 62), Some([255, 255, 255, 255]));
    // Sampled off the diagonal so the square corners are measured on their straight edges.
    assert_eq!(snapshot.pixel(62, 10), Some([0, 0, 0, 255]));
    assert_eq!(snapshot.pixel(10, 62), Some([0, 0, 0, 255]));
    // The rounded corners cut a visible arc rather than one antialiased pixel.
    assert_eq!(snapshot.pixel(6, 6), Some([255, 255, 255, 255]));
    assert_eq!(snapshot.pixel(57, 57), Some([255, 255, 255, 255]));
}

#[cfg(target_os = "macos")]
struct BorderStyleVisualView;

#[cfg(target_os = "macos")]
impl View for BorderStyleVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(Color::BLACK)
            .flex_col()
            .child(
                div()
                    .id("solid-box")
                    .w_full()
                    .h(20.0)
                    .flex_none()
                    .border(2.0, Color::WHITE),
            )
            .child(
                div()
                    .id("dashed-box")
                    .w_full()
                    .h(20.0)
                    .flex_none()
                    .border(2.0, Color::WHITE)
                    .border_dashed(),
            )
            .child(
                div()
                    .id("dotted-box")
                    .w_full()
                    .h(20.0)
                    .flex_none()
                    .border(2.0, Color::WHITE)
                    .border_dotted(),
            )
    }
}

#[cfg(target_os = "macos")]
#[test]
fn visual_dashed_and_dotted_borders_leave_evenly_spaced_gaps() {
    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(60.0, 60.0),
            BorderStyleVisualView,
        )
        .unwrap();
    let mut visual = cx.visual(view.window_handle()).unwrap();
    let snapshot = visual.capture_screenshot().unwrap();

    let bright_in_row = |row: u32| {
        (0..snapshot.width())
            .filter(|x| {
                snapshot
                    .pixel(*x, row)
                    .is_some_and(|pixel| pixel[0] > 200 && pixel[1] > 200 && pixel[2] > 200)
            })
            .count()
    };
    // The first physical row inside each box's top border.
    let solid = bright_in_row(1);
    let dashed = bright_in_row(41);
    let dotted = bright_in_row(81);
    assert_eq!(
        solid,
        snapshot.width() as usize,
        "a solid border is continuous"
    );
    assert!(
        (30..solid).contains(&dashed),
        "a dashed border must leave gaps: {dashed} of {solid}"
    );
    assert!(
        dotted < dashed,
        "dots are shorter than dashes: {dotted} vs {dashed}"
    );
    assert!(dotted > 10, "dots must still paint: {dotted}");
}

#[cfg(target_os = "macos")]
struct OutlineVisualView;

#[cfg(target_os = "macos")]
impl View for OutlineVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div().size_full().bg(Color::WHITE).p(10.0).child(
            div()
                .id("outlined")
                .size_full()
                .bg(Color::BLACK)
                .outline(2.0, Color::rgb8(255, 0, 0))
                .outline_offset(2.0),
        )
    }
}

#[cfg(target_os = "macos")]
#[test]
fn visual_outlines_paint_outside_the_border_box_without_changing_layout() {
    let (mut cx, view) = Application::new()
        .into_test_context(WindowOptions::default().size(40.0, 40.0), OutlineVisualView)
        .unwrap();
    let mut visual = cx.visual(view.window_handle()).unwrap();
    // The outline never participates in layout.
    visual
        .assert_element_bounds("outlined", Rect::new(10.0, 10.0, 20.0, 20.0), 0.0)
        .unwrap();
    let snapshot = visual.capture_screenshot().unwrap();

    let row = 40;
    // Background, then the offset ring, then the gap, then the element itself.
    assert_eq!(snapshot.pixel(10, row), Some([255, 255, 255, 255]));
    assert_eq!(snapshot.pixel(13, row), Some([255, 0, 0, 255]));
    assert_eq!(snapshot.pixel(18, row), Some([255, 255, 255, 255]));
    assert_eq!(snapshot.pixel(25, row), Some([0, 0, 0, 255]));
    // The ring is symmetric on the opposite edge.
    assert_eq!(snapshot.pixel(66, row), Some([255, 0, 0, 255]));
}

#[cfg(target_os = "macos")]
fn quadrant_image() -> crate::Image {
    crate::Image::from_rgba(
        2,
        2,
        vec![
            255, 0, 0, 255, // top-left red
            0, 255, 0, 255, // top-right green
            0, 0, 255, 255, // bottom-left blue
            255, 255, 255, 255, // bottom-right white
        ],
    )
    .unwrap()
}

#[cfg(target_os = "macos")]
#[track_caller]
fn assert_pixel_near(snapshot: &crate::VisualSnapshot, x: u32, y: u32, expected: [u8; 4]) {
    let actual = snapshot
        .pixel(x, y)
        .expect("the pixel is inside the capture");
    for channel in 0..4 {
        assert!(
            actual[channel].abs_diff(expected[channel]) <= 8,
            "pixel ({x}, {y}) is {actual:?}, expected about {expected:?}"
        );
    }
}

#[cfg(target_os = "macos")]
struct BackgroundImageVisualView;

#[cfg(target_os = "macos")]
impl View for BackgroundImageVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let image = quadrant_image();
        div()
            .size_full()
            .flex_row()
            .bg(Color::BLACK)
            .child(div().id("bg-scaled").w(17.0).h(17.0).flex_none().bg_image(
                image.clone(),
                crate::BackgroundSize::Fixed(17.0, 17.0),
                crate::BackgroundRepeat::NoRepeat,
                crate::BackgroundPosition::TOP_LEFT,
            ))
            .child(div().id("bg-tiled").w(16.0).h(16.0).flex_none().bg_image(
                image,
                crate::BackgroundSize::Fixed(3.0, 3.0),
                crate::BackgroundRepeat::Repeat,
                crate::BackgroundPosition::TOP_LEFT,
            ))
    }
}

#[cfg(target_os = "macos")]
#[test]
fn visual_background_images_scale_tile_and_stay_inside_the_element() {
    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(35.0, 18.0),
            BackgroundImageVisualView,
        )
        .unwrap();
    let mut visual = cx.visual(view.window_handle()).unwrap();
    visual
        .assert_element_bounds("bg-scaled", Rect::new(0.0, 0.0, 17.0, 17.0), 0.0)
        .unwrap();
    visual
        .assert_element_bounds("bg-tiled", Rect::new(17.0, 0.0, 16.0, 16.0), 0.0)
        .unwrap();
    let snapshot = visual.capture_screenshot().unwrap();
    assert_eq!((snapshot.width(), snapshot.height()), (70, 36));

    // One tile stretched over the whole element keeps every source texel in its own quadrant.
    assert_pixel_near(&snapshot, 8, 8, [255, 0, 0, 255]);
    assert_pixel_near(&snapshot, 25, 8, [0, 255, 0, 255]);
    assert_pixel_near(&snapshot, 8, 25, [0, 0, 255, 255]);
    assert_pixel_near(&snapshot, 25, 25, [255, 255, 255, 255]);

    // Tiling repeats the same texel grid across the element without leaving it.
    assert_pixel_near(&snapshot, 35, 1, [255, 0, 0, 255]);
    assert_pixel_near(&snapshot, 38, 1, [0, 255, 0, 255]);
    assert_pixel_near(&snapshot, 35, 4, [0, 0, 255, 255]);
    assert_pixel_near(&snapshot, 41, 1, [255, 0, 0, 255]);
    assert_pixel_near(&snapshot, 44, 4, [255, 255, 255, 255]);
    // Nothing is painted past the element's own box.
    assert_eq!(snapshot.pixel(68, 35), Some([0, 0, 0, 255]));
    assert_eq!(snapshot.pixel(35, 35), Some([0, 0, 0, 255]));
}

#[cfg(target_os = "macos")]
struct RoundedBackgroundImageVisualView;

#[cfg(target_os = "macos")]
impl View for RoundedBackgroundImageVisualView {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div().size_full().bg(Color::BLACK).child(
            div()
                .id("rounded-bg")
                .size_full()
                .rounded(8.0)
                .bg_image_cover(quadrant_image()),
        )
    }
}

#[cfg(target_os = "macos")]
#[test]
fn visual_background_images_respect_rounded_corners() {
    let (mut cx, view) = Application::new()
        .into_test_context(
            WindowOptions::default().size(16.0, 16.0),
            RoundedBackgroundImageVisualView,
        )
        .unwrap();
    let mut visual = cx.visual(view.window_handle()).unwrap();
    let snapshot = visual.capture_screenshot().unwrap();
    // The rounded corner masks the raster background back to the black parent.
    assert_eq!(snapshot.pixel(0, 0), Some([0, 0, 0, 255]));
    assert_eq!(snapshot.pixel(31, 31), Some([0, 0, 0, 255]));
    // A covering background still paints its source texels in the middle of the element.
    assert_pixel_near(&snapshot, 6, 6, [255, 0, 0, 255]);
    assert_pixel_near(&snapshot, 25, 25, [255, 255, 255, 255]);
}
