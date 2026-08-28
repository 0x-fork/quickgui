use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    mem,
    ops::Range,
    rc::Rc,
    sync::Arc,
};

use bytemuck::{Pod, Zeroable};
use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphColor, Cursor, FontSystem, Metrics, Resolution,
    Shaping as GlyphShaping, Style as GlyphStyle, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport, Wrap,
    cosmic_text::{
        Align as GlyphAlign, DecorationSpan, Ellipsize, EllipsizeHeightLimit, LayoutGlyph,
        LayoutRun, UnderlineStyle as GlyphUnderlineStyle,
    },
};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;
use wgpu::{
    Adapter, BindGroup, BufferAddress, ColorTargetState, CommandEncoderDescriptor,
    CompositeAlphaMode, Device, DeviceDescriptor, FragmentState, Instance, InstanceDescriptor,
    LoadOp, MultisampleState, Operations, PipelineCompilationOptions, PresentMode, PrimitiveState,
    PrimitiveTopology, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, RequestAdapterOptions, ShaderStages, Surface, SurfaceColorSpace,
    SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, util::DeviceExt,
};
use winit::{event_loop::ActiveEventLoop, window::Window};

use crate::{
    AssetError, Assets, Color as UiColor, FontFallbacks, FontFamily, FontFeatures, FontSource,
    MAX_TEXT_HIGHLIGHTS, PerformanceProfile, Point, Quad, Rect, RenderStats, Scene, ScenePlane,
    Size, TextAlign, TextHighlight, TextId, TextOverflow, TextShaping, TextStyle, TextUnderline,
    TextWrap, WindowBackgroundAppearance,
    assets::resolve_fonts,
    custom_shader_renderer::CustomShaderRenderer,
    font::{glyph_family, normalize_fallbacks},
    image_renderer::ImageRenderer,
    path_renderer::PathRenderer,
    scene::{PrimitiveRef, Shadow, ShapeRef, WavyUnderline},
    svg_renderer::SvgRenderer,
};

