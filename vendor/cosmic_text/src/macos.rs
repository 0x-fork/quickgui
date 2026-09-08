// SPDX-License-Identifier: MIT OR Apache-2.0

//! CoreText grayscale UI masks. Font bytes and variation coordinates are the same ones used
//! for shaping. Work happens only on glyph-atlas misses; contexts and font instances are reused.

use crate::{CacheKey, CacheKeyFlags, Font, FontSystem, HashMap, SwashContent, SwashImage};
use core_foundation::{
    array::{CFArrayGetCount, CFArrayGetValueAtIndex},
    base::{CFType, CFTypeRef, TCFType},
    data::CFData,
    dictionary::CFDictionary,
    number::CFNumber,
    string::{CFString, CFStringRef},
};
use core_graphics::{
    color_space::CGColorSpace,
    context::{CGContext, CGTextDrawingMode},
    geometry::{CGAffineTransform, CGPoint, CGRect},
};
use foreign_types::ForeignType;
use std::{
    ptr,
    sync::{Arc, OnceLock},
};
use swash::{scale::Source, zeno::Placement};

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFPreferencesCurrentApplication: CFStringRef;
    fn CFPreferencesCopyAppValue(key: CFStringRef, application: CFStringRef) -> CFTypeRef;
}

#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    static kCTFontVariationAttribute: CFStringRef;
    fn CTFontManagerCreateFontDescriptorsFromData(data: CFTypeRef) -> CFTypeRef;
    fn CTFontDescriptorCreateCopyWithAttributes(
        descriptor: CFTypeRef,
        attrs: CFTypeRef,
    ) -> CFTypeRef;
    fn CTFontCreateWithFontDescriptor(
        descriptor: CFTypeRef,
        size: f64,
        matrix: *const CGAffineTransform,
    ) -> CFTypeRef;
    fn CTFontGetBoundingRectsForGlyphs(
        font: CFTypeRef,
        orientation: u32,
        glyphs: *const u16,
        rects: *mut CGRect,
        count: isize,
    ) -> CGRect;
    fn CTFontDrawGlyphs(
        font: CFTypeRef,
        glyphs: *const u16,
        positions: *const CGPoint,
        count: usize,
        context: *mut core_graphics::sys::CGContext,
    );
}

fn owned(value: CFTypeRef) -> Option<CFType> {
    (!value.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(value) })
}

struct FontBytes(Arc<Font>);
impl AsRef<[u8]> for FontBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.data()
    }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
struct FontKey {
    id: fontdb::ID,
    weight: fontdb::Weight,
    optical_size: u32,
    logical_size: u32,
}

pub(crate) fn dilation_for_color(color: crate::Color) -> u8 {
    static SMOOTHING: OnceLock<bool> = OnceLock::new();
    let allowed = *SMOOTHING.get_or_init(|| {
        let key = CFString::new("AppleFontSmoothing");
        let value = unsafe {
            CFPreferencesCopyAppValue(key.as_concrete_TypeRef(), kCFPreferencesCurrentApplication)
        };
        owned(value)
            .and_then(|value| value.downcast_into::<CFNumber>())
            .and_then(|value| value.to_i64())
            != Some(0)
    });
    if !allowed {
        return 0;
    }
    let brightness = (0.2126 * f32::from(color.r())
        + 0.7152 * f32::from(color.g())
        + 0.0722 * f32::from(color.b()))
        / 255.0;
    (brightness * 4.0).round().clamp(0.0, 4.0) as u8
}

fn logical_size(key: CacheKey) -> f32 {
    let optical = f32::from_bits(key.optical_size_bits);
    if optical.is_finite() && optical > 0.0 {
        optical
    } else {
        f32::from_bits(key.font_size_bits)
    }
}

#[derive(Default)]
pub(crate) struct Rasterizer {
    fonts: HashMap<FontKey, CFType>,
    context: Option<CGContext>,
    color_context: Option<CGContext>,
}

