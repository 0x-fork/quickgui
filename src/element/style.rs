use super::*;

impl Element {
    pub fn padding_axis(mut self, vertical: f32, horizontal: f32) -> Self {
        self.layout.padding = TaffyRect {
            left: LengthPercentage::length(horizontal),
            right: LengthPercentage::length(horizontal),
            top: LengthPercentage::length(vertical),
            bottom: LengthPercentage::length(vertical),
        };
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.visual.background = Some(color);
        self
    }

    /// Set the opacity of this element and all of its descendants.
    ///
    /// Opacity is paint-only: transparent elements keep their layout, pointer behavior, focus,
    /// and accessibility semantics. Nested opacity values multiply like GPUI and the web.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.visual.opacity = finite_opacity(opacity);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        let width = finite_nonnegative(width);
        self.visual.border_widths = Insets::all(width);
        self.visual.border_color = Some(color);
        self.layout.border = TaffyRect {
            left: LengthPercentage::length(width),
            right: LengthPercentage::length(width),
            top: LengthPercentage::length(width),
            bottom: LengthPercentage::length(width),
        };
        self
    }

    /// Set the widths of the inside border in top, right, bottom, left order.
    pub fn border_widths(mut self, widths: Insets) -> Self {
        let widths = Insets {
            top: finite_nonnegative(widths.top),
            right: finite_nonnegative(widths.right),
            bottom: finite_nonnegative(widths.bottom),
            left: finite_nonnegative(widths.left),
        };
        self.visual.border_widths = widths;
        self.layout.border = TaffyRect {
            left: LengthPercentage::length(widths.left),
            right: LengthPercentage::length(widths.right),
            top: LengthPercentage::length(widths.top),
            bottom: LengthPercentage::length(widths.bottom),
        };
        self
    }

    /// Set the shared color used by every non-zero border edge.
    pub fn border_color(mut self, color: Color) -> Self {
        self.visual.border_color = Some(color);
        self
    }

    pub fn border_top_width(mut self, width: f32) -> Self {
        let width = finite_nonnegative(width);
        self.visual.border_widths.top = width;
        self.layout.border.top = LengthPercentage::length(width);
        self
    }

    pub fn border_right_width(mut self, width: f32) -> Self {
        let width = finite_nonnegative(width);
        self.visual.border_widths.right = width;
        self.layout.border.right = LengthPercentage::length(width);
        self
    }

    pub fn border_bottom_width(mut self, width: f32) -> Self {
        let width = finite_nonnegative(width);
        self.visual.border_widths.bottom = width;
        self.layout.border.bottom = LengthPercentage::length(width);
        self
    }

    pub fn border_left_width(mut self, width: f32) -> Self {
        let width = finite_nonnegative(width);
        self.visual.border_widths.left = width;
        self.layout.border.left = LengthPercentage::length(width);
        self
    }

    pub fn border_top(self, width: f32, color: Color) -> Self {
        self.border_top_width(width).border_color(color)
    }

    pub fn border_right(self, width: f32, color: Color) -> Self {
        self.border_right_width(width).border_color(color)
    }

    pub fn border_bottom(self, width: f32, color: Color) -> Self {
        self.border_bottom_width(width).border_color(color)
    }

    pub fn border_left(self, width: f32, color: Color) -> Self {
        self.border_left_width(width).border_color(color)
    }

    pub fn rounded(mut self, radius: f32) -> Self {
        self.visual.radius = radius.max(0.0);
        self
    }

    pub fn rounded_sm(self) -> Self {
        self.rounded(4.0)
    }
    pub fn rounded_md(self) -> Self {
        self.rounded(6.0)
    }
    pub fn rounded_lg(self) -> Self {
        self.rounded(8.0)
    }
    pub fn rounded_xl(self) -> Self {
        self.rounded(12.0)
    }
    pub fn rounded_2xl(self) -> Self {
        self.rounded(16.0)
    }

    /// Paint one CSS-like box shadow without affecting layout.
    pub fn shadow(mut self, shadow: BoxShadow) -> Self {
        self.visual.shadows = Some(Arc::from([shadow]));
        self
    }

    /// Paint multiple CSS-like box shadows in declaration order.
    ///
    /// As on the web, the first shadow is painted on top of later shadows.
    pub fn shadows(mut self, shadows: impl IntoIterator<Item = BoxShadow>) -> Self {
        self.visual.shadows = Some(collect_box_shadows(shadows));
        self
    }

    pub fn shadow_none(mut self) -> Self {
        self.visual.shadows = None;
        self
    }

    pub fn shadow_sm(self) -> Self {
        self.shadow(shadow_sm_preset())
    }

    pub fn shadow_md(self) -> Self {
        self.shadows(shadow_md_preset())
    }

    pub fn shadow_lg(self) -> Self {
        self.shadows(shadow_lg_preset())
    }

    pub fn shadow_xl(self) -> Self {
        self.shadows(shadow_xl_preset())
    }

    pub fn shadow_2xl(self) -> Self {
        self.shadow(shadow_2xl_preset())
    }

    /// Choose how image, SVG, or path content is fitted into its layout box.
    pub fn object_fit(mut self, fit: ObjectFit) -> Self {
        match &mut self.kind {
            ElementKind::Image(image) => image.object_fit = fit,
            ElementKind::Svg(svg) => svg.object_fit = fit,
            ElementKind::Path(path) => path.object_fit = fit,
            _ => {}
        }
        self
    }

    /// Override a path element's inherited text color with a solid color or linear gradient.
    pub fn path_background(mut self, background: impl Into<Background>) -> Self {
        if let ElementKind::Path(path) = &mut self.kind {
            path.background = Some(background.into());
        }
        self
    }

    /// Replace the four parameter vectors supplied to this custom shader instance.
    pub fn shader_parameters(mut self, parameters: impl Into<ShaderParameters>) -> Self {
        let ElementKind::CustomShader(shader) = &mut self.kind else {
            panic!("shader_parameters can only be applied to a custom shader element");
        };
        shader.parameters = parameters.into();
        self
    }

    /// Apply a render-only transform to SVG content without affecting flexbox layout.
    pub fn svg_transform(mut self, transform: SvgTransform) -> Self {
        if let ElementKind::Svg(svg) = &mut self.kind {
            svg.transform = transform;
        }
        self
    }

    /// Render an image in grayscale without creating another decoded image.
    pub fn grayscale(mut self, grayscale: bool) -> Self {
        if let ElementKind::Image(image) = &mut self.kind {
            image.grayscale = grayscale;
        }
        self
    }

    /// Render a replacement element when an image resource has been loading for 200 ms.
    pub fn with_loading<E: IntoElement + 'static>(
        mut self,
        render: impl Fn() -> E + 'static,
    ) -> Self {
        if let ElementKind::Image(image) = &mut self.kind {
            image.loading = Some(ImageReplacement::new(render));
        }
        self
    }

    /// Render a replacement element when an image resource fails to load.
    pub fn with_fallback<E: IntoElement + 'static>(
        mut self,
        render: impl Fn() -> E + 'static,
    ) -> Self {
        if let ElementKind::Image(image) = &mut self.kind {
            image.fallback = Some(ImageReplacement::new(render));
        }
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.typography.color = Some(color);
        self
    }

    pub fn text_size(mut self, size: f32) -> Self {
        self.typography.font_size = Some(size.max(1.0));
        self
    }

    pub fn line_height(mut self, height: f32) -> Self {
        self.typography.line_height = Some(height.max(1.0));
        self
    }

    /// Quantize glyph advances to one logical monospace cell width.
    pub fn monospace_width(mut self, width: f32) -> Self {
        self.typography.monospace_width = Some(width.max(1.0));
        self
    }

    pub fn text_xs(self) -> Self {
        self.text_size(12.0).line_height(16.0)
    }
    pub fn text_sm(self) -> Self {
        self.text_size(14.0).line_height(20.0)
    }
    pub fn text_base(self) -> Self {
        self.text_size(16.0).line_height(24.0)
    }
    pub fn text_lg(self) -> Self {
        self.text_size(18.0).line_height(26.0)
    }
    pub fn text_xl(self) -> Self {
        self.text_size(20.0).line_height(28.0)
    }
    pub fn text_2xl(self) -> Self {
        self.text_size(24.0).line_height(32.0)
    }
    pub fn text_3xl(self) -> Self {
        self.text_size(30.0).line_height(36.0)
    }

    pub fn font_weight(mut self, weight: Weight) -> Self {
        self.typography.weight = Some(weight);
        self
    }

    pub fn font_normal(self) -> Self {
        self.font_weight(Weight::NORMAL)
    }

    pub fn font_medium(self) -> Self {
        self.font_weight(Weight::MEDIUM)
    }

    pub fn font_semibold(self) -> Self {
        self.font_weight(Weight::SEMIBOLD)
    }

    pub fn font_bold(self) -> Self {
        self.font_weight(Weight::BOLD)
    }

    /// Set the inherited font slant.
    pub fn font_style(mut self, style: GlyphStyle) -> Self {
        self.typography.font_style = Some(style);
        self
    }

    pub fn italic(self) -> Self {
        self.font_style(GlyphStyle::Italic)
    }

    pub fn not_italic(self) -> Self {
        self.font_style(GlyphStyle::Normal)
    }

    /// Optically thicken descendant glyph stems without selecting a heavier font face.
    pub fn font_thicken(mut self, thicken: bool) -> Self {
        self.typography.font_thicken = Some(thicken);
        self
    }

    /// Underline descendant text with the font's native single-line metrics.
    pub fn underline(mut self) -> Self {
        self.typography.underline = Some(TextUnderline::Single);
        self.typography.underline_wavy = Some(false);
        self.typography.underline_thickness = Some(1.0);
        self
    }

    /// Underline descendant text with two native lines.
    pub fn double_underline(mut self) -> Self {
        self.typography.underline = Some(TextUnderline::Double);
        self.typography.underline_wavy = Some(false);
        self.typography.underline_thickness = Some(1.0);
        self
    }

    /// Strike through descendant text with the font's native metrics.
    pub fn line_through(mut self) -> Self {
        self.typography.strikethrough = Some(true);
        self
    }

    /// Remove inherited underline and strikethrough decoration.
    pub fn text_decoration_none(mut self) -> Self {
        self.typography.underline = Some(TextUnderline::None);
        self.typography.underline_color = Some(None);
        self.typography.underline_wavy = Some(false);
        self.typography.underline_thickness = Some(1.0);
        self.typography.strikethrough = Some(false);
        self.typography.strikethrough_color = Some(None);
        self
    }

    /// Set the inherited underline color, enabling a single underline when needed.
    pub fn text_decoration_color(mut self, color: Color) -> Self {
        if self
            .typography
            .underline
            .is_none_or(|underline| underline == TextUnderline::None)
        {
            self.typography.underline = Some(TextUnderline::Single);
            self.typography.underline_wavy = Some(false);
            self.typography.underline_thickness = Some(1.0);
        }
        self.typography.underline_color = Some(Some(color));
        self
    }

    /// Use an ordinary solid underline, enabling one when no underline is inherited.
    pub fn text_decoration_solid(mut self) -> Self {
        let enables_underline = self
            .typography
            .underline
            .is_none_or(|underline| underline == TextUnderline::None);
        if enables_underline {
            self.typography.underline = Some(TextUnderline::Single);
            self.typography.underline_thickness = Some(1.0);
        }
        self.typography.underline_wavy = Some(false);
        self
    }

    /// Use a GPU-rendered spell-checker-style wavy underline.
    pub fn text_decoration_wavy(mut self) -> Self {
        let enables_underline = self
            .typography
            .underline
            .is_none_or(|underline| underline == TextUnderline::None);
        if enables_underline {
            self.typography.underline = Some(TextUnderline::Single);
            self.typography.underline_thickness = Some(1.0);
        }
        self.typography.underline_wavy = Some(true);
        self
    }

    fn text_decoration_thickness(mut self, thickness: f32) -> Self {
        if self
            .typography
            .underline
            .is_none_or(|underline| underline == TextUnderline::None)
        {
            self.typography.underline = Some(TextUnderline::Single);
        }
        self.typography.underline_thickness = Some(thickness);
        self
    }

    pub fn text_decoration_0(self) -> Self {
        self.text_decoration_thickness(0.0)
    }

    pub fn text_decoration_1(self) -> Self {
        self.text_decoration_thickness(1.0)
    }

    pub fn text_decoration_2(self) -> Self {
        self.text_decoration_thickness(2.0)
    }

    pub fn text_decoration_4(self) -> Self {
        self.text_decoration_thickness(4.0)
    }

    pub fn text_decoration_8(self) -> Self {
        self.text_decoration_thickness(8.0)
    }

    pub fn font_family(mut self, family: impl Into<FontFamily>) -> Self {
        let family = family.into();
        assert_valid_font_family(&family);
        self.typography.family = Some(family);
        self
    }

    /// Set the inherited OpenType feature table used while shaping descendant text.
    pub fn font_features(mut self, features: FontFeatures) -> Self {
        self.typography.features = Some(features);
        self
    }

    /// Set the ordered families tried after the primary family and before platform fallbacks.
    ///
    /// Passing an empty stack intentionally clears an inherited custom fallback stack.
    pub fn font_fallbacks(mut self, fallbacks: FontFallbacks) -> Self {
        self.typography.fallbacks = Some((!fallbacks.is_empty()).then_some(fallbacks));
        self
    }

    /// Replace the complete inherited font configuration.
    pub fn font(mut self, font: Font) -> Self {
        assert_valid_font_family(&font.family);
        self.typography.family = Some(font.family);
        self.typography.features = Some(font.features);
        self.typography.fallbacks = Some(normalize_fallbacks(font.fallbacks));
        self.typography.weight = Some(font.weight);
        self.typography.font_style = Some(font.style);
        self
    }

    /// Align descendant text lines within their assigned element width.
    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.typography.align = Some(align);
        self
    }

    pub fn text_left(self) -> Self {
        self.text_align(TextAlign::Left)
    }

    pub fn text_center(self) -> Self {
        self.text_align(TextAlign::Center)
    }

    pub fn text_right(self) -> Self {
        self.text_align(TextAlign::Right)
    }

    pub fn text_justify(self) -> Self {
        self.text_align(TextAlign::Justify)
    }

    pub fn no_wrap(mut self) -> Self {
        self.typography.wrap = Some(TextWrap::None);
        self
    }

    /// Set inherited CSS-like whitespace behavior.
    pub fn white_space(mut self, white_space: WhiteSpace) -> Self {
        self.typography.wrap = Some(white_space.into());
        self
    }

    /// Allow ordinary word wrapping, matching GPUI and `white-space: normal` on the web.
    pub fn whitespace_normal(self) -> Self {
        self.white_space(WhiteSpace::Normal)
    }

    /// Keep each logical line unwrapped, matching GPUI and `white-space: nowrap` on the web.
    pub fn whitespace_nowrap(self) -> Self {
        self.white_space(WhiteSpace::Nowrap)
    }

    /// Set the replacement used when text exceeds its assigned width.
    pub fn text_overflow(mut self, overflow: TextOverflow) -> Self {
        self.typography.text_overflow = Some(overflow);
        self
    }

    /// Preserve the start of overflowing text and append an ellipsis.
    pub fn text_ellipsis(self) -> Self {
        self.text_overflow(TextOverflow::ellipsis())
    }

    /// Preserve the end of overflowing text and prepend an ellipsis.
    pub fn text_ellipsis_start(self) -> Self {
        self.text_overflow(TextOverflow::ellipsis_start())
    }

    /// Preserve both ends of overflowing text and place an ellipsis in the middle.
    pub fn text_ellipsis_middle(self) -> Self {
        self.text_overflow(TextOverflow::ellipsis_middle())
    }

    /// Limit descendant text leaves to a fixed number of visible lines.
    ///
    /// Combine this with [`Self::text_ellipsis`] when the final visible line should carry an
    /// ellipsis. Values below one are normalized to one.
    pub fn line_clamp(mut self, lines: usize) -> Self {
        self.typography.line_clamp = Some(lines.max(1));
        self.overflow_hidden()
    }

    /// Web-style single-line truncation: no wrapping, hidden overflow, and a trailing ellipsis.
    pub fn truncate(self) -> Self {
        self.overflow_hidden().whitespace_nowrap().text_ellipsis()
    }

    /// Use cheap one-glyph-per-character shaping for app-controlled text and fonts.
    ///
    /// Keep the default advanced mode for complex scripts, ligatures, or general font fallback.
    pub fn text_shaping_basic(mut self) -> Self {
        self.typography.shaping = Some(TextShaping::Basic);
        self
    }

    pub fn text_shaping(mut self, shaping: TextShaping) -> Self {
        self.typography.shaping = Some(shaping);
        self
    }

    pub fn wrap(mut self) -> Self {
        self.typography.wrap = Some(TextWrap::Word);
        self
    }

    pub fn overflow_hidden(mut self) -> Self {
        self.layout.overflow = TaffyPoint {
            x: Overflow::Hidden,
            y: Overflow::Hidden,
        };
        self
    }

    pub fn overflow_y_scroll(mut self) -> Self {
        self.layout.overflow = TaffyPoint {
            x: Overflow::Hidden,
            y: Overflow::Scroll,
        };
        self
    }

    /// Follow the end of an ordinary vertical overflow container when `revision` changes.
    ///
    /// The first declaration starts at the end. Later revisions keep following only while the
    /// user was already at the previous end; scrolling away pauses following, and returning to
    /// the end resumes it on the next revision. This performs no scheduling or per-frame work.
    pub fn scroll_to_end(mut self, revision: u64) -> Self {
        self.scroll_to_end_revision = Some(revision);
        self
    }

    /// Bind this clipped viewport to a fixed-height [`VirtualList`].
    ///
    /// The mounted rows remain application-controlled, while QuickGUI owns the native-style
    /// retained scrollbar, wheel routing, pointer capture, hover expansion, and one-shot
    /// autohide. Offset changes rebuild the virtualized view only when the offset actually moves;
    /// hover and visibility changes stay paint-only.
    pub fn virtual_scroll(mut self, list: &VirtualList) -> Self {
        self.layout.overflow.y = Overflow::Hidden;
        self.virtual_scroll = Some(VirtualScrollStyle {
            handle: list.scroll_handle(),
            max_offset_y: list.max_scroll_offset(),
            measurement_revision: 0,
            mount: list.scroll_mount(),
        });
        self
    }

    /// Bind this clipped viewport to a differently sized [`ListState`].
    ///
    /// Use [`ListState::visible_rows`] and [`ListState::render_rows`] to mount a bounded normal-flow
    /// slice. QuickGUI measures those rows after Taffy layout, preserves the logical top item while
    /// estimates converge, and schedules only the correcting rebuilds that actually changed a
    /// measurement. Wheel input, scrollbar capture, hover expansion, and autohide use the same
    /// retained path as ordinary scrolling and fixed-height virtualization.
    pub fn variable_virtual_scroll(mut self, list: &ListState) -> Self {
        self.layout.overflow.y = Overflow::Hidden;
        let (handle, max_offset_y, measurement_revision, mount) = list.scroll_binding();
        self.virtual_scroll = Some(VirtualScrollStyle {
            handle,
            max_offset_y,
            measurement_revision,
            mount,
        });
        self
    }

    pub fn absolute(mut self) -> Self {
        self.layout.position = Position::Absolute;
        self
    }

    /// Create a stacking context within the current render plane.
    ///
    /// Values are relative to the nearest ancestor stacking context. Elements sharing a value stay
    /// batched and retain source order.
    pub fn z_index(mut self, value: i16) -> Self {
        self.z_index = Some(value);
        self
    }

    /// Paint this subtree in the viewport overlay plane above ordinary application content.
    ///
    /// The element becomes absolutely positioned, escapes ancestor clipping, and blocks pointer
    /// events from falling through its own bounds.
    pub fn overlay(mut self) -> Self {
        self.layout.position = Position::Absolute;
        self.plane = Some(ScenePlane::Overlay);
        self.portal = true;
        self.blocks_pointer = true;
        self
    }

    /// Position this floating element relative to a stable element ID.
    pub fn anchor_to(mut self, target: impl Into<ElementId>, placement: AnchorPlacement) -> Self {
        self = self.overlay();
        self.anchor = Some(AnchorStyle {
            target: AnchorTarget::Element(target.into()),
            placement,
            gap: DEFAULT_ANCHOR_GAP,
            viewport_margin: DEFAULT_VIEWPORT_MARGIN,
        });
        self
    }

    /// Position a viewport overlay relative to a logical point.
    ///
    /// This is the cursor-point counterpart of [`Self::anchor_to`] and is intended for context
    /// menus. Placement uses the same flip, alternate-alignment, and viewport-clamping rules.
    pub fn anchor_at(mut self, point: crate::Point, placement: AnchorPlacement) -> Self {
        self = self.overlay();
        let point = crate::Point::new(
            if point.x.is_finite() { point.x } else { 0.0 },
            if point.y.is_finite() { point.y } else { 0.0 },
        );
        self.anchor = Some(AnchorStyle {
            target: AnchorTarget::Point(point),
            placement,
            gap: 0.0,
            viewport_margin: DEFAULT_VIEWPORT_MARGIN,
        });
        self
    }

    /// Set the distance between an anchored surface and its trigger.
    pub fn anchor_gap(mut self, gap: f32) -> Self {
        if let Some(anchor) = &mut self.anchor {
            anchor.gap = gap.max(0.0);
        }
        self
    }

    /// Set the minimum distance between an anchored surface and the content viewport edge.
    pub fn viewport_margin(mut self, margin: f32) -> Self {
        if let Some(anchor) = &mut self.anchor {
            anchor.viewport_margin = margin.max(0.0);
        }
        self
    }

    /// Show a delayed, pointer-passive GPU tooltip while this element is hovered.
    ///
    /// The detached tooltip tree is laid out only after its exact delay expires. Entering a
    /// pending tooltip schedules no frame loop, and moving between ordinary points inside the
    /// trigger does not rebuild the application view.
    pub fn tooltip(mut self, tooltip: impl Into<Tooltip>) -> Self {
        let tooltip = tooltip.into();
        if self.accessibility.description.is_none() {
            self.accessibility.description = tooltip.accessibility_description.clone();
        }
        self.tooltip = Some(tooltip);
        self
    }

    /// Prevent pointer events inside this element from reaching lower visual layers.
    pub fn block_pointer(mut self) -> Self {
        self.blocks_pointer = true;
        self
    }

    /// Set web-style native window dragging behavior for this element's layout box.
    pub fn app_region(mut self, region: AppRegion) -> Self {
        self.app_region = Some(region);
        self
    }

    /// Make this element a native window drag region.
    pub fn app_region_drag(self) -> Self {
        self.app_region(AppRegion::Drag)
    }

    /// Restore normal pointer input inside an ancestor window drag region.
    pub fn app_region_no_drag(self) -> Self {
        self.app_region(AppRegion::NoDrag)
    }

    pub fn relative(mut self) -> Self {
        self.layout.position = Position::Relative;
        self
    }

    pub fn top(mut self, value: f32) -> Self {
        self.layout.inset.top = LengthPercentageAuto::length(value);
        self
    }

    pub fn right(mut self, value: f32) -> Self {
        self.layout.inset.right = LengthPercentageAuto::length(value);
        self
    }

    pub fn bottom(mut self, value: f32) -> Self {
        self.layout.inset.bottom = LengthPercentageAuto::length(value);
        self
    }

    pub fn left(mut self, value: f32) -> Self {
        self.layout.inset.left = LengthPercentageAuto::length(value);
        self
    }

    pub fn inset_0(mut self) -> Self {
        let zero = LengthPercentageAuto::length(0.0);
        self.layout.inset = TaffyRect {
            left: zero,
            right: zero,
            top: zero,
            bottom: zero,
        };
        self
    }

    // ---------------------------------------------------------------------------------------
    // Direction-relative alignment and extended text styling.
    //
    // Everything below inherits through the subtree like the other typography helpers and is
    // resolved once per layout build, so retained shaping keys stay canonical.
    // ---------------------------------------------------------------------------------------

    /// Align text to the inline start edge (left in LTR, right in RTL).
    pub fn text_start(self) -> Self {
        self.text_align(TextAlign::Start)
    }

    /// Align text to the inline end edge (right in LTR, left in RTL).
    pub fn text_end(self) -> Self {
        self.text_align(TextAlign::End)
    }

    /// Force the base paragraph direction used when shaping bidirectional text.
    ///
    /// [`Element::rtl`] already sets this for its subtree. Use this helper to override shaping
    /// direction without mirroring layout.
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.typography.direction = Some(direction);
        self
    }

    /// Paint one drop shadow beneath this subtree's glyphs.
    ///
    /// The offset and color are exact. `blur` is approximated: the shadow is painted as a set of
    /// offset copies spread over the blur radius with proportionally reduced alpha, which reads
    /// like a soft shadow but is not a true Gaussian blur. Offsets are clamped to
    /// [`TextShadow::MAX_OFFSET`] and the blur radius to [`TextShadow::MAX_BLUR`].
    pub fn text_shadow(mut self, offset_x: f32, offset_y: f32, blur: f32, color: Color) -> Self {
        self.typography.shadow = Some(Some(TextShadow::new(offset_x, offset_y, blur, color)));
        self
    }

    /// Remove any inherited text shadow.
    pub fn text_shadow_none(mut self) -> Self {
        self.typography.shadow = Some(None);
        self
    }

    /// Add `spacing` logical pixels of advance after every glyph cluster.
    pub fn letter_spacing(mut self, spacing: f32) -> Self {
        self.typography.letter_spacing = Some(sane_text_spacing(spacing));
        self
    }

    /// Add `spacing` logical pixels of advance after every space character.
    pub fn word_spacing(mut self, spacing: f32) -> Self {
        self.typography.word_spacing = Some(sane_text_spacing(spacing));
        self
    }

    /// Case-map non-editable text before shaping.
    ///
    /// The transform applies to [`text`](crate::text) and [`styled_text`](crate::styled_text)
    /// content only. Selection, copy, and accessibility keep reporting the original string, and
    /// editable [`text_input`](crate::text_input) content is never transformed so that its value,
    /// caret indices, and IME state stay byte-identical to the controlled value.
    pub fn text_transform(mut self, transform: TextTransform) -> Self {
        self.typography.transform = Some(Some(transform));
        self
    }

    /// Render text uppercased.
    pub fn uppercase(self) -> Self {
        self.text_transform(TextTransform::Uppercase)
    }

    /// Render text lowercased.
    pub fn lowercase(self) -> Self {
        self.text_transform(TextTransform::Lowercase)
    }

    /// Render every word's first character uppercased.
    pub fn capitalize(self) -> Self {
        self.text_transform(TextTransform::Capitalize)
    }

    /// Remove an inherited case mapping.
    pub fn text_transform_none(mut self) -> Self {
        self.typography.transform = Some(None);
        self
    }

    /// Draw a line above this subtree's text.
    pub fn overline(mut self) -> Self {
        self.typography.overline = Some(true);
        self
    }

    /// Draw a colored line above this subtree's text.
    pub fn overline_color(mut self, color: Color) -> Self {
        self.typography.overline = Some(true);
        self.typography.overline_color = Some(Some(color));
        self
    }

    /// Choose where a line may break inside a run of characters.
    pub fn word_break(mut self, word_break: WordBreak) -> Self {
        self.typography.word_break = Some(word_break);
        self
    }

    /// Choose whether an otherwise unbreakable word may be broken to avoid overflow.
    pub fn overflow_wrap(mut self, overflow_wrap: OverflowWrap) -> Self {
        self.typography.overflow_wrap = Some(overflow_wrap);
        self
    }

    /// Choose whether author-placed soft hyphens (`U+00AD`) may render at a break.
    pub fn hyphens(mut self, hyphens: Hyphens) -> Self {
        self.typography.hyphens = Some(hyphens);
        self
    }
}