pub(crate) struct StyledTextGeometry {
    pub backgrounds: Vec<TextPaintRect>,
    pub decorations: Vec<TextPaintRect>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TextPaintKind {
    Solid,
    WavyUnderline {
        baseline: f32,
        amplitude: f32,
        thickness: f32,
        wavelength: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TextPaintRect {
    pub rect: Rect,
    pub color: UiColor,
    pub kind: TextPaintKind,
}

/// CPU text-layout surface shared by native and headless WGPU renderers.
///
/// Keeping this narrow lets deterministic visual tests exercise the production Taffy and paint
/// paths without introducing a native window. Generic callers remain statically dispatched in
/// the native hot path.
pub(crate) trait TextLayoutEngine {
    fn measure_text(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size;

    fn measure_styled_text(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: &Arc<[TextHighlight]>,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size;

    #[allow(clippy::too_many_arguments)]
    fn text_geometry(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        visible_y: Range<f32>,
    ) -> StyledTextGeometry;

    #[allow(clippy::too_many_arguments)]
    fn text_caret_position_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        index: usize,
    ) -> Point;

    #[allow(clippy::too_many_arguments)]
    fn text_index_for_point_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        point: Point,
    ) -> usize;

    #[allow(clippy::too_many_arguments)]
    fn text_selection_rects_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        visible_y: Range<f32>,
        start: usize,
        end: usize,
    ) -> Vec<Rect>;
}

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2_app_kit::NSView;
#[cfg(target_os = "macos")]
use objc2_foundation::CGSize;
#[cfg(target_os = "macos")]
use objc2_quartz_core::CAMetalLayer;
#[cfg(target_os = "macos")]
use raw_window_metal::Layer as MetalLayer;
#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};
#[cfg(target_os = "macos")]
use wgpu::SurfaceTargetUnsafe;
#[cfg(target_os = "macos")]
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

const INITIAL_SHAPE_CAPACITY: usize = 256;
const BUFFERED_FRAMES: usize = 3;
const MAX_RETAINED_TEXT_AREAS: usize = 256;
const MAX_RETAINED_TEXT_LAYOUTS: usize = 256;
// Text separated by intersecting paint primitives needs independent glyph vertex buffers to
// preserve painter's order. Keep enough for a layered application scene without retaining every
// pathological overlap forever. Each Glyphon renderer starts with a 4 KiB buffer.
const MAX_RETAINED_TEXT_RENDERERS: usize = 32;
const MAX_RETAINED_TEXT_COLORS: usize = 64;
const TEXT_RETENTION_FRAMES: u64 = 8;
const BASIC_FRAGMENT_MIN_BYTES: usize = 24;

#[derive(Debug, Error)]
pub(crate) enum RendererInitError {
    #[error("could not create a WGPU surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("no compatible graphics adapter was found: {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    #[error("could not create a graphics device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("the selected graphics adapter cannot render to this window surface")]
    IncompatibleSurface,
    #[error("the selected graphics adapter does not expose an alpha-capable window surface")]
    TransparentSurfaceUnsupported,
    #[error("could not create the macOS Metal surface: {0}")]
    PlatformSurface(String),
}

#[derive(Debug, Error)]
pub(crate) enum RendererError {
    #[error("text id {0:?} was submitted more than once in one frame")]
    DuplicateTextId(TextId),
    #[error("text preparation failed: {0}")]
    PrepareText(#[from] glyphon::PrepareError),
    #[error("text rendering failed: {0}")]
    RenderText(#[from] glyphon::RenderError),
    #[error("could not recreate a lost window surface: {0}")]
    RecreateSurface(String),
    #[error("could not initialize native-view composition: {0}")]
    NativeComposition(String),
    #[error("this window surface cannot switch to alpha compositing")]
    TransparentSurfaceUnsupported,
    #[error("GPU submission did not complete: {0}")]
    DevicePoll(#[from] wgpu::PollError),
    #[error("custom shader pipeline creation failed: {0}")]
    CustomShader(String),
}

// RenderStats intentionally travels inline once per frame. Boxing it would add a heap allocation
// to the hottest renderer return path merely to shrink the two exceptional unit variants.
#[allow(clippy::large_enum_variant)]
pub(crate) enum RenderOutcome {
    Presented(RenderStats),
    Retry,
    Occluded,
}

/// Share the heavyweight WGPU context and immutable device pipelines across compatible windows.
/// Surface state, uniforms, upload buffers, glyph atlases, text layouts, and bounded asset caches
/// remain window-local.
#[derive(Clone)]
pub(crate) struct GpuContext {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    shape_pipeline: ShapePipeline,
    text_cache: Cache,
}

pub(crate) struct GpuRenderer {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    opaque_alpha_mode: CompositeAlphaMode,
    transparent_alpha_mode: Option<CompositeAlphaMode>,
    desired_surface_size: (u32, u32),
    /// The extent WGPU validates surface operations against. On macOS this is a grow-only
    /// capacity during resize bursts; the CAMetalLayer drawable itself remains exact-sized.
    configured_surface_size: Option<(u32, u32)>,
    #[cfg(target_os = "macos")]
    metal_layer: MetalLayer,
    #[cfg(target_os = "macos")]
    metal_drawable_size: Option<(u32, u32)>,
    #[cfg(target_os = "macos")]
    appkit_view: Retained<NSView>,
    #[cfg(target_os = "macos")]
    live_resize_transaction: bool,
    shapes: ShapeRenderer,
    // These primitive families are absent from ordinary shape-and-text windows. Construct their
    // window-local pipelines and caches only after the first scene that actually uses them.
    path: Option<PathRenderer>,
    custom_shader: Option<CustomShaderRenderer>,
    image: Option<ImageRenderer>,
    svg: Option<SvgRenderer>,
    text: TextSystem,
    #[cfg(target_os = "macos")]
    overlay_surface: Option<OverlaySurface>,
    #[cfg(target_os = "macos")]
    overlay_view: Option<NonNull<c_void>>,
    #[cfg(target_os = "macos")]
    composition_active: bool,
    #[cfg(target_os = "macos")]
    overlay_active: bool,
    window: Arc<Window>,
}

#[cfg(target_os = "macos")]
struct OverlaySurface {
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    view: NonNull<c_void>,
    metal_layer: MetalLayer,
}

impl GpuRenderer {
    pub async fn new(
        window: Arc<Window>,
        event_loop: &ActiveEventLoop,
        profile: PerformanceProfile,
        background_appearance: WindowBackgroundAppearance,
        font_system: SharedFontSystem,
        shared: Option<&GpuContext>,
    ) -> Result<Self, RendererInitError> {
        let instance = shared.map_or_else(
            || {
                Instance::new(InstanceDescriptor::new_with_display_handle(Box::new(
                    event_loop.owned_display_handle(),
                )))
            },
            |context| context.instance.clone(),
        );
        #[cfg(target_os = "macos")]
        let (surface, metal_layer, appkit_view) = create_macos_window_surface(&instance, &window)?;
        #[cfg(not(target_os = "macos"))]
        let surface = instance.create_surface(window.clone())?;
        let (adapter, device, queue) = if let Some(context) =
            shared.filter(|context| context.adapter.is_surface_supported(&surface))
        {
            (
                context.adapter.clone(),
                context.device.clone(),
                context.queue.clone(),
            )
        } else {
            let adapter = instance
                .request_adapter(&RequestAdapterOptions {
                    power_preference: profile.into(),
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await?;
            let (device, queue) = adapter.request_device(&DeviceDescriptor::default()).await?;
            (adapter, device, queue)
        };
        let capabilities = surface.get_capabilities(&adapter);
        let format = preferred_surface_format(&capabilities.formats)
            .ok_or(RendererInitError::IncompatibleSurface)?;
        let opaque_alpha_mode = opaque_surface_alpha_mode(&capabilities.alpha_modes)
            .ok_or(RendererInitError::IncompatibleSurface)?;
        let transparent_alpha_mode = transparent_surface_alpha_mode(&capabilities.alpha_modes);
        let alpha_mode = if background_appearance.is_transparent() {
            transparent_alpha_mode.ok_or(RendererInitError::TransparentSurfaceUnsupported)?
        } else {
            opaque_alpha_mode
        };
        let physical_size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: physical_size.width.max(1),
            height: physical_size.height.max(1),
            present_mode: PresentMode::Fifo,
            alpha_mode,
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
            color_space: SurfaceColorSpace::Auto,
        };
        let configured_surface_size = if physical_size.width > 0 && physical_size.height > 0 {
            surface.configure(&device, &config);
            Some((config.width, config.height))
        } else {
            None
        };
        let desired_surface_size = (config.width, config.height);

        let shapes = ShapeRenderer::new(
            &device,
            format,
            shared.map(|context| &context.shape_pipeline),
        );
        let text = TextSystem::new(
            &device,
            &queue,
            format,
            font_system,
            shared.map(|context| &context.text_cache),
        );
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            config,
            opaque_alpha_mode,
            transparent_alpha_mode,
            desired_surface_size,
            configured_surface_size,
            #[cfg(target_os = "macos")]
            metal_layer,
            #[cfg(target_os = "macos")]
            metal_drawable_size: configured_surface_size,
            #[cfg(target_os = "macos")]
            appkit_view,
            #[cfg(target_os = "macos")]
            live_resize_transaction: false,
            shapes,
            path: None,
            custom_shader: None,
            image: None,
            svg: None,
            text,
            #[cfg(target_os = "macos")]
            overlay_surface: None,
            #[cfg(target_os = "macos")]
            overlay_view: None,
            #[cfg(target_os = "macos")]
            composition_active: false,
            #[cfg(target_os = "macos")]
            overlay_active: false,
            window,
        })
    }

    pub(crate) fn context(&self) -> GpuContext {
        GpuContext {
            instance: self.instance.clone(),
            adapter: self.adapter.clone(),
            device: self.device.clone(),
            queue: self.queue.clone(),
            shape_pipeline: self.shapes.pipeline.clone(),
            text_cache: self.text.cache.clone(),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.desired_surface_size = (width.max(1), height.max(1));
    }

    /// Switch only the surface compositor mode; retained GPU resources and pipelines are reused.
    pub(crate) fn set_transparent(&mut self, transparent: bool) -> Result<bool, RendererError> {
        let alpha_mode = if transparent {
            self.transparent_alpha_mode
                .ok_or(RendererError::TransparentSurfaceUnsupported)?
        } else {
            self.opaque_alpha_mode
        };
        if self.config.alpha_mode == alpha_mode {
            return Ok(false);
        }
        self.config.alpha_mode = alpha_mode;
        #[cfg(target_os = "macos")]
        set_metal_layer_opaque(&self.metal_layer, !transparent);
        #[cfg(not(target_os = "macos"))]
        {
            // Other backends apply the compositor policy through surface configuration. On
            // macOS the policy is solely CAMetalLayer.opaque, and reconfiguring an active layer
            // can strand its last presented drawable in the window server. Mutating the layer
            // directly mirrors native Metal renderers and keeps the drawable chain alive.
            self.configured_surface_size = None;
        }
        Ok(true)
    }

    /// Apply only the latest queued size immediately before drawable acquisition.
    fn configure_surface_for_frame(&mut self) {
        #[cfg(target_os = "macos")]
        {
            self.configure_macos_surface_for_frame();
        }
        #[cfg(not(target_os = "macos"))]
        {
            let size = self.desired_surface_size;
            if self.configured_surface_size == Some(size) {
                return;
            }
            self.config.width = size.0;
            self.config.height = size.1;
            self.surface.configure(&self.device, &self.config);
            self.configured_surface_size = Some(size);
        }
    }

    fn reconfigure_base_surface_after_error(&mut self) {
        self.surface.configure(&self.device, &self.config);
        let configured = (self.config.width, self.config.height);
        self.configured_surface_size = Some(configured);
        #[cfg(target_os = "macos")]
        {
            // `Surface::configure` restores CAMetalLayer's drawable size to WGPU's validated
            // extent. Keep our tracking honest so the next stable frame can snap it back to the
            // exact window size when the capacity is intentionally larger.
            set_metal_drawable_size(&self.metal_layer, configured);
            self.metal_drawable_size = Some(configured);
        }
    }

    /// Resize a Metal-backed GUI surface without making WGPU wait for the entire device on every
    /// AppKit frame change.
    ///
    /// WGPU recreates a configured surface only after `Device::maintain(Wait)`, even though a
    /// CAMetalLayer drawable size can change independently of the configured WGPU extent. Keep
    /// WGPU's validated extent as a bounded grow-only capacity, but keep the actual layer drawable
    /// exact-sized on every frame. That avoids both the blocking surface reconfiguration path and
    /// Core Animation resampling text while a resize gesture is in progress. A full configure is
    /// needed only when the window exceeds the validated capacity.
    #[cfg(target_os = "macos")]
    fn configure_macos_surface_for_frame(&mut self) {
        let desired = self.desired_surface_size;
        let maximum = self.device.limits().max_texture_dimension_2d;
        let capacity = self.configured_surface_size.map_or(desired, |current| {
            grow_surface_capacity(current, desired, maximum)
        });
        if self.configured_surface_size != Some(capacity) {
            self.config.width = capacity.0;
            self.config.height = capacity.1;
            self.surface.configure(&self.device, &self.config);
            if let Some(overlay) = &mut self.overlay_surface {
                overlay.config.width = capacity.0;
                overlay.config.height = capacity.1;
                overlay.surface.configure(&self.device, &overlay.config);
            }
            self.configured_surface_size = Some(capacity);
            self.metal_drawable_size = Some(capacity);
        }
        // `Surface::configure` resets the layer to the capacity extent. Restore the exact current
        // drawable immediately, and update it on subsequent frames without reconfiguring WGPU.
        // Glyph quads then land on native backing pixels instead of passing through a changing
        // Core Animation scale that makes text stems shimmer between thick and thin.
        if self.metal_drawable_size != Some(desired) {
            set_metal_drawable_size(&self.metal_layer, desired);
            if let Some(overlay) = &self.overlay_surface {
                set_metal_drawable_size(&overlay.metal_layer, desired);
            }
            self.metal_drawable_size = Some(desired);
        }
    }

    #[cfg(target_os = "macos")]
    fn update_live_resize_transaction(&mut self) {
        let resize_transaction = unsafe { self.appkit_view.inLiveResize() };
        if self.live_resize_transaction != resize_transaction {
            set_presents_with_transaction(&self.metal_layer, resize_transaction);
            set_metal_display_sync(&self.metal_layer, !resize_transaction);
            if let Some(overlay) = &self.overlay_surface {
                set_presents_with_transaction(&overlay.metal_layer, resize_transaction);
                set_metal_display_sync(&overlay.metal_layer, !resize_transaction);
            }
            self.live_resize_transaction = resize_transaction;
        }
        // AppKit already paces a real live-resize transaction at its composition boundary. FIFO
        // display sync inside `nextDrawable` would wait for a second boundary and halve visible
        // resize throughput, so that one native interaction presents transactionally without the
        // additional wait. Programmatic resizes retain display sync: they are driven by presented
        // frames and must not enqueue work faster than the display can retire its drawable
        // storage. Ordinary scrolling and animation use the same bounded FIFO policy.
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn enable_native_composition(
        &mut self,
        view: NonNull<c_void>,
    ) -> Result<(), RendererError> {
        if self.overlay_view != Some(view) {
            self.overlay_surface = None;
            self.overlay_view = Some(view);
        }
        self.composition_active = true;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn set_native_composition_active(&mut self, active: bool) {
        self.composition_active = active && self.overlay_view.is_some();
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn set_native_overlay_active(&mut self, active: bool) -> Result<(), RendererError> {
        if !active {
            self.overlay_active = false;
            return Ok(());
        }
        if self.overlay_surface.is_none() {
            let view = self
                .overlay_view
                .ok_or_else(|| RendererError::NativeComposition("missing overlay view".into()))?;
            let overlay = create_overlay_surface(
                &self.instance,
                &self.adapter,
                &self.device,
                &self.config,
                view,
            )?;
            set_presents_with_transaction(&overlay.metal_layer, self.live_resize_transaction);
            set_metal_display_sync(&overlay.metal_layer, !self.live_resize_transaction);
            set_metal_drawable_size(
                &overlay.metal_layer,
                self.metal_drawable_size
                    .unwrap_or(self.desired_surface_size),
            );
            self.overlay_surface = Some(overlay);
        }
        self.overlay_active = true;
        Ok(())
    }

    pub(crate) fn measure_text(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size {
        self.text
            .measure(id, content, style, None, max_width, scale_factor)
    }

    pub(crate) fn measure_styled_text(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: &Arc<[TextHighlight]>,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size {
        self.text.measure(
            id,
            content,
            style,
            Some(highlights),
            max_width,
            scale_factor,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_geometry(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        visible_y: Range<f32>,
    ) -> StyledTextGeometry {
        self.text.text_geometry(
            id,
            content,
            style,
            highlights,
            width,
            scale_factor,
            visible_y,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_caret_position_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        index: usize,
    ) -> Point {
        self.text
            .caret_position(id, content, style, highlights, width, scale_factor, index)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_index_for_point_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        point: Point,
    ) -> usize {
        self.text.index_for_point(
            id,
            content,
            style,
            highlights,
            TextHitTest {
                width,
                scale: scale_factor,
                point,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_selection_rects_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        visible_y: Range<f32>,
        start: usize,
        end: usize,
    ) -> Vec<Rect> {
        self.text.selection_rects(
            id,
            content,
            style,
            highlights,
            width,
            scale_factor,
            visible_y,
            start,
            end,
        )
    }

    /// Wait for the latest submitted frame before removing a platform launch cover.
    pub(crate) fn wait_for_submitted_work(&self) -> Result<(), RendererError> {
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        Ok(())
    }

    pub fn render(
        &mut self,
        scene: &Scene,
        scale_factor: f32,
    ) -> Result<RenderOutcome, RendererError> {
        let physical_size = self.window.inner_size();
        if physical_size.width == 0 || physical_size.height == 0 {
            return Ok(RenderOutcome::Occluded);
        }
        self.resize(physical_size.width, physical_size.height);
        self.configure_surface_for_frame();
        #[cfg(target_os = "macos")]
        self.update_live_resize_transaction();

        let logical_viewport = Rect::new(
            0.0,
            0.0,
            physical_size.width as f32 / scale_factor,
            physical_size.height as f32 / scale_factor,
        );
        let (quad_count, shadow_count, shape_draw_calls) = self.shapes.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_size.width,
            physical_size.height,
            scale_factor,
        );
        if self.path.is_none()
            && scene
                .paint_layers()
                .iter()
                .any(|layer| !layer.paths().is_empty())
        {
            self.path = Some(PathRenderer::new(&self.device, self.config.format));
        }
        let path_stats = if let Some(path) = &mut self.path {
            path.prepare(
                &self.device,
                &self.queue,
                scene,
                logical_viewport,
                physical_size.width,
                physical_size.height,
                scale_factor,
            )
        } else {
            Default::default()
        };
        if self.custom_shader.is_none()
            && scene
                .paint_layers()
                .iter()
                .any(|layer| !layer.custom_shaders().is_empty())
        {
            self.custom_shader = Some(CustomShaderRenderer::new(&self.device, self.config.format));
        }
        let custom_shader_stats = if let Some(custom_shader) = &mut self.custom_shader {
            custom_shader
                .prepare(
                    &self.device,
                    &self.queue,
                    scene,
                    logical_viewport,
                    physical_size.width,
                    physical_size.height,
                    scale_factor,
                )
                .map_err(RendererError::CustomShader)?
        } else {
            Default::default()
        };
        if self.image.is_none()
            && scene
                .paint_layers()
                .iter()
                .any(|layer| !layer.images().is_empty())
        {
            self.image = Some(ImageRenderer::new(&self.device, self.config.format));
        }
        let image_stats = if let Some(image) = &mut self.image {
            image.prepare(
                &self.device,
                &self.queue,
                scene,
                logical_viewport,
                physical_size.width,
                physical_size.height,
                scale_factor,
            )
        } else {
            Default::default()
        };
        if self.svg.is_none()
            && scene
                .paint_layers()
                .iter()
                .any(|layer| !layer.svgs().is_empty())
        {
            self.svg = Some(SvgRenderer::new(&self.device, self.config.format));
        }
        let svg_stats = if let Some(svg) = &mut self.svg {
            svg.prepare(
                &self.device,
                &self.queue,
                scene,
                logical_viewport,
                physical_size.width,
                physical_size.height,
                scale_factor,
            )
        } else {
            Default::default()
        };
        let (text_count, reshaped, text_draw_calls) = self.text.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_size.width,
            physical_size.height,
            scale_factor,
        )?;

        #[cfg(target_os = "macos")]
        let composed = self.composition_active;
        #[cfg(not(target_os = "macos"))]
        let composed = false;

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                // A surface texture must not outlive reconfiguration. In particular, Metal can
                // otherwise wait on the drawable we still own while rebuilding its pool.
                drop(frame);
                self.reconfigure_base_surface_after_error();
                return Ok(RenderOutcome::Retry);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Outdated => {
                self.reconfigure_base_surface_after_error();
                return Ok(RenderOutcome::Retry);
            }
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(RenderOutcome::Occluded),
            wgpu::CurrentSurfaceTexture::Lost => {
                #[cfg(target_os = "macos")]
                let surface = create_surface_from_metal_layer(&self.instance, &self.metal_layer);
                #[cfg(not(target_os = "macos"))]
                let surface = self.instance.create_surface(self.window.clone());
                self.surface =
                    surface.map_err(|error| RendererError::RecreateSurface(error.to_string()))?;
                self.reconfigure_base_surface_after_error();
                return Ok(RenderOutcome::Retry);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                panic!("WGPU reported a surface validation error")
            }
        };

        #[cfg(target_os = "macos")]
        let overlay_frame = if composed && self.overlay_active {
            let overlay = self
                .overlay_surface
                .as_mut()
                .expect("active composition owns an overlay surface");
            match overlay.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => Some(frame),
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    drop(frame);
                    overlay.surface.configure(&self.device, &overlay.config);
                    set_metal_drawable_size(
                        &overlay.metal_layer,
                        self.metal_drawable_size
                            .unwrap_or(self.desired_surface_size),
                    );
                    return Ok(RenderOutcome::Retry);
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Outdated => {
                    overlay.surface.configure(&self.device, &overlay.config);
                    set_metal_drawable_size(
                        &overlay.metal_layer,
                        self.metal_drawable_size
                            .unwrap_or(self.desired_surface_size),
                    );
                    return Ok(RenderOutcome::Retry);
                }
                wgpu::CurrentSurfaceTexture::Occluded => return Ok(RenderOutcome::Occluded),
                wgpu::CurrentSurfaceTexture::Lost => {
                    let view = overlay.view;
                    let overlay = create_overlay_surface(
                        &self.instance,
                        &self.adapter,
                        &self.device,
                        &self.config,
                        view,
                    )?;
                    set_presents_with_transaction(
                        &overlay.metal_layer,
                        self.live_resize_transaction,
                    );
                    set_metal_display_sync(&overlay.metal_layer, !self.live_resize_transaction);
                    set_metal_drawable_size(
                        &overlay.metal_layer,
                        self.metal_drawable_size
                            .unwrap_or(self.desired_surface_size),
                    );
                    self.overlay_surface = Some(overlay);
                    return Ok(RenderOutcome::Retry);
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    panic!("WGPU reported an overlay surface validation error")
                }
            }
        } else {
            None
        };
        #[cfg(not(target_os = "macos"))]
        let overlay_frame: Option<wgpu::SurfaceTexture> = None;

        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let overlay_view = overlay_frame
            .as_ref()
            .map(|frame| frame.texture.create_view(&TextureViewDescriptor::default()));
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("quickgui frame encoder"),
            });
        {
            let clear = scene.background();
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("quickgui main pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: clear.r as f64,
                            g: clear.g as f64,
                            b: clear.b as f64,
                            a: clear.a as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            for (layer, paint_layer) in scene.paint_layers().iter().enumerate() {
                if composed && paint_layer.key().plane != ScenePlane::Base {
                    continue;
                }
                for order in 0..=paint_layer.max_order() {
                    self.shapes.render_order(&mut pass, layer, order);
                    if let Some(path) = &self.path {
                        path.render_order(&mut pass, layer, order);
                    }
                    if let Some(custom_shader) = &self.custom_shader {
                        custom_shader.render_order(&mut pass, layer, order);
                    }
                    if let Some(image) = &self.image {
                        image.render_order(&mut pass, layer, order);
                    }
                    if let Some(svg) = &self.svg {
                        svg.render_order(&mut pass, layer, order);
                    }
                    self.text.render_order(&mut pass, layer, order)?;
                }
            }
        }
        if let Some(view) = &overlay_view {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("quickgui overlay pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            for (layer, paint_layer) in scene.paint_layers().iter().enumerate() {
                if paint_layer.key().plane != ScenePlane::Overlay {
                    continue;
                }
                for order in 0..=paint_layer.max_order() {
                    self.shapes.render_order(&mut pass, layer, order);
                    if let Some(path) = &self.path {
                        path.render_order(&mut pass, layer, order);
                    }
                    if let Some(custom_shader) = &self.custom_shader {
                        custom_shader.render_order(&mut pass, layer, order);
                    }
                    if let Some(image) = &self.image {
                        image.render_order(&mut pass, layer, order);
                    }
                    if let Some(svg) = &self.svg {
                        svg.render_order(&mut pass, layer, order);
                    }
                    self.text.render_order(&mut pass, layer, order)?;
                }
            }
        }

        self.window.pre_present_notify();
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        if let Some(frame) = overlay_frame {
            self.queue.present(frame);
        }
        self.text.finish_frame();
        let (retained_text_areas, retained_text_layouts, retained_text_renderers) =
            self.text.retained_counts();

        Ok(RenderOutcome::Presented(RenderStats {
            quads: quad_count,
            shadows: shadow_count,
            images: image_stats.images,
            image_uploads: image_stats.uploads,
            gpu_image_cache_bytes: image_stats.cache_bytes,
            cpu_image_cache_bytes: 0,
            image_resource_entries: 0,
            image_resources_loading: 0,
            image_resources_failed: 0,
            animated_images: 0,
            active_animations: 0,
            svgs: svg_stats.svgs,
            svg_rasterizations: svg_stats.rasterizations,
            gpu_svg_cache_bytes: svg_stats.cache_bytes,
            paths: path_stats.paths,
            path_vertices: path_stats.vertices,
            skipped_paths: path_stats.skipped_paths,
            custom_shader_instances: custom_shader_stats.instances,
            custom_shader_compilations: custom_shader_stats.compiled_pipelines,
            cached_custom_shader_pipelines: custom_shader_stats.cached_pipelines,
            skipped_custom_shader_instances: custom_shader_stats.skipped_instances,
            text_areas: text_count,
            draw_calls: shape_draw_calls
                + path_stats.draw_calls
                + custom_shader_stats.draw_calls
                + image_stats.draw_calls
                + svg_stats.draw_calls
                + text_draw_calls,
            reshaped_text_areas: reshaped,
            retained_text_areas,
            retained_text_layouts,
            retained_text_renderers,
            cached_text_areas: text_count.saturating_sub(reshaped),
        }))
    }

    #[allow(dead_code)]
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }
}

impl TextLayoutEngine for GpuRenderer {
    fn measure_text(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size {
        GpuRenderer::measure_text(self, id, content, style, max_width, scale_factor)
    }

    fn measure_styled_text(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: &Arc<[TextHighlight]>,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size {
        GpuRenderer::measure_styled_text(
            self,
            id,
            content,
            style,
            highlights,
            max_width,
            scale_factor,
        )
    }

    fn text_geometry(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        visible_y: Range<f32>,
    ) -> StyledTextGeometry {
        GpuRenderer::text_geometry(
            self,
            id,
            content,
            style,
            highlights,
            width,
            scale_factor,
            visible_y,
        )
    }

    fn text_caret_position_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        index: usize,
    ) -> Point {
        GpuRenderer::text_caret_position_with_highlights(
            self,
            id,
            content,
            style,
            highlights,
            width,
            scale_factor,
            index,
        )
    }

    fn text_index_for_point_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        point: Point,
    ) -> usize {
        GpuRenderer::text_index_for_point_with_highlights(
            self,
            id,
            content,
            style,
            highlights,
            width,
            scale_factor,
            point,
        )
    }

    fn text_selection_rects_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        visible_y: Range<f32>,
        start: usize,
        end: usize,
    ) -> Vec<Rect> {
        GpuRenderer::text_selection_rects_with_highlights(
            self,
            id,
            content,
            style,
            highlights,
            width,
            scale_factor,
            visible_y,
            start,
            end,
        )
    }
}

/// Lazily constructed WGPU target for deterministic visual tests.
///
/// It owns the same bounded pipeline and text-cache implementations as a native renderer, but no
/// Winit window, surface, event loop, timer, or presentation loop.
#[cfg(any(test, feature = "test-support"))]
pub(crate) struct OffscreenRenderer {
    _instance: Instance,
    _adapter: Adapter,
    device: Device,
    queue: Queue,
    format: TextureFormat,
    shapes: ShapeRenderer,
    path: PathRenderer,
    custom_shader: CustomShaderRenderer,
    image: ImageRenderer,
    svg: SvgRenderer,
    text: TextSystem,
    #[cfg(test)]
    last_reshaped_text_areas: usize,
}

#[cfg(any(test, feature = "test-support"))]
impl OffscreenRenderer {
    pub(crate) fn validate_snapshot_size(
        logical_size: Size,
        scale_factor: f32,
    ) -> Result<(), crate::VisualTestError> {
        visual_physical_size(logical_size, scale_factor).map(|_| ())
    }

    pub(crate) async fn new(
        profile: PerformanceProfile,
        font_system: SharedFontSystem,
    ) -> Result<Self, crate::VisualTestError> {
        let instance = Instance::new(InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: profile.into(),
                compatible_surface: None,
                ..Default::default()
            })
            .await
            .map_err(|error| crate::VisualTestError::Adapter(error.to_string()))?;
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default())
            .await
            .map_err(|error| crate::VisualTestError::Device(error.to_string()))?;
        let format = TextureFormat::Rgba8UnormSrgb;
        let shapes = ShapeRenderer::new(&device, format, None);
        let path = PathRenderer::new(&device, format);
        let custom_shader = CustomShaderRenderer::new(&device, format);
        let image = ImageRenderer::new(&device, format);
        let svg = SvgRenderer::new(&device, format);
        let text = TextSystem::new(&device, &queue, format, font_system, None);
        Ok(Self {
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            format,
            shapes,
            path,
            custom_shader,
            image,
            svg,
            text,
            #[cfg(test)]
            last_reshaped_text_areas: 0,
        })
    }

    #[cfg(test)]
    pub(crate) fn last_reshaped_text_areas(&self) -> usize {
        self.last_reshaped_text_areas
    }

    pub(crate) fn render_to_snapshot(
        &mut self,
        scene: &Scene,
        logical_size: Size,
        scale_factor: f32,
    ) -> Result<crate::VisualSnapshot, crate::VisualTestError> {
        let target_size = visual_physical_size(logical_size, scale_factor)?;
        self.render_to_snapshot_with_target(scene, logical_size, scale_factor, target_size)
    }

    fn render_to_snapshot_with_target(
        &mut self,
        scene: &Scene,
        logical_size: Size,
        scale_factor: f32,
        target_size: (u32, u32),
    ) -> Result<crate::VisualSnapshot, crate::VisualTestError> {
        let (physical_width, physical_height) = visual_physical_size(logical_size, scale_factor)?;
        let (target_width, target_height) = target_size;
        crate::visual_test::validate_snapshot_dimensions(target_width, target_height)?;
        let logical_viewport = Rect::new(0.0, 0.0, logical_size.width, logical_size.height);
        self.shapes.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_width,
            physical_height,
            scale_factor,
        );
        self.path.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_width,
            physical_height,
            scale_factor,
        );
        self.custom_shader
            .prepare(
                &self.device,
                &self.queue,
                scene,
                logical_viewport,
                physical_width,
                physical_height,
                scale_factor,
            )
            .map_err(crate::VisualTestError::Render)?;
        self.image.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_width,
            physical_height,
            scale_factor,
        );
        self.svg.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_width,
            physical_height,
            scale_factor,
        );
        let (_, _reshaped, _) = self
            .text
            .prepare(
                &self.device,
                &self.queue,
                scene,
                logical_viewport,
                physical_width,
                physical_height,
                scale_factor,
            )
            .map_err(|error| crate::VisualTestError::Render(error.to_string()))?;
        #[cfg(test)]
        {
            self.last_reshaped_text_areas = _reshaped;
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("quickgui visual-test target"),
            size: wgpu::Extent3d {
                width: target_width,
                height: target_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        let unpadded_bytes_per_row = target_width
            .checked_mul(4)
            .ok_or(crate::VisualTestError::TooLarge { bytes: u64::MAX })?;
        let padded_bytes_per_row = unpadded_bytes_per_row
            .checked_add(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT - 1)
            .map(|value| value / wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            .and_then(|rows| rows.checked_mul(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT))
            .ok_or(crate::VisualTestError::TooLarge { bytes: u64::MAX })?;
        let readback_bytes = u64::from(padded_bytes_per_row)
            .checked_mul(u64::from(target_height))
            .ok_or(crate::VisualTestError::TooLarge { bytes: u64::MAX })?;
        if readback_bytes > crate::MAX_VISUAL_TEST_BYTES {
            return Err(crate::VisualTestError::TooLarge {
                bytes: readback_bytes,
            });
        }
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quickgui visual-test readback"),
            size: readback_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("quickgui visual-test encoder"),
            });
        {
            let clear = scene.background();
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("quickgui visual-test pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: clear.r as f64,
                            g: clear.g as f64,
                            b: clear.b as f64,
                            a: clear.a as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            for (layer, paint_layer) in scene.paint_layers().iter().enumerate() {
                for order in 0..=paint_layer.max_order() {
                    self.shapes.render_order(&mut pass, layer, order);
                    self.path.render_order(&mut pass, layer, order);
                    self.custom_shader.render_order(&mut pass, layer, order);
                    self.image.render_order(&mut pass, layer, order);
                    self.svg.render_order(&mut pass, layer, order);
                    self.text
                        .render_order(&mut pass, layer, order)
                        .map_err(|error| crate::VisualTestError::Render(error.to_string()))?;
                }
            }
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(target_height),
                },
            },
            wgpu::Extent3d {
                width: target_width,
                height: target_height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        self.text.finish_frame();

        let slice = readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| crate::VisualTestError::Readback(error.to_string()))?;
        receiver
            .recv()
            .map_err(|error| crate::VisualTestError::Readback(error.to_string()))?
            .map_err(|error| crate::VisualTestError::Readback(error.to_string()))?;
        let mapped = slice
            .get_mapped_range()
            .map_err(|error| crate::VisualTestError::Readback(error.to_string()))?;
        let tight_row = unpadded_bytes_per_row as usize;
        let padded_row = padded_bytes_per_row as usize;
        let tight_bytes = tight_row
            .checked_mul(target_height as usize)
            .ok_or(crate::VisualTestError::TooLarge { bytes: u64::MAX })?;
        let mut rgba = Vec::with_capacity(tight_bytes);
        for row in mapped.chunks_exact(padded_row).take(target_height as usize) {
            rgba.extend_from_slice(&row[..tight_row]);
        }
        drop(mapped);
        readback.unmap();
        crate::VisualSnapshot::from_rgba(target_width, target_height, rgba)
    }
}

#[cfg(any(test, feature = "test-support"))]
impl TextLayoutEngine for OffscreenRenderer {
    fn measure_text(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size {
        self.text
            .measure(id, content, style, None, max_width, scale_factor)
    }

    fn measure_styled_text(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: &Arc<[TextHighlight]>,
        max_width: Option<f32>,
        scale_factor: f32,
    ) -> Size {
        self.text.measure(
            id,
            content,
            style,
            Some(highlights),
            max_width,
            scale_factor,
        )
    }

    fn text_geometry(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        visible_y: Range<f32>,
    ) -> StyledTextGeometry {
        self.text.text_geometry(
            id,
            content,
            style,
            highlights,
            width,
            scale_factor,
            visible_y,
        )
    }

    fn text_caret_position_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        index: usize,
    ) -> Point {
        self.text
            .caret_position(id, content, style, highlights, width, scale_factor, index)
    }

    fn text_index_for_point_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        point: Point,
    ) -> usize {
        self.text.index_for_point(
            id,
            content,
            style,
            highlights,
            TextHitTest {
                width,
                scale: scale_factor,
                point,
            },
        )
    }

    fn text_selection_rects_with_highlights(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale_factor: f32,
        visible_y: Range<f32>,
        start: usize,
        end: usize,
    ) -> Vec<Rect> {
        self.text.selection_rects(
            id,
            content,
            style,
            highlights,
            width,
            scale_factor,
            visible_y,
            start,
            end,
        )
    }
}

#[cfg(any(test, feature = "test-support"))]
fn visual_physical_size(
    logical_size: Size,
    scale_factor: f32,
) -> Result<(u32, u32), crate::VisualTestError> {
    if !logical_size.width.is_finite()
        || !logical_size.height.is_finite()
        || logical_size.width <= 0.0
        || logical_size.height <= 0.0
        || !scale_factor.is_finite()
        || scale_factor <= 0.0
    {
        return Err(crate::VisualTestError::InvalidDimensions);
    }
    let physical_width = (logical_size.width * scale_factor).round();
    let physical_height = (logical_size.height * scale_factor).round();
    if physical_width < 1.0
        || physical_height < 1.0
        || physical_width > crate::MAX_VISUAL_TEST_DIMENSION as f32
        || physical_height > crate::MAX_VISUAL_TEST_DIMENSION as f32
    {
        return Err(crate::VisualTestError::InvalidDimensions);
    }
    let physical_width = physical_width as u32;
    let physical_height = physical_height as u32;
    crate::visual_test::validate_snapshot_dimensions(physical_width, physical_height)?;
    Ok((physical_width, physical_height))
}

#[cfg(target_os = "macos")]
fn create_macos_window_surface(
    instance: &Instance,
    window: &Window,
) -> Result<(Surface<'static>, MetalLayer, Retained<NSView>), RendererInitError> {
    let handle = window
        .window_handle()
        .map_err(|error| RendererInitError::PlatformSurface(error.to_string()))?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err(RendererInitError::PlatformSurface(
            "the window does not expose an AppKit view".to_owned(),
        ));
    };
    let appkit_view = unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) }
        .ok_or_else(|| {
            RendererInitError::PlatformSurface("the AppKit view pointer is null".to_owned())
        })?;
    let metal_layer =
        unsafe { MetalLayer::from_ns_view(NonNull::from(appkit_view.as_ref()).cast::<c_void>()) };
    let surface = create_surface_from_metal_layer(instance, &metal_layer)?;
    Ok((surface, metal_layer, appkit_view))
}

