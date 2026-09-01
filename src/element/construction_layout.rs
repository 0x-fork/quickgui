use super::*;

impl Element {
    pub(super) fn container() -> Self {
        let default_text = TextStyle::default();
        Self {
            explicit_id: None,
            runtime_id: ElementId::new(0),
            kind: ElementKind::Container,
            layout: Style::default(),
            visibility: Visibility::Visible,
            visual: VisualStyle::default(),
            typography: TypographyStyle::default(),
            resolved_typography: default_text,
            hover: ElementStateStyle::default(),
            active: ElementStateStyle::default(),
            focus: ElementStateStyle::default(),
            disabled_style: ElementStateStyle::default(),
            invalid_style: ElementStateStyle::default(),
            dragging: ElementStateStyle::default(),
            drag_over: ElementStateStyle::default(),
            clickable: false,
            pointer_listener: false,
            scroll_wheel_listener: false,
            touch_listener: false,
            context_menu_listener: false,
            mouse_listeners: None,
            key_listeners: None,
            action_listeners: None,
            mouse_pressure_listener: false,
            pinch_listener: false,
            rotation_listener: false,
            smart_magnify_listener: false,
            drag_source: false,
            drop_target: false,
            drop_predicates: Vec::new(),
            cursor_style: None,
            cursor_style_explicit: false,
            user_select: UserSelect::Auto,
            resolved_user_select: false,
            focusable: false,
            focus_on_pointer: true,
            hit_slop: Insets::default(),
            focus_trap: false,
            restore_previous_focus: false,
            key_context: None,
            tab_index: 0,
            auto_focus: false,
            form: false,
            form_submitter: false,
            activation_target: None,
            accessibility: AccessibilityStyle::default(),
            tab_list_behavior: None,
            plane: None,
            z_index: None,
            portal: false,
            anchor: None,
            tooltip: None,
            app_region: None,
            virtual_scroll: None,
            scroll_to_end_revision: None,
            list_item_measurement: None,
            animation: None,
            spring: None,
            transition: None,
            blocks_pointer: false,
            dismiss_policy: DismissPolicy::default(),
            restore_focus: None,
            children: Vec::new(),
            taffy_node: None,
        }
    }