impl Rasterizer {
    fn font(&mut self, system: &mut FontSystem, key: CacheKey) -> Option<CFType> {
        let size = logical_size(key);
        if !size.is_finite() || size <= 0.0 {
            return None;
        }
        let font_key = FontKey {
            id: key.font_id,
            weight: key.font_weight,
            optical_size: key.optical_size_bits,
            logical_size: size.to_bits(),
        };
        if let Some(font) = self.fonts.get(&font_key) {
            return Some(font.clone());
        }
        let font = system.get_font_with_optical_size(
            key.font_id,
            key.font_weight,
            key.optical_size_bits,
        )?;
        let index = font.face_index();
        // CFData retains the Arc through its allocator. CoreText can retain these bytes beyond
        // this call without copying a complete font for every size or depending on a file path.
        let data = CFData::from_arc(Arc::new(FontBytes(font)));
        let descriptors =
            owned(unsafe { CTFontManagerCreateFontDescriptorsFromData(data.as_CFTypeRef()) })?;
        let count = unsafe { CFArrayGetCount(descriptors.as_CFTypeRef().cast()) };
        if isize::try_from(index).ok()? >= count {
            return None;
        }
        let descriptor =
            unsafe { CFArrayGetValueAtIndex(descriptors.as_CFTypeRef().cast(), index as isize) };
        let mut axes = vec![(
            CFNumber::from(0x77676874_i32),
            CFNumber::from(i32::from(key.font_weight.0)),
        )];
        // Also set the default opsz explicitly. CoreText otherwise derives it from the physical
        // font size, turning a Retina body label into display-size outlines again.
        let optical_size = f32::from_bits(key.optical_size_bits);
        let swash_font = system.get_font(key.font_id, key.font_weight)?;
        if let Some(axis) = swash_font
            .as_swash()
            .variations()
            .find_by_tag(u32::from_be_bytes(*b"opsz"))
        {
            let size = if optical_size.is_finite() && optical_size > 0.0 {
                optical_size
            } else {
                axis.default_value()
            };
            axes.push((
                CFNumber::from(0x6f70737a_i32),
                CFNumber::from(f64::from(size.clamp(axis.min_value(), axis.max_value()))),
            ));
        }
        let variations = CFDictionary::from_CFType_pairs(&axes);
        let attrs = CFDictionary::from_CFType_pairs(&[(
            unsafe { CFString::wrap_under_get_rule(kCTFontVariationAttribute) },
            variations.as_CFType(),
        )]);
        let descriptor = owned(unsafe {
            CTFontDescriptorCreateCopyWithAttributes(descriptor, attrs.as_CFTypeRef())
        })?;
        let font = owned(unsafe {
            CTFontCreateWithFontDescriptor(
                descriptor.as_CFTypeRef(),
                f64::from(logical_size(key)),
                ptr::null(),
            )
        })?;
        if self.fonts.len() >= 128 {
            self.fonts.clear();
        }
        self.fonts.insert(font_key, font.clone());
        Some(font)
    }