#[cfg(target_os = "macos")]
fn create_surface_from_metal_layer(
    instance: &Instance,
    layer: &MetalLayer,
) -> Result<Surface<'static>, wgpu::CreateSurfaceError> {
    unsafe {
        instance.create_surface_unsafe(SurfaceTargetUnsafe::CoreAnimationLayer(
            layer.as_ptr().as_ptr(),
        ))
    }
}

#[cfg(target_os = "macos")]
fn set_presents_with_transaction(layer: &MetalLayer, enabled: bool) {
    let layer = unsafe { layer.as_ptr().cast::<CAMetalLayer>().as_ref() };
    unsafe { layer.setPresentsWithTransaction(enabled) };
}

#[cfg(target_os = "macos")]
fn set_metal_display_sync(layer: &MetalLayer, enabled: bool) {
    let layer = unsafe { layer.as_ptr().cast::<CAMetalLayer>().as_ref() };
    // SAFETY: the renderer owns this CAMetalLayer and updates its presentation policy on the
    // AppKit application thread before acquiring the frame's drawable.
    unsafe { layer.setDisplaySyncEnabled(enabled) };
}

#[cfg(target_os = "macos")]
fn set_metal_drawable_size(layer: &MetalLayer, size: (u32, u32)) {
    let layer = unsafe { layer.as_ptr().cast::<CAMetalLayer>().as_ref() };
    // SAFETY: the renderer owns this CAMetalLayer and calls this on AppKit's application thread.
    // Both dimensions have already been clamped to at least one physical pixel.
    unsafe { layer.setDrawableSize(CGSize::new(f64::from(size.0), f64::from(size.1))) };
}

fn grow_surface_capacity(current: (u32, u32), required: (u32, u32), maximum: u32) -> (u32, u32) {
    fn grow(current: u32, required: u32, maximum: u32) -> u32 {
        if required <= current {
            return current;
        }
        current
            .saturating_add(current / 2)
            .max(required)
            .min(maximum)
    }

    (
        grow(current.0, required.0, maximum),
        grow(current.1, required.1, maximum),
    )
}

#[cfg(target_os = "macos")]
fn set_metal_layer_opaque(layer: &MetalLayer, opaque: bool) {
    let layer = unsafe { layer.as_ptr().cast::<CAMetalLayer>().as_ref() };
    layer.setOpaque(opaque);
}

#[cfg(target_os = "macos")]
fn create_overlay_surface(
    instance: &Instance,
    adapter: &Adapter,
    device: &Device,
    base_config: &SurfaceConfiguration,
    view: NonNull<c_void>,
) -> Result<OverlaySurface, RendererError> {
    let metal_layer = unsafe { MetalLayer::from_ns_view(view) };
    let surface = create_surface_from_metal_layer(instance, &metal_layer)
        .map_err(|error| RendererError::NativeComposition(error.to_string()))?;
    let capabilities = surface.get_capabilities(adapter);
    if !capabilities.formats.contains(&base_config.format) {
        return Err(RendererError::NativeComposition(
            "the overlay surface does not support the base surface format".to_owned(),
        ));
    }
    let alpha_mode = capabilities
        .alpha_modes
        .iter()
        .copied()
        .find(|mode| *mode == CompositeAlphaMode::PreMultiplied)
        .or_else(|| {
            capabilities
                .alpha_modes
                .iter()
                .copied()
                .find(|mode| *mode == CompositeAlphaMode::PostMultiplied)
        })
        .or_else(|| {
            capabilities
                .alpha_modes
                .iter()
                .copied()
                .find(|mode| *mode == CompositeAlphaMode::Auto)
        })
        .ok_or_else(|| {
            RendererError::NativeComposition(
                "the overlay surface does not expose a transparent alpha mode".to_owned(),
            )
        })?;
    let mut config = base_config.clone();
    config.alpha_mode = alpha_mode;
    surface.configure(device, &config);
    Ok(OverlaySurface {
        surface,
        config,
        view,
        metal_layer,
    })
}

fn preferred_surface_format(formats: &[TextureFormat]) -> Option<TextureFormat> {
    formats
        .iter()
        .copied()
        .find(TextureFormat::is_srgb)
        .or_else(|| formats.first().copied())
}

fn opaque_surface_alpha_mode(modes: &[CompositeAlphaMode]) -> Option<CompositeAlphaMode> {
    modes
        .iter()
        .copied()
        .find(|mode| *mode == CompositeAlphaMode::Opaque)
        .or_else(|| modes.first().copied())
}

fn transparent_surface_alpha_mode(modes: &[CompositeAlphaMode]) -> Option<CompositeAlphaMode> {
    [
        CompositeAlphaMode::PreMultiplied,
        CompositeAlphaMode::PostMultiplied,
        CompositeAlphaMode::Inherit,
    ]
    .into_iter()
    .find(|preferred| modes.contains(preferred))
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ViewUniform {
    viewport: [f32; 2],
    scale: f32,
    _padding: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ShapeInstance {
    geometry: [f32; 4],
    primary: [f32; 4],
    secondary: [f32; 4],
    clip: [f32; 4],
    params: [f32; 4],
    subject: [f32; 4],
}

const SHAPE_MODE_QUAD: f32 = 0.0;
const SHAPE_MODE_DROP_SHADOW: f32 = 1.0;
const SHAPE_MODE_INSET_SHADOW: f32 = 2.0;
const SHAPE_MODE_WAVY_UNDERLINE: f32 = 3.0;
const SHADOW_SIGMA_PER_BLUR_RADIUS: f32 = 0.5;
const SHADOW_MARGIN_SIGMAS: f32 = 3.0;

#[derive(Clone)]
struct ShapePipeline {
    pipeline: Arc<RenderPipeline>,
    bind_group_layout: Arc<wgpu::BindGroupLayout>,
    format: TextureFormat,
}

struct ShapeRenderer {
    pipeline: ShapePipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: BindGroup,
    instance_buffers: Vec<wgpu::Buffer>,
    instance_capacities: Vec<usize>,
    active_buffer: usize,
    batches: Vec<ShapeBatch>,
    layer_batches: Vec<Range<usize>>,
    pending: Vec<OrderedShape>,
    instances: Vec<ShapeInstance>,
}

#[derive(Clone, Copy)]
struct OrderedShape {
    order: u32,
    instance: ShapeInstance,
}

struct ShapeBatch {
    order: u32,
    instances: Range<u32>,
}

impl ShapePipeline {
    fn new(device: &Device, format: TextureFormat) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("quickgui view bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quickgui shape pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("quad.wgsl"));
        let attributes = [
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 16,
                shader_location: 1,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 32,
                shader_location: 2,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 48,
                shader_location: 3,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 64,
                shader_location: 4,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 80,
                shader_location: 5,
            },
        ];
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("quickgui shape pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[Some(VertexBufferLayout {
                    array_stride: mem::size_of::<ShapeInstance>() as BufferAddress,
                    step_mode: VertexStepMode::Instance,
                    attributes: &attributes,
                })],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline: Arc::new(pipeline),
            bind_group_layout: Arc::new(bind_group_layout),
            format,
        }
    }
}

impl ShapeRenderer {
    fn new(device: &Device, format: TextureFormat, shared: Option<&ShapePipeline>) -> Self {
        let pipeline = shared
            .filter(|pipeline| pipeline.format == format)
            .cloned()
            .unwrap_or_else(|| ShapePipeline::new(device, format));
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quickgui view uniform"),
            contents: bytemuck::bytes_of(&ViewUniform::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quickgui view bind group"),
            layout: &pipeline.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let instance_buffers = (0..BUFFERED_FRAMES)
            .map(|_| create_shape_instance_buffer(device, INITIAL_SHAPE_CAPACITY))
            .collect();
        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            instance_buffers,
            instance_capacities: vec![INITIAL_SHAPE_CAPACITY; BUFFERED_FRAMES],
            active_buffer: 0,
            batches: Vec::with_capacity(16),
            layer_batches: Vec::with_capacity(4),
            pending: Vec::with_capacity(INITIAL_SHAPE_CAPACITY),
            instances: Vec::with_capacity(INITIAL_SHAPE_CAPACITY),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        viewport: Rect,
        physical_width: u32,
        physical_height: u32,
        scale: f32,
    ) -> (usize, usize, usize) {
        self.instances.clear();
        self.batches.clear();
        self.layer_batches.clear();
        self.pending.clear();
        let mut quads = 0;
        let mut shadows = 0;
        for layer in scene.paint_layers() {
            let batch_start = self.batches.len();
            self.pending.clear();
            for item in layer.paint() {
                let PrimitiveRef::Shape(shape) = item.primitive else {
                    continue;
                };
                let instance = match shape {
                    ShapeRef::Quad(index) => {
                        let quad = &layer.quads()[index];
                        let clip = quad.clip.unwrap_or(viewport);
                        let Some(clip) = clip.intersection(viewport) else {
                            continue;
                        };
                        if !quad.rect.intersects(clip) {
                            continue;
                        }
                        quads += 1;
                        quad_instance(quad, clip)
                    }
                    ShapeRef::WavyUnderline(index) => {
                        let underline = &layer.wavy_underlines()[index];
                        let clip = underline.clip.unwrap_or(viewport);
                        let Some(clip) = clip.intersection(viewport) else {
                            continue;
                        };
                        if !underline.rect.intersects(clip) {
                            continue;
                        }
                        quads += 1;
                        wavy_underline_instance(underline, clip)
                    }
                    ShapeRef::Shadow(index) => {
                        let shadow = &layer.shadows()[index];
                        let clip = shadow.clip.unwrap_or(viewport);
                        let Some(clip) = clip.intersection(viewport) else {
                            continue;
                        };
                        let Some(instance) = shadow_instance(shadow, clip) else {
                            continue;
                        };
                        shadows += 1;
                        instance
                    }
                };
                self.pending.push(OrderedShape {
                    order: item.order,
                    instance,
                });
            }
            self.pending.sort_unstable_by_key(|shape| shape.order);
            for shape in &self.pending {
                let index = self.instances.len() as u32;
                self.instances.push(shape.instance);
                if self.batches.len() > batch_start
                    && self
                        .batches
                        .last()
                        .is_some_and(|batch| batch.order == shape.order)
                {
                    self.batches
                        .last_mut()
                        .expect("the previous shape batch exists")
                        .instances
                        .end = index + 1;
                } else {
                    self.batches.push(ShapeBatch {
                        order: shape.order,
                        instances: index..index + 1,
                    });
                }
            }
            self.layer_batches.push(batch_start..self.batches.len());
        }

        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&ViewUniform {
                viewport: [physical_width as f32, physical_height as f32],
                scale,
                _padding: 0.0,
            }),
        );
        self.active_buffer = (self.active_buffer + 1) % BUFFERED_FRAMES;
        let required = self.instances.len().max(1);
        if required > self.instance_capacities[self.active_buffer] {
            let capacity = required.next_power_of_two();
            self.instance_buffers[self.active_buffer] =
                create_shape_instance_buffer(device, capacity);
            self.instance_capacities[self.active_buffer] = capacity;
        }
        if !self.instances.is_empty() {
            queue.write_buffer(
                &self.instance_buffers[self.active_buffer],
                0,
                bytemuck::cast_slice(&self.instances),
            );
        }
        (quads, shadows, self.batches.len())
    }

    fn render_order<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        layer: usize,
        order: u32,
    ) {
        let Some(batches) = self.layer_batches.get(layer) else {
            return;
        };
        let Some(batch) = self.batches[batches.clone()]
            .iter()
            .find(|batch| batch.order == order)
        else {
            return;
        };
        pass.set_pipeline(&self.pipeline.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.instance_buffers[self.active_buffer].slice(..));
        pass.draw(0..6, batch.instances.clone());
    }
}

fn quad_instance(quad: &Quad, clip: Rect) -> ShapeInstance {
    ShapeInstance {
        geometry: rect_array(quad.rect),
        primary: quad.fill.as_array(),
        secondary: quad.border_color.as_array(),
        clip: clip_array(clip),
        params: [
            SHAPE_MODE_QUAD,
            quad.radius.max(0.0),
            quad.border_width.max(0.0),
            0.0,
        ],
        subject: rect_array(quad.rect),
    }
}

fn wavy_underline_instance(underline: &WavyUnderline, clip: Rect) -> ShapeInstance {
    ShapeInstance {
        geometry: rect_array(underline.rect),
        primary: underline.color.as_array(),
        secondary: [0.0; 4],
        clip: clip_array(clip),
        params: [
            SHAPE_MODE_WAVY_UNDERLINE,
            underline.baseline,
            underline.thickness,
            underline.wavelength,
        ],
        subject: [underline.amplitude, 0.0, 0.0, 0.0],
    }
}

fn shadow_instance(shadow: &Shadow, clip: Rect) -> Option<ShapeInstance> {
    let style = shadow.style;
    let blur = style.blur();
    let element_rect = shadow.element_rect;
    let (geometry, subject, mode, subject_radius, element_radius) = if style.is_inset() {
        (
            element_rect,
            dilate_rect(element_rect.translate(style.offset()), -style.spread()),
            SHAPE_MODE_INSET_SHADOW,
            (shadow.radius - style.spread()).max(0.0),
            shadow.radius,
        )
    } else {
        let subject = dilate_rect(element_rect.translate(style.offset()), style.spread());
        if subject.is_empty() {
            return None;
        }
        let margin = blur * SHADOW_SIGMA_PER_BLUR_RADIUS * SHADOW_MARGIN_SIGMAS + 1.0;
        (
            dilate_rect(subject, margin),
            subject,
            SHAPE_MODE_DROP_SHADOW,
            shadow.radius,
            0.0,
        )
    };
    if geometry.is_empty() || !geometry.intersects(clip) {
        return None;
    }
    Some(ShapeInstance {
        geometry: rect_array(geometry),
        primary: style.color().as_array(),
        secondary: [0.0; 4],
        clip: clip_array(clip),
        params: [mode, subject_radius.max(0.0), element_radius.max(0.0), blur],
        subject: rect_array(subject),
    })
}

fn dilate_rect(rect: Rect, amount: f32) -> Rect {
    let left = rect.x - amount;
    let top = rect.y - amount;
    let right = rect.right() + amount;
    let bottom = rect.bottom() + amount;
    let width = (right - left).max(0.0);
    let height = (bottom - top).max(0.0);
    Rect::new(
        if width > 0.0 {
            left
        } else {
            (left + right) * 0.5
        },
        if height > 0.0 {
            top
        } else {
            (top + bottom) * 0.5
        },
        width,
        height,
    )
}

fn rect_array(rect: Rect) -> [f32; 4] {
    [rect.x, rect.y, rect.width, rect.height]
}

fn clip_array(rect: Rect) -> [f32; 4] {
    [rect.x, rect.y, rect.right(), rect.bottom()]
}

fn create_shape_instance_buffer(device: &Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quickgui shape instance buffer"),
        size: (capacity * mem::size_of::<ShapeInstance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[derive(Clone, Debug)]
struct TextLayoutKey {
    content: Arc<str>,
    highlights: Option<Arc<[TextHighlight]>>,
    width: Option<f32>,
    font_size: f32,
    line_height: f32,
    family: FontFamily,
    features: FontFeatures,
    fallbacks: Option<FontFallbacks>,
    weight: glyphon::Weight,
    font_style: GlyphStyle,
    underline: TextUnderline,
    underline_color: Option<UiColor>,
    underline_wavy: bool,
    underline_thickness: f32,
    strikethrough: bool,
    strikethrough_color: Option<UiColor>,
    align: TextAlign,
    wrap: TextWrap,
    text_overflow: Option<TextOverflow>,
    line_clamp: Option<usize>,
    shaping: TextShaping,
    scale: f32,
}

impl PartialEq for TextLayoutKey {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
            && highlights_equal(&self.highlights, &other.highlights)
            && self.width.map(f32::to_bits) == other.width.map(f32::to_bits)
            && self.font_size.to_bits() == other.font_size.to_bits()
            && self.line_height.to_bits() == other.line_height.to_bits()
            && self.family == other.family
            && self.features == other.features
            && self.fallbacks == other.fallbacks
            && self.weight == other.weight
            && self.font_style == other.font_style
            && self.underline == other.underline
            && optional_color_bits(self.underline_color)
                == optional_color_bits(other.underline_color)
            && self.underline_wavy == other.underline_wavy
            && self.underline_thickness.to_bits() == other.underline_thickness.to_bits()
            && self.strikethrough == other.strikethrough
            && optional_color_bits(self.strikethrough_color)
                == optional_color_bits(other.strikethrough_color)
            && self.align == other.align
            && self.wrap == other.wrap
            && self.text_overflow == other.text_overflow
            && self.line_clamp == other.line_clamp
            && self.shaping == other.shaping
            && self.scale.to_bits() == other.scale.to_bits()
    }
}

impl Eq for TextLayoutKey {}

impl TextLayoutKey {
    /// Whether two entries have identical shaping input and differ, if at all, only in the
    /// available layout width.
    ///
    /// Cosmic Text keeps shaped glyph runs when `Buffer::set_size` invalidates layout, so a
    /// stable text id can cheaply reflow during window resizing without rebuilding its content,
    /// attributes, font matches, and glyph shaping.
    fn same_except_width(&self, other: &Self) -> bool {
        self.text_overflow.as_ref().is_none_or(uses_cosmic_ellipsis)
            && other
                .text_overflow
                .as_ref()
                .is_none_or(uses_cosmic_ellipsis)
            && self.text_overflow == other.text_overflow
            && self.content == other.content
            && highlights_equal(&self.highlights, &other.highlights)
            && self.font_size.to_bits() == other.font_size.to_bits()
            && self.line_height.to_bits() == other.line_height.to_bits()
            && self.family == other.family
            && self.features == other.features
            && self.fallbacks == other.fallbacks
            && self.weight == other.weight
            && self.font_style == other.font_style
            && self.underline == other.underline
            && optional_color_bits(self.underline_color)
                == optional_color_bits(other.underline_color)
            && self.underline_wavy == other.underline_wavy
            && self.underline_thickness.to_bits() == other.underline_thickness.to_bits()
            && self.strikethrough == other.strikethrough
            && optional_color_bits(self.strikethrough_color)
                == optional_color_bits(other.strikethrough_color)
            && self.align == other.align
            && self.wrap == other.wrap
            && self.line_clamp == other.line_clamp
            && self.shaping == other.shaping
            && self.scale.to_bits() == other.scale.to_bits()
    }
}

impl Hash for TextLayoutKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.content.hash(state);
        hash_highlights(&self.highlights, state);
        self.width.map(f32::to_bits).hash(state);
        self.font_size.to_bits().hash(state);
        self.line_height.to_bits().hash(state);
        self.family.hash(state);
        self.features.hash(state);
        self.fallbacks.hash(state);
        self.weight.hash(state);
        self.font_style.hash(state);
        self.underline.hash(state);
        optional_color_bits(self.underline_color).hash(state);
        self.underline_wavy.hash(state);
        self.underline_thickness.to_bits().hash(state);
        self.strikethrough.hash(state);
        optional_color_bits(self.strikethrough_color).hash(state);
        self.align.hash(state);
        self.wrap.hash(state);
        self.text_overflow.hash(state);
        self.line_clamp.hash(state);
        self.shaping.hash(state);
        self.scale.to_bits().hash(state);
    }
}

