use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    mem,
    ops::Range,
    sync::Arc,
};

use bytemuck::{Pod, Zeroable};
use glyphon::{
    Attrs, Buffer, Cache, Cursor, Family, FontSystem, Metrics, Resolution, Shaping as GlyphShaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Wrap,
};
use thiserror::Error;
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
    FontFamily, PerformanceProfile, Quad, Rect, RenderStats, Scene, ScenePlane, Size, TextId,
    TextShaping, TextStyle, TextWrap,
    image_renderer::ImageRenderer,
    path_renderer::PathRenderer,
    scene::{PrimitiveRef, Shadow, ShapeRef},
    svg_renderer::SvgRenderer,
};

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2_app_kit::NSView;
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
const MAX_RETAINED_TEXT_RENDERERS: usize = 8;
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
    #[error("GPU submission did not complete: {0}")]
    DevicePoll(#[from] wgpu::PollError),
}

pub(crate) enum RenderOutcome {
    Presented(RenderStats),
    Retry,
    Occluded,
}

pub(crate) struct GpuRenderer {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    configured_surface_size: Option<(u32, u32)>,
    #[cfg(target_os = "macos")]
    metal_layer: MetalLayer,
    #[cfg(target_os = "macos")]
    appkit_view: Retained<NSView>,
    #[cfg(target_os = "macos")]
    live_resize_transaction: bool,
    shapes: ShapeRenderer,
    path: PathRenderer,
    image: ImageRenderer,
    svg: SvgRenderer,
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
    ) -> Result<Self, RendererInitError> {
        let instance = Instance::new(InstanceDescriptor::new_with_display_handle(Box::new(
            event_loop.owned_display_handle(),
        )));
        #[cfg(target_os = "macos")]
        let (surface, metal_layer, appkit_view) = create_macos_window_surface(&instance, &window)?;
        #[cfg(not(target_os = "macos"))]
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: profile.into(),
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await?;
        let (device, queue) = adapter.request_device(&DeviceDescriptor::default()).await?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = preferred_surface_format(&capabilities.formats)
            .ok_or(RendererInitError::IncompatibleSurface)?;
        let physical_size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: physical_size.width.max(1),
            height: physical_size.height.max(1),
            present_mode: PresentMode::Fifo,
            alpha_mode: capabilities
                .alpha_modes
                .iter()
                .copied()
                .find(|mode| *mode == CompositeAlphaMode::Opaque)
                .unwrap_or_else(|| capabilities.alpha_modes[0]),
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

        let shapes = ShapeRenderer::new(&device, format);
        let path = PathRenderer::new(&device, format);
        let image = ImageRenderer::new(&device, format);
        let svg = SvgRenderer::new(&device, format);
        let text = TextSystem::new(&device, &queue, format);
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            config,
            configured_surface_size,
            #[cfg(target_os = "macos")]
            metal_layer,
            #[cfg(target_os = "macos")]
            appkit_view,
            #[cfg(target_os = "macos")]
            live_resize_transaction: false,
            shapes,
            path,
            image,
            svg,
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

    pub fn resize(&mut self, width: u32, height: u32) {
        let configured_width = width.max(1);
        let configured_height = height.max(1);
        if self.config.width == configured_width && self.config.height == configured_height {
            return;
        }
        self.config.width = configured_width;
        self.config.height = configured_height;
    }

    /// Apply only the latest queued size immediately before drawable acquisition.
    ///
    /// AppKit can deliver many `Resized` callbacks during one live-resize gesture. Configuring a
    /// Metal surface synchronously in each callback serializes the window server and GPU. Keeping
    /// `resize` cheap lets Winit coalesce redraws while this method configures exactly once for the
    /// frame that will actually be presented.
    fn configure_surface_for_frame(&mut self) {
        let size = (self.config.width, self.config.height);
        if self.configured_surface_size == Some(size) {
            return;
        }

        self.surface.configure(&self.device, &self.config);
        #[cfg(target_os = "macos")]
        if let Some(overlay) = &mut self.overlay_surface {
            overlay.config.width = size.0;
            overlay.config.height = size.1;
            overlay.surface.configure(&self.device, &overlay.config);
        }
        self.configured_surface_size = Some(size);
    }

    #[cfg(target_os = "macos")]
    fn update_live_resize_transaction(&mut self) {
        let live_resize = unsafe { self.appkit_view.inLiveResize() };
        if self.live_resize_transaction == live_resize {
            return;
        }
        set_presents_with_transaction(&self.metal_layer, live_resize);
        if let Some(overlay) = &self.overlay_surface {
            set_presents_with_transaction(&overlay.metal_layer, live_resize);
        }
        self.live_resize_transaction = live_resize;
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
            .measure(id, content, style, max_width, scale_factor)
    }

    pub(crate) fn text_caret_x(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        width: f32,
        scale_factor: f32,
        index: usize,
    ) -> f32 {
        self.text
            .caret_x(id, content, style, width, scale_factor, index)
    }

    pub(crate) fn text_index_for_x(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        width: f32,
        scale_factor: f32,
        x: f32,
    ) -> usize {
        self.text
            .index_for_x(id, content, style, width, scale_factor, x)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn text_selection_spans(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        width: f32,
        scale_factor: f32,
        start: usize,
        end: usize,
    ) -> Vec<(f32, f32)> {
        self.text
            .selection_spans(id, content, style, width, scale_factor, start, end)
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
        let path_stats = self.path.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_size.width,
            physical_size.height,
            scale_factor,
        );
        let image_stats = self.image.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_size.width,
            physical_size.height,
            scale_factor,
        );
        let svg_stats = self.svg.prepare(
            &self.device,
            &self.queue,
            scene,
            logical_viewport,
            physical_size.width,
            physical_size.height,
            scale_factor,
        );
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
                self.surface.configure(&self.device, &self.config);
                self.configured_surface_size = Some((self.config.width, self.config.height));
                frame
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                self.configured_surface_size = Some((self.config.width, self.config.height));
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
                self.surface.configure(&self.device, &self.config);
                self.configured_surface_size = Some((self.config.width, self.config.height));
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
                    overlay.surface.configure(&self.device, &overlay.config);
                    Some(frame)
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Outdated => {
                    overlay.surface.configure(&self.device, &overlay.config);
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
                    self.path.render_order(&mut pass, layer, order);
                    self.image.render_order(&mut pass, layer, order);
                    self.svg.render_order(&mut pass, layer, order);
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
                    self.path.render_order(&mut pass, layer, order);
                    self.image.render_order(&mut pass, layer, order);
                    self.svg.render_order(&mut pass, layer, order);
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
            text_areas: text_count,
            draw_calls: shape_draw_calls
                + path_stats.draw_calls
                + image_stats.draw_calls
                + svg_stats.draw_calls
                + text_draw_calls,
            reshaped_text_areas: reshaped,
            cached_text_areas: text_count.saturating_sub(reshaped),
        }))
    }

    #[allow(dead_code)]
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }
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
const SHADOW_SIGMA_PER_BLUR_RADIUS: f32 = 0.5;
const SHADOW_MARGIN_SIGMAS: f32 = 3.0;