    pub(super) fn text(content: Arc<str>) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::Text(content);
        element.accessibility.role = AccessibilityRole::Label;
        element
    }

    pub(super) fn styled_text(content: StyledText) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::StyledText(content);
        element.accessibility.role = AccessibilityRole::Label;
        element
    }

    pub(super) fn image(source: ImageSource) -> Self {
        let mut element = Self::container();
        let resolved = if let Some(image) = source.image() {
            ImageResolution::Ready(image.clone())
        } else if let Some(animation) = source.animated() {
            ImageResolution::Animated(animation.clone())
        } else {
            ImageResolution::Loading
        };
        element.kind = ElementKind::Image(ImageElement {
            source,
            object_fit: ObjectFit::Contain,
            grayscale: false,
            resolved,
            loading: None,
            fallback: None,
        });
        element.accessibility.role = AccessibilityRole::Image;
        element
    }

    pub(super) fn svg(svg: Svg) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::Svg(SvgElement {
            svg,
            object_fit: ObjectFit::Contain,
            transform: SvgTransform::IDENTITY,
        });
        element.accessibility.role = AccessibilityRole::Image;
        element
    }

    pub(super) fn path(path: Path) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::Path(PathElement {
            path,
            object_fit: ObjectFit::Contain,
            background: None,
        });
        element.accessibility.role = AccessibilityRole::Image;
        element
    }

    pub(super) fn canvas(painter: impl for<'a> Fn(Rect, &mut Canvas<'a>) + 'static) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::Canvas(CanvasElement {
            painter: Rc::new(painter),
        });
        // Match the default dimensions of the web canvas element while still allowing normal
        // width/height utilities to override them.
        element.layout.size = TaffySize {
            width: Dimension::length(300.0),
            height: Dimension::length(150.0),
        };
        element
    }

    pub(super) fn custom_shader(shader: CustomShader) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::CustomShader(ShaderElement {
            shader,
            parameters: ShaderParameters::default(),
        });
        element.layout.size = TaffySize {
            width: Dimension::length(300.0),
            height: Dimension::length(150.0),
        };
        element
    }

    pub(super) fn text_input(
        value: Arc<str>,
        highlights: Arc<[TextHighlight]>,
        multiline: bool,
    ) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::TextInput(TextInputElement {
            value,
            highlights,
            placeholder: Arc::from(""),
            multiline,
            password: false,
            constraints: InputConstraints::default(),
        });
        element.layout.size = TaffySize {
            width: Dimension::length(if multiline { 320.0 } else { 240.0 }),
            height: Dimension::length(if multiline { 160.0 } else { 40.0 }),
        };
        element.layout.overflow = TaffyPoint {
            x: Overflow::Hidden,
            y: Overflow::Hidden,
        };
        element.visual.background = Some(Color::rgb8(28, 30, 35));
        element.visual.border_color = Some(Color::rgb8(70, 74, 85));
        element.visual.border_widths = Insets::all(1.0);
        element.visual.radius = 8.0;
        element.focus = ElementStateStyle::default().border(2.0, Color::rgb8(94, 234, 212));
        element.invalid_style =
            ElementStateStyle::default().border(2.0, Color::rgb8(248, 113, 113));
        element.focusable = true;
        element.set_implicit_cursor(CursorStyle::IBeam);
        element.accessibility.role = if multiline {
            AccessibilityRole::MultilineTextInput
        } else {
            AccessibilityRole::TextInput
        };
        element.typography.wrap = Some(if multiline {
            TextWrap::Word
        } else {
            TextWrap::None
        });
        element
    }

    #[cfg(target_os = "macos")]
    pub(super) fn native_view(view: MacNativeView) -> Self {
        let mut element = Self::container();
        element.kind = ElementKind::NativeView(view);
        element.layout.size = TaffySize {
            width: Dimension::length(320.0),
            height: Dimension::length(200.0),
        };
        element.blocks_pointer = true;
        element
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.bind_listener_id(id.into());
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        assert!(
            !matches!(self.kind, ElementKind::ContainerQuery(_)),
            "container_query contents must come from its size callback"
        );
        self.children.push(child.into_element());
        self
    }

    pub fn children<I, E>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: IntoElement,
    {
        assert!(
            !matches!(self.kind, ElementKind::ContainerQuery(_)),
            "container_query contents must come from its size callback"
        );
        self.children
            .extend(children.into_iter().map(IntoElement::into_element));
        self
    }

    pub fn when(self, condition: bool, apply: impl FnOnce(Self) -> Self) -> Self {
        if condition { apply(self) } else { self }
    }

    /// Lay out children with the CSS block algorithm.
    pub fn block(mut self) -> Self {
        self.layout.display = Display::Block;
        self
    }

    pub fn flex(mut self) -> Self {
        self.layout.display = Display::Flex;
        self
    }

    /// Lay out children with the CSS Grid algorithm.
    pub fn grid(mut self) -> Self {
        self.layout.display = Display::Grid;
        self
    }

    /// Remove this element and its complete subtree from layout and interaction.
    ///
    /// This matches GPUI and CSS `display: none`: descendants do not paint, receive input,
    /// participate in focus or accessibility, resolve container queries or image resources, or
    /// keep animation clocks active. Calling [`Element::block`], [`Element::flex`], or
    /// [`Element::grid`] later on the same declaration restores a display mode.
    pub fn hidden(mut self) -> Self {
        self.layout.display = Display::None;
        self
    }

    pub(crate) fn is_display_none(&self) -> bool {
        self.layout.display == Display::None
    }

    /// Paint this element subtree while retaining its existing display mode.
    pub fn visible(mut self) -> Self {
        self.visibility = Visibility::Visible;
        self
    }

    /// Suppress painting and interaction for this subtree without removing its layout box.
    pub fn invisible(mut self) -> Self {
        self.visibility = Visibility::Hidden;
        self
    }

    pub(crate) fn is_visibility_hidden(&self) -> bool {
        self.visibility == Visibility::Hidden
    }

    /// Set `count` equal `minmax(0, 1fr)` columns, matching GPUI's `grid_cols` helper.
    pub fn grid_cols(mut self, count: u16) -> Self {
        self.layout.grid_template_columns = equal_grid_tracks(count, EqualGridTrackSizing::Zero);
        self
    }

    /// Set equal columns with a `min-content` minimum.
    pub fn grid_cols_min_content(mut self, count: u16) -> Self {
        self.layout.grid_template_columns =
            equal_grid_tracks(count, EqualGridTrackSizing::MinContent);
        self
    }

    /// Set content-sized columns using `minmax(0, max-content)`.
    pub fn grid_cols_max_content(mut self, count: u16) -> Self {
        self.layout.grid_template_columns =
            equal_grid_tracks(count, EqualGridTrackSizing::MaxContent);
        self
    }

    /// Set `count` equal `minmax(0, 1fr)` rows, matching GPUI's `grid_rows` helper.
    pub fn grid_rows(mut self, count: u16) -> Self {
        self.layout.grid_template_rows = equal_grid_tracks(count, EqualGridTrackSizing::Zero);
        self
    }

    /// Set equal rows with a `min-content` minimum.
    pub fn grid_rows_min_content(mut self, count: u16) -> Self {
        self.layout.grid_template_rows = equal_grid_tracks(count, EqualGridTrackSizing::MinContent);
        self
    }

    /// Set content-sized rows using `minmax(0, max-content)`.
    pub fn grid_rows_max_content(mut self, count: u16) -> Self {
        self.layout.grid_template_rows = equal_grid_tracks(count, EqualGridTrackSizing::MaxContent);
        self
    }

    /// Set an explicit web-style column template. At most [`MAX_GRID_TRACKS`] entries are kept.
    pub fn grid_template_columns(mut self, tracks: impl IntoIterator<Item = GridTrack>) -> Self {
        self.layout.grid_template_columns = tracks
            .into_iter()
            .take(usize::from(MAX_GRID_TRACKS))
            .map(|track| GridTemplateComponent::Single(track.0))
            .collect();
        self
    }

    /// Set an explicit web-style row template. At most [`MAX_GRID_TRACKS`] entries are kept.
    pub fn grid_template_rows(mut self, tracks: impl IntoIterator<Item = GridTrack>) -> Self {
        self.layout.grid_template_rows = tracks
            .into_iter()
            .take(usize::from(MAX_GRID_TRACKS))
            .map(|track| GridTemplateComponent::Single(track.0))
            .collect();
        self
    }

    /// Auto-place items row by row.
    pub fn grid_flow_row(mut self) -> Self {
        self.layout.grid_auto_flow = GridAutoFlow::Row;
        self
    }

    /// Auto-place items column by column.
    pub fn grid_flow_col(mut self) -> Self {
        self.layout.grid_auto_flow = GridAutoFlow::Column;
        self
    }

    /// Densely backfill holes while auto-placing items row by row.
    pub fn grid_flow_row_dense(mut self) -> Self {
        self.layout.grid_auto_flow = GridAutoFlow::RowDense;
        self
    }

    /// Densely backfill holes while auto-placing items column by column.
    pub fn grid_flow_col_dense(mut self) -> Self {
        self.layout.grid_auto_flow = GridAutoFlow::ColumnDense;
        self
    }

    /// Start this grid item at a one-based CSS column line. Zero restores `auto`.
    pub fn col_start(mut self, start: i16) -> Self {
        self.layout.grid_column.start = bounded_grid_line(start);
        self
    }

    pub fn col_start_auto(mut self) -> Self {
        self.layout.grid_column.start = GridPlacement::Auto;
        self
    }

    /// End this grid item at a one-based CSS column line. Zero restores `auto`.
    pub fn col_end(mut self, end: i16) -> Self {
        self.layout.grid_column.end = bounded_grid_line(end);
        self
    }

    pub fn col_end_auto(mut self) -> Self {
        self.layout.grid_column.end = GridPlacement::Auto;
        self
    }

    /// Span this item across a bounded number of columns. Zero is treated as one.
    pub fn col_span(mut self, span: u16) -> Self {
        let span = bounded_grid_span(span);
        self.layout.grid_column = TaffyLine {
            start: span.clone(),
            end: span,
        };
        self
    }

    /// Span from the first to the final explicit column line.
    pub fn col_span_full(mut self) -> Self {
        self.layout.grid_column = TaffyLine {
            start: bounded_grid_line(1),
            end: bounded_grid_line(-1),
        };
        self
    }

    /// Start this grid item at a one-based CSS row line. Zero restores `auto`.
    pub fn row_start(mut self, start: i16) -> Self {
        self.layout.grid_row.start = bounded_grid_line(start);
        self
    }

    pub fn row_start_auto(mut self) -> Self {
        self.layout.grid_row.start = GridPlacement::Auto;
        self
    }

    /// End this grid item at a one-based CSS row line. Zero restores `auto`.
    pub fn row_end(mut self, end: i16) -> Self {
        self.layout.grid_row.end = bounded_grid_line(end);
        self
    }

    pub fn row_end_auto(mut self) -> Self {
        self.layout.grid_row.end = GridPlacement::Auto;
        self
    }

    /// Span this item across a bounded number of rows. Zero is treated as one.
    pub fn row_span(mut self, span: u16) -> Self {
        let span = bounded_grid_span(span);
        self.layout.grid_row = TaffyLine {
            start: span.clone(),
            end: span,
        };
        self
    }

    /// Span from the first to the final explicit row line.
    pub fn row_span_full(mut self) -> Self {
        self.layout.grid_row = TaffyLine {
            start: bounded_grid_line(1),
            end: bounded_grid_line(-1),
        };
        self
    }

    pub fn flex_row(mut self) -> Self {
        self.layout.display = Display::Flex;
        self.layout.flex_direction = FlexDirection::Row;
        self
    }

    pub fn flex_row_reverse(mut self) -> Self {
        self.layout.display = Display::Flex;
        self.layout.flex_direction = FlexDirection::RowReverse;
        self
    }

    pub fn flex_col(mut self) -> Self {
        self.layout.display = Display::Flex;
        self.layout.flex_direction = FlexDirection::Column;
        self
    }

    pub fn flex_col_reverse(mut self) -> Self {
        self.layout.display = Display::Flex;
        self.layout.flex_direction = FlexDirection::ColumnReverse;
        self
    }

    pub fn flex_wrap(mut self) -> Self {
        self.layout.flex_wrap = FlexWrap::Wrap;
        self
    }

    pub fn flex_wrap_reverse(mut self) -> Self {
        self.layout.flex_wrap = FlexWrap::WrapReverse;
        self
    }

    pub fn flex_nowrap(mut self) -> Self {
        self.layout.flex_wrap = FlexWrap::NoWrap;
        self
    }

    pub fn flex_1(mut self) -> Self {
        self.layout.flex_grow = 1.0;
        self.layout.flex_shrink = 1.0;
        self.layout.flex_basis = Dimension::length(0.0);
        self
    }

    pub fn flex_auto(mut self) -> Self {
        self.layout.flex_grow = 1.0;
        self.layout.flex_shrink = 1.0;
        self.layout.flex_basis = Dimension::auto();
        self
    }

    pub fn flex_initial(mut self) -> Self {
        self.layout.flex_grow = 0.0;
        self.layout.flex_shrink = 1.0;
        self.layout.flex_basis = Dimension::auto();
        self
    }

    pub fn flex_none(mut self) -> Self {
        self.layout.flex_grow = 0.0;
        self.layout.flex_shrink = 0.0;
        self.layout.flex_basis = Dimension::auto();
        self
    }

    /// Set an absolute logical-pixel flex basis.
    pub fn flex_basis(mut self, basis: f32) -> Self {
        self.layout.flex_basis = Dimension::length(finite_nonnegative(basis));
        self
    }

    pub fn flex_basis_auto(mut self) -> Self {
        self.layout.flex_basis = Dimension::auto();
        self
    }

    pub fn flex_grow(mut self, grow: f32) -> Self {
        self.layout.flex_grow = finite_nonnegative(grow);
        self
    }

    pub fn flex_grow_0(self) -> Self {
        self.flex_grow(0.0)
    }

    pub fn flex_grow_1(self) -> Self {
        self.flex_grow(1.0)
    }

    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.layout.flex_shrink = finite_nonnegative(shrink);
        self
    }

    pub fn flex_shrink_0(self) -> Self {
        self.flex_shrink(0.0)
    }

    pub fn flex_shrink_1(self) -> Self {
        self.flex_shrink(1.0)
    }

    pub fn items_start(mut self) -> Self {
        self.layout.align_items = Some(AlignItems::FLEX_START);
        self
    }

    pub fn items_center(mut self) -> Self {
        self.layout.align_items = Some(AlignItems::CENTER);
        self
    }

    pub fn items_end(mut self) -> Self {
        self.layout.align_items = Some(AlignItems::FLEX_END);
        self
    }

    pub fn items_baseline(mut self) -> Self {
        self.layout.align_items = Some(AlignItems::BASELINE);
        self
    }

    pub fn items_stretch(mut self) -> Self {
        self.layout.align_items = Some(AlignItems::STRETCH);
        self
    }

    pub fn self_start(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::START);
        self
    }

    pub fn self_end(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::END);
        self
    }

    pub fn self_flex_start(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::FLEX_START);
        self
    }

    pub fn self_flex_end(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::FLEX_END);
        self
    }

    pub fn self_center(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::CENTER);
        self
    }

    pub fn self_baseline(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::BASELINE);
        self
    }

    pub fn self_stretch(mut self) -> Self {
        self.layout.align_self = Some(AlignSelf::STRETCH);
        self
    }

    pub fn justify_start(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::START);
        self
    }

    pub fn justify_center(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::CENTER);
        self
    }

    pub fn justify_end(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::END);
        self
    }

    pub fn justify_between(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::SPACE_BETWEEN);
        self
    }

    pub fn justify_around(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::SPACE_AROUND);
        self
    }

    pub fn justify_evenly(mut self) -> Self {
        self.layout.justify_content = Some(JustifyContent::SPACE_EVENLY);
        self
    }

    pub fn content_normal(mut self) -> Self {
        self.layout.align_content = None;
        self
    }

    pub fn content_center(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::CENTER);
        self
    }

    pub fn content_start(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::FLEX_START);
        self
    }

    pub fn content_end(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::FLEX_END);
        self
    }

    pub fn content_between(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::SPACE_BETWEEN);
        self
    }

    pub fn content_around(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::SPACE_AROUND);
        self
    }

    pub fn content_evenly(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::SPACE_EVENLY);
        self
    }

    pub fn content_stretch(mut self) -> Self {
        self.layout.align_content = Some(AlignContent::STRETCH);
        self
    }

    /// Preserve `width / height` when exactly one axis is definite.
    pub fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.layout.aspect_ratio = (ratio.is_finite() && ratio > 0.0).then_some(ratio);
        self
    }

    pub fn aspect_square(self) -> Self {
        self.aspect_ratio(1.0)
    }

    pub fn gap(mut self, value: f32) -> Self {
        let value = finite_nonnegative(value);
        self.layout.gap = TaffySize {
            width: LengthPercentage::length(value),
            height: LengthPercentage::length(value),
        };
        self
    }

    pub fn gap_x(mut self, value: f32) -> Self {
        self.layout.gap.width = LengthPercentage::length(finite_nonnegative(value));
        self
    }

    pub fn gap_y(mut self, value: f32) -> Self {
        self.layout.gap.height = LengthPercentage::length(finite_nonnegative(value));
        self
    }

    pub fn gap_1(self) -> Self {
        self.gap(SPACING_UNIT)
    }
    pub fn gap_2(self) -> Self {
        self.gap(SPACING_UNIT * 2.0)
    }
    pub fn gap_3(self) -> Self {
        self.gap(SPACING_UNIT * 3.0)
    }
    pub fn gap_4(self) -> Self {
        self.gap(SPACING_UNIT * 4.0)
    }
    pub fn gap_5(self) -> Self {
        self.gap(SPACING_UNIT * 5.0)
    }
    pub fn gap_6(self) -> Self {
        self.gap(SPACING_UNIT * 6.0)
    }

    spacing_scale_methods!(gap_x;
        gap_x_0 => 0.0,
        gap_x_1 => 1.0,
        gap_x_2 => 2.0,
        gap_x_3 => 3.0,
        gap_x_4 => 4.0,
        gap_x_5 => 5.0,
        gap_x_6 => 6.0,
    );
    spacing_scale_methods!(gap_y;
        gap_y_0 => 0.0,
        gap_y_1 => 1.0,
        gap_y_2 => 2.0,
        gap_y_3 => 3.0,
        gap_y_4 => 4.0,
        gap_y_5 => 5.0,
        gap_y_6 => 6.0,
    );

    /// Set all four margins in logical pixels. Finite negative margins are supported.
    pub fn m(self, value: f32) -> Self {
        self.margin(value, value, value, value)
    }

    pub fn mx(mut self, value: f32) -> Self {
        let value = LengthPercentageAuto::length(finite_length(value));
        self.layout.margin.left = value;
        self.layout.margin.right = value;
        self
    }

    pub fn my(mut self, value: f32) -> Self {
        let value = LengthPercentageAuto::length(finite_length(value));
        self.layout.margin.top = value;
        self.layout.margin.bottom = value;
        self
    }

    pub fn mt(mut self, value: f32) -> Self {
        self.layout.margin.top = LengthPercentageAuto::length(finite_length(value));
        self
    }

    pub fn mr(mut self, value: f32) -> Self {
        self.layout.margin.right = LengthPercentageAuto::length(finite_length(value));
        self
    }

    pub fn mb(mut self, value: f32) -> Self {
        self.layout.margin.bottom = LengthPercentageAuto::length(finite_length(value));
        self
    }

    pub fn ml(mut self, value: f32) -> Self {
        self.layout.margin.left = LengthPercentageAuto::length(finite_length(value));
        self
    }

    /// Set margins in CSS order: top, right, bottom, left.
    pub fn margin(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.layout.margin = TaffyRect {
            left: LengthPercentageAuto::length(finite_length(left)),
            right: LengthPercentageAuto::length(finite_length(right)),
            top: LengthPercentageAuto::length(finite_length(top)),
            bottom: LengthPercentageAuto::length(finite_length(bottom)),
        };
        self
    }

    pub fn m_auto(mut self) -> Self {
        self.layout.margin = TaffyRect::auto();
        self
    }

    pub fn mx_auto(mut self) -> Self {
        self.layout.margin.left = LengthPercentageAuto::auto();
        self.layout.margin.right = LengthPercentageAuto::auto();
        self
    }

    pub fn my_auto(mut self) -> Self {
        self.layout.margin.top = LengthPercentageAuto::auto();
        self.layout.margin.bottom = LengthPercentageAuto::auto();
        self
    }

    pub fn mt_auto(mut self) -> Self {
        self.layout.margin.top = LengthPercentageAuto::auto();
        self
    }

    pub fn mr_auto(mut self) -> Self {
        self.layout.margin.right = LengthPercentageAuto::auto();
        self
    }

    pub fn mb_auto(mut self) -> Self {
        self.layout.margin.bottom = LengthPercentageAuto::auto();
        self
    }

    pub fn ml_auto(mut self) -> Self {
        self.layout.margin.left = LengthPercentageAuto::auto();
        self
    }

    spacing_scale_methods!(m;
        m_0 => 0.0, m_1 => 1.0, m_2 => 2.0, m_3 => 3.0, m_4 => 4.0, m_5 => 5.0,
        m_6 => 6.0, m_8 => 8.0, m_10 => 10.0, m_12 => 12.0, m_16 => 16.0,
        m_20 => 20.0, m_24 => 24.0, m_32 => 32.0,
    );
    spacing_scale_methods!(mx;
        mx_0 => 0.0, mx_1 => 1.0, mx_2 => 2.0, mx_3 => 3.0, mx_4 => 4.0, mx_5 => 5.0,
        mx_6 => 6.0, mx_8 => 8.0, mx_10 => 10.0, mx_12 => 12.0, mx_16 => 16.0,
        mx_20 => 20.0, mx_24 => 24.0, mx_32 => 32.0,
    );
    spacing_scale_methods!(my;
        my_0 => 0.0, my_1 => 1.0, my_2 => 2.0, my_3 => 3.0, my_4 => 4.0, my_5 => 5.0,
        my_6 => 6.0, my_8 => 8.0, my_10 => 10.0, my_12 => 12.0, my_16 => 16.0,
        my_20 => 20.0, my_24 => 24.0, my_32 => 32.0,
    );
    spacing_scale_methods!(mt;
        mt_0 => 0.0, mt_1 => 1.0, mt_2 => 2.0, mt_3 => 3.0, mt_4 => 4.0, mt_5 => 5.0,
        mt_6 => 6.0, mt_8 => 8.0, mt_10 => 10.0, mt_12 => 12.0, mt_16 => 16.0,
        mt_20 => 20.0, mt_24 => 24.0, mt_32 => 32.0,
    );
    spacing_scale_methods!(mr;
        mr_0 => 0.0, mr_1 => 1.0, mr_2 => 2.0, mr_3 => 3.0, mr_4 => 4.0, mr_5 => 5.0,
        mr_6 => 6.0, mr_8 => 8.0, mr_10 => 10.0, mr_12 => 12.0, mr_16 => 16.0,
        mr_20 => 20.0, mr_24 => 24.0, mr_32 => 32.0,
    );
    spacing_scale_methods!(mb;
        mb_0 => 0.0, mb_1 => 1.0, mb_2 => 2.0, mb_3 => 3.0, mb_4 => 4.0, mb_5 => 5.0,
        mb_6 => 6.0, mb_8 => 8.0, mb_10 => 10.0, mb_12 => 12.0, mb_16 => 16.0,
        mb_20 => 20.0, mb_24 => 24.0, mb_32 => 32.0,
    );
    spacing_scale_methods!(ml;
        ml_0 => 0.0, ml_1 => 1.0, ml_2 => 2.0, ml_3 => 3.0, ml_4 => 4.0, ml_5 => 5.0,
        ml_6 => 6.0, ml_8 => 8.0, ml_10 => 10.0, ml_12 => 12.0, ml_16 => 16.0,
        ml_20 => 20.0, ml_24 => 24.0, ml_32 => 32.0,
    );

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.layout.size = TaffySize {
            width: Dimension::length(width),
            height: Dimension::length(height),
        };
        self
    }

    pub fn size_full(mut self) -> Self {
        self.layout.size = TaffySize {
            width: Dimension::percent(1.0),
            height: Dimension::percent(1.0),
        };
        self
    }

    pub fn w(mut self, width: f32) -> Self {
        self.layout.size.width = Dimension::length(width);
        self
    }

    pub fn h(mut self, height: f32) -> Self {
        self.layout.size.height = Dimension::length(height);
        self
    }

    pub fn w_full(mut self) -> Self {
        self.layout.size.width = Dimension::percent(1.0);
        self
    }

    pub fn h_full(mut self) -> Self {
        self.layout.size.height = Dimension::percent(1.0);
        self
    }

    pub fn min_w(mut self, width: f32) -> Self {
        self.layout.min_size.width = Dimension::length(width);
        self
    }

    pub fn min_h(mut self, height: f32) -> Self {
        self.layout.min_size.height = Dimension::length(height);
        self
    }

    pub fn max_w(mut self, width: f32) -> Self {
        self.layout.max_size.width = Dimension::length(width);
        self
    }

    pub fn max_h(mut self, height: f32) -> Self {
        self.layout.max_size.height = Dimension::length(height);
        self
    }

    pub fn h_8(self) -> Self {
        self.h(SPACING_UNIT * 8.0)
    }
    pub fn h_10(self) -> Self {
        self.h(SPACING_UNIT * 10.0)
    }
    pub fn h_12(self) -> Self {
        self.h(SPACING_UNIT * 12.0)
    }

    pub fn p(self, value: f32) -> Self {
        self.padding(value, value, value, value)
    }

    pub fn px(mut self, value: f32) -> Self {
        self.layout.padding.left = LengthPercentage::length(value);
        self.layout.padding.right = LengthPercentage::length(value);
        self
    }

    pub fn py(mut self, value: f32) -> Self {
        self.layout.padding.top = LengthPercentage::length(value);
        self.layout.padding.bottom = LengthPercentage::length(value);
        self
    }

    pub fn p_1(self) -> Self {
        self.p(SPACING_UNIT)
    }
    pub fn p_2(self) -> Self {
        self.p(SPACING_UNIT * 2.0)
    }
    pub fn p_3(self) -> Self {
        self.p(SPACING_UNIT * 3.0)
    }
    pub fn p_4(self) -> Self {
        self.p(SPACING_UNIT * 4.0)
    }
    pub fn p_5(self) -> Self {
        self.p(SPACING_UNIT * 5.0)
    }
    pub fn p_6(self) -> Self {
        self.p(SPACING_UNIT * 6.0)
    }
    pub fn px_2(self) -> Self {
        self.px(SPACING_UNIT * 2.0)
    }
    pub fn px_3(self) -> Self {
        self.px(SPACING_UNIT * 3.0)
    }
    pub fn px_4(self) -> Self {
        self.px(SPACING_UNIT * 4.0)
    }
    pub fn py_1(self) -> Self {
        self.py(SPACING_UNIT)
    }
    pub fn py_2(self) -> Self {
        self.py(SPACING_UNIT * 2.0)
    }

    pub fn padding(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.layout.padding = TaffyRect {
            left: LengthPercentage::length(left),
            right: LengthPercentage::length(right),
            top: LengthPercentage::length(top),
            bottom: LengthPercentage::length(bottom),
        };
        self
    }
}