fn highlights_equal(
    left: &Option<Arc<[TextHighlight]>>,
    right: &Option<Arc<[TextHighlight]>>,
) -> bool {
    let left = left.as_deref().unwrap_or_default();
    let right = right.as_deref().unwrap_or_default();
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.range == right.range
                && optional_color_bits(left.style.color) == optional_color_bits(right.style.color)
                && left.style.family == right.style.family
                && left.style.features == right.style.features
                && left.style.fallbacks == right.style.fallbacks
                && left.style.weight == right.style.weight
                && left.style.glyph_style == right.style.glyph_style
                && left.style.underline == right.style.underline
                && optional_color_bits(left.style.underline_color)
                    == optional_color_bits(right.style.underline_color)
                && left.style.underline_wavy == right.style.underline_wavy
                && left.style.underline_thickness.map(f32::to_bits)
                    == right.style.underline_thickness.map(f32::to_bits)
                && left.style.strikethrough == right.style.strikethrough
                && optional_color_bits(left.style.strikethrough_color)
                    == optional_color_bits(right.style.strikethrough_color)
        })
}

fn hash_highlights<H: Hasher>(highlights: &Option<Arc<[TextHighlight]>>, state: &mut H) {
    let highlights = highlights.as_deref().unwrap_or_default();
    highlights.len().hash(state);
    for highlight in highlights {
        highlight.range.start.hash(state);
        highlight.range.end.hash(state);
        optional_color_bits(highlight.style.color).hash(state);
        highlight.style.family.hash(state);
        highlight.style.features.hash(state);
        highlight.style.fallbacks.hash(state);
        highlight.style.weight.hash(state);
        highlight.style.glyph_style.hash(state);
        highlight.style.underline.hash(state);
        optional_color_bits(highlight.style.underline_color).hash(state);
        highlight.style.underline_wavy.hash(state);
        highlight
            .style
            .underline_thickness
            .map(f32::to_bits)
            .hash(state);
        highlight.style.strikethrough.hash(state);
        optional_color_bits(highlight.style.strikethrough_color).hash(state);
    }
}

fn optional_color_bits(color: Option<UiColor>) -> Option<[u32; 4]> {
    color.map(|color| {
        [
            color.r.to_bits(),
            color.g.to_bits(),
            color.b.to_bits(),
            color.a.to_bits(),
        ]
    })
}

#[derive(Clone, Debug)]
struct ProjectedText {
    content: Arc<str>,
    highlights: Option<Arc<[TextHighlight]>>,
    mapping: TextProjection,
}

#[derive(Clone, Debug)]
enum TextProjection {
    Identity,
    Truncated {
        original_len: usize,
        retained: Arc<[ProjectionSpan]>,
        insertion_display: Range<usize>,
        omitted_original: Range<usize>,
    },
}

#[derive(Clone, Debug)]
struct ProjectionSpan {
    display: Range<usize>,
    original: Range<usize>,
}

impl ProjectedText {
    fn identity(content: Arc<str>, highlights: Option<Arc<[TextHighlight]>>) -> Self {
        Self {
            content,
            highlights,
            mapping: TextProjection::Identity,
        }
    }

    fn display_to_original(&self, index: usize) -> usize {
        let mut index = index.min(self.content.len());
        while !self.content.is_char_boundary(index) {
            index -= 1;
        }
        let TextProjection::Truncated {
            original_len,
            retained,
            insertion_display,
            omitted_original,
        } = &self.mapping
        else {
            return index;
        };
        for span in retained.iter() {
            if index >= span.display.start && index <= span.display.end {
                return (span.original.start + index.saturating_sub(span.display.start))
                    .min(span.original.end);
            }
        }
        if index <= insertion_display.start {
            omitted_original.start
        } else if index >= insertion_display.end {
            omitted_original.end.min(*original_len)
        } else {
            let midpoint =
                insertion_display.start + (insertion_display.end - insertion_display.start) / 2;
            if index <= midpoint {
                omitted_original.start
            } else {
                omitted_original.end.min(*original_len)
            }
        }
    }

    fn original_to_display(&self, index: usize) -> usize {
        let TextProjection::Truncated {
            original_len,
            retained,
            insertion_display,
            omitted_original,
        } = &self.mapping
        else {
            return index.min(self.content.len());
        };
        let index = index.min(*original_len);
        for span in retained.iter() {
            if index >= span.original.start && index <= span.original.end {
                return (span.display.start + index.saturating_sub(span.original.start))
                    .min(span.display.end);
            }
        }
        let distance_to_start = index.saturating_sub(omitted_original.start);
        let distance_to_end = omitted_original.end.saturating_sub(index);
        if distance_to_start <= distance_to_end {
            insertion_display.start
        } else {
            insertion_display.end
        }
    }

    fn display_ranges_for_original(&self, range: Range<usize>) -> Vec<Range<usize>> {
        let TextProjection::Truncated {
            original_len,
            retained,
            insertion_display,
            omitted_original,
        } = &self.mapping
        else {
            let start = range.start.min(self.content.len());
            let end = range.end.min(self.content.len()).max(start);
            return (start < end).then_some(start..end).into_iter().collect();
        };
        let start = range.start.min(*original_len);
        let end = range.end.min(*original_len).max(start);
        if start == end {
            return Vec::new();
        }

        let mut projected = Vec::with_capacity(retained.len() + 1);
        for span in retained.iter() {
            let intersection_start = start.max(span.original.start);
            let intersection_end = end.min(span.original.end);
            if intersection_start < intersection_end {
                projected.push(
                    span.display.start + intersection_start - span.original.start
                        ..span.display.start + intersection_end - span.original.start,
                );
            }
        }
        if start < omitted_original.end
            && end > omitted_original.start
            && !insertion_display.is_empty()
        {
            projected.push(insertion_display.clone());
        }
        projected.sort_unstable_by_key(|range| range.start);
        let mut merged: Vec<Range<usize>> = Vec::with_capacity(projected.len());
        for range in projected {
            if let Some(previous) = merged.last_mut()
                && range.start <= previous.end
            {
                previous.end = previous.end.max(range.end);
            } else {
                merged.push(range);
            }
        }
        merged
    }
}

struct TextEntry {
    key: TextLayoutKey,
    buffer: Arc<Buffer>,
    projection: Arc<ProjectedText>,
    last_used_frame: u64,
}

struct SharedTextEntry {
    buffer: Arc<Buffer>,
    projection: Arc<ProjectedText>,
    last_used_frame: u64,
}

struct TextSystem {
    font_system: SharedFontSystem,
    cache: Cache,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderers: Vec<TextRenderer>,
    buffers: HashMap<TextId, TextEntry>,
    shared_buffers: HashMap<TextLayoutKey, SharedTextEntry>,
    colors: HashMap<[u32; 4], glyphon::Color>,
    seen: HashSet<TextId>,
    visible: Vec<VisibleText>,
    batches: Vec<TextBatch>,
    layer_batches: Vec<Range<usize>>,
    eviction_keys: Vec<TextId>,
    shared_eviction_keys: Vec<TextLayoutKey>,
    frame: u64,
}

#[derive(Clone)]
struct VisibleText {
    order: u32,
    buffer: Arc<Buffer>,
    left: f32,
    top: f32,
    bounds: TextBounds,
    color: glyphon::Color,
    opacity: f32,
}

struct TextBatch {
    order: u32,
    visible: Range<usize>,
    renderer: usize,
}

#[derive(Clone, Copy, Debug)]
struct TextHitTest {
    width: f32,
    scale: f32,
    point: Point,
}