struct ShapeRenderer {
    pipeline: RenderPipeline,
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

impl ShapeRenderer {
    fn new(device: &Device, format: TextureFormat) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quickgui view uniform"),
            contents: bytemuck::bytes_of(&ViewUniform::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quickgui view bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
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
        pass.set_pipeline(&self.pipeline);
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
    width: Option<f32>,
    font_size: f32,
    line_height: f32,
    family: FontFamily,
    weight: glyphon::Weight,
    wrap: TextWrap,
    shaping: TextShaping,
    scale: f32,
}

impl PartialEq for TextLayoutKey {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
            && self.width.map(f32::to_bits) == other.width.map(f32::to_bits)
            && self.font_size.to_bits() == other.font_size.to_bits()
            && self.line_height.to_bits() == other.line_height.to_bits()
            && self.family == other.family
            && self.weight == other.weight
            && self.wrap == other.wrap
            && self.shaping == other.shaping
            && self.scale.to_bits() == other.scale.to_bits()
    }
}

impl Eq for TextLayoutKey {}

impl Hash for TextLayoutKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.content.hash(state);
        self.width.map(f32::to_bits).hash(state);
        self.font_size.to_bits().hash(state);
        self.line_height.to_bits().hash(state);
        self.family.hash(state);
        self.weight.hash(state);
        self.wrap.hash(state);
        self.shaping.hash(state);
        self.scale.to_bits().hash(state);
    }
}

