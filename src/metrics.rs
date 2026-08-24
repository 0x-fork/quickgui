use std::time::Duration;

/// Renderer work submitted for the most recently completed frame.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RenderStats {
    pub quads: usize,
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