impl TextSystem {
    fn new(
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
        font_system: SharedFontSystem,
        shared_cache: Option<&Cache>,
    ) -> Self {
        let swash_cache = SwashCache::new();
        let cache = shared_cache.cloned().unwrap_or_else(|| Cache::new(device));
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let renderer = TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);
        Self {
            font_system,
            cache,
            swash_cache,
            viewport,
            atlas,
            renderers: vec![renderer],
            buffers: HashMap::with_capacity(512),
            shared_buffers: HashMap::with_capacity(512),
            colors: HashMap::with_capacity(MAX_RETAINED_TEXT_COLORS),
            seen: HashSet::with_capacity(128),
            visible: Vec::with_capacity(128),
            batches: Vec::with_capacity(16),
            layer_batches: Vec::with_capacity(4),
            eviction_keys: Vec::new(),
            shared_eviction_keys: Vec::new(),
            frame: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        viewport_rect: Rect,
        physical_width: u32,
        physical_height: u32,
        scale: f32,
    ) -> Result<(usize, usize, usize), RendererError> {
        self.frame = self.frame.wrapping_add(1);
        self.seen.clear();
        self.visible.clear();
        self.batches.clear();
        self.layer_batches.clear();
        let mut text_count = 0;
        let mut reshaped = 0;

        for layer in scene.paint_layers() {
            let start = self.visible.len();
            for item in layer.paint() {
                let PrimitiveRef::Text(index) = item.primitive else {
                    continue;
                };
                let run = &layer.text_runs()[index];
                let clip = run.clip.unwrap_or(viewport_rect);
                let Some(clip) = clip.intersection(viewport_rect) else {
                    continue;
                };
                if !run.bounds.intersects(clip) {
                    continue;
                }
                if !self.seen.insert(run.id) {
                    return Err(RendererError::DuplicateTextId(run.id));
                }
                text_count += 1;
                let color_key = [
                    run.style.color.r.to_bits(),
                    run.style.color.g.to_bits(),
                    run.style.color.b.to_bits(),
                    run.style.color.a.to_bits(),
                ];
                let color = if let Some(color) = self.colors.get(&color_key) {
                    *color
                } else {
                    if self.colors.len() >= MAX_RETAINED_TEXT_COLORS {
                        self.colors.clear();
                    }
                    let rgba = run.style.color.to_srgba8();
                    let color = glyphon::Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3]);
                    self.colors.insert(color_key, color);
                    color
                };
                let bounds = physical_text_bounds(clip, scale);
                let left = run.bounds.x * scale;
                let top = run.bounds.y * scale;
                let run_reshaped = if run.highlights.is_none()
                    && should_fragment_basic_text(&run.content, &run.style)
                {
                    let mut fragment_left = left;
                    let mut fragment_reshaped = false;
                    for fragment in BasicTextFragments::new(&run.content) {
                        let content = Arc::<str>::from(fragment);
                        let (buffer, _projection, was_reshaped) = self
                            .update_shared_text_entry(content, &run.style, None, scale, self.frame);
                        let width = text_buffer_width(&buffer);
                        self.visible.push(VisibleText {
                            order: item.order,
                            buffer,
                            left: fragment_left,
                            top,
                            bounds,
                            color,
                            opacity: run.opacity,
                        });
                        fragment_left += width;
                        fragment_reshaped |= was_reshaped;
                    }
                    fragment_reshaped
                } else {
                    let was_reshaped = self.update_text_entry(
                        run.id,
                        &run.content,
                        &run.style,
                        run.highlights.as_ref(),
                        Some(run.bounds.width),
                        scale,
                        self.frame,
                    );
                    self.visible.push(VisibleText {
                        order: item.order,
                        buffer: Arc::clone(&self.buffers[&run.id].buffer),
                        left,
                        top,
                        bounds,
                        color,
                        opacity: run.opacity,
                    });
                    was_reshaped
                };
                if run_reshaped {
                    reshaped += 1;
                }
            }
            self.visible[start..].sort_unstable_by_key(|text| text.order);
            let batch_start = self.batches.len();
            let mut visible_start = start;
            while visible_start < self.visible.len() {
                let order = self.visible[visible_start].order;
                let visible_end = self.visible[visible_start..]
                    .iter()
                    .position(|text| text.order != order)
                    .map_or(self.visible.len(), |offset| visible_start + offset);
                self.batches.push(TextBatch {
                    order,
                    visible: visible_start..visible_end,
                    renderer: self.batches.len(),
                });
                visible_start = visible_end;
            }
            self.layer_batches.push(batch_start..self.batches.len());
        }

        self.viewport.update(
            queue,
            Resolution {
                width: physical_width,
                height: physical_height,
            },
        );

        let draw_calls = self.batches.len();
        while self.renderers.len() < draw_calls.max(1) {
            self.renderers.push(TextRenderer::new(
                &mut self.atlas,
                device,
                MultisampleState::default(),
                None,
            ));
        }

        let font_system = self.font_system.clone();
        let Self {
            swash_cache,
            viewport,
            atlas,
            renderers,
            visible,
            batches,
            ..
        } = self;
        let mut font_system = font_system.borrow_mut();
        for batch in batches.iter() {
            let areas = visible[batch.visible.clone()].iter().map(|item| TextArea {
                buffer: item.buffer.as_ref(),
                left: item.left,
                top: item.top,
                scale: 1.0,
                bounds: item.bounds,
                default_color: item.color,
                opacity: item.opacity,
                custom_glyphs: &[],
            });
            renderers[batch.renderer].prepare(
                device,
                queue,
                &mut font_system,
                atlas,
                viewport,
                areas,
                swash_cache,
            )?;
        }
        Ok((text_count, reshaped, draw_calls))
    }

    fn measure(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        max_width: Option<f32>,
        scale: f32,
    ) -> Size {
        let width = canonical_text_width(
            style.wrap,
            style.align,
            style.text_overflow.is_some(),
            max_width,
        );
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(id, content, style, highlights, width, scale, next_frame);
        let buffer = &self.buffers[&id].buffer;
        let mut measured_width = 0.0_f32;
        let mut measured_height = 0.0_f32;
        for run in buffer
            .layout_runs()
            .take(style.line_clamp.unwrap_or(usize::MAX))
        {
            measured_width = measured_width.max(run.line_w);
            measured_height = measured_height.max(run.line_top + run.line_height);
        }
        if measured_height == 0.0 {
            measured_height = style.line_height * scale;
        }
        // Cosmic Text's unbounded `line_w` can land exactly on its later wrapping threshold.
        // Reserve one physical pixel so an intrinsically sized single line does not reflow only
        // after Taffy feeds that measured width back into the paint layout.
        let measured_width = (measured_width.ceil() + 1.0) / scale;
        let measured_width = match width {
            Some(width) => measured_width.min(width),
            None => measured_width,
        };
        Size::new(measured_width, measured_height.ceil() / scale)
    }

    #[allow(clippy::too_many_arguments)]
    fn caret_position(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale: f32,
        index: usize,
    ) -> Point {
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(
            id,
            content,
            style,
            highlights,
            Some(width),
            scale,
            next_frame,
        );
        let entry = &self.buffers[&id];
        let display_index = entry.projection.original_to_display(index);
        let cursor = text_cursor_for_byte_index(&entry.projection.content, display_index);
        entry
            .buffer
            .cursor_position(&cursor)
            .map(|(x, y)| Point::new(x / scale, y / scale))
            .unwrap_or(Point::ZERO)
    }

    fn index_for_point(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        hit: TextHitTest,
    ) -> usize {
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(
            id,
            content,
            style,
            highlights,
            Some(hit.width),
            hit.scale,
            next_frame,
        );
        let entry = &self.buffers[&id];
        entry
            .buffer
            .hit(
                hit.point.x.max(0.0) * hit.scale,
                hit.point.y.max(0.0) * hit.scale,
            )
            .map(|cursor| byte_index_for_text_cursor(&entry.projection.content, cursor))
            .map(|index| entry.projection.display_to_original(index))
            .unwrap_or(content.len())
    }

    #[allow(clippy::too_many_arguments)]
    fn selection_rects(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale: f32,
        visible_y: Range<f32>,
        start: usize,
        end: usize,
    ) -> Vec<Rect> {
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(
            id,
            content,
            style,
            highlights,
            Some(width),
            scale,
            next_frame,
        );
        let entry = &self.buffers[&id];
        let display_ranges = entry.projection.display_ranges_for_original(start..end);
        let mut rectangles = Vec::new();
        for display_range in display_ranges {
            let start = text_cursor_for_byte_index(&entry.projection.content, display_range.start);
            let end = text_cursor_for_byte_index(&entry.projection.content, display_range.end);
            for run in entry
                .buffer
                .layout_runs()
                .take(style.line_clamp.unwrap_or(usize::MAX))
            {
                let top = run.line_top / scale;
                let bottom = top + run.line_height / scale;
                if top >= visible_y.end || bottom <= visible_y.start {
                    continue;
                }
                let line_top = run.line_top;
                rectangles.extend(text_selection_spans(&run, start, end).into_iter().map(
                    |(x, width)| {
                        Rect::new(
                            x / scale,
                            line_top / scale,
                            width / scale,
                            run.line_height / scale,
                        )
                    },
                ));
            }
        }
        rectangles
    }

    #[allow(clippy::too_many_arguments)]
    fn text_geometry(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: f32,
        scale: f32,
        visible_y: Range<f32>,
    ) -> StyledTextGeometry {
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(
            id,
            content,
            style,
            highlights,
            Some(width),
            scale,
            next_frame,
        );
        let entry = &self.buffers[&id];
        collect_styled_text_geometry(
            entry.buffer.as_ref(),
            entry.projection.highlights.as_deref().unwrap_or_default(),
            style,
            scale,
            visible_y,
            style.line_clamp,
        )
    }

    fn render_order<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        layer: usize,
        order: u32,
    ) -> Result<(), RendererError> {
        let Some(batches) = self.layer_batches.get(layer) else {
            return Ok(());
        };
        let Some(batch) = self.batches[batches.clone()]
            .iter()
            .find(|batch| batch.order == order)
        else {
            return Ok(());
        };
        self.renderers[batch.renderer].render(&self.atlas, &self.viewport, pass)?;
        Ok(())
    }

    fn finish_frame(&mut self) {
        // Glyphon's prepared renderers own all GPU state needed after command submission. Drop
        // the temporary CPU-buffer references now instead of retaining them until the next
        // `prepare`, which both lowers idle memory ownership and lets a stable text id reclaim its
        // uniquely cached Buffer for width-only reflow during the following layout pass.
        self.visible.clear();
        self.atlas.trim();
        if self.renderers.len() > MAX_RETAINED_TEXT_RENDERERS {
            self.renderers.truncate(MAX_RETAINED_TEXT_RENDERERS);
        }
        let oldest_allowed = self.frame.saturating_sub(TEXT_RETENTION_FRAMES);
        self.eviction_keys.clear();
        self.eviction_keys.extend(
            self.buffers
                .iter()
                .filter_map(|(id, entry)| (entry.last_used_frame < oldest_allowed).then_some(*id)),
        );
        for id in self.eviction_keys.drain(..) {
            self.buffers.remove(&id);
        }

        if self.buffers.len() > MAX_RETAINED_TEXT_AREAS {
            let mut by_age: Vec<_> = self
                .buffers
                .iter()
                .map(|(id, entry)| (*id, entry.last_used_frame))
                .collect();
            by_age.sort_unstable_by_key(|(_, age)| *age);
            let remove_count = self.buffers.len() - MAX_RETAINED_TEXT_AREAS;
            for (id, _) in by_age.into_iter().take(remove_count) {
                self.buffers.remove(&id);
            }
        }

        self.shared_eviction_keys.clear();
        self.shared_eviction_keys.extend(
            self.shared_buffers
                .iter()
                .filter(|(_, entry)| entry.last_used_frame < oldest_allowed)
                .map(|(key, _)| key.clone()),
        );
        for key in self.shared_eviction_keys.drain(..) {
            self.shared_buffers.remove(&key);
        }

        if self.shared_buffers.len() > MAX_RETAINED_TEXT_LAYOUTS {
            let mut by_age: Vec<_> = self
                .shared_buffers
                .iter()
                .map(|(key, entry)| (key.clone(), entry.last_used_frame))
                .collect();
            by_age.sort_unstable_by_key(|(_, age)| *age);
            let remove_count = self.shared_buffers.len() - MAX_RETAINED_TEXT_LAYOUTS;
            for (key, _) in by_age.into_iter().take(remove_count) {
                self.shared_buffers.remove(&key);
            }
        }
    }

    fn retained_counts(&self) -> (usize, usize, usize) {
        (
            self.buffers.len(),
            self.shared_buffers.len(),
            self.renderers.len(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn update_text_entry(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        highlights: Option<&Arc<[TextHighlight]>>,
        width: Option<f32>,
        scale: f32,
        frame: u64,
    ) -> bool {
        // Left-aligned unwrapped glyph layout is independent of paint bounds. Other alignments need
        // the assigned width so shaping, hit testing, selection, and rendering share exact x data.
        let width = canonical_text_width(
            style.wrap,
            style.align,
            style.text_overflow.is_some(),
            width,
        );
        let key = TextLayoutKey {
            content: content.clone(),
            highlights: highlights.cloned(),
            width,
            font_size: style.font_size,
            line_height: style.line_height,
            family: style.family.clone(),
            features: style.features.clone(),
            fallbacks: normalize_fallbacks(style.fallbacks.clone()),
            weight: style.weight,
            font_style: style.font_style,
            underline: style.underline,
            underline_color: style.underline_color,
            underline_wavy: style.underline_wavy,
            underline_thickness: style.underline_thickness,
            strikethrough: style.strikethrough,
            strikethrough_color: style.strikethrough_color,
            align: style.align,
            wrap: style.wrap,
            text_overflow: style.text_overflow.clone(),
            line_clamp: style.line_clamp,
            shaping: style.shaping,
            scale,
        };

        if let Some(buffer) = self.buffers.get_mut(&id).and_then(|entry| {
            entry.last_used_frame = frame;
            (entry.key == key).then(|| Arc::clone(&entry.buffer))
        }) {
            self.shared_buffers
                .entry(key)
                .and_modify(|entry| entry.last_used_frame = frame)
                .or_insert(SharedTextEntry {
                    buffer,
                    projection: Arc::clone(&self.buffers[&id].projection),
                    last_used_frame: frame,
                });
            return false;
        }

        let previous = self.buffers.remove(&id);

        // Prefer a layout already shared by another text id or retained from a recent frame.
        if let Some(entry) = self.shared_buffers.get_mut(&key) {
            entry.last_used_frame = frame;
            self.buffers.insert(
                id,
                TextEntry {
                    key,
                    buffer: Arc::clone(&entry.buffer),
                    projection: Arc::clone(&entry.projection),
                    last_used_frame: frame,
                },
            );
            return false;
        }

        // A stable text id commonly changes only its width while a window is being resized. The
        // per-id entry and shared-layout entry are normally the only owners after `finish_frame`.
        // Reclaim that Buffer and ask Cosmic Text for a relayout; its shaped line cache remains
        // intact. If another id genuinely shares the old layout, keep it immutable and fall back
        // to the normal shared/new-buffer path.
        let reusable = previous.and_then(|entry| {
            let shared = self.shared_buffers.get(&entry.key)?;
            if !Arc::ptr_eq(&entry.buffer, &shared.buffer) || Arc::strong_count(&entry.buffer) != 2
            {
                return None;
            }
            self.shared_buffers.remove(&entry.key);
            if entry.key.same_except_width(&key) {
                Arc::try_unwrap(entry.buffer)
                    .ok()
                    .map(|buffer| (buffer, entry.projection))
            } else {
                // A stable id with a projected/truncated layout cannot reflow the shortened
                // buffer, but its superseded width must not linger for the retention window.
                // Removing the sole shared owner drops that CPU buffer immediately while layouts
                // genuinely shared by another id remain cached.
                None
            }
        });

        let (buffer, projection, reshaped) = if let Some((mut buffer, projection)) = reusable {
            let mut font_system = self.font_system.borrow_mut();
            reflow_text_buffer(&mut buffer, &mut font_system, key.width, scale);
            drop(font_system);
            let buffer = Arc::new(buffer);
            self.shared_buffers.insert(
                key.clone(),
                SharedTextEntry {
                    buffer: Arc::clone(&buffer),
                    projection: Arc::clone(&projection),
                    last_used_frame: frame,
                },
            );
            (buffer, projection, true)
        } else {
            self.update_shared_text_entry_for_key(key.clone(), style, scale, frame)
        };
        self.buffers.insert(
            id,
            TextEntry {
                key,
                buffer,
                projection,
                last_used_frame: frame,
            },
        );
        reshaped
    }

    fn update_shared_text_entry(
        &mut self,
        content: Arc<str>,
        style: &TextStyle,
        width: Option<f32>,
        scale: f32,
        frame: u64,
    ) -> (Arc<Buffer>, Arc<ProjectedText>, bool) {
        let key = TextLayoutKey {
            content,
            highlights: None,
            width: canonical_text_width(
                style.wrap,
                style.align,
                style.text_overflow.is_some(),
                width,
            ),
            font_size: style.font_size,
            line_height: style.line_height,
            family: style.family.clone(),
            features: style.features.clone(),
            fallbacks: normalize_fallbacks(style.fallbacks.clone()),
            weight: style.weight,
            font_style: style.font_style,
            underline: style.underline,
            underline_color: style.underline_color,
            underline_wavy: style.underline_wavy,
            underline_thickness: style.underline_thickness,
            strikethrough: style.strikethrough,
            strikethrough_color: style.strikethrough_color,
            align: style.align,
            wrap: style.wrap,
            text_overflow: style.text_overflow.clone(),
            line_clamp: style.line_clamp,
            shaping: style.shaping,
            scale,
        };
        self.update_shared_text_entry_for_key(key, style, scale, frame)
    }

    fn update_shared_text_entry_for_key(
        &mut self,
        key: TextLayoutKey,
        style: &TextStyle,
        scale: f32,
        frame: u64,
    ) -> (Arc<Buffer>, Arc<ProjectedText>, bool) {
        if let Some(entry) = self.shared_buffers.get_mut(&key) {
            entry.last_used_frame = frame;
            (
                Arc::clone(&entry.buffer),
                Arc::clone(&entry.projection),
                false,
            )
        } else {
            let (buffer, projection) = {
                let mut font_system = self.font_system.borrow_mut();
                let (projection, buffer) = prepare_text_buffer(
                    &mut font_system,
                    &key.content,
                    key.highlights.as_ref(),
                    style,
                    key.width,
                    scale,
                );
                (Arc::new(buffer), Arc::new(projection))
            };
            self.shared_buffers.insert(
                key.clone(),
                SharedTextEntry {
                    buffer: Arc::clone(&buffer),
                    projection: Arc::clone(&projection),
                    last_used_frame: frame,
                },
            );
            (buffer, projection, true)
        }
    }
}

fn create_font_system() -> FontSystem {
    #[cfg(target_os = "macos")]
    {
        let mut font_system = FontSystem::new();
        // NISC18030.ttf advertises `GB18030 Bitmap` as a monospaced CJK face, but its
        // non-scalable metrics produce infinite advances in Cosmic Text and Swash cannot
        // rasterize its glyphs. Cosmic Text otherwise ranks it ahead of the scalable macOS CJK
        // fallbacks for `Family::Monospace`, which invalidates the complete wrapped buffer.
        // Remove the unusable face once, before any font-match cache is populated; this adds no
        // shaping or frame-time work and allows the normal script fallback to select a scalable
        // face such as PingFang.
        let incompatible = font_system
            .db()
            .faces()
            .filter(|face| face.monospaced && face.post_script_name == "GB18030Bitmap")
            .map(|face| face.id)
            .collect::<Vec<_>>();
        if !incompatible.is_empty() {
            let database = font_system.db_mut();
            for id in incompatible {
                database.remove_face(id);
            }
        }
        font_system
    }
    #[cfg(not(target_os = "macos"))]
    {
        FontSystem::new()
    }
}

/// One application-wide mutable shaping database. Winit serializes every renderer callback on the
/// main thread, so sharing it removes per-window system-font metadata without adding locks or
/// cross-thread traffic. Text buffers, glyph atlases, and eviction caches remain window-local.
pub(crate) type SharedFontSystem = Rc<RefCell<FontSystem>>;

pub(crate) fn create_shared_font_system(
    assets: &Assets,
    fonts: &[FontSource],
) -> Result<SharedFontSystem, AssetError> {
    let resolved = resolve_fonts(assets, fonts)?;
    let mut font_system = create_font_system();
    for (index, (source, expected_faces)) in resolved
        .sources
        .iter()
        .zip(resolved.declared_faces)
        .enumerate()
    {
        let loaded =
            font_system
                .db_mut()
                .load_font_source(glyphon::cosmic_text::fontdb::Source::Binary(
                    source.backing(),
                ));
        if loaded.len() != expected_faces {
            return Err(AssetError::InvalidFont { index });
        }
    }
    Ok(Rc::new(RefCell::new(font_system)))
}

fn text_cursor_for_byte_index(content: &str, index: usize) -> Cursor {
    let mut index = index.min(content.len());
    while !content.is_char_boundary(index) {
        index -= 1;
    }

    let mut offset = 0;
    for (line, text) in content.split('\n').enumerate() {
        let line_end = offset + text.len();
        if index <= line_end {
            return Cursor::new(line, index - offset);
        }
        offset = line_end.saturating_add(1);
    }
    Cursor::new(content.lines().count(), 0)
}

/// Return the visual selection spans for one retained layout run.
///
/// Cosmic Text 0.19's `LayoutRun::highlight` treats every line outside a same-line selection as
/// selected because it only compares line equality. Keep its grapheme/BiDi span construction but
/// reject lines outside the ordered cursor interval explicitly.
fn text_selection_spans(
    run: &LayoutRun<'_>,
    cursor_start: Cursor,
    cursor_end: Cursor,
) -> Vec<(f32, f32)> {
    if run.line_i < cursor_start.line || run.line_i > cursor_end.line {
        return Vec::new();
    }
    let selection_start = if run.line_i == cursor_start.line {
        cursor_start.index.min(run.text.len())
    } else {
        0
    };
    let selection_end = if run.line_i == cursor_end.line {
        cursor_end.index.min(run.text.len())
    } else {
        run.text.len()
    };
    if selection_start >= selection_end {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut visual_range: Option<(f32, f32)> = None;
    for glyph in run.glyphs {
        let cluster = &run.text[glyph.start..glyph.end];
        let grapheme_count = cluster.grapheme_indices(true).count().max(1);
        let grapheme_width = glyph.w / grapheme_count as f32;
        let mut grapheme_x = glyph.x;
        for (offset, grapheme) in cluster.grapheme_indices(true) {
            let grapheme_start = glyph.start + offset;
            let grapheme_end = grapheme_start + grapheme.len();
            if grapheme_end > selection_start && grapheme_start < selection_end {
                visual_range = Some(match visual_range {
                    Some((minimum, maximum)) => (
                        minimum.min(grapheme_x),
                        maximum.max(grapheme_x + grapheme_width),
                    ),
                    None => (grapheme_x, grapheme_x + grapheme_width),
                });
            } else if let Some((minimum, maximum)) = visual_range.take()
                && maximum > minimum
            {
                results.push((minimum, maximum - minimum));
            }
            grapheme_x += grapheme_width;
        }
    }
    if let Some((minimum, maximum)) = visual_range
        && maximum > minimum
    {
        results.push((minimum, maximum - minimum));
    }
    results
}

fn byte_index_for_text_cursor(content: &str, cursor: Cursor) -> usize {
    let mut offset = 0;
    for (line, text) in content.split('\n').enumerate() {
        if line == cursor.line {
            let mut index = cursor.index.min(text.len());
            while !text.is_char_boundary(index) {
                index -= 1;
            }
            return offset + index;
        }
        offset = offset.saturating_add(text.len()).saturating_add(1);
    }
    content.len()
}

#[derive(Clone, Copy, Debug)]
enum TruncationPlacement {
    End {
        prefix_end: usize,
    },
    Start {
        suffix_start: usize,
    },
    Middle {
        prefix_end: usize,
        suffix_start: usize,
    },
}

fn prepare_text_buffer(
    font_system: &mut FontSystem,
    content: &Arc<str>,
    highlights: Option<&Arc<[TextHighlight]>>,
    style: &TextStyle,
    width: Option<f32>,
    scale: f32,
) -> (ProjectedText, Buffer) {
    let identity = ProjectedText::identity(content.clone(), highlights.cloned());
    let original_buffer = shape_projected_text(font_system, &identity, style, width, scale);
    let Some(overflow) = style.text_overflow.as_ref() else {
        return (identity, original_buffer);
    };
    if uses_cosmic_ellipsis(overflow) {
        // Cosmic Text retains the original shaped runs and applies Unicode-aware ellipsizing as
        // part of line layout. It therefore handles the common GPUI `…` path in one buffer and
        // can reflow that same buffer in place as the assigned width changes.
        return (identity, original_buffer);
    }
    let Some(width) = width
        .filter(|width| width.is_finite())
        .map(|width| width.max(0.0))
    else {
        return (identity, original_buffer);
    };
    let max_lines = style.line_clamp.unwrap_or(1).max(1);
    if text_buffer_fits(&original_buffer, width, max_lines, scale) {
        return (identity, original_buffer);
    }

    let mut boundaries = content
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if boundaries.first().copied() != Some(0) {
        boundaries.insert(0, 0);
    }
    if boundaries.last().copied() != Some(content.len()) {
        boundaries.push(content.len());
    }
    let graphemes = boundaries.len().saturating_sub(1);
    let affix = match overflow {
        TextOverflow::Truncate(affix)
        | TextOverflow::TruncateStart(affix)
        | TextOverflow::TruncateMiddle(affix) => affix,
    };

    let projection_for_count = |retained: usize| {
        let placement = match overflow {
            TextOverflow::Truncate(_) => TruncationPlacement::End {
                prefix_end: boundaries[retained],
            },
            TextOverflow::TruncateStart(_) => TruncationPlacement::Start {
                suffix_start: boundaries[graphemes - retained],
            },
            TextOverflow::TruncateMiddle(_) => {
                // GPUI biases middle truncation toward the recognizable prefix while still
                // retaining the path/identifier suffix.
                let prefix_graphemes = retained.saturating_mul(2).div_ceil(3);
                let suffix_graphemes = retained.saturating_sub(prefix_graphemes);
                TruncationPlacement::Middle {
                    prefix_end: boundaries[prefix_graphemes],
                    suffix_start: boundaries[graphemes - suffix_graphemes],
                }
            }
        };
        build_text_projection(content, highlights, affix, placement)
    };

    // The original did not fit, so at least one grapheme must be omitted. Each probe shapes an
    // immutable candidate only on this cache miss; stable frames reuse the selected buffer.
    let mut low = 0usize;
    let mut high = graphemes.saturating_sub(1);
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        let projection = projection_for_count(middle);
        let candidate = shape_projected_text(font_system, &projection, style, Some(width), scale);
        if text_buffer_fits(&candidate, width, max_lines, scale) {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let projection = projection_for_count(low);
    let buffer = shape_projected_text(font_system, &projection, style, Some(width), scale);
    (projection, buffer)
}

fn shape_projected_text(
    font_system: &mut FontSystem,
    projection: &ProjectedText,
    style: &TextStyle,
    width: Option<f32>,
    scale: f32,
) -> Buffer {
    let metrics = Metrics::new(style.font_size * scale, style.line_height * scale);
    let mut buffer = Buffer::new(font_system, metrics);
    configure_text_buffer(
        &mut buffer,
        font_system,
        &projection.content,
        style,
        projection.highlights.as_deref(),
        width,
        scale,
    );
    buffer
}

fn text_buffer_fits(buffer: &Buffer, width: f32, max_lines: usize, scale: f32) -> bool {
    let maximum = width.max(0.0) * scale + 0.5;
    let mut lines = 0usize;
    for run in buffer.layout_runs() {
        lines += 1;
        if lines > max_lines || run.line_w > maximum {
            return false;
        }
    }
    true
}

fn build_text_projection(
    content: &Arc<str>,
    highlights: Option<&Arc<[TextHighlight]>>,
    affix: &Arc<str>,
    placement: TruncationPlacement,
) -> ProjectedText {
    let mut display = String::with_capacity(content.len().saturating_add(affix.len()));
    let mut retained = Vec::with_capacity(2);
    let (insertion_display, omitted_original) = match placement {
        TruncationPlacement::End { prefix_end } => {
            display.push_str(&content[..prefix_end]);
            if prefix_end > 0 {
                retained.push(ProjectionSpan {
                    display: 0..prefix_end,
                    original: 0..prefix_end,
                });
            }
            let insertion_start = display.len();
            display.push_str(affix);
            (insertion_start..display.len(), prefix_end..content.len())
        }
        TruncationPlacement::Start { suffix_start } => {
            display.push_str(affix);
            let insertion_display = 0..display.len();
            let suffix_display_start = display.len();
            display.push_str(&content[suffix_start..]);
            if suffix_start < content.len() {
                retained.push(ProjectionSpan {
                    display: suffix_display_start..display.len(),
                    original: suffix_start..content.len(),
                });
            }
            (insertion_display, 0..suffix_start)
        }
        TruncationPlacement::Middle {
            prefix_end,
            suffix_start,
        } => {
            display.push_str(&content[..prefix_end]);
            if prefix_end > 0 {
                retained.push(ProjectionSpan {
                    display: 0..prefix_end,
                    original: 0..prefix_end,
                });
            }
            let insertion_start = display.len();
            display.push_str(affix);
            let insertion_display = insertion_start..display.len();
            let suffix_display_start = display.len();
            display.push_str(&content[suffix_start..]);
            if suffix_start < content.len() {
                retained.push(ProjectionSpan {
                    display: suffix_display_start..display.len(),
                    original: suffix_start..content.len(),
                });
            }
            (insertion_display, prefix_end..suffix_start)
        }
    };
    let projected_highlights = project_text_highlights(
        highlights,
        &retained,
        insertion_display.clone(),
        omitted_original.clone(),
        placement,
    );
    ProjectedText {
        content: Arc::from(display),
        highlights: projected_highlights,
        mapping: TextProjection::Truncated {
            original_len: content.len(),
            retained: retained.into(),
            insertion_display,
            omitted_original,
        },
    }
}

fn project_text_highlights(
    highlights: Option<&Arc<[TextHighlight]>>,
    retained: &[ProjectionSpan],
    insertion_display: Range<usize>,
    omitted_original: Range<usize>,
    placement: TruncationPlacement,
) -> Option<Arc<[TextHighlight]>> {
    let highlights = highlights?.as_ref();
    if highlights.is_empty() {
        return Some(Arc::from([]));
    }
    let mut projected = Vec::with_capacity(highlights.len().min(MAX_TEXT_HIGHLIGHTS));
    for span in retained {
        for highlight in highlights {
            let start = span.original.start.max(highlight.range.start);
            let end = span.original.end.min(highlight.range.end);
            if start < end {
                projected.push(TextHighlight {
                    range: span.display.start + start - span.original.start
                        ..span.display.start + end - span.original.start,
                    style: highlight.style.clone(),
                });
            }
        }
    }

    if !insertion_display.is_empty()
        && let Some(style) = affix_highlight_style(highlights, &omitted_original, placement)
    {
        projected.push(TextHighlight {
            range: insertion_display,
            style,
        });
    }
    projected.sort_unstable_by_key(|highlight| highlight.range.start);

    let mut merged: Vec<TextHighlight> = Vec::with_capacity(projected.len());
    for highlight in projected {
        if let Some(previous) = merged.last_mut()
            && previous.range.end == highlight.range.start
            && previous.style == highlight.style
        {
            previous.range.end = highlight.range.end;
        } else if merged.len() < MAX_TEXT_HIGHLIGHTS {
            merged.push(highlight);
        }
    }
    Some(merged.into())
}

fn affix_highlight_style(
    highlights: &[TextHighlight],
    omitted: &Range<usize>,
    placement: TruncationPlacement,
) -> Option<crate::HighlightStyle> {
    let prefer_previous = !matches!(placement, TruncationPlacement::Start { .. });
    if prefer_previous {
        highlights
            .iter()
            .rev()
            .find(|highlight| {
                highlight.range.start < omitted.start && highlight.range.end >= omitted.start
            })
            .or_else(|| {
                highlights.iter().find(|highlight| {
                    highlight.range.start <= omitted.start && highlight.range.end > omitted.start
                })
            })
            .map(|highlight| highlight.style.clone())
    } else {
        highlights
            .iter()
            .find(|highlight| {
                highlight.range.start <= omitted.end && highlight.range.end > omitted.end
            })
            .or_else(|| {
                highlights.iter().rev().find(|highlight| {
                    highlight.range.start < omitted.end && highlight.range.end >= omitted.end
                })
            })
            .map(|highlight| highlight.style.clone())
    }
}

fn should_fragment_basic_text(content: &str, style: &TextStyle) -> bool {
    style.shaping == TextShaping::Basic
        && style.wrap == TextWrap::None
        && style.align == TextAlign::Left
        && style.text_overflow.is_none()
        && style.line_clamp.is_none()
        && content.len() >= BASIC_FRAGMENT_MIN_BYTES
        && content
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
        && content.bytes().any(|byte| byte.is_ascii_digit())
}

struct BasicTextFragments<'a> {
    content: &'a str,
    cursor: usize,
}

impl<'a> BasicTextFragments<'a> {
    fn new(content: &'a str) -> Self {
        Self { content, cursor: 0 }
    }
}

impl<'a> Iterator for BasicTextFragments<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self.content.as_bytes();
        let start = self.cursor;
        let first = *bytes.get(start)?;
        if first.is_ascii_digit() {
            self.cursor += 1;
            if self.cursor < bytes.len() && bytes[self.cursor].is_ascii_digit() {
                self.cursor += 1;
            }
        } else {
            self.cursor += 1;
            while self.cursor < bytes.len() && !bytes[self.cursor].is_ascii_digit() {
                self.cursor += 1;
            }
        }
        Some(&self.content[start..self.cursor])
    }
}

fn text_buffer_width(buffer: &Buffer) -> f32 {
    buffer
        .layout_runs()
        .next()
        .map_or(0.0, |layout| layout.line_w)
}

fn canonical_text_width(
    wrap: TextWrap,
    align: TextAlign,
    has_text_overflow: bool,
    width: Option<f32>,
) -> Option<f32> {
    match (wrap, align, has_text_overflow) {
        (TextWrap::None, TextAlign::Left, false) => None,
        (TextWrap::None | TextWrap::Word | TextWrap::Glyph, _, _) => width,
    }
}

fn collect_styled_text_geometry(
    buffer: &Buffer,
    highlights: &[TextHighlight],
    style: &TextStyle,
    scale: f32,
    visible_y: Range<f32>,
    line_clamp: Option<usize>,
) -> StyledTextGeometry {
    const MAX_TEXT_GEOMETRY_RECTS: usize = MAX_TEXT_HIGHLIGHTS * 4;

    let scale = scale.max(f32::EPSILON);
    let mut backgrounds = Vec::with_capacity(highlights.len().min(32));
    let mut decorations = Vec::with_capacity(highlights.len().min(32));
    for run in buffer.layout_runs().take(line_clamp.unwrap_or(usize::MAX)) {
        let line_top = run.line_top / scale;
        let line_bottom = (run.line_top + run.line_height) / scale;
        if line_top >= visible_y.end || line_bottom <= visible_y.start {
            continue;
        }

        let mut start = 0;
        while start < run.glyphs.len() {
            let metadata = run.glyphs[start].metadata;
            let mut end = start + 1;
            while end < run.glyphs.len() && run.glyphs[end].metadata == metadata {
                end += 1;
            }
            if let Some(background) = metadata
                .checked_sub(1)
                .and_then(|index| highlights.get(index))
                .and_then(|highlight| highlight.style.background)
                .filter(|color| color.a > 0.0)
            {
                let mut left = f32::INFINITY;
                let mut right = f32::NEG_INFINITY;
                for glyph in &run.glyphs[start..end] {
                    left = left.min(glyph.x);
                    right = right.max(glyph.x + glyph.w);
                }
                if right > left && backgrounds.len() < MAX_TEXT_GEOMETRY_RECTS {
                    backgrounds.push(TextPaintRect {
                        rect: Rect::new(
                            left / scale,
                            line_top,
                            (right - left) / scale,
                            run.line_height / scale,
                        ),
                        color: background,
                        kind: TextPaintKind::Solid,
                    });
                }
            }
            start = end;
        }
        collect_decoration_geometry(
            &run,
            highlights,
            style,
            scale,
            MAX_TEXT_GEOMETRY_RECTS,
            &mut decorations,
        );
    }
    StyledTextGeometry {
        backgrounds,
        decorations,
    }
}

#[derive(Clone, Copy)]
struct ResolvedUnderlinePaint {
    wavy: bool,
    thickness: f32,
}

fn resolved_underline_paint(
    metadata: usize,
    highlights: &[TextHighlight],
    style: &TextStyle,
) -> ResolvedUnderlinePaint {
    if let Some(highlight) = metadata
        .checked_sub(1)
        .and_then(|index| highlights.get(index))
        .filter(|highlight| highlight.style.underline != TextUnderline::None)
    {
        return ResolvedUnderlinePaint {
            wavy: highlight
                .style
                .underline_wavy
                .unwrap_or(style.underline_wavy),
            thickness: highlight
                .style
                .underline_thickness
                .unwrap_or(style.underline_thickness),
        };
    }
    ResolvedUnderlinePaint {
        wavy: style.underline_wavy,
        thickness: style.underline_thickness,
    }
}

fn collect_decoration_geometry(
    run: &LayoutRun<'_>,
    highlights: &[TextHighlight],
    style: &TextStyle,
    scale: f32,
    limit: usize,
    decorations: &mut Vec<TextPaintRect>,
) {
    for span in run.decorations {
        let mut start = span.glyph_range.start;
        while start < span.glyph_range.end && decorations.len() < limit {
            let metadata = run.glyphs[start].metadata;
            let mut end = start + 1;
            while end < span.glyph_range.end && run.glyphs[end].metadata == metadata {
                end += 1;
            }
            collect_decoration_group(
                run,
                span,
                start,
                end,
                metadata,
                highlights,
                style,
                scale,
                limit,
                decorations,
            );
            start = end;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_decoration_group(
    run: &LayoutRun<'_>,
    span: &DecorationSpan,
    start: usize,
    end: usize,
    metadata: usize,
    highlights: &[TextHighlight],
    style: &TextStyle,
    scale: f32,
    limit: usize,
    decorations: &mut Vec<TextPaintRect>,
) {
    let glyphs = &run.glyphs[start..end];
    let Some((x, width)) = decoration_horizontal_span(glyphs) else {
        return;
    };
    let data = &span.data;
    let text_decoration = &data.text_decoration;
    let fallback_color = glyphs[0]
        .color_opt
        .unwrap_or_else(|| glyph_color(style.color));
    let font_size = glyphs[0].font_size;

    if !matches!(text_decoration.underline, GlyphUnderlineStyle::None) {
        let underline = resolved_underline_paint(metadata, highlights, style);
        let thickness = (underline.thickness.max(0.0) * scale).max(0.0);
        if thickness > 0.0 {
            let color = ui_color(
                text_decoration
                    .underline_color_opt
                    .unwrap_or(fallback_color),
            );
            let y = run.line_y - data.underline_metrics.offset * font_size;
            push_underline_geometry(
                decorations,
                limit,
                x,
                y,
                width,
                thickness,
                color,
                underline.wavy,
                scale,
            );
            if matches!(text_decoration.underline, GlyphUnderlineStyle::Double) {
                push_underline_geometry(
                    decorations,
                    limit,
                    x,
                    y + thickness * 2.0,
                    width,
                    thickness,
                    color,
                    underline.wavy,
                    scale,
                );
            }
        }
    }

    if text_decoration.strikethrough && decorations.len() < limit {
        let color = ui_color(
            text_decoration
                .strikethrough_color_opt
                .unwrap_or(fallback_color),
        );
        let thickness = (data.strikethrough_metrics.thickness * font_size)
            .max(1.0)
            .ceil();
        let y = run.line_y - data.strikethrough_metrics.offset * font_size;
        push_solid_decoration(decorations, limit, x, y, width, thickness, color, scale);
    }
}

fn decoration_horizontal_span(glyphs: &[LayoutGlyph]) -> Option<(f32, f32)> {
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    for glyph in glyphs {
        left = left.min(glyph.x);
        right = right.max(glyph.x + glyph.w);
    }
    let width = right - left;
    (width > 0.0).then_some((left, width))
}

#[allow(clippy::too_many_arguments)]
fn push_underline_geometry(
    decorations: &mut Vec<TextPaintRect>,
    limit: usize,
    x: f32,
    y: f32,
    width: f32,
    thickness: f32,
    color: UiColor,
    wavy: bool,
    scale: f32,
) {
    if !wavy {
        push_solid_decoration(
            decorations,
            limit,
            x,
            y,
            width,
            thickness.ceil(),
            color,
            scale,
        );
        return;
    }
    if decorations.len() >= limit || color.a <= 0.0 {
        return;
    }

    let logical_x = x.floor() / scale;
    let logical_width = width.floor() / scale;
    let logical_thickness = thickness / scale;
    if logical_width <= 0.0 || logical_thickness <= 0.0 {
        return;
    }
    let baseline = y.floor() / scale + logical_thickness * 0.5;
    let amplitude = (logical_thickness * 1.5).clamp(1.0, 4.0);
    let wavelength = (amplitude * 4.0).max(4.0);
    let antialias_margin = scale.recip();
    decorations.push(TextPaintRect {
        rect: Rect::new(
            logical_x,
            baseline - amplitude - logical_thickness * 0.5 - antialias_margin,
            logical_width,
            amplitude * 2.0 + logical_thickness + antialias_margin * 2.0,
        ),
        color,
        kind: TextPaintKind::WavyUnderline {
            baseline,
            amplitude,
            thickness: logical_thickness,
            wavelength,
        },
    });
}

#[allow(clippy::too_many_arguments)]
fn push_solid_decoration(
    decorations: &mut Vec<TextPaintRect>,
    limit: usize,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: UiColor,
    scale: f32,
) {
    let x = x as i32;
    let y = y as i32;
    let width = width as u32;
    let height = height as u32;
    if decorations.len() >= limit || width == 0 || height == 0 || color.a <= 0.0 {
        return;
    }
    decorations.push(TextPaintRect {
        rect: Rect::new(
            x as f32 / scale,
            y as f32 / scale,
            width as f32 / scale,
            height as f32 / scale,
        ),
        color,
        kind: TextPaintKind::Solid,
    });
}

fn configure_text_buffer(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    content: &str,
    style: &TextStyle,
    highlights: Option<&[TextHighlight]>,
    width: Option<f32>,
    scale: f32,
) {
    let metrics = Metrics::new(style.font_size * scale, style.line_height * scale);
    buffer.set_metrics_and_size(metrics, width.map(|value| value * scale), None);
    buffer.set_wrap(match style.wrap {
        TextWrap::None => Wrap::None,
        TextWrap::Word => Wrap::Word,
        TextWrap::Glyph => Wrap::Glyph,
    });
    buffer.set_ellipsize(cosmic_ellipsize(style));
    let mut attrs = Attrs::new()
        .family(glyph_family(&style.family))
        .weight(style.weight)
        .style(style.font_style)
        .font_features(style.features.cosmic());
    if let Some(fallbacks) = style.fallbacks.as_ref() {
        attrs = attrs.font_fallbacks(fallbacks.cosmic());
    }
    attrs = match style.underline {
        TextUnderline::None => attrs,
        TextUnderline::Single => attrs.underline(GlyphUnderlineStyle::Single),
        TextUnderline::Double => attrs.underline(GlyphUnderlineStyle::Double),
    };
    if let Some(color) = style.underline_color {
        attrs = attrs.underline_color(glyph_color(color));
    }
    if style.strikethrough {
        attrs = attrs.strikethrough();
    }
    if let Some(color) = style.strikethrough_color {
        attrs = attrs.strikethrough_color(glyph_color(color));
    }
    let shaping = match style.shaping {
        TextShaping::Advanced => GlyphShaping::Advanced,
        TextShaping::Basic => GlyphShaping::Basic,
    };
    if let Some(highlights) = highlights.filter(|highlights| !highlights.is_empty()) {
        let mut spans = Vec::with_capacity(highlights.len() * 2 + 1);
        let mut cursor = 0;
        for (index, highlight) in highlights.iter().enumerate() {
            if cursor < highlight.range.start {
                spans.push((&content[cursor..highlight.range.start], attrs.clone()));
            }
            let mut highlight_attrs = attrs
                .clone()
                .metadata(index + 1)
                .family(
                    highlight
                        .style
                        .family
                        .as_ref()
                        .map_or_else(|| glyph_family(&style.family), glyph_family),
                )
                .weight(highlight.style.weight.unwrap_or(style.weight))
                .style(highlight.style.glyph_style.unwrap_or(style.font_style));
            highlight_attrs.font_features = highlight
                .style
                .features
                .as_ref()
                .unwrap_or(&style.features)
                .cosmic();
            highlight_attrs.font_fallbacks = match highlight.style.fallbacks.as_ref() {
                Some(fallbacks) => fallbacks.as_ref().map(FontFallbacks::cosmic),
                None => style.fallbacks.as_ref().map(FontFallbacks::cosmic),
            };
            if let Some(color) = highlight.style.color {
                highlight_attrs = highlight_attrs.color(glyph_color(color));
            }
            highlight_attrs = match highlight.style.underline {
                TextUnderline::None => highlight_attrs,
                TextUnderline::Single => highlight_attrs.underline(GlyphUnderlineStyle::Single),
                TextUnderline::Double => highlight_attrs.underline(GlyphUnderlineStyle::Double),
            };
            if let Some(color) = highlight.style.underline_color {
                highlight_attrs = highlight_attrs.underline_color(glyph_color(color));
            }
            if highlight.style.strikethrough {
                highlight_attrs = highlight_attrs.strikethrough();
            }
            if let Some(color) = highlight.style.strikethrough_color {
                highlight_attrs = highlight_attrs.strikethrough_color(glyph_color(color));
            }
            spans.push((&content[highlight.range.clone()], highlight_attrs));
            cursor = highlight.range.end;
        }
        if cursor < content.len() {
            spans.push((&content[cursor..], attrs.clone()));
        }
        buffer.set_rich_text(spans, &attrs, shaping, Some(glyph_alignment(style.align)));
    } else {
        buffer.set_text(content, &attrs, shaping, Some(glyph_alignment(style.align)));
    }
    buffer.shape_until_scroll(font_system, false);
}

fn uses_cosmic_ellipsis(overflow: &TextOverflow) -> bool {
    match overflow {
        TextOverflow::Truncate(affix)
        | TextOverflow::TruncateStart(affix)
        | TextOverflow::TruncateMiddle(affix) => affix.as_ref() == "…",
    }
}

fn cosmic_ellipsize(style: &TextStyle) -> Ellipsize {
    let limit = EllipsizeHeightLimit::Lines(style.line_clamp.unwrap_or(1).max(1));
    match style.text_overflow.as_ref() {
        Some(TextOverflow::Truncate(affix)) if affix.as_ref() == "…" => Ellipsize::End(limit),
        Some(TextOverflow::TruncateStart(affix)) if affix.as_ref() == "…" => {
            Ellipsize::Start(limit)
        }
        Some(TextOverflow::TruncateMiddle(affix)) if affix.as_ref() == "…" => {
            Ellipsize::Middle(limit)
        }
        None
        | Some(
            TextOverflow::Truncate(_)
            | TextOverflow::TruncateStart(_)
            | TextOverflow::TruncateMiddle(_),
        ) => Ellipsize::None,
    }
}

/// Reflow a configured buffer while preserving its shaped glyph-line cache.
fn reflow_text_buffer(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    width: Option<f32>,
    scale: f32,
) {
    buffer.set_size(width.map(|value| value * scale), None);
    buffer.shape_until_scroll(font_system, false);
}

fn glyph_alignment(align: TextAlign) -> GlyphAlign {
    match align {
        TextAlign::Left => GlyphAlign::Left,
        TextAlign::Center => GlyphAlign::Center,
        TextAlign::Right => GlyphAlign::Right,
        TextAlign::Justify => GlyphAlign::Justified,
    }
}

fn glyph_color(color: UiColor) -> GlyphColor {
    let [red, green, blue, alpha] = color.to_srgba8();
    GlyphColor::rgba(red, green, blue, alpha)
}

fn ui_color(color: GlyphColor) -> UiColor {
    let [red, green, blue, alpha] = color.as_rgba();
    UiColor::rgba8(red, green, blue, alpha)
}

fn physical_text_bounds(rect: Rect, scale: f32) -> TextBounds {
    TextBounds {
        left: saturating_i32((rect.x * scale).floor()),
        top: saturating_i32((rect.y * scale).floor()),
        right: saturating_i32((rect.right() * scale).ceil()),
        bottom: saturating_i32((rect.bottom() * scale).ceil()),
    }
}

fn saturating_i32(value: f32) -> i32 {
    value.clamp(i32::MIN as f32, i32::MAX as f32) as i32
}

impl From<PerformanceProfile> for wgpu::PowerPreference {
    fn from(value: PerformanceProfile) -> Self {
        match value {
            PerformanceProfile::Balanced => wgpu::PowerPreference::None,
            PerformanceProfile::LowPower => wgpu::PowerPreference::LowPower,
            PerformanceProfile::HighPerformance => wgpu::PowerPreference::HighPerformance,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BoxShadow, Color, CustomShader, CustomShaderPrimitive, FontFeatureTag, HighlightStyle,
        Image, ImagePrimitive, PathBuilder, PathPrimitive, StyledText, Svg, SvgPrimitive,
    };

    fn fixture_font_system() -> FontSystem {
        let mut database = glyphon::cosmic_text::fontdb::Database::new();
        database
            .load_font_data(include_bytes!("../tests/fixtures/fonts/Inter-Regular.ttf").to_vec());
        database
            .load_font_data(include_bytes!("../tests/fixtures/fonts/NotoSansHebrew.ttf").to_vec());
        FontSystem::new_with_locale_and_db("en-US".to_owned(), database)
    }

    #[test]
    fn application_font_system_handle_is_shared_without_a_lock() {
        let fonts = create_shared_font_system(&Assets::default(), &[]).unwrap();
        assert!(Rc::ptr_eq(&fonts, &fonts.clone()));
    }

    #[test]
    fn advanced_shaping_uses_ordered_custom_fallbacks_before_platform_fonts() {
        fn shaped_families(buffer: &Buffer, font_system: &FontSystem) -> Vec<(usize, String)> {
            buffer
                .layout_runs()
                .flat_map(|run| run.glyphs.iter())
                .map(|glyph| {
                    (
                        glyph.start,
                        font_system.db().face(glyph.font_id).unwrap().families[0]
                            .0
                            .clone(),
                    )
                })
                .collect()
        }

        let mut font_system = fixture_font_system();
        let missing_then_noto = TextStyle::new(16.0, Color::WHITE)
            .family(FontFamily::named("Missing QuickGUI Test Primary"))
            .font_fallbacks(FontFallbacks::from_fonts([
                "Missing QuickGUI Test Fallback",
                "Noto Sans Hebrew",
                "Inter",
            ]));
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 22.0));
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            "A",
            &missing_then_noto,
            None,
            None,
            1.0,
        );
        assert_eq!(
            shaped_families(&buffer, &font_system),
            vec![(0, "Noto Sans Hebrew".to_owned())]
        );

        let inter_then_noto = TextStyle::new(16.0, Color::WHITE)
            .family(FontFamily::named("Missing QuickGUI Test Primary"))
            .font_fallbacks(FontFallbacks::from_fonts(["Inter", "Noto Sans Hebrew"]));
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            "Aא",
            &inter_then_noto,
            None,
            None,
            1.0,
        );
        let families = shaped_families(&buffer, &font_system);
        assert!(families.contains(&(0, "Inter".to_owned())));
        assert!(families.contains(&(1, "Noto Sans Hebrew".to_owned())));
    }

    #[test]
    fn opentype_features_affect_plain_and_rich_text_shaping() {
        fn glyph_ids(
            font_system: &mut FontSystem,
            style: &TextStyle,
            highlights: Option<&[TextHighlight]>,
            content: &str,
        ) -> Vec<(usize, u16)> {
            let mut buffer = Buffer::new(font_system, Metrics::new(16.0, 22.0));
            configure_text_buffer(
                &mut buffer,
                font_system,
                content,
                style,
                highlights,
                None,
                1.0,
            );
            buffer
                .layout_runs()
                .flat_map(|run| run.glyphs.iter())
                .map(|glyph| (glyph.metadata, glyph.glyph_id))
                .collect()
        }

        let mut font_system = fixture_font_system();
        let enabled = TextStyle::new(16.0, Color::WHITE).family(FontFamily::named("Inter"));
        let disabled = enabled
            .clone()
            .font_features(FontFeatures::new().disable(FontFeatureTag::CONTEXTUAL_ALTERNATES));
        let enabled_ids = glyph_ids(&mut font_system, &enabled, None, "|>")
            .into_iter()
            .map(|(_, glyph_id)| glyph_id)
            .collect::<Vec<_>>();
        let disabled_ids = glyph_ids(&mut font_system, &disabled, None, "|>")
            .into_iter()
            .map(|(_, glyph_id)| glyph_id)
            .collect::<Vec<_>>();
        assert_ne!(enabled_ids, disabled_ids);

        let highlighted = StyledText::new("|>|>").with_highlights([(
            2..4,
            HighlightStyle::default()
                .font_features(FontFeatures::new().disable(FontFeatureTag::CONTEXTUAL_ALTERNATES)),
        )]);
        let rich_ids = glyph_ids(
            &mut font_system,
            &enabled,
            Some(highlighted.highlights()),
            highlighted.content(),
        );
        assert_eq!(
            rich_ids
                .iter()
                .filter_map(|(metadata, glyph_id)| (*metadata == 0).then_some(*glyph_id))
                .collect::<Vec<_>>(),
            enabled_ids
        );
        assert_eq!(
            rich_ids
                .iter()
                .filter_map(|(metadata, glyph_id)| (*metadata == 1).then_some(*glyph_id))
                .collect::<Vec<_>>(),
            disabled_ids
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_opentype_bytes_register_in_the_application_font_database() {
        let Ok(bytes) = std::fs::read("/System/Library/Fonts/SFNSMono.ttf") else {
            return;
        };
        create_shared_font_system(&Assets::default(), &[FontSource::from(bytes)]).unwrap();
    }

    #[test]
    fn surface_format_prefers_srgb() {
        assert_eq!(
            preferred_surface_format(&[TextureFormat::Rgba8Unorm, TextureFormat::Bgra8UnormSrgb]),
            Some(TextureFormat::Bgra8UnormSrgb)
        );
    }

    #[test]
    fn surface_alpha_modes_keep_opaque_and_transparent_paths_explicit() {
        let modes = [
            CompositeAlphaMode::Opaque,
            CompositeAlphaMode::PostMultiplied,
            CompositeAlphaMode::PreMultiplied,
        ];
        assert_eq!(
            opaque_surface_alpha_mode(&modes),
            Some(CompositeAlphaMode::Opaque)
        );
        assert_eq!(
            transparent_surface_alpha_mode(&modes),
            Some(CompositeAlphaMode::PreMultiplied)
        );
        assert_eq!(
            transparent_surface_alpha_mode(&[
                CompositeAlphaMode::Opaque,
                CompositeAlphaMode::PostMultiplied,
            ]),
            Some(CompositeAlphaMode::PostMultiplied)
        );
        assert_eq!(
            transparent_surface_alpha_mode(&[CompositeAlphaMode::Opaque]),
            None
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn offscreen_gpu_pipelines_apply_one_shared_subtree_opacity() {
        let font_system = create_shared_font_system(&Assets::default(), &[]).unwrap();
        let mut renderer = pollster::block_on(OffscreenRenderer::new(
            PerformanceProfile::Balanced,
            font_system,
        ))
        .unwrap();
        let mut scene = Scene::new();
        scene.clear(Color::BLACK);
        let previous = scene.multiply_opacity(0.5);

        scene.push_quad(Quad::new(Rect::new(0.0, 0.0, 16.0, 16.0), Color::WHITE));
        let image = Image::from_rgba(1, 1, Arc::<[u8]>::from([255, 255, 255, 255])).unwrap();
        scene.push_image(ImagePrimitive::new(image, Rect::new(16.0, 0.0, 16.0, 16.0)));
        let svg = Svg::from_svg(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="white"/></svg>"#,
        )
        .unwrap();
        scene.push_svg(SvgPrimitive::new(
            svg,
            Rect::new(32.0, 0.0, 16.0, 16.0),
            Color::WHITE,
        ));
        let mut path = PathBuilder::fill();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(16.0, 0.0));
        path.line_to(Point::new(16.0, 16.0));
        path.line_to(Point::new(0.0, 16.0));
        path.close();
        scene.push_path(
            PathPrimitive::new(path.build().unwrap(), Color::WHITE).translate(48.0, 0.0),
        );
        let shader = CustomShader::new(
            "fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> { return vec4<f32>(1.0, 1.0, 1.0, 1.0 + input.uv.x * 0.0); }",
        )
        .unwrap();
        scene.push_custom_shader(CustomShaderPrimitive::new(
            shader,
            Rect::new(64.0, 0.0, 16.0, 16.0),
        ));
        scene.restore_opacity(previous);
        scene.finish();

        let snapshot = renderer
            .render_to_snapshot(&scene, Size::new(80.0, 16.0), 1.0)
            .unwrap();
        let samples = [8, 24, 40, 56, 72].map(|x| snapshot.pixel(x, 8).unwrap());
        for sample in samples {
            assert!(sample[0] > 170 && sample[0] < 205, "{sample:?}");
            assert_eq!(sample[0], sample[1]);
            assert_eq!(sample[1], sample[2]);
            assert_eq!(sample[3], 255);
        }
        for sample in samples.windows(2) {
            assert!(sample[0][0].abs_diff(sample[1][0]) <= 2, "{samples:?}");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn oversized_target_keeps_primitive_clips_in_logical_viewport_space() {
        let font_system = create_shared_font_system(&Assets::default(), &[]).unwrap();
        let mut renderer = pollster::block_on(OffscreenRenderer::new(
            PerformanceProfile::Balanced,
            font_system,
        ))
        .unwrap();
        let mut scene = Scene::new();
        scene.clear(Color::BLACK);

        // Model the live-resize path: layout and projection use a 64px current viewport while
        // Metal retains a 128px render target. Drawable-pixel clipping would incorrectly discard
        // every primitive below y=32 logical after projection doubles its target position.
        scene.push_quad(Quad::new(Rect::new(0.0, 40.0, 16.0, 16.0), Color::WHITE));
        let image = Image::from_rgba(1, 1, Arc::<[u8]>::from([255, 255, 255, 255])).unwrap();
        scene.push_image(ImagePrimitive::new(
            image,
            Rect::new(16.0, 40.0, 16.0, 16.0),
        ));
        let svg = Svg::from_svg(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="white"/></svg>"#,
        )
        .unwrap();
        scene.push_svg(SvgPrimitive::new(
            svg,
            Rect::new(32.0, 40.0, 16.0, 16.0),
            Color::WHITE,
        ));
        let shader = CustomShader::new(
            "fn quickgui_fragment(input: QuickGuiShaderInput) -> vec4<f32> { return vec4<f32>(1.0, 1.0, 1.0, 1.0 + input.uv.x * 0.0); }",
        )
        .unwrap();
        scene.push_custom_shader(CustomShaderPrimitive::new(
            shader,
            Rect::new(48.0, 40.0, 16.0, 16.0),
        ));
        scene.finish();

        let snapshot = renderer
            .render_to_snapshot_with_target(&scene, Size::new(64.0, 64.0), 1.0, (128, 128))
            .unwrap();
        let samples = [16, 48, 80, 112].map(|x| snapshot.pixel(x, 96).unwrap());
        for sample in samples {
            assert!(sample[0] > 240, "{sample:?}");
            assert!(sample[1] > 240, "{sample:?}");
            assert!(sample[2] > 240, "{sample:?}");
            assert_eq!(sample[3], 255);
        }
    }

    #[test]
    fn surface_capacity_grows_geometrically_and_never_shrinks() {
        assert_eq!(
            grow_surface_capacity((900, 640), (760, 520), 16_384),
            (900, 640)
        );
        assert_eq!(
            grow_surface_capacity((900, 640), (920, 650), 16_384),
            (1_350, 960)
        );
        assert_eq!(
            grow_surface_capacity((1_350, 960), (1_180, 720), 16_384),
            (1_350, 960)
        );
        assert_eq!(
            grow_surface_capacity((12_000, 12_000), (16_000, 15_000), 16_384),
            (16_384, 16_384)
        );
    }

    #[test]
    fn text_clip_rounds_outward() {
        let bounds = physical_text_bounds(Rect::new(1.1, 2.2, 3.3, 4.4), 2.0);
        assert_eq!(
            (bounds.left, bounds.top, bounds.right, bounds.bottom),
            (2, 4, 9, 14)
        );
    }

    #[test]
    fn text_layout_width_is_retained_only_when_wrapping_or_alignment_needs_it() {
        assert_eq!(
            canonical_text_width(TextWrap::None, TextAlign::Left, false, Some(640.0)),
            None
        );
        assert_eq!(
            canonical_text_width(TextWrap::None, TextAlign::Center, false, Some(640.0)),
            Some(640.0)
        );
        assert_eq!(
            canonical_text_width(TextWrap::None, TextAlign::Right, false, Some(640.0)),
            Some(640.0)
        );
        assert_eq!(
            canonical_text_width(TextWrap::None, TextAlign::Justify, false, Some(640.0)),
            Some(640.0)
        );
        assert_eq!(
            canonical_text_width(TextWrap::Word, TextAlign::Left, false, Some(640.0)),
            Some(640.0)
        );
        assert_eq!(
            canonical_text_width(TextWrap::Glyph, TextAlign::Left, false, Some(640.0)),
            Some(640.0)
        );
        assert_eq!(
            canonical_text_width(TextWrap::None, TextAlign::Left, true, Some(640.0)),
            Some(640.0)
        );
    }

    #[test]
    fn text_layout_keys_allow_reflow_only_for_width_changes() {
        let content: Arc<str> = Arc::from("stable wrapped text");
        let key = TextLayoutKey {
            content,
            highlights: None,
            width: Some(320.0),
            font_size: 14.0,
            line_height: 20.0,
            family: FontFamily::SansSerif,
            features: FontFeatures::new(),
            fallbacks: None,
            weight: glyphon::Weight::NORMAL,
            font_style: GlyphStyle::Normal,
            underline: TextUnderline::None,
            underline_color: None,
            underline_wavy: false,
            underline_thickness: 1.0,
            strikethrough: false,
            strikethrough_color: None,
            align: TextAlign::Left,
            wrap: TextWrap::Word,
            text_overflow: None,
            line_clamp: None,
            shaping: TextShaping::Advanced,
            scale: 2.0,
        };
        let mut narrower = key.clone();
        narrower.width = Some(180.0);
        assert!(key.same_except_width(&narrower));

        let mut different_style = narrower.clone();
        different_style.font_size = 15.0;
        assert!(!key.same_except_width(&different_style));

        let mut italic = key.clone();
        italic.font_style = GlyphStyle::Italic;
        assert!(!key.same_except_width(&italic));

        let mut different_features = key.clone();
        different_features.features =
            FontFeatures::new().disable(FontFeatureTag::CONTEXTUAL_ALTERNATES);
        assert!(!key.same_except_width(&different_features));

        let mut different_fallbacks = key.clone();
        different_fallbacks.fallbacks = Some(FontFallbacks::from_fonts(["Noto Sans Hebrew"]));
        assert!(!key.same_except_width(&different_fallbacks));

        let mut underlined = key.clone();
        underlined.underline = TextUnderline::Single;
        underlined.underline_color = Some(Color::rgb8(56, 189, 248));
        assert!(!key.same_except_width(&underlined));

        let mut wavy = underlined.clone();
        wavy.underline_wavy = true;
        assert!(!underlined.same_except_width(&wavy));

        let mut thick = underlined.clone();
        thick.underline_thickness = 4.0;
        assert!(!underlined.same_except_width(&thick));

        let mut different_content = narrower;
        different_content.content = Arc::from("different wrapped text");
        assert!(!key.same_except_width(&different_content));

        let mut overflowing = key.clone();
        overflowing.text_overflow = Some(TextOverflow::ellipsis());
        let mut overflowing_narrower = overflowing.clone();
        overflowing_narrower.width = Some(180.0);
        assert!(overflowing.same_except_width(&overflowing_narrower));

        let mut custom = key.clone();
        custom.text_overflow = Some(TextOverflow::Truncate(Arc::from("...")));
        let mut custom_narrower = custom.clone();
        custom_narrower.width = Some(180.0);
        assert!(!custom.same_except_width(&custom_narrower));
    }

    #[test]
    fn unicode_text_projection_maps_retained_ranges_and_ellipsis_to_original_bytes() {
        let content: Arc<str> = Arc::from("alpha🙂beta界gamma");
        let prefix_end = content.find("beta").unwrap() + "beta".len();
        let suffix_start = content.find("gamma").unwrap();
        let projection = build_text_projection(
            &content,
            None,
            &Arc::from("…"),
            TruncationPlacement::Middle {
                prefix_end,
                suffix_start,
            },
        );

        assert_eq!(projection.content.as_ref(), "alpha🙂beta…gamma");
        let ellipsis_start = projection.content.find('…').unwrap();
        assert_eq!(projection.display_to_original(ellipsis_start), prefix_end);
        assert_eq!(
            projection.display_to_original(ellipsis_start + "…".len()),
            suffix_start
        );
        assert_eq!(
            projection.original_to_display(content.find('界').unwrap()),
            ellipsis_start
        );
        assert_eq!(
            projection.display_ranges_for_original(0..content.len()),
            vec![0..projection.content.len()]
        );
    }

    #[test]
    fn custom_affix_projection_remaps_styled_ranges_without_splitting_unicode() {
        let content: Arc<str> = Arc::from("prefix🙂hidden界suffix");
        let suffix_start = content.find("suffix").unwrap();
        let styled = StyledText::new(content.clone()).with_highlights([
            (0.."prefix".len(), HighlightStyle::default().font_bold()),
            (
                suffix_start..content.len(),
                HighlightStyle::default().underline(),
            ),
        ]);
        let projection = build_text_projection(
            &content,
            Some(styled.shared_highlights()),
            &Arc::from("[...]"),
            TruncationPlacement::Middle {
                prefix_end: "prefix".len(),
                suffix_start,
            },
        );

        assert_eq!(projection.content.as_ref(), "prefix[...]suffix");
        let highlights = projection.highlights.as_deref().unwrap();
        assert_eq!(highlights.len(), 2);
        assert_eq!(highlights[0].range, 0.."prefix[...]".len());
        assert_eq!(&projection.content[highlights[1].range.clone()], "suffix");
        assert_eq!(
            projection.display_ranges_for_original(suffix_start..content.len()),
            vec!["prefix[...]".len()..projection.content.len()]
        );
    }

    #[test]
    fn standard_ellipsis_uses_cosmic_texts_single_reflowable_original_buffer() {
        let content: Arc<str> =
            Arc::from("A very long filename with emoji 🙂 and an important-final-suffix.quickgui");
        let mut font_system = create_font_system();
        for (overflow, ellipsize) in [
            (
                TextOverflow::ellipsis(),
                Ellipsize::End(EllipsizeHeightLimit::Lines(1)),
            ),
            (
                TextOverflow::ellipsis_start(),
                Ellipsize::Start(EllipsizeHeightLimit::Lines(1)),
            ),
            (
                TextOverflow::ellipsis_middle(),
                Ellipsize::Middle(EllipsizeHeightLimit::Lines(1)),
            ),
        ] {
            let style = TextStyle::new(14.0, Color::WHITE)
                .wrap(TextWrap::None)
                .text_overflow(overflow);
            let (projection, buffer) =
                prepare_text_buffer(&mut font_system, &content, None, &style, Some(150.0), 1.0);
            assert!(Arc::ptr_eq(&projection.content, &content));
            assert!(matches!(projection.mapping, TextProjection::Identity));
            assert_eq!(buffer.ellipsize(), ellipsize);
            assert!(text_buffer_fits(&buffer, 150.0, 1, 1.0));
        }
    }

    #[test]
    fn custom_overflow_affixes_use_the_unicode_safe_visible_projection() {
        let content: Arc<str> =
            Arc::from("A very long filename with emoji 🙂 and an important-final-suffix.quickgui");
        let mut font_system = create_font_system();
        for (overflow, expected_start, expected_end, expected_affix) in [
            (TextOverflow::Truncate(Arc::from("...")), "A", "...", "..."),
            (
                TextOverflow::TruncateStart(Arc::from("...")),
                "...",
                "quickgui",
                "...",
            ),
            (
                TextOverflow::TruncateMiddle(Arc::from("[...]")),
                "A",
                "gui",
                "[...]",
            ),
        ] {
            let style = TextStyle::new(14.0, Color::WHITE)
                .wrap(TextWrap::None)
                .text_overflow(overflow);
            let (projection, buffer) =
                prepare_text_buffer(&mut font_system, &content, None, &style, Some(150.0), 1.0);
            assert!(projection.content.contains(expected_affix));
            assert!(
                projection.content.starts_with(expected_start),
                "projected {:?} did not start with {expected_start:?}",
                projection.content
            );
            assert!(
                projection.content.ends_with(expected_end),
                "projected {:?} did not end with {expected_end:?}",
                projection.content
            );
            assert!(projection.content.len() < content.len());
            assert_eq!(buffer.ellipsize(), Ellipsize::None);
            assert!(text_buffer_fits(&buffer, 150.0, 1, 1.0));
        }
    }

    #[test]
    fn multiline_ellipsis_fits_the_requested_line_clamp() {
        let content: Arc<str> = Arc::from(
            "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron",
        );
        let style = TextStyle::new(14.0, Color::WHITE)
            .line_height(20.0)
            .wrap(TextWrap::Word)
            .text_overflow(TextOverflow::ellipsis())
            .line_clamp(2);
        let mut font_system = create_font_system();
        let (projection, buffer) =
            prepare_text_buffer(&mut font_system, &content, None, &style, Some(110.0), 1.0);

        assert!(Arc::ptr_eq(&projection.content, &content));
        assert_eq!(
            buffer.ellipsize(),
            Ellipsize::End(EllipsizeHeightLimit::Lines(2))
        );
        assert!(buffer.layout_runs().count() <= 2);
        assert!(text_buffer_fits(&buffer, 110.0, 2, 1.0));
    }

    #[test]
    fn fitting_overflow_text_keeps_the_original_shared_content() {
        let content: Arc<str> = Arc::from("Short label");
        let style = TextStyle::new(14.0, Color::WHITE)
            .wrap(TextWrap::None)
            .text_overflow(TextOverflow::ellipsis());
        let mut font_system = create_font_system();
        let (projection, buffer) =
            prepare_text_buffer(&mut font_system, &content, None, &style, Some(400.0), 1.0);

        assert!(Arc::ptr_eq(&projection.content, &content));
        assert!(matches!(projection.mapping, TextProjection::Identity));
        assert!(text_buffer_fits(&buffer, 400.0, 1, 1.0));
    }

    #[test]
    fn width_only_reflow_preserves_the_shaped_glyph_cache() {
        let content = "Resize reuses shaped glyphs while recomputing wrapped line placement";
        let style = TextStyle::new(14.0, Color::WHITE)
            .line_height(20.0)
            .wrap(TextWrap::Word);
        let mut font_system = create_font_system();
        let mut buffer = Buffer::new(
            &mut font_system,
            Metrics::new(style.font_size, style.line_height),
        );
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            content,
            &style,
            None,
            Some(480.0),
            1.0,
        );
        let wide_lines = buffer.layout_runs().count();
        let shaped_line = buffer.lines[0]
            .shape_opt()
            .expect("configuration shapes the first paragraph")
            as *const _ as usize;

        reflow_text_buffer(&mut buffer, &mut font_system, Some(90.0), 1.0);

        assert_eq!(buffer.size(), (Some(90.0), None));
        assert!(buffer.layout_runs().count() > wide_lines);
        assert_eq!(
            buffer.lines[0]
                .shape_opt()
                .expect("relayout retains the shaped paragraph") as *const _ as usize,
            shaped_line
        );
    }

    #[test]
    fn text_alignment_changes_shaped_line_origins_with_one_bounded_buffer() {
        let mut font_system = create_font_system();
        let mut origin = |align| {
            let style = TextStyle::new(14.0, Color::WHITE)
                .wrap(TextWrap::None)
                .align(align);
            let mut buffer = Buffer::new(
                &mut font_system,
                Metrics::new(style.font_size, style.line_height),
            );
            configure_text_buffer(
                &mut buffer,
                &mut font_system,
                "Align me",
                &style,
                None,
                Some(240.0),
                1.0,
            );
            buffer
                .cursor_position(&Cursor::new(0, 0))
                .expect("the line has a caret origin")
                .0
        };

        let left = origin(TextAlign::Left);
        let center = origin(TextAlign::Center);
        let right = origin(TextAlign::Right);
        assert!(left < center, "centered text must move right of left text");
        assert!(
            center < right,
            "right text must move right of centered text"
        );
        assert_eq!(glyph_alignment(TextAlign::Justify), GlyphAlign::Justified);
    }

    #[test]
    fn multiline_layout_maps_carets_hits_and_selection_per_line() {
        let content = "first line\nsecond line";
        let style = TextStyle::new(14.0, Color::WHITE).line_height(20.0);
        let mut font_system = create_font_system();
        let mut buffer = Buffer::new(
            &mut font_system,
            Metrics::new(style.font_size, style.line_height),
        );
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            content,
            &style,
            None,
            Some(240.0),
            1.0,
        );

        let second_line = content.find("second").expect("second line exists");
        let caret = text_cursor_for_byte_index(content, second_line + 3);
        let (_, caret_y) = buffer.cursor_position(&caret).expect("caret is laid out");
        assert!(caret_y >= style.line_height);
        let hit = buffer
            .hit(0.0, caret_y + style.line_height * 0.5)
            .expect("second line is hittable");
        assert_eq!(hit.line, 1);
        assert_eq!(byte_index_for_text_cursor(content, hit), second_line);

        let selection_start = text_cursor_for_byte_index(content, 2);
        let selection_end = text_cursor_for_byte_index(content, content.len() - 2);
        let highlighted_lines = buffer
            .layout_runs()
            .filter(|run| !text_selection_spans(run, selection_start, selection_end).is_empty())
            .count();
        assert_eq!(highlighted_lines, 2);

        let trailing_newline = "first line\n";
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            trailing_newline,
            &style,
            None,
            Some(240.0),
            1.0,
        );
        let trailing_caret = text_cursor_for_byte_index(trailing_newline, trailing_newline.len());
        let (_, trailing_y) = buffer
            .cursor_position(&trailing_caret)
            .expect("a trailing empty line has caret geometry");
        assert!(trailing_y >= style.line_height);

        let wrapped = "alpha beta gamma delta epsilon";
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            wrapped,
            &style,
            None,
            Some(60.0),
            1.0,
        );
        assert!(buffer.layout_runs().count() >= 3);
        let gamma = wrapped.find("gamma").expect("gamma exists");
        let wrapped_caret = text_cursor_for_byte_index(wrapped, gamma + 2);
        let (wrapped_x, wrapped_y) = buffer
            .cursor_position(&wrapped_caret)
            .expect("a wrapped caret has visual-line geometry");
        assert!(wrapped_y >= style.line_height);
        let wrapped_hit = buffer
            .hit(wrapped_x, wrapped_y + style.line_height * 0.5)
            .expect("the wrapped visual line is hittable");
        let wrapped_index = byte_index_for_text_cursor(wrapped, wrapped_hit);
        assert!((gamma..=gamma + "gamma".len()).contains(&wrapped_index));
    }

    #[test]
    fn styled_text_uses_one_wrapped_buffer_for_glyphs_backgrounds_and_decorations() {
        let content: Arc<str> = Arc::from("status ready deprecated");
        let ready = content.find("ready").unwrap();
        let deprecated = content.find("deprecated").unwrap();
        let styled = StyledText::new(content.clone()).with_highlights([
            (
                0..6,
                HighlightStyle::default()
                    .background(Color::rgba8(30, 64, 175, 96))
                    .font_semibold(),
            ),
            (
                ready..ready + "ready".len(),
                HighlightStyle::default()
                    .color(Color::rgb8(56, 189, 248))
                    .double_underline(),
            ),
            (
                deprecated..content.len(),
                HighlightStyle::default()
                    .italic()
                    .strikethrough_color(Color::rgb8(248, 113, 113)),
            ),
        ]);
        let style = TextStyle::new(14.0, Color::WHITE).line_height(20.0);
        let mut font_system = create_font_system();
        let mut buffer = Buffer::new(
            &mut font_system,
            Metrics::new(style.font_size, style.line_height),
        );

        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            &content,
            &style,
            Some(styled.highlights()),
            Some(90.0),
            1.0,
        );

        assert!(buffer.layout_runs().count() >= 2);
        let metadata: HashSet<_> = buffer
            .layout_runs()
            .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.metadata))
            .collect();
        assert!(metadata.contains(&1));
        assert!(metadata.contains(&2));
        assert!(metadata.contains(&3));

        let geometry = collect_styled_text_geometry(
            &buffer,
            styled.highlights(),
            &style,
            1.0,
            0.0..1_000.0,
            None,
        );
        assert!(!geometry.backgrounds.is_empty());
        assert!(geometry.decorations.len() >= 3);
        assert_eq!(geometry.backgrounds[0].color, Color::rgba8(30, 64, 175, 96));

        let clipped = collect_styled_text_geometry(
            &buffer,
            styled.highlights(),
            &style,
            1.0,
            10_000.0..10_020.0,
            None,
        );
        assert!(clipped.backgrounds.is_empty());
        assert!(clipped.decorations.is_empty());
    }

    #[test]
    fn inherited_base_text_decorations_use_the_same_bounded_buffer() {
        let content: Arc<str> = Arc::from("inherited decoration");
        let underline = Color::rgb8(56, 189, 248);
        let strike = Color::rgb8(248, 113, 113);
        let style = TextStyle::new(16.0, Color::WHITE)
            .line_height(24.0)
            .font_style(GlyphStyle::Italic)
            .underline_color(underline)
            .strikethrough_color(strike);
        let mut font_system = create_font_system();
        let mut buffer = Buffer::new(
            &mut font_system,
            Metrics::new(style.font_size, style.line_height),
        );

        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            &content,
            &style,
            None,
            Some(240.0),
            1.0,
        );
        let geometry = collect_styled_text_geometry(&buffer, &[], &style, 1.0, 0.0..1_000.0, None);

        assert!(geometry.backgrounds.is_empty());
        assert!(geometry.decorations.len() >= 2);
        assert!(
            geometry
                .decorations
                .iter()
                .any(|decoration| decoration.color == underline)
        );
        assert!(
            geometry
                .decorations
                .iter()
                .any(|decoration| decoration.color == strike)
        );
    }

    #[test]
    fn custom_underline_geometry_preserves_waves_thickness_and_zero() {
        let content: Arc<str> = Arc::from("diagnostic underline");
        let accent = Color::rgb8(248, 113, 113);
        let mut font_system = create_font_system();
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 24.0));

        let wavy = TextStyle::new(16.0, Color::TRANSPARENT)
            .line_height(24.0)
            .underline_color(accent)
            .text_decoration_4()
            .text_decoration_wavy();
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            &content,
            &wavy,
            None,
            Some(240.0),
            2.0,
        );
        let geometry = collect_styled_text_geometry(&buffer, &[], &wavy, 2.0, 0.0..1_000.0, None);
        assert_eq!(geometry.decorations.len(), 1);
        let decoration = geometry.decorations[0];
        assert_eq!(decoration.color, accent);
        assert!(matches!(
            decoration.kind,
            TextPaintKind::WavyUnderline {
                thickness,
                amplitude,
                wavelength,
                ..
            } if thickness == 4.0 && amplitude == 4.0 && wavelength == 16.0
        ));

        let solid = wavy.clone().text_decoration_solid().text_decoration_2();
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            &content,
            &solid,
            None,
            Some(240.0),
            2.0,
        );
        let geometry = collect_styled_text_geometry(&buffer, &[], &solid, 2.0, 0.0..1_000.0, None);
        assert_eq!(geometry.decorations.len(), 1);
        assert_eq!(geometry.decorations[0].kind, TextPaintKind::Solid);
        assert_eq!(geometry.decorations[0].rect.height, 2.0);

        let zero = solid.text_decoration_0();
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            &content,
            &zero,
            None,
            Some(240.0),
            2.0,
        );
        let geometry = collect_styled_text_geometry(&buffer, &[], &zero, 2.0, 0.0..1_000.0, None);
        assert!(geometry.decorations.is_empty());
    }

    #[test]
    fn highlighted_wavy_ranges_split_an_inherited_solid_decoration_span() {
        let content: Arc<str> = Arc::from("solid wavy");
        let wavy_start = content.find("wavy").unwrap();
        let styled = StyledText::new(content.clone()).with_highlights([(
            wavy_start..content.len(),
            HighlightStyle::default()
                .text_decoration_wavy()
                .text_decoration_2(),
        )]);
        let style = TextStyle::new(16.0, Color::WHITE).underline();
        let mut font_system = create_font_system();
        let mut buffer = Buffer::new(
            &mut font_system,
            Metrics::new(style.font_size, style.line_height),
        );
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            &content,
            &style,
            Some(styled.highlights()),
            Some(240.0),
            1.0,
        );
        let geometry = collect_styled_text_geometry(
            &buffer,
            styled.highlights(),
            &style,
            1.0,
            0.0..1_000.0,
            None,
        );
        assert!(
            geometry
                .decorations
                .iter()
                .any(|decoration| decoration.kind == TextPaintKind::Solid)
        );
        assert!(geometry.decorations.iter().any(|decoration| matches!(
            decoration.kind,
            TextPaintKind::WavyUnderline { thickness: 2.0, .. }
        )));
    }

    #[test]
    fn styled_multiline_selection_respects_exact_byte_range() {
        let content: Arc<str> =
            Arc::from("fn render() {\n    let state = \"GPU cached\";\n    deprecated_api();\n}");
        let selected_start = content.find("GPU cached").unwrap();
        let selected_end = selected_start + "GPU cached".len();
        let deprecated = content.find("deprecated_api").unwrap();
        let styled = StyledText::new(content.clone()).with_highlights([
            (0..2, HighlightStyle::default().font_bold()),
            (
                selected_start..selected_end,
                HighlightStyle::default().background(Color::rgba8(14, 116, 144, 72)),
            ),
            (
                deprecated..deprecated + "deprecated_api".len(),
                HighlightStyle::default().strikethrough(),
            ),
        ]);
        let style = TextStyle::new(14.0, Color::WHITE)
            .family(FontFamily::Monospace)
            .wrap(TextWrap::None)
            .line_height(22.0);
        let mut font_system = create_font_system();
        let mut buffer = Buffer::new(
            &mut font_system,
            Metrics::new(style.font_size, style.line_height),
        );
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            &content,
            &style,
            Some(styled.highlights()),
            Some(640.0),
            1.0,
        );

        let start = text_cursor_for_byte_index(&content, selected_start);
        let end = text_cursor_for_byte_index(&content, selected_end);
        let selections = buffer
            .layout_runs()
            .flat_map(|run| {
                let line_width = run.line_w;
                text_selection_spans(&run, start, end)
                    .into_iter()
                    .map(move |(_, width)| (run.line_i, width, line_width))
            })
            .collect::<Vec<_>>();

        assert_eq!(selections.len(), 1);
        assert_eq!(selections[0].0, 1);
        assert!(selections[0].1 < selections[0].2);
    }

    #[test]
    fn background_color_only_changes_reuse_the_shaped_highlight_key() {
        use std::collections::hash_map::DefaultHasher;

        let cool = StyledText::new("cached").with_highlights([(
            0..6,
            HighlightStyle::default().background(Color::rgb8(14, 116, 144)),
        )]);
        let warm = StyledText::new("cached").with_highlights([(
            0..6,
            HighlightStyle::default().background(Color::rgb8(190, 24, 93)),
        )]);
        let cool = Some(cool.shared_highlights().clone());
        let warm = Some(warm.shared_highlights().clone());

        assert!(highlights_equal(&cool, &warm));
        let mut cool_hash = DefaultHasher::new();
        hash_highlights(&cool, &mut cool_hash);
        let mut warm_hash = DefaultHasher::new();
        hash_highlights(&warm, &mut warm_hash);
        assert_eq!(cool_hash.finish(), warm_hash.finish());

        let foreground = StyledText::new("cached").with_highlights([(
            0..6,
            HighlightStyle::default().color(Color::rgb8(14, 116, 144)),
        )]);
        assert!(!highlights_equal(
            &cool,
            &Some(foreground.shared_highlights().clone())
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_font_fallback_excludes_the_unscalable_gb18030_bitmap_face() {
        let font_system = create_font_system();
        assert!(
            font_system
                .db()
                .faces()
                .all(|face| face.post_script_name != "GB18030Bitmap")
        );
    }

    #[test]
    fn multiline_monospaced_styled_text_keeps_rasterizable_glyphs_on_every_line() {
        let content: Arc<str> = Arc::from("first red\nsecond blue\nthird 東京 مرحبا 🙂");
        let red = content.find("red").unwrap();
        let blue = content.find("blue").unwrap();
        let styled = StyledText::new(content.clone()).with_highlights([
            (
                red..red + 3,
                HighlightStyle::default()
                    .color(Color::rgb8(248, 113, 113))
                    .background(Color::rgba8(127, 29, 29, 96)),
            ),
            (
                blue..blue + 4,
                HighlightStyle::default()
                    .color(Color::rgb8(96, 165, 250))
                    .underline(),
            ),
        ]);
        let style = TextStyle::new(14.0, Color::WHITE)
            .family(FontFamily::Monospace)
            .line_height(22.0);
        let mut font_system = create_font_system();
        let mut buffer = Buffer::new(
            &mut font_system,
            Metrics::new(style.font_size, style.line_height),
        );
        configure_text_buffer(
            &mut buffer,
            &mut font_system,
            &content,
            &style,
            Some(styled.highlights()),
            Some(400.0),
            1.0,
        );

        let runs: Vec<_> = buffer.layout_runs().collect();
        assert_eq!(runs.len(), 3);
        assert_eq!(
            runs.iter().map(|run| run.text).collect::<Vec<_>>(),
            ["first red", "second blue", "third 東京 مرحبا 🙂"]
        );
        assert!(runs.iter().all(|run| run.line_w.is_finite()));
        assert!(runs.iter().all(|run| !run.glyphs.is_empty()));

        let mut swash = SwashCache::new();
        for run in runs {
            for glyph in run.glyphs.iter().filter(|glyph| glyph.w > 0.0) {
                assert!(glyph.x.is_finite() && glyph.w.is_finite());
                assert!(
                    swash
                        .get_image_uncached(
                            &mut font_system,
                            glyph.physical((0.0, 0.0), 1.0).cache_key,
                        )
                        .is_some(),
                    "every fallback glyph in a monospaced rich-text buffer must rasterize"
                );
            }
        }
    }

    #[test]
    fn basic_text_fragments_isolate_changing_numeric_fields() {
        let fragments: Vec<_> =
            BasicTextFragments::new("Row 014440    The quick brown fox").collect();
        assert_eq!(
            fragments,
            ["Row ", "01", "44", "40", "    The quick brown fox"]
        );
    }

    #[test]
    fn basic_text_fragmentation_requires_explicit_safe_shaping() {
        let content = "Row 014440    The quick brown fox";
        let mut style = TextStyle::new(14.0, Color::WHITE).wrap(TextWrap::None);
        assert!(!should_fragment_basic_text(content, &style));
        style.shaping = TextShaping::Basic;
        assert!(should_fragment_basic_text(content, &style));
        assert!(!should_fragment_basic_text(
            content,
            &style.clone().align(TextAlign::Center)
        ));
        assert!(!should_fragment_basic_text("short 123", &style));
        assert!(!should_fragment_basic_text(
            "Row 014440\tThe quick brown fox",
            &style
        ));
    }

    #[test]
    fn dilation_expands_and_collapses_around_the_original_center() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(dilate_rect(rect, 3.0), Rect::new(7.0, 17.0, 106.0, 56.0));
        assert_eq!(dilate_rect(rect, -60.0), Rect::new(60.0, 45.0, 0.0, 0.0));
    }

    #[test]
    fn one_wavy_underline_span_is_one_analytic_shape_instance() {
        let underline = WavyUnderline::new(
            Rect::new(10.0, 20.0, 120.0, 6.0),
            23.0,
            1.5,
            2.0,
            6.0,
            Color::rgb8(248, 113, 113),
        );
        let instance = wavy_underline_instance(&underline, Rect::new(0.0, 0.0, 200.0, 100.0));
        assert_eq!(instance.geometry, [10.0, 20.0, 120.0, 6.0]);
        assert_eq!(instance.subject, [1.5, 0.0, 0.0, 0.0]);
        assert_eq!(instance.params, [SHAPE_MODE_WAVY_UNDERLINE, 23.0, 2.0, 6.0]);
    }

    #[test]
    fn drop_shadow_geometry_includes_offset_spread_and_three_sigma_margin() {
        let shadow = Shadow::new(
            Rect::new(10.0, 20.0, 100.0, 50.0),
            BoxShadow::new(4.0, 6.0, Color::WHITE)
                .blur_radius(10.0)
                .spread_radius(3.0),
        )
        .radius(8.0);
        let instance = shadow_instance(&shadow, Rect::new(-100.0, -100.0, 500.0, 500.0)).unwrap();
        assert_eq!(instance.subject, [11.0, 23.0, 106.0, 56.0]);
        assert_eq!(instance.geometry, [-5.0, 7.0, 138.0, 88.0]);
        assert_eq!(instance.params, [SHAPE_MODE_DROP_SHADOW, 8.0, 0.0, 10.0]);
    }

    #[test]
    fn inset_shadow_uses_a_spread_contracting_translated_hole() {
        let shadow = Shadow::new(
            Rect::new(10.0, 20.0, 100.0, 50.0),
            BoxShadow::new(2.0, 3.0, Color::WHITE)
                .blur_radius(4.0)
                .spread_radius(5.0)
                .inset(true),
        )
        .radius(8.0);
        let instance = shadow_instance(&shadow, Rect::new(0.0, 0.0, 500.0, 500.0)).unwrap();
        assert_eq!(instance.geometry, [10.0, 20.0, 100.0, 50.0]);
        assert_eq!(instance.subject, [17.0, 28.0, 90.0, 40.0]);
        assert_eq!(instance.params, [SHAPE_MODE_INSET_SHADOW, 3.0, 8.0, 4.0]);
    }
}