struct TextEntry {
    key: TextLayoutKey,
    buffer: Arc<Buffer>,
    last_used_frame: u64,
}

struct SharedTextEntry {
    buffer: Arc<Buffer>,
    last_used_frame: u64,
}

struct TextSystem {
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderers: Vec<TextRenderer>,
    buffers: HashMap<TextId, TextEntry>,
    shared_buffers: HashMap<TextLayoutKey, SharedTextEntry>,
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
}

struct TextBatch {
    order: u32,
    visible: Range<usize>,
    renderer: usize,
}

impl TextSystem {
    fn new(device: &Device, queue: &Queue, format: TextureFormat) -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let renderer = TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);
        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            renderers: vec![renderer],
            buffers: HashMap::with_capacity(512),
            shared_buffers: HashMap::with_capacity(512),
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
                let rgba = run.style.color.to_srgba8();
                let color = glyphon::Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3]);
                let bounds = physical_text_bounds(clip, scale);
                let left = run.bounds.x * scale;
                let top = run.bounds.y * scale;
                let run_reshaped = if should_fragment_basic_text(&run.content, &run.style) {
                    let mut fragment_left = left;
                    let mut fragment_reshaped = false;
                    for fragment in BasicTextFragments::new(&run.content) {
                        let content = Arc::<str>::from(fragment);
                        let (buffer, was_reshaped) = self
                            .update_shared_text_entry(content, &run.style, None, scale, self.frame);
                        let width = text_buffer_width(&buffer);
                        self.visible.push(VisibleText {
                            order: item.order,
                            buffer,
                            left: fragment_left,
                            top,
                            bounds,
                            color,
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

        let Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            renderers,
            visible,
            batches,
            ..
        } = self;
        for batch in batches.iter() {
            let areas = visible[batch.visible.clone()].iter().map(|item| TextArea {
                buffer: item.buffer.as_ref(),
                left: item.left,
                top: item.top,
                scale: 1.0,
                bounds: item.bounds,
                default_color: item.color,
                custom_glyphs: &[],
            });
            renderers[batch.renderer].prepare(
                device,
                queue,
                font_system,
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
        max_width: Option<f32>,
        scale: f32,
    ) -> Size {
        let width = match style.wrap {
            TextWrap::None => None,
            TextWrap::Word | TextWrap::Glyph => max_width,
        };
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(id, content, style, width, scale, next_frame);
        let buffer = &self.buffers[&id].buffer;
        let mut measured_width = 0.0_f32;
        let mut measured_height = 0.0_f32;
        for run in buffer.layout_runs() {
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

    fn caret_x(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        width: f32,
        scale: f32,
        index: usize,
    ) -> f32 {
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(id, content, style, Some(width), scale, next_frame);
        self.buffers[&id]
            .buffer
            .cursor_position(&Cursor::new(0, index.min(content.len())))
            .map(|(x, _)| x / scale)
            .unwrap_or(0.0)
    }

    fn index_for_x(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        width: f32,
        scale: f32,
        x: f32,
    ) -> usize {
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(id, content, style, Some(width), scale, next_frame);
        self.buffers[&id]
            .buffer
            .hit(x.max(0.0) * scale, style.line_height * scale * 0.5)
            .map(|cursor| cursor.index.min(content.len()))
            .unwrap_or(content.len())
    }

    #[allow(clippy::too_many_arguments)]
    fn selection_spans(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        width: f32,
        scale: f32,
        start: usize,
        end: usize,
    ) -> Vec<(f32, f32)> {
        let next_frame = self.frame.wrapping_add(1);
        self.update_text_entry(id, content, style, Some(width), scale, next_frame);
        let start = Cursor::new(0, start.min(content.len()));
        let end = Cursor::new(0, end.min(content.len()));
        self.buffers[&id]
            .buffer
            .layout_runs()
            .next()
            .map(|run| {
                run.highlight(start, end)
                    .map(|(x, width)| (x / scale, width / scale))
                    .collect()
            })
            .unwrap_or_default()
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

    fn update_text_entry(
        &mut self,
        id: TextId,
        content: &Arc<str>,
        style: &TextStyle,
        width: Option<f32>,
        scale: f32,
        frame: u64,
    ) -> bool {
        // An unwrapped line's glyph layout is independent of its paint bounds. Taffy measures
        // intrinsic text with no width, while painting supplies the element width for clipping;
        // keeping that irrelevant width in the cache key would shape every no-wrap label twice.
        let width = canonical_text_width(style.wrap, width);
        let key = TextLayoutKey {
            content: content.clone(),
            width,
            font_size: style.font_size,
            line_height: style.line_height,
            family: style.family.clone(),
            weight: style.weight,
            wrap: style.wrap,
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
                    last_used_frame: frame,
                });
            return false;
        }

        let (buffer, reshaped) =
            self.update_shared_text_entry_for_key(key.clone(), style, scale, frame);
        self.buffers.insert(
            id,
            TextEntry {
                key,
                buffer,
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
    ) -> (Arc<Buffer>, bool) {
        let key = TextLayoutKey {
            content,
            width: canonical_text_width(style.wrap, width),
            font_size: style.font_size,
            line_height: style.line_height,
            family: style.family.clone(),
            weight: style.weight,
            wrap: style.wrap,
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
    ) -> (Arc<Buffer>, bool) {
        if let Some(entry) = self.shared_buffers.get_mut(&key) {
            entry.last_used_frame = frame;
            (Arc::clone(&entry.buffer), false)
        } else {
            let metrics = Metrics::new(style.font_size * scale, style.line_height * scale);
            let mut buffer = Buffer::new(&mut self.font_system, metrics);
            configure_text_buffer(
                &mut buffer,
                &mut self.font_system,
                &key.content,
                style,
                key.width,
                scale,
            );
            let buffer = Arc::new(buffer);
            self.shared_buffers.insert(
                key.clone(),
                SharedTextEntry {
                    buffer: Arc::clone(&buffer),
                    last_used_frame: frame,
                },
            );
            (buffer, true)
        }
    }
}

fn should_fragment_basic_text(content: &str, style: &TextStyle) -> bool {
    style.shaping == TextShaping::Basic
        && style.wrap == TextWrap::None
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

fn canonical_text_width(wrap: TextWrap, width: Option<f32>) -> Option<f32> {
    match wrap {
        TextWrap::None => None,
        TextWrap::Word | TextWrap::Glyph => width,
    }
}

fn configure_text_buffer(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    content: &str,
    style: &TextStyle,
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
    let family = match &style.family {
        FontFamily::SansSerif => Family::SansSerif,
        FontFamily::Serif => Family::Serif,
        FontFamily::Monospace => Family::Monospace,
        FontFamily::Named(name) => Family::Name(name),
    };
    let attrs = Attrs::new().family(family).weight(style.weight);
    let shaping = match style.shaping {
        TextShaping::Advanced => GlyphShaping::Advanced,
        TextShaping::Basic => GlyphShaping::Basic,
    };
    buffer.set_text(content, &attrs, shaping, None);
    buffer.shape_until_scroll(font_system, false);
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
    use crate::{BoxShadow, Color};

    #[test]
    fn surface_format_prefers_srgb() {
        assert_eq!(
            preferred_surface_format(&[TextureFormat::Rgba8Unorm, TextureFormat::Bgra8UnormSrgb]),
            Some(TextureFormat::Bgra8UnormSrgb)
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
    fn no_wrap_text_layout_ignores_paint_width() {
        assert_eq!(canonical_text_width(TextWrap::None, Some(640.0)), None);
        assert_eq!(
            canonical_text_width(TextWrap::Word, Some(640.0)),
            Some(640.0)
        );
        assert_eq!(
            canonical_text_width(TextWrap::Glyph, Some(640.0)),
            Some(640.0)
        );
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
