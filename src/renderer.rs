use std::{
    collections::{HashMap, HashSet},
    mem,
    ops::Range,
    sync::Arc,
};

use bytemuck::{Pod, Zeroable};
use glyphon::{
    Attrs, Buffer, Cache, Cursor, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Wrap,
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
    FontFamily, PerformanceProfile, Rect, RenderStats, Scene, ScenePlane, Size, TextId, TextStyle,
    TextWrap,
};

#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};
#[cfg(target_os = "macos")]
use wgpu::SurfaceTargetUnsafe;
#[cfg(target_os = "macos")]
use winit::raw_window_handle::{
    AppKitDisplayHandle, AppKitWindowHandle, RawDisplayHandle, RawWindowHandle,
};

const INITIAL_QUAD_CAPACITY: usize = 256;
const BUFFERED_FRAMES: usize = 3;
const MAX_RETAINED_TEXT_AREAS: usize = 256;
const MAX_RETAINED_TEXT_RENDERERS: usize = 8;
const TEXT_RETENTION_FRAMES: u64 = 8;

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
    quad: QuadRenderer,
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
        if physical_size.width > 0 && physical_size.height > 0 {
            surface.configure(&device, &config);
        }

        let quad = QuadRenderer::new(&device, format);
        let text = TextSystem::new(&device, &queue, format);
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            config,
            quad,
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
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        if width > 0 && height > 0 {
            self.surface.configure(&self.device, &self.config);
            #[cfg(target_os = "macos")]
            if let Some(overlay) = &mut self.overlay_surface {
                overlay.config.width = width;
                overlay.config.height = height;
                overlay.surface.configure(&self.device, &overlay.config);
            }
        }
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
            self.overlay_surface = Some(create_overlay_surface(
                &self.instance,
                &self.adapter,
                &self.device,
                &self.config,
                view,
            )?);
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

    pub fn render(
        &mut self,
        scene: &Scene,
        scale_factor: f32,
    ) -> Result<RenderOutcome, RendererError> {
        let physical_size = self.window.inner_size();
        if physical_size.width == 0 || physical_size.height == 0 {
            return Ok(RenderOutcome::Occluded);
        }

        let logical_viewport = Rect::new(
            0.0,
            0.0,
            physical_size.width as f32 / scale_factor,
            physical_size.height as f32 / scale_factor,
        );
        let (quad_count, quad_draw_calls) = self.quad.prepare(
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
                frame
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(RenderOutcome::Retry);
            }
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(RenderOutcome::Occluded),
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = self
                    .instance
                    .create_surface(self.window.clone())
                    .map_err(|error| RendererError::RecreateSurface(error.to_string()))?;
                self.surface.configure(&self.device, &self.config);
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
                    self.overlay_surface = Some(create_overlay_surface(
                        &self.instance,
                        &self.adapter,
                        &self.device,
                        &self.config,
                        view,
                    )?);
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
                self.quad.render_layer(&mut pass, layer);
                self.text.render_layer(&mut pass, layer)?;
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
                self.quad.render_layer(&mut pass, layer);
                self.text.render_layer(&mut pass, layer)?;
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
            text_areas: text_count,
            draw_calls: quad_draw_calls + text_draw_calls,
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
fn create_overlay_surface(
    instance: &Instance,
    adapter: &Adapter,
    device: &Device,
    base_config: &SurfaceConfiguration,
    view: NonNull<c_void>,
) -> Result<OverlaySurface, RendererError> {
    let surface = unsafe {
        instance.create_surface_unsafe(SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: Some(RawDisplayHandle::AppKit(AppKitDisplayHandle::new())),
            raw_window_handle: RawWindowHandle::AppKit(AppKitWindowHandle::new(view)),
        })
    }
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
struct QuadInstance {
    rect: [f32; 4],
    fill: [f32; 4],
    border: [f32; 4],
    clip: [f32; 4],
    radius_and_border: [f32; 2],
    _padding: [f32; 2],
}

struct QuadRenderer {
    pipeline: RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: BindGroup,
    instance_buffers: Vec<wgpu::Buffer>,
    instance_capacities: Vec<usize>,
    active_buffer: usize,
    layer_ranges: Vec<Range<u32>>,
    instances: Vec<QuadInstance>,
}

impl QuadRenderer {
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
            label: Some("quickgui quad pipeline layout"),
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
                format: VertexFormat::Float32x2,
                offset: 64,
                shader_location: 4,
            },
        ];
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("quickgui quad pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[Some(VertexBufferLayout {
                    array_stride: mem::size_of::<QuadInstance>() as BufferAddress,
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
            .map(|_| create_instance_buffer(device, INITIAL_QUAD_CAPACITY))
            .collect();
        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            instance_buffers,
            instance_capacities: vec![INITIAL_QUAD_CAPACITY; BUFFERED_FRAMES],
            active_buffer: 0,
            layer_ranges: Vec::with_capacity(4),
            instances: Vec::with_capacity(INITIAL_QUAD_CAPACITY),
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
    ) -> (usize, usize) {
        self.instances.clear();
        self.layer_ranges.clear();
        let mut draw_calls = 0;
        for layer in scene.paint_layers() {
            let start = self.instances.len() as u32;
            for quad in layer.quads() {
                let clip = quad.clip.unwrap_or(viewport);
                let Some(clip) = clip.intersection(viewport) else {
                    continue;
                };
                if !quad.rect.intersects(clip) {
                    continue;
                }
                self.instances.push(QuadInstance {
                    rect: [quad.rect.x, quad.rect.y, quad.rect.width, quad.rect.height],
                    fill: quad.fill.as_array(),
                    border: quad.border_color.as_array(),
                    clip: [clip.x, clip.y, clip.right(), clip.bottom()],
                    radius_and_border: [quad.radius, quad.border_width],
                    _padding: [0.0; 2],
                });
            }
            let end = self.instances.len() as u32;
            draw_calls += usize::from(start != end);
            self.layer_ranges.push(start..end);
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
            self.instance_buffers[self.active_buffer] = create_instance_buffer(device, capacity);
            self.instance_capacities[self.active_buffer] = capacity;
        }
        if !self.instances.is_empty() {
            queue.write_buffer(
                &self.instance_buffers[self.active_buffer],
                0,
                bytemuck::cast_slice(&self.instances),
            );
        }
        (self.instances.len(), draw_calls)
    }

    fn render_layer<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>, layer: usize) {
        let Some(range) = self.layer_ranges.get(layer) else {
            return;
        };
        if range.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.instance_buffers[self.active_buffer].slice(..));
        pass.draw(0..6, range.clone());
    }
}

