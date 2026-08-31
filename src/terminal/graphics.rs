use crate::{Canvas, Color, Rect};

/// Physical and logical dimensions of one terminal grid cell.
///
/// Ghostty receives physical pixel dimensions, so text, cursors, and synthesized glyphs must all
/// derive their logical geometry from those same rounded values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CellMetrics {
    pub(super) logical_width: f32,
    pub(super) logical_height: f32,
    pub(super) physical_width: u32,
    pub(super) physical_height: u32,
    scale_factor: f32,
}

impl CellMetrics {
    pub(super) fn new(
        font_size: f32,
        line_height: f32,
        cell_width_ratio: f32,
        scale_factor: f32,
    ) -> Self {
        let scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor
        } else {
            1.0
        };
        let physical_width = ((font_size * cell_width_ratio * scale_factor)
            .round()
            .max(1.0)) as u32;
        let physical_height = ((line_height * scale_factor).round().max(1.0)) as u32;
        Self {
            logical_width: physical_width as f32 / scale_factor,
            logical_height: physical_height as f32 / scale_factor,
            physical_width,
            physical_height,
            scale_factor,
        }
    }
}

pub(super) fn is_block_element(character: char) -> bool {
    matches!(character as u32, 0x2580..=0x259f)
}

/// Paint one Unicode Block Elements glyph using Ghostty's cell-fraction rules.
///
/// These characters intentionally bypass font rasterization: a full block must cover every pixel
/// in its terminal cell, and complementary halves overlap their shared pixel when a cell dimension
/// is odd. This is what keeps terminal art continuous across columns and rows.
pub(super) fn paint_block(
    canvas: &mut Canvas<'_>,
    column: u16,
    row: u16,
    character: char,
    metrics: CellMetrics,
    color: Color,
) {
    if color.a <= 0.0 {
        return;
    }
    let origin_x = u32::from(column).saturating_mul(metrics.physical_width);
    let origin_y = u32::from(row).saturating_mul(metrics.physical_height);
    for_each_block_rect(
        character,
        metrics.physical_width,
        metrics.physical_height,
        |rect| {
            let color = color.with_alpha(color.a * f32::from(rect.alpha) / 255.0);
            canvas.fill_rect(
                Rect::new(
                    (origin_x + rect.x) as f32 / metrics.scale_factor,
                    (origin_y + rect.y) as f32 / metrics.scale_factor,
                    rect.width as f32 / metrics.scale_factor,
                    rect.height as f32 / metrics.scale_factor,
                ),
                color,
            );
        },
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BlockRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    alpha: u8,
}

fn for_each_block_rect(
    character: char,
    cell_width: u32,
    cell_height: u32,
    mut paint: impl FnMut(BlockRect),
) {
    let mut rect = |x, y, width, height, alpha| {
        if width > 0 && height > 0 && alpha > 0 {
            paint(BlockRect {
                x,
                y,
                width,
                height,
                alpha,
            });
        }
    };
    let codepoint = character as u32;
    match codepoint {
        0x2580 => {
            let height = rounded_fraction(cell_height, 1, 2);
            rect(0, 0, cell_width, height, 255);
        }
        0x2581..=0x2587 => {
            let height = rounded_fraction(cell_height, codepoint - 0x2580, 8);
            rect(
                0,
                cell_height.saturating_sub(height),
                cell_width,
                height,
                255,
            );
        }
        0x2588 => rect(0, 0, cell_width, cell_height, 255),
        0x2589..=0x258f => {
            let width = rounded_fraction(cell_width, 0x2590 - codepoint, 8);
            rect(0, 0, width, cell_height, 255);
        }
        0x2590 => {
            let width = rounded_fraction(cell_width, 1, 2);
            rect(cell_width.saturating_sub(width), 0, width, cell_height, 255);
        }
        0x2591..=0x2593 => rect(
            0,
            0,
            cell_width,
            cell_height,
            ((codepoint - 0x2590) * 0x40) as u8,
        ),
        0x2594 => {
            let height = rounded_fraction(cell_height, 1, 8);
            rect(0, 0, cell_width, height, 255);
        }
        0x2595 => {
            let width = rounded_fraction(cell_width, 1, 8);
            rect(cell_width.saturating_sub(width), 0, width, cell_height, 255);
        }
        0x2596..=0x259f => {
            let quadrants = match codepoint {
                0x2596 => 0b0100,
                0x2597 => 0b1000,
                0x2598 => 0b0001,
                0x2599 => 0b1101,
                0x259a => 0b1001,
                0x259b => 0b0111,
                0x259c => 0b1011,
                0x259d => 0b0010,
                0x259e => 0b0110,
                0x259f => 0b1110,
                _ => unreachable!(),
            };
            let half_width_max = rounded_fraction(cell_width, 1, 2);
            let half_height_max = rounded_fraction(cell_height, 1, 2);
            let half_width_min = cell_width.saturating_sub(half_width_max);
            let half_height_min = cell_height.saturating_sub(half_height_max);
            if quadrants & 0b0001 != 0 {
                rect(0, 0, half_width_max, half_height_max, 255);
            }
            if quadrants & 0b0010 != 0 {
                rect(half_width_min, 0, half_width_max, half_height_max, 255);
            }
            if quadrants & 0b0100 != 0 {
                rect(0, half_height_min, half_width_max, half_height_max, 255);
            }
            if quadrants & 0b1000 != 0 {
                rect(
                    half_width_min,
                    half_height_min,
                    half_width_max,
                    half_height_max,
                    255,
                );
            }
        }
        _ => {}
    }
}

fn rounded_fraction(size: u32, numerator: u32, denominator: u32) -> u32 {
    debug_assert!(denominator > 0);
    ((u64::from(size) * u64::from(numerator) + u64::from(denominator / 2)) / u64::from(denominator))
        as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_metrics_snap_to_device_pixels() {
        assert_eq!(
            CellMetrics::new(14.0, 20.5, 0.6, 2.0),
            CellMetrics {
                logical_width: 8.5,
                logical_height: 20.5,
                physical_width: 17,
                physical_height: 41,
                scale_factor: 2.0,
            }
        );
    }

    #[test]
    fn block_geometry_matches_ghostty_pixel_rounding() {
        let rects = |character| {
            let mut rects = Vec::new();
            for_each_block_rect(character, 17, 41, |rect| rects.push(rect));
            rects
        };

        assert_eq!(
            rects('█'),
            [BlockRect {
                x: 0,
                y: 0,
                width: 17,
                height: 41,
                alpha: 255,
            }]
        );
        assert_eq!(
            rects('▀'),
            [BlockRect {
                x: 0,
                y: 0,
                width: 17,
                height: 21,
                alpha: 255,
            }]
        );
        assert_eq!(
            rects('▄'),
            [BlockRect {
                x: 0,
                y: 20,
                width: 17,
                height: 21,
                alpha: 255,
            }]
        );
        assert_eq!(
            rects('▌'),
            [BlockRect {
                x: 0,
                y: 0,
                width: 9,
                height: 41,
                alpha: 255,
            }]
        );
        assert_eq!(
            rects('▐'),
            [BlockRect {
                x: 8,
                y: 0,
                width: 9,
                height: 41,
                alpha: 255,
            }]
        );
        assert_eq!(rects('░')[0].alpha, 0x40);
        assert_eq!(rects('▒')[0].alpha, 0x80);
        assert_eq!(rects('▓')[0].alpha, 0xc0);
        assert_eq!(rects('a'), []);
    }
}
