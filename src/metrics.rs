use std::time::Duration;

/// Renderer work submitted for the most recently completed frame.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RenderStats {
    pub quads: usize,
    /// Analytic drop and inset shadows submitted with the instanced shape draw.
    pub shadows: usize,
    pub images: usize,
    /// Images uploaded to a GPU texture during this frame.
    pub image_uploads: usize,
    /// Decoded RGBA bytes retained in the renderer's bounded texture cache.
    pub gpu_image_cache_bytes: u64,
    /// Decoded RGBA bytes retained by asynchronous image resources on the CPU.
    pub cpu_image_cache_bytes: u64,
    pub image_resource_entries: usize,
    pub image_resources_loading: usize,
    pub image_resources_failed: usize,
    pub animated_images: usize,
    pub active_animations: usize,
    pub svgs: usize,
    /// SVG masks rasterized and uploaded during this frame.
    pub svg_rasterizations: usize,
    /// One-channel alpha bytes retained in the renderer's bounded SVG cache.
    pub gpu_svg_cache_bytes: u64,
    pub paths: usize,
    /// De-indexed tessellated path vertices uploaded during this frame.
    pub path_vertices: usize,
    /// Visible paths omitted because the bounded per-frame GPU path budget was exhausted.
    pub skipped_paths: usize,
    /// Visible application-WGSL rectangles submitted through the instanced custom pipeline.
    pub custom_shader_instances: usize,
    /// New custom shader pipelines compiled during this frame.
    pub custom_shader_compilations: usize,
    /// Custom pipelines retained by this window's bounded cache.
    pub cached_custom_shader_pipelines: usize,
    /// Visible custom rectangles omitted by the per-frame shader or instance limits.
    pub skipped_custom_shader_instances: usize,
    pub text_areas: usize,
    pub draw_calls: usize,
    /// Text buffers whose content, metrics, or wrapping changed this frame.
    pub reshaped_text_areas: usize,
    pub cached_text_areas: usize,
}

/// Lightweight CPU-side frame telemetry. It is intentionally allocation-free.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameMetrics {
    pub frame_number: u64,
    pub cpu_time: Duration,
    pub smoothed_cpu_time: Duration,
    pub render: RenderStats,
}

impl FrameMetrics {
    pub fn cpu_milliseconds(self) -> f64 {
        self.cpu_time.as_secs_f64() * 1_000.0
    }

    pub fn smoothed_cpu_milliseconds(self) -> f64 {
        self.smoothed_cpu_time.as_secs_f64() * 1_000.0
    }
}

#[derive(Debug, Default)]
pub(crate) struct MetricsTracker {
    metrics: FrameMetrics,
}

impl MetricsTracker {
    pub fn current(&self) -> FrameMetrics {
        self.metrics
    }

    pub fn record(&mut self, cpu_time: Duration, render: RenderStats) {
        let smoothed = if self.metrics.frame_number == 0 {
            cpu_time
        } else {
            // An exponential moving average settles quickly without storing a sample ring.
            self.metrics.smoothed_cpu_time.mul_f64(0.9) + cpu_time.mul_f64(0.1)
        };
        self.metrics = FrameMetrics {
            frame_number: self.metrics.frame_number + 1,
            cpu_time,
            smoothed_cpu_time: smoothed,
            render,
        };
    }
}