fn create_instance_buffer(device: &Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quickgui quad instance buffer"),
        size: (capacity * mem::size_of::<QuadInstance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[derive(Clone, Debug, PartialEq)]
struct TextLayoutKey {
    content: Arc<str>,
    width: Option<f32>,
    font_size: f32,
    line_height: f32,
    family: FontFamily,
    weight: glyphon::Weight,
    wrap: TextWrap,
    scale: f32,
}

struct TextEntry {
    key: TextLayoutKey,
    buffer: Buffer,
    last_used_frame: u64,
}

struct TextSystem {
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderers: Vec<TextRenderer>,
    buffers: HashMap<TextId, TextEntry>,
    seen: HashSet<TextId>,
    visible: Vec<VisibleText>,
    layer_ranges: Vec<Range<usize>>,
    layer_renderers: Vec<Option<usize>>,
    eviction_keys: Vec<TextId>,
    frame: u64,
}

#[derive(Clone, Copy)]
struct VisibleText {
    id: TextId,
    left: f32,
    top: f32,
    bounds: TextBounds,
    color: glyphon::Color,
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
            seen: HashSet::with_capacity(128),
            visible: Vec::with_capacity(128),
            layer_ranges: Vec::with_capacity(4),
            layer_renderers: Vec::with_capacity(4),
            eviction_keys: Vec::new(),
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
        self.layer_ranges.clear();
        self.layer_renderers.clear();
        let mut reshaped = 0;

        for layer in scene.paint_layers() {
            let start = self.visible.len();
            for run in layer.text_runs() {
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
                if self.update_text_entry(
                    run.id,
                    &run.content,
                    &run.style,
                    Some(run.bounds.width),
                    scale,
                    self.frame,
                ) {
                    reshaped += 1;
                }
                let rgba = run.style.color.to_srgba8();
                self.visible.push(VisibleText {
                    id: run.id,
                    left: run.bounds.x * scale,
                    top: run.bounds.y * scale,
                    bounds: physical_text_bounds(clip, scale),
                    color: glyphon::Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3]),
                });
            }
            self.layer_ranges.push(start..self.visible.len());
        }

        self.viewport.update(
            queue,
            Resolution {
                width: physical_width,
                height: physical_height,
            },
        );

        let draw_calls = self
            .layer_ranges
            .iter()
            .filter(|range| !range.is_empty())
            .count();
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
            buffers,
            visible,
            layer_ranges,
            layer_renderers,
            ..
        } = self;
        let mut renderer_index = 0;
        for range in layer_ranges.iter() {
            if range.is_empty() {
                layer_renderers.push(None);
                continue;
            }
            let areas = visible[range.clone()].iter().map(|item| TextArea {
                buffer: &buffers[&item.id].buffer,
                left: item.left,
                top: item.top,
                scale: 1.0,
                bounds: item.bounds,
                default_color: item.color,
                custom_glyphs: &[],
            });
            renderers[renderer_index].prepare(
                device,
                queue,
                font_system,
                atlas,
                viewport,
                areas,
                swash_cache,
            )?;
            layer_renderers.push(Some(renderer_index));
            renderer_index += 1;
        }
        Ok((visible.len(), reshaped, draw_calls))
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

    fn render_layer<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        layer: usize,
    ) -> Result<(), RendererError> {
        let Some(renderer) = self
            .layer_renderers
            .get(layer)
            .and_then(|renderer| *renderer)
        else {
            return Ok(());
        };
        self.renderers[renderer].render(&self.atlas, &self.viewport, pass)?;
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
        let key = TextLayoutKey {
            content: content.clone(),
            width,
            font_size: style.font_size,
            line_height: style.line_height,
            family: style.family.clone(),
            weight: style.weight,
            wrap: style.wrap,
            scale,
        };

        if let Some(entry) = self.buffers.get_mut(&id) {
            entry.last_used_frame = frame;
            if entry.key == key {
                return false;
            }
            configure_text_buffer(
                &mut entry.buffer,
                &mut self.font_system,
                content,
                style,
                width,
                scale,
            );
            entry.key = key;
            return true;
        }

        let metrics = Metrics::new(style.font_size * scale, style.line_height * scale);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        configure_text_buffer(
            &mut buffer,
            &mut self.font_system,
            content,
            style,
            width,
            scale,
        );
        self.buffers.insert(
            id,
            TextEntry {
                key,
                buffer,
                last_used_frame: frame,
            },
        );
        true
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
    buffer.set_text(content, &attrs, Shaping::Advanced, None);
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
}