    pub(crate) fn image(&mut self, system: &mut FontSystem, key: CacheKey) -> Option<SwashImage> {
        if !key.flags.contains(CacheKeyFlags::NATIVE_RASTERIZATION)
            || key.flags.contains(CacheKeyFlags::PIXEL_FONT)
        {
            return None;
        }
        let font = self.font(system, key)?;
        let source = system.get_font(key.font_id, key.font_weight)?;
        let colored = source
            .as_swash()
            .color_strikes()
            .find_by_largest_ppem(key.glyph_id)
            .is_some();
        // Mixed COLR fonts can also contain ordinary outline glyphs. Preserve Swash's
        // per-glyph color selection instead of baking their foreground into an RGBA tile.
        if source.has_color_glyphs && !colored {
            return None;
        }
        let scale = f64::from(f32::from_bits(key.font_size_bits) / logical_size(key));
        if !scale.is_finite() || scale <= 0.0 {
            return None;
        }
        let mut bounds = unsafe {
            CTFontGetBoundingRectsForGlyphs(
                font.as_CFTypeRef(),
                0,
                &key.glyph_id,
                ptr::null_mut(),
                1,
            )
        };
        if ![
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            bounds.size.height,
        ]
        .iter()
        .all(|v| v.is_finite())
            || bounds.size.width <= 0.0
            || bounds.size.height <= 0.0
        {
            return None;
        }
        let skew = if key.flags.contains(CacheKeyFlags::FAKE_ITALIC) {
            14_f64.to_radians().tan()
        } else {
            0.0
        };
        let transform = CGAffineTransform {
            a: 1.0,
            b: 0.0,
            c: skew,
            d: 1.0,
            tx: 0.0,
            ty: 0.0,
        };
        if skew != 0.0 {
            bounds = bounds.apply_transform(&transform);
        }
        bounds = bounds.apply_transform(&CGAffineTransform {
            a: scale,
            d: scale,
            b: 0.0,
            c: 0.0,
            tx: 0.0,
            ty: 0.0,
        });
        let offset_x = f64::from(key.x_bin.as_float());
        let offset_y = f64::from(key.y_bin.as_float());
        let thicken = key.flags.contains(CacheKeyFlags::FONT_THICKEN);
        let stroke = if thicken {
            f64::from(f32::from_bits(key.font_size_bits)) * 0.02
        } else {
            0.0
        };
        let margin = 1.0 + stroke * 0.5;
        let left = (bounds.origin.x + offset_x - margin).floor();
        let top = (bounds.origin.y + bounds.size.height - offset_y + margin).ceil();
        let right = (bounds.origin.x + bounds.size.width + offset_x + margin).ceil();
        let bottom = (bounds.origin.y - offset_y - margin).floor();
        let width = (right - left) as usize;
        let height = (top - bottom) as usize;
        // Bound both the reusable scratch context and outputs for pathological fonts/sizes.
        if width == 0 || height == 0 || width > 2048 || height > 2048 {
            return None;
        }
        let slot = if colored {
            &mut self.color_context
        } else {
            &mut self.context
        };
        let needs_context = slot
            .as_ref()
            .is_none_or(|context| context.width() < width || context.height() < height);
        let channels = if colored { 4 } else { 1 };
        if needs_context {
            let old_width = slot.as_ref().map_or(0, |c| c.width());
            let old_height = slot.as_ref().map_or(0, |c| c.height());
            let context_width = width.next_power_of_two().max(old_width);
            let context_height = height.next_power_of_two().max(old_height);
            let color_space = if colored {
                CGColorSpace::create_device_rgb()
            } else {
                CGColorSpace::create_device_gray()
            };
            let context = CGContext::create_bitmap_context(
                None,
                context_width,
                context_height,
                8,
                context_width * channels,
                &color_space,
                if colored { 1 } else { 7 },
            );
            context.set_allows_antialiasing(true);
            context.set_should_antialias(true);
            context.set_allows_font_smoothing(true);
            context.set_allows_font_subpixel_quantization(false);
            context.set_should_subpixel_quantize_fonts(false);
            context.set_allows_font_subpixel_positioning(true);
            context.set_should_subpixel_position_fonts(true);
            *slot = Some(context);
        }
        let context = slot.as_mut()?;
        let stride = context.bytes_per_row();
        let start_row = context.height() - height;
        let pixels = context.data();
        for row in start_row..start_row + height {
            pixels[row * stride..row * stride + width * channels].fill(0);
        }
        context.save();
        context.translate(-left, -bottom);
        context.scale(scale, scale);
        context.set_text_matrix(&transform);
        context.set_should_smooth_fonts(key.dilation > 0);
        context.set_gray_fill_color(f64::from(key.dilation.min(4)) * 0.25, 1.0);
        let luminance = f64::from(key.dilation.min(4)) * 0.25;
        context.set_rgb_stroke_color(luminance, luminance, luminance, 1.0);
        context.set_line_width(stroke / scale);
        context.set_text_drawing_mode(if thicken {
            CGTextDrawingMode::CGTextFillStroke
        } else {
            CGTextDrawingMode::CGTextFill
        });
        // Positions are transformed by the text matrix as well as the CTM.
        let position = CGPoint::new((offset_x + skew * offset_y) / scale, -offset_y / scale);
        unsafe {
            CTFontDrawGlyphs(
                font.as_CFTypeRef(),
                &key.glyph_id,
                &position,
                1,
                context.as_ptr(),
            );
        }
        context.restore();
        let pixels = context.data();
        let mut data = Vec::with_capacity(width * height * channels);
        for row in start_row..start_row + height {
            data.extend_from_slice(&pixels[row * stride..row * stride + width * channels]);
        }
        if colored {
            // CoreGraphics stores premultiplied RGBA; Glyphon's color atlas takes straight RGBA.
            for pixel in data.chunks_exact_mut(4) {
                let alpha = u32::from(pixel[3]);
                for channel in &mut pixel[..3] {
                    *channel = if alpha == 0 {
                        0
                    } else {
                        ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8
                    };
                }
            }
        }
        Some(SwashImage {
            source: if colored {
                Source::ColorOutline(0)
            } else {
                Source::Outline
            },
            content: if colored {
                SwashContent::Color
            } else {
                SwashContent::Mask
            },
            placement: Placement {
                left: left as i32,
                top: top as i32,
                width: width as u32,
                height: height as u32,
            },
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, SubpixelBin};

    fn fixture() -> (FontSystem, CacheKey) {
        let mut db = fontdb::Database::new();
        db.load_font_data(
            include_bytes!("../../../tests/fixtures/fonts/Inter-Regular.ttf").to_vec(),
        );
        let id = db.faces().next().unwrap().id;
        let mut system = FontSystem::new_with_locale_and_db("en-US".into(), db);
        let font = system.get_font(id, fontdb::Weight::NORMAL).unwrap();
        let glyph = font.as_swash().charmap().map('M');
        let (mut key, _, _) = CacheKey::new(
            id,
            glyph,
            28.0,
            (0.0, 0.0),
            fontdb::Weight::NORMAL,
            CacheKeyFlags::NATIVE_RASTERIZATION | CacheKeyFlags::DISABLE_HINTING,
        );
        key.optical_size_bits = 14.0_f32.to_bits();
        (system, key)
    }

    #[test]
    fn coretext_context_reuse_keeps_masks_and_subpixel_positions_stable() {
        let (mut system, key) = fixture();
        let mut rasterizer = Rasterizer::default();
        let initial = rasterizer.image(&mut system, key).unwrap();
        assert!(initial.data.iter().any(|alpha| (1..255).contains(alpha)));
        let mut shifted = key;
        shifted.x_bin = SubpixelBin::Two;
        assert_ne!(
            initial.data,
            rasterizer.image(&mut system, shifted).unwrap().data
        );
        for size in [10.0_f32, 128.0, 14.0, 50.0] {
            let mut other = key;
            other.font_size_bits = size.to_bits();
            rasterizer.image(&mut system, other).unwrap();
        }
        assert_eq!(
            initial.data,
            rasterizer.image(&mut system, key).unwrap().data
        );
        assert_eq!(
            rasterizer.fonts.len(),
            1,
            "DPI changes reuse the logical CTFont"
        );
        let mut portable_key = key;
        portable_key
            .flags
            .remove(CacheKeyFlags::NATIVE_RASTERIZATION);
        let swash = crate::SwashCache::new()
            .get_image_uncached(&mut system, portable_key)
            .unwrap();
        assert_ne!(
            initial.data, swash.data,
            "the native path must not silently use Swash"
        );
    }

    #[test]
    fn coretext_font_cache_and_scratch_storage_stay_bounded() {
        let (mut system, mut key) = fixture();
        let mut rasterizer = Rasterizer::default();
        for size in 8..150 {
            key.optical_size_bits = (size as f32).to_bits();
            key.font_size_bits = (size as f32).to_bits();
            rasterizer.image(&mut system, key).unwrap();
            assert!(rasterizer.fonts.len() <= 128);
        }
        let context = rasterizer.context.as_ref().unwrap();
        assert!(context.width() <= 2048 && context.height() <= 2048);
        key.flags |= CacheKeyFlags::PIXEL_FONT;
        assert!(rasterizer.image(&mut system, key).is_none());
        key.flags.remove(CacheKeyFlags::PIXEL_FONT);
        key.font_size_bits = f32::NAN.to_bits();
        assert!(rasterizer.image(&mut system, key).is_none());
    }

    #[test]
    fn coretext_smoothing_variants_ignore_opacity_and_keep_color_emoji() {
        let (mut system, key) = fixture();
        let white = key.with_color(Color::rgb(255, 255, 255));
        assert_eq!(white, key.with_color(Color::rgba(255, 255, 255, 128)));
        assert_eq!(key.with_color(Color::rgb(0, 0, 0)).dilation, 0);
        let mut rasterizer = Rasterizer::default();
        let black = rasterizer.image(&mut system, key).unwrap();
        let light = rasterizer.image(&mut system, white).unwrap();
        assert!(
            light.data.iter().map(|a| u64::from(*a)).sum::<u64>()
                >= black.data.iter().map(|a| u64::from(*a)).sum::<u64>()
        );

        system
            .db_mut()
            .load_font_file("/System/Library/Fonts/Apple Color Emoji.ttc")
            .unwrap();
        let id = system
            .db()
            .faces()
            .find(|face| face.post_script_name == "AppleColorEmoji")
            .unwrap()
            .id;
        let font = system.get_font(id, fontdb::Weight::NORMAL).unwrap();
        let glyph = font.as_swash().charmap().map('🙂');
        let (mut emoji_key, _, _) = CacheKey::new(
            id,
            glyph,
            40.0,
            (0.0, 0.0),
            fontdb::Weight::NORMAL,
            CacheKeyFlags::NATIVE_RASTERIZATION,
        );
        emoji_key.optical_size_bits = 20.0_f32.to_bits();
        let emoji = rasterizer.image(&mut system, emoji_key).unwrap();
        assert_eq!(emoji.content, SwashContent::Color);
        assert!(emoji
            .data
            .chunks_exact(4)
            .any(|p| p[3] > 128 && p[0].abs_diff(p[2]) > 20));
    }
}
